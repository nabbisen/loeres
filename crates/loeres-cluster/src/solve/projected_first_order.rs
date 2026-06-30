//! RFC 016 — the first std-side projected first-order kernel: the typed solve
//! entrypoint and the thin `ClusterJob` adapter.
//!
//! Aligned with RFC 006 semantics over dynamic `DenseVector` storage. The typed
//! entrypoint mutates the iterate in place and returns a
//! [`ProjectedFirstOrderSolveRecord`]; the adapter is a `&self`-safe template
//! that allocates a local iterate and workspace per run (F1). Validation runs
//! here (the first real `ValidateAllInputs` scan path) while
//! `ClusterValidationPolicy::resolve` stays pure.

use loeres::validation::ValidationScope;
use loeres::{
    ContiguousVectorAccess, ContiguousVectorAccessMut, FiniteScalar, MetricScalar, SolveReport,
    SolverError, VectorAccess,
};
use loeres_backend_std::DenseVector;

use crate::batch::{BatchItemOutcome, ClusterSolution};
use crate::model::{
    ClusterProjectedFirstOrderProblem, ClusterProjectedFirstOrderWorkspace,
    ProjectedFirstOrderConfig, ProjectedFirstOrderFiniteEvidence, ProjectedFirstOrderSolveRecord,
};
use crate::runtime::ClusterValidationPolicy;
use crate::solve::{ClusterExecutionContext, ClusterJob};

/// Checked `usize -> u32` for `DimensionMismatch` payloads (I7); overflow folds
/// to [`SolverError::InvalidDimension`].
fn dim_u32(n: usize) -> Result<u32, SolverError> {
    u32::try_from(n).map_err(|_| SolverError::InvalidDimension)
}

/// One projected gradient step over the whole iterate, returning the largest
/// per-coordinate change `max_i |x_next[i] - x[i]|` (D8). In-loop non-finite
/// gradient, bound, or candidate maps to [`SolverError::NumericalDomain`]
/// (F4/C3) — never `Solved` over NaN/Inf, even under `TrustedByCaller`.
fn projected_step_dyn<S>(
    x: &mut DenseVector<S>,
    grad: &DenseVector<S>,
    lo: &DenseVector<S>,
    hi: &DenseVector<S>,
    alpha: S,
    n: usize,
) -> Result<S, SolverError>
where
    S: FiniteScalar + MetricScalar,
{
    let grad_s = grad
        .as_contiguous()
        .ok_or(SolverError::InternalInvariantViolation)?;
    let lo_s = lo
        .as_contiguous()
        .ok_or(SolverError::InternalInvariantViolation)?;
    let hi_s = hi
        .as_contiguous()
        .ok_or(SolverError::InternalInvariantViolation)?;
    let x_s = x
        .as_contiguous_mut()
        .ok_or(SolverError::InternalInvariantViolation)?;
    if grad_s.len() != n || lo_s.len() != n || hi_s.len() != n || x_s.len() != n {
        return Err(SolverError::InternalInvariantViolation);
    }

    let mut max_change = S::zero();
    for (((xi, &gi), &loi), &hii) in x_s.iter_mut().zip(grad_s).zip(lo_s).zip(hi_s) {
        if !gi.is_finite() || !loi.is_finite() || !hii.is_finite() {
            return Err(SolverError::NumericalDomain);
        }
        let projected = xi.sub(alpha.mul(gi)).clamp(loi, hii);
        if !projected.is_finite() {
            return Err(SolverError::NumericalDomain);
        }
        let change = projected.sub(*xi).abs();
        *xi = projected;
        max_change = max_change.max(change);
    }
    Ok(max_change)
}

/// Solve a dynamic box/bound-constrained projected first-order problem.
///
/// `x` is the in/out iterate; `workspace` supplies reusable gradient scratch.
/// Converged and not-converged both return `Ok` (the status/error split);
/// fail-safe failures return `Err`. Cancellation returns
/// [`SolverError::Cancelled`], which the RFC 008 executor normalizes to
/// `Cancelled`.
///
/// # Errors
/// Structural/validation failures and in-loop numerical-domain failures per the
/// RFC 016 §3.7 error mapping.
pub fn solve_projected_first_order_dyn<P, S>(
    problem: &P,
    x: &mut DenseVector<S>,
    workspace: &mut ClusterProjectedFirstOrderWorkspace<S>,
    config: &ProjectedFirstOrderConfig<S>,
    ctx: &ClusterExecutionContext,
) -> Result<ProjectedFirstOrderSolveRecord, SolverError>
where
    P: ClusterProjectedFirstOrderProblem<S>,
    S: FiniteScalar + MetricScalar,
{
    // D7: cheap cancellation exit before any scan.
    if ctx.is_cancelled() {
        return Err(SolverError::Cancelled);
    }

    // D6: numeric config.
    config.validate()?;
    workspace.reset_for_entry();

    // (a) Structural checks — always run, never skippable (F3/F6).
    let n = problem.dimension();
    if n == 0 {
        return Err(SolverError::InvalidDimension);
    }
    {
        let (lo, hi) = problem.bounds();
        for len in [x.len(), workspace.dimension(), lo.len(), hi.len()] {
            if len != n {
                return Err(SolverError::DimensionMismatch {
                    lhs: dim_u32(n)?,
                    rhs: dim_u32(len)?,
                });
            }
        }
    }
    let alpha = problem.step_scale();
    if !alpha.is_finite() {
        return Err(SolverError::NonFiniteInput);
    }
    if alpha <= S::zero() {
        return Err(SolverError::InvalidInput);
    }

    // (b) Finite-value scans — policy-governed (C2: step_scale is structural above).
    let finite_evidence = match ctx.validation_policy() {
        ClusterValidationPolicy::TrustedByCaller(t)
            if t.scope.contains(ValidationScope::FINITE) =>
        {
            ProjectedFirstOrderFiniteEvidence::Trusted(t)
        }
        // RespectBackendValidationState has no provided-state channel in v1, so it
        // scans here / fills missing coverage here (B2; provided/cached state is
        // RFC 015-owned). ValidateAllInputs likewise scans. Either way: Scanned.
        _ => ProjectedFirstOrderFiniteEvidence::Scanned,
    };
    let finite_trusted = matches!(
        finite_evidence,
        ProjectedFirstOrderFiniteEvidence::Trusted(_)
    );
    {
        let (lo, hi) = problem.bounds();
        if !finite_trusted {
            lo.validate_finite()?;
            hi.validate_finite()?;
            x.validate_finite()?;
        }
        // Structural ordering: classify only finite lo > hi as InvalidInput (C3).
        // Non-finite bounds were caught above under scanning policies; under
        // trust they propagate to the hot-loop candidate check (NumericalDomain).
        for i in 0..n {
            let l = lo.get(i)?;
            let h = hi.get(i)?;
            if l.is_finite() && h.is_finite() && l > h {
                return Err(SolverError::InvalidInput);
            }
        }
    }

    // Problem-specific hook (F3), after the universal checks.
    problem.validate_boundary()?;

    // checked_scope: PROBLEM_CONFIG always (universal checks ran); FINITE only when
    // the kernel actually scanned it. The finite-discharge mode is in finite_evidence.
    let checked_scope = if finite_trusted {
        ValidationScope::PROBLEM_CONFIG
    } else {
        ValidationScope::PROBLEM_CONFIG.union(ValidationScope::FINITE)
    };

    // Iteration (C4 single-scratch in-place; C5 counting mirrors RFC 006).
    let mut executed: u32 = 0;
    while executed < config.max_iterations {
        let poll = ctx.poll_interval();
        if (poll == 0 || executed % poll == 0) && ctx.is_cancelled() {
            return Err(SolverError::Cancelled);
        }
        problem.gradient_at(x, workspace.gradient_mut())?;
        let change = {
            let (lo, hi) = problem.bounds();
            projected_step_dyn(x, workspace.gradient(), lo, hi, alpha, n)?
        };
        executed += 1;
        if change.lte_tolerance(config.tolerance) {
            return Ok(ProjectedFirstOrderSolveRecord {
                report: SolveReport::converged_early(executed),
                checked_scope,
                finite: finite_evidence,
            });
        }
    }
    Ok(ProjectedFirstOrderSolveRecord {
        report: SolveReport::not_converged_cap(config.max_iterations),
        checked_scope,
        finite: finite_evidence,
    })
}

/// A `&self`-safe template adapter erasing a projected first-order solve into a
/// [`ClusterJob`] (F1). Holds only immutable inputs; each `run_boxed` allocates a
/// local iterate clone and a local workspace once, before the loop — no shared
/// mutable state, no per-iteration allocation.
pub struct ClusterProjectedFirstOrderJob<P, S> {
    problem: P,
    initial: DenseVector<S>,
    config: ProjectedFirstOrderConfig<S>,
}

impl<P, S> ClusterProjectedFirstOrderJob<P, S> {
    /// Build a job from the problem, a starting iterate, and the numeric config.
    pub fn new(problem: P, initial: DenseVector<S>, config: ProjectedFirstOrderConfig<S>) -> Self {
        Self {
            problem,
            initial,
            config,
        }
    }
}

impl<P, S> ClusterJob<S> for ClusterProjectedFirstOrderJob<P, S>
where
    P: ClusterProjectedFirstOrderProblem<S> + Send + Sync + 'static,
    S: FiniteScalar + MetricScalar + Send + Sync + 'static,
{
    fn run_boxed(&self, ctx: &ClusterExecutionContext) -> BatchItemOutcome<S> {
        let mut x = self.initial.clone();
        let mut workspace = match ClusterProjectedFirstOrderWorkspace::new(self.problem.dimension())
        {
            Ok(w) => w,
            Err(error) => return BatchItemOutcome::Failed { error },
        };
        match solve_projected_first_order_dyn(
            &self.problem,
            &mut x,
            &mut workspace,
            &self.config,
            ctx,
        ) {
            Ok(record) => BatchItemOutcome::Solved {
                solution: ClusterSolution::DenseVector(x),
                report: record.report,
            },
            // Ride the RFC 008 F9 normalization (Failed { Cancelled } -> Cancelled).
            Err(SolverError::Cancelled) => BatchItemOutcome::Failed {
                error: SolverError::Cancelled,
            },
            Err(error) => BatchItemOutcome::Failed { error },
        }
    }
}

#[cfg(test)]
mod tests;
