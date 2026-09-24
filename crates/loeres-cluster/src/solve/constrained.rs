//! RFC 027 S3 — the box/linear-inequality constrained projected first-order
//! cluster kernel and its thin `ClusterJob` adapter.
//!
//! The same algorithm as the device kernel (`loeres-device`'s
//! `solve_constrained_projected_first_order`), over runtime-sized storage:
//! bounded Dykstra over `m + 1` sets (RFC 027 Amendment 3, §0.3.1) — each
//! halfspace `{aᵢᵀx ≤ bᵢ}` is its own set whose increment, always parallel to
//! `aᵢ`, is the scalar multiplier `λᵢ`; the box is the one remaining set, exact
//! `clamp` with its own `n`-length increment. There is no polyhedron-level
//! increment vector. Multipliers persist for the whole projection call (§0.3.2),
//! and the projection converges only when the iterate change, the multiplier
//! change and the terminal constraint violation are all within
//! `projection_tolerance` in the same sweep (§0.3.3).
//!
//! Consumes [`loeres::QuadraticProgram`] directly through the RFC 002 access
//! traits, so `A` may be dense or CSR without this module knowing which; every
//! access is the fallible `get`, no contiguous fast path is required or used.
//! `step_scale` is its own parameter, since the contract carries none (§0.2.1).
//!
//! `m = 0` is accepted at runtime — a zero-row `MatrixAccess`, canonically
//! core's `MatrixView` over an empty slice (§0.2.2) — and short-circuits to the
//! single exact box projection with no Dykstra sweep, so it performs precisely
//! the RFC 016 step (§0.2.3). It additionally validates `step_scale` against
//! `2/L` (RFC 032), so a provably divergent step is rejected where RFC 016 would
//! run to its cap.
//!
//! Validation per RFC 012/016: structural checks always run; finite scans of
//! `Q, c, A, b, lo, hi, x₀` are skippable under `TrustedByCaller(FINITE)`;
//! in-loop finiteness is never skippable and maps to
//! [`SolverError::NumericalDomain`].

use loeres::validation::ValidationScope;
use loeres::{
    ContiguousVectorAccessMut, DivisibleScalar, FiniteScalar, MatrixAccess, MetricScalar,
    QuadraticProgram, SolveReport, SolverError, VectorAccess, VectorAccessMut,
};
use loeres_backend_std::DenseVector;

use crate::batch::{BatchItemOutcome, ClusterSolution};
use crate::model::ProjectedFirstOrderFiniteEvidence;
use crate::runtime::ClusterValidationPolicy;
use crate::solve::{ClusterExecutionContext, ClusterJob};

fn dim_u32(n: usize) -> Result<u32, SolverError> {
    u32::try_from(n).map_err(|_| SolverError::InvalidDimension)
}

fn require_len(actual: usize, expected: usize) -> Result<(), SolverError> {
    if actual == expected {
        Ok(())
    } else {
        Err(SolverError::DimensionMismatch {
            lhs: dim_u32(actual)?,
            rhs: dim_u32(expected)?,
        })
    }
}

fn slot<S: Copy>(values: &[S], index: usize) -> Result<S, SolverError> {
    values
        .get(index)
        .copied()
        .ok_or(SolverError::InternalInvariantViolation)
}

fn set_slot<S>(values: &mut [S], index: usize, value: S) -> Result<(), SolverError> {
    match values.get_mut(index) {
        Some(target) => {
            *target = value;
            Ok(())
        }
        None => Err(SolverError::InternalInvariantViolation),
    }
}

/// Numeric configuration for the constrained cluster kernel, orthogonal to the
/// RFC 008 orchestration config. Mirrors [`crate::ProjectedFirstOrderConfig`]
/// (both tolerances finite and `> 0`) and adds the inner projection bounds.
#[derive(Clone, Copy, Debug)]
pub struct ConstrainedProjectedConfig<S> {
    /// Maximum outer gradient iterations (`> 0`).
    pub max_iterations: u32,
    /// Outer convergence tolerance on the max coordinate change of the whole
    /// outer step (gradient move plus projection); finite, `> 0`.
    pub tolerance: S,
    /// Maximum Dykstra sweeps per outer-iteration projection (`> 0`).
    ///
    /// **The cap must suit `projection_tolerance`.** Dykstra converges linearly
    /// at a rate set by the angles between constraint normals, so tight
    /// tolerances need far more sweeps than loose ones (roughly 2700 at `1e-12`
    /// on the review-054 polytope). There is no default; a cap that is too small
    /// is not an error, it is reported as `projection_cap_hits > 0` and a
    /// non-zero `max_constraint_violation`.
    pub projection_max_sweeps: u32,
    /// Inner convergence tolerance (finite, `> 0`): a projection is converged
    /// only when, in one sweep, the iterate's change, the multipliers' change
    /// and the terminal constraint violation are all within it (§0.3.3).
    pub projection_tolerance: S,
}

impl<S: FiniteScalar + MetricScalar> ConstrainedProjectedConfig<S> {
    /// Validate the numeric configuration.
    ///
    /// # Errors
    /// [`SolverError::InvalidInput`] for a zero cap or a non-positive tolerance;
    /// [`SolverError::NonFiniteInput`] for a non-finite tolerance.
    pub fn validate(&self) -> Result<(), SolverError> {
        if self.max_iterations == 0 || self.projection_max_sweeps == 0 {
            return Err(SolverError::InvalidInput);
        }
        for tolerance in [self.tolerance, self.projection_tolerance] {
            if !tolerance.is_finite() {
                return Err(SolverError::NonFiniteInput);
            }
            if tolerance <= S::zero() {
                return Err(SolverError::InvalidInput);
            }
        }
        Ok(())
    }
}

/// Reusable scratch for the constrained cluster kernel, sized to `(n, m)` and
/// allocated once at construction; the solve loop never allocates.
///
/// Five buffers, matching the device workspace's `3N + 2M` (RFC 027 §11.4):
/// `gradient`, `outer_previous` and `box_increment` (each `n`), and
/// `multipliers` and `row_norms_sq` (each `m`). `outer_previous` must survive
/// unchanged across every sweep of a projection so the outer criterion can
/// compare the final iterate against it (RFC 016's criterion, unchanged);
/// `gradient`'s role ends once the candidate is formed, so the projection
/// reuses it as the per-sweep "iterate before this sweep" snapshot. `m` may be
/// zero, in which case the two `m`-length buffers are empty.
#[derive(Clone, Debug)]
pub struct ClusterConstrainedWorkspace<S> {
    gradient: DenseVector<S>,
    outer_previous: DenseVector<S>,
    box_increment: DenseVector<S>,
    multipliers: Vec<S>,
    row_norms_sq: Vec<S>,
}

impl<S: FiniteScalar + MetricScalar> ClusterConstrainedWorkspace<S> {
    /// Allocate scratch for `variables = n` and `constraints = m`.
    ///
    /// # Errors
    /// [`SolverError::InvalidDimension`] when `variables == 0`. `constraints`
    /// may be zero.
    pub fn new(variables: usize, constraints: usize) -> Result<Self, SolverError> {
        if variables == 0 {
            return Err(SolverError::InvalidDimension);
        }
        let vector = || DenseVector::from_vec(vec![S::zero(); variables]);
        Ok(Self {
            gradient: vector()?,
            outer_previous: vector()?,
            box_increment: vector()?,
            multipliers: vec![S::zero(); constraints],
            row_norms_sq: vec![S::zero(); constraints],
        })
    }

    /// The number of variables this workspace was sized for.
    #[must_use]
    pub fn variables(&self) -> usize {
        self.gradient.len()
    }

    /// The number of constraints this workspace was sized for.
    #[must_use]
    pub fn constraints(&self) -> usize {
        self.multipliers.len()
    }

    fn reset_for_entry(&mut self) {
        for vector in [
            &mut self.gradient,
            &mut self.outer_previous,
            &mut self.box_increment,
        ] {
            zero_dense(vector);
        }
        for value in self
            .multipliers
            .iter_mut()
            .chain(self.row_norms_sq.iter_mut())
        {
            *value = S::zero();
        }
    }
}

fn zero_dense<S: FiniteScalar>(vector: &mut DenseVector<S>) {
    // `DenseVector` is always contiguous, so this always runs.
    if let Some(slice) = vector.as_contiguous_mut() {
        for value in slice.iter_mut() {
            *value = S::zero();
        }
    }
}

/// Typed solve outcome: the terminal report, honest validation evidence (as
/// [`crate::ProjectedFirstOrderSolveRecord`]), and the two fields RFC 027
/// §11.3 requires so the kernel never claims feasibility it did not verify.
#[derive(Clone, Copy, Debug)]
pub struct ConstrainedSolveRecord<S> {
    /// Terminal report (RFC 014).
    pub report: SolveReport,
    /// Outer iterations whose Dykstra projection hit `projection_max_sweeps`
    /// without converging.
    pub projection_cap_hits: u32,
    /// `max(0, maxᵢ(aᵢᵀx − bᵢ))` at the returned iterate (zero when `m = 0`;
    /// box violation is zero by construction).
    pub max_constraint_violation: S,
    /// Structural/finite scopes verified directly.
    pub checked_scope: ValidationScope,
    /// How the finite invariant was discharged.
    pub finite: ProjectedFirstOrderFiniteEvidence,
}

fn scan_vector<V>(vector: &V) -> Result<(), SolverError>
where
    V: VectorAccess,
    V::Scalar: FiniteScalar,
{
    for index in 0..vector.len() {
        if !vector.get(index)?.is_finite() {
            return Err(SolverError::NonFiniteInput);
        }
    }
    Ok(())
}

fn scan_matrix<M>(matrix: &M) -> Result<(), SolverError>
where
    M: MatrixAccess,
    M::Scalar: FiniteScalar,
{
    let dims = matrix.dims();
    for row in 0..dims.rows {
        for col in 0..dims.cols {
            if !matrix.get(row, col)?.is_finite() {
                return Err(SolverError::NonFiniteInput);
            }
        }
    }
    Ok(())
}

fn poll_cancelled(ctx: &ClusterExecutionContext, index: u32) -> bool {
    let poll = ctx.poll_interval();
    (poll == 0 || index % poll == 0) && ctx.is_cancelled()
}

/// `max(0, maxᵢ(aᵢᵀx − bᵢ))`.
fn max_constraint_violation<P, S>(
    problem: &P,
    x: &DenseVector<S>,
    m: usize,
) -> Result<S, SolverError>
where
    P: QuadraticProgram<S>,
    S: FiniteScalar + MetricScalar,
{
    let a = problem.constraint_matrix();
    let b = problem.constraint_rhs();
    let mut worst = S::zero();
    for i in 0..m {
        let mut dot = S::zero();
        for j in 0..x.len() {
            dot = dot.add(a.get(i, j)?.mul(x.get(j)?));
        }
        let violation = dot.sub(b.get(i)?);
        if !violation.is_finite() {
            return Err(SolverError::NumericalDomain);
        }
        worst = worst.max(violation);
    }
    Ok(worst.max(S::zero()))
}

/// Bounded Dykstra projection `x ← Π_C(x)` for `C = {lo ≤ x ≤ hi} ∩ {Ax ≤ b}`
/// with `m ≥ 1` (RFC 027 §11.2, §11.3, Amendment 3). Resets the box increment
/// and multipliers once per call. Returns whether the sweep cap bound without
/// convergence. See the device kernel's `dykstra_project` for the derivation of
/// each step; `workspace.gradient` is the per-sweep snapshot here as there.
fn dykstra_project<P, S>(
    problem: &P,
    x: &mut DenseVector<S>,
    workspace: &mut ClusterConstrainedWorkspace<S>,
    config: &ConstrainedProjectedConfig<S>,
    ctx: &ClusterExecutionContext,
    m: usize,
) -> Result<Projection, SolverError>
where
    P: QuadraticProgram<S>,
    S: FiniteScalar + MetricScalar + DivisibleScalar,
{
    let n = x.len();
    zero_dense(&mut workspace.box_increment);
    for value in workspace.multipliers.iter_mut() {
        *value = S::zero();
    }

    let lo = problem.lower_bounds();
    let hi = problem.upper_bounds();
    let a = problem.constraint_matrix();
    let b = problem.constraint_rhs();
    let tolerance = config.projection_tolerance;

    // RFC 034: two scalar snapshots of `max|λ|`, no copy of the multipliers.
    let midpoint_sweep = config.projection_max_sweeps / 2;
    let final_sweep = config.projection_max_sweeps - 1;
    let mut midpoint_multiplier = S::zero();
    let mut final_multiplier = S::zero();

    for sweep in 0..config.projection_max_sweeps {
        if poll_cancelled(ctx, sweep) {
            return Err(SolverError::Cancelled);
        }
        for j in 0..n {
            workspace.gradient.set(j, x.get(j)?)?;
        }

        // One cyclic Hildreth pass over the halfspaces.
        let mut lambda_change = S::zero();
        for i in 0..m {
            let mut dot = S::zero();
            for j in 0..n {
                dot = dot.add(a.get(i, j)?.mul(x.get(j)?));
            }
            let bi = b.get(i)?;
            if !dot.is_finite() || !bi.is_finite() {
                return Err(SolverError::NumericalDomain);
            }
            let residual = dot.sub(bi);
            let step = residual.checked_div(slot(&workspace.row_norms_sq, i)?)?;
            let old_lambda = slot(&workspace.multipliers, i)?;
            let new_lambda = old_lambda.add(step).max(S::zero());
            let diff = new_lambda.sub(old_lambda);
            lambda_change = lambda_change.max(diff.abs());
            for j in 0..n {
                let adjusted = x.get(j)?.sub(diff.mul(a.get(i, j)?));
                if !adjusted.is_finite() {
                    return Err(SolverError::NumericalDomain);
                }
                x.set(j, adjusted)?;
            }
            set_slot(&mut workspace.multipliers, i, new_lambda)?;
        }

        if sweep == midpoint_sweep {
            midpoint_multiplier = largest_multiplier(&workspace.multipliers);
        }
        if sweep == final_sweep {
            final_multiplier = largest_multiplier(&workspace.multipliers);
        }

        // Box correction: target = x + box_increment(old), exact clamp.
        let mut change = S::zero();
        for j in 0..n {
            let (loj, hij) = (lo.get(j)?, hi.get(j)?);
            if !loj.is_finite() || !hij.is_finite() {
                return Err(SolverError::NumericalDomain);
            }
            let target = x.get(j)?.add(workspace.box_increment.get(j)?);
            let projected = target.clamp(loj, hij);
            if !projected.is_finite() {
                return Err(SolverError::NumericalDomain);
            }
            workspace.box_increment.set(j, target.sub(projected))?;
            x.set(j, projected)?;
            change = change.max(projected.sub(workspace.gradient.get(j)?).abs());
        }

        if change.lte_tolerance(tolerance)
            && lambda_change.lte_tolerance(tolerance)
            && max_constraint_violation(problem, x, m)?.lte_tolerance(tolerance)
        {
            return Ok(Projection {
                capped: false,
                multipliers_diverging: false,
            });
        }
    }
    Ok(Projection {
        capped: true,
        multipliers_diverging: multipliers_are_diverging(midpoint_multiplier, final_multiplier),
    })
}

/// What one Dykstra projection reports (private; RFC 033, RFC 034).
#[derive(Copy, Clone)]
struct Projection {
    /// The sweep cap bound before the projection converged (RFC 033).
    capped: bool,
    /// RFC 034 condition 2: `max|λ|` at the final sweep is at least 1.5 times its
    /// value at the midpoint sweep — the multipliers grow linearly, which they do
    /// on an infeasible system and do not on a feasible one, however slowly it
    /// converges. Only meaningful when `capped`.
    multipliers_diverging: bool,
}

/// `max|λ|` over the multipliers.
fn largest_multiplier<S: MetricScalar>(multipliers: &[S]) -> S {
    multipliers
        .iter()
        .fold(S::zero(), |largest, &value| largest.max(value.abs()))
}

/// RFC 034 condition 2: `final ≥ 1.5 × midpoint`, written `2·final ≥ 3·midpoint`
/// so it needs no division and no conversion from a float.
///
/// **The factor is 1.5, not 2; do not "tidy" it.** Linear divergence from zero
/// gives a ratio approaching exactly 2 from below (`1.99889` at 200 sweeps on the
/// three-halfspace cycle), so a threshold *at* 2 misses real cases, while feasible
/// cases sit at `0.976` to `1.000`. Raising it to 2 is a regression.
fn multipliers_are_diverging<S: MetricScalar>(midpoint: S, last: S) -> bool {
    let one = S::one();
    let two = one.add(one);
    let three = two.add(one);
    two.mul(last) >= three.mul(midpoint)
}

/// The report for a *stationary* outer step, which is where RFC 027 §0.5.1,
/// RFC 033 and RFC 034 decide between the three outcomes:
///
/// - **`converged`** when the iterate is feasible and the final projection was
///   exact (not capped);
/// - **`Infeasible`** (RFC 034) only when **all three** hold — the final
///   projection capped, its multipliers were diverging (`max|λ|` at the final
///   sweep ≥ 1.5 × at the midpoint), **and** the violation exceeds
///   `projection_tolerance`. The third condition is what excludes every feasible
///   problem whose multipliers are identically zero (a ratio alone reads `0/0`
///   there); dropping it, or the other two, is wrong;
/// - otherwise `NotConverged` with `NoProgress`, unchanged.
fn stationary_report(
    feasible: bool,
    last: Projection,
    executed: u32,
    converged: SolveReport,
) -> SolveReport {
    if feasible && !last.capped {
        converged
    } else if !feasible && last.capped && last.multipliers_diverging {
        SolveReport::infeasible(executed)
    } else {
        SolveReport::not_converged_stalled(executed)
    }
}

/// Solve a dynamic box/linear-inequality constrained projected first-order
/// problem (RFC 027 §11.3).
///
/// `x` is the in/out iterate. Converged and not-converged both return `Ok`;
/// fail-safe failures return `Err`; cancellation returns
/// [`SolverError::Cancelled`]. An infeasible polyhedron is not special-cased: it
/// runs each projection to `projection_max_sweeps` and reports its true
/// violation with `projection_cap_hits > 0`. `Converged` requires three things
/// together: the iterate is feasible (RFC 027 §0.5.1, violation within
/// `projection_tolerance`), the outer step is stationary, and the final outer
/// iteration's projection returned without hitting `projection_max_sweeps`
/// (RFC 033). A stationary outer step that fails the first or the third reports
/// `NotConverged` with `NoProgress`. `projection_cap_hits` still counts every
/// capped projection; only the final one decides the status.
///
/// # Errors
/// Structural/validation failures per RFC 016 §3.7, plus
/// [`SolverError::InvalidInput`] for an all-zero constraint row and
/// [`SolverError::Overflow`] for a constraint row whose squared norm overflows;
/// in-loop non-finite values map to [`SolverError::NumericalDomain`].
///
/// # What this does not claim (RFC 027 §11.6)
///
/// - LP is expressible (`Q = 0`) but not solved.
/// - Infeasibility is not detected: it is reported as `NotConverged` with
///   `NoProgress`, with `projection_cap_hits > 0` and a positive violation that
///   does not shrink as `projection_max_sweeps` is raised. It is never an error.
/// - The projection is inexact by design. Dykstra converges linearly at a rate
///   set by the angles between constraint normals; nearly parallel constraints
///   can make the inner cap bind routinely. `projection_max_sweeps` has no
///   default and must suit `projection_tolerance`.
/// - The step is bounded, not chosen for you (RFC 032). For symmetric positive
///   semidefinite `Q`: `step_scale ≥ 2/L` (`L = maxᵢ Qᵢᵢ ≤ λ_max`) is provably
///   divergent and is rejected as [`SolverError::InvalidInput`];
///   `step_scale < 2/U` (`U = maxᵢ Σⱼ|Qᵢⱼ| ≥ λ_max`) is provably convergent; the
///   band `2/U ≤ step_scale < 2/L` is **accepted and no claim is made**. See
///   `QuadraticProgram::curvature_bounds` and `suggested_step_scale`. No numeric
///   convergence rate is claimed.
/// - `Q` symmetric positive semidefinite is a caller precondition, not verified.
/// - Device and cluster results agree within tolerance, not bitwise (RFC 013).
pub fn solve_constrained_projected_first_order_dyn<P, S>(
    problem: &P,
    step_scale: S,
    x: &mut DenseVector<S>,
    workspace: &mut ClusterConstrainedWorkspace<S>,
    config: &ConstrainedProjectedConfig<S>,
    ctx: &ClusterExecutionContext,
) -> Result<ConstrainedSolveRecord<S>, SolverError>
where
    P: QuadraticProgram<S>,
    S: FiniteScalar + MetricScalar + DivisibleScalar,
{
    if ctx.is_cancelled() {
        return Err(SolverError::Cancelled);
    }
    config.validate()?;
    workspace.reset_for_entry();

    // (a) Structural checks — always run, never skippable.
    let shape = problem.shape()?;
    let (n, m) = (shape.variables, shape.constraints);
    require_len(x.len(), n)?;
    require_len(workspace.variables(), n)?;
    require_len(workspace.constraints(), m)?;
    if !step_scale.is_finite() {
        return Err(SolverError::NonFiniteInput);
    }
    if step_scale <= S::zero() {
        return Err(SolverError::InvalidInput);
    }

    // (b) Finite scans — policy-governed.
    let finite_evidence = match ctx.validation_policy() {
        ClusterValidationPolicy::TrustedByCaller(t)
            if t.scope.contains(ValidationScope::FINITE) =>
        {
            ProjectedFirstOrderFiniteEvidence::Trusted(t)
        }
        _ => ProjectedFirstOrderFiniteEvidence::Scanned,
    };
    let finite_trusted = matches!(
        finite_evidence,
        ProjectedFirstOrderFiniteEvidence::Trusted(_)
    );
    if !finite_trusted {
        scan_vector(problem.linear_term())?;
        scan_matrix(problem.hessian())?;
        scan_vector(problem.lower_bounds())?;
        scan_vector(problem.upper_bounds())?;
        if m > 0 {
            scan_matrix(problem.constraint_matrix())?;
            scan_vector(problem.constraint_rhs())?;
        }
        scan_vector(x)?;
    }

    // RFC 032: a step at or above 2/L, with L = max diag(Q) <= lambda_max, is
    // provably divergent. The indeterminate band 2/U <= step < 2/L is NOT
    // rejected. Written as `step * L >= 2` (the same test for L > 0, no division;
    // never true for L = 0). An O(n) scan; a non-finite diagonal under trust makes
    // the comparison false and is left to the hot loop, as before this rule.
    {
        let hessian = problem.hessian();
        let mut largest_diagonal = S::zero();
        for j in 0..n {
            largest_diagonal = largest_diagonal.max(hessian.get(j, j)?);
        }
        if step_scale.mul(largest_diagonal) >= S::one().add(S::one()) {
            return Err(SolverError::InvalidInput);
        }
    }

    // Structural: only a *finite* lo > hi is InvalidInput; non-finite bounds
    // under trust reach the hot-loop check (NumericalDomain).
    {
        let (lo, hi) = (problem.lower_bounds(), problem.upper_bounds());
        for j in 0..n {
            let (l, h) = (lo.get(j)?, hi.get(j)?);
            if l.is_finite() && h.is_finite() && l > h {
                return Err(SolverError::InvalidInput);
            }
        }
    }

    // Row norms: validated `> 0` and finite, overflow → Overflow; an all-zero
    // row is InvalidInput (a zero-*row matrix*, m = 0, is valid and skips this).
    {
        let a = problem.constraint_matrix();
        for i in 0..m {
            let mut sum_sq = S::zero();
            for j in 0..n {
                let aij = a.get(i, j)?;
                sum_sq = sum_sq.add(aij.mul(aij));
            }
            if sum_sq.is_nan() {
                return Err(SolverError::NumericalDomain);
            }
            if sum_sq.is_infinite() {
                return Err(SolverError::Overflow);
            }
            if sum_sq <= S::zero() {
                return Err(SolverError::InvalidInput);
            }
            set_slot(&mut workspace.row_norms_sq, i, sum_sq)?;
        }
    }

    let checked_scope = if finite_trusted {
        ValidationScope::PROBLEM_CONFIG
    } else {
        ValidationScope::PROBLEM_CONFIG.union(ValidationScope::FINITE)
    };

    let mut projection_cap_hits: u32 = 0;
    let mut executed: u32 = 0;
    while executed < config.max_iterations {
        if poll_cancelled(ctx, executed) {
            return Err(SolverError::Cancelled);
        }
        for j in 0..n {
            workspace.outer_previous.set(j, x.get(j)?)?;
        }
        problem.gradient_into(x, &mut workspace.gradient)?;

        // What *this* iteration's projection reported (RFC 033: capped; RFC 034:
        // multipliers diverging); read only at the early exit, where this is the
        // final iteration. `m = 0` has no inner projection, so it never caps.
        let mut last_projection = Projection {
            capped: false,
            multipliers_diverging: false,
        };
        if m == 0 {
            // §0.2.3: the single exact box projection, no sweep — the RFC 016
            // step, operation for operation.
            let (lo, hi) = (problem.lower_bounds(), problem.upper_bounds());
            for j in 0..n {
                let (gj, loj, hij) = (workspace.gradient.get(j)?, lo.get(j)?, hi.get(j)?);
                if !gj.is_finite() || !loj.is_finite() || !hij.is_finite() {
                    return Err(SolverError::NumericalDomain);
                }
                let projected = x.get(j)?.sub(step_scale.mul(gj)).clamp(loj, hij);
                if !projected.is_finite() {
                    return Err(SolverError::NumericalDomain);
                }
                x.set(j, projected)?;
            }
        } else {
            for j in 0..n {
                let gj = workspace.gradient.get(j)?;
                let candidate = x.get(j)?.sub(step_scale.mul(gj));
                if !gj.is_finite() || !candidate.is_finite() {
                    return Err(SolverError::NumericalDomain);
                }
                x.set(j, candidate)?;
            }
            last_projection = dykstra_project(problem, x, workspace, config, ctx, m)?;
            if last_projection.capped {
                projection_cap_hits += 1;
            }
        }

        executed += 1;
        let mut change = S::zero();
        for j in 0..n {
            change = change.max(x.get(j)?.sub(workspace.outer_previous.get(j)?).abs());
        }
        if change.lte_tolerance(config.tolerance) {
            // RFC 027 §0.5.1: a stationary outer step is `Converged` only at a
            // feasible iterate; a capped projection has a fixed point even when
            // the polyhedron is empty. RFC 033: and only when the projection that
            // produced it was not capped, since a capped projection returns a
            // feasible point that need not be *the* projection.
            let violation = max_constraint_violation(problem, x, m)?;
            let report = stationary_report(
                violation.lte_tolerance(config.projection_tolerance),
                last_projection,
                executed,
                SolveReport::converged_early(executed),
            );
            return Ok(ConstrainedSolveRecord {
                report,
                projection_cap_hits,
                max_constraint_violation: violation,
                checked_scope,
                finite: finite_evidence,
            });
        }
    }
    Ok(ConstrainedSolveRecord {
        report: SolveReport::not_converged_cap(config.max_iterations),
        projection_cap_hits,
        max_constraint_violation: max_constraint_violation(problem, x, m)?,
        checked_scope,
        finite: finite_evidence,
    })
}

/// A `&self`-safe template adapter erasing a constrained solve into a
/// [`ClusterJob`], as [`crate::ClusterProjectedFirstOrderJob`] does: it holds
/// only immutable inputs, and each `run_boxed` allocates a local iterate clone
/// and workspace once, before the loop.
///
/// The erased [`BatchItemOutcome`] carries only the core [`SolveReport`], so
/// `projection_cap_hits` and `max_constraint_violation` are **not** visible
/// through the batch seam; callers who need them use the typed entrypoint
/// [`solve_constrained_projected_first_order_dyn`].
pub struct ClusterConstrainedJob<P, S> {
    problem: P,
    step_scale: S,
    initial: DenseVector<S>,
    config: ConstrainedProjectedConfig<S>,
}

impl<P, S> ClusterConstrainedJob<P, S> {
    /// Build a job from the problem, its step scale, a starting iterate, and the
    /// numeric config.
    pub fn new(
        problem: P,
        step_scale: S,
        initial: DenseVector<S>,
        config: ConstrainedProjectedConfig<S>,
    ) -> Self {
        Self {
            problem,
            step_scale,
            initial,
            config,
        }
    }
}

impl<P, S> ClusterJob<S> for ClusterConstrainedJob<P, S>
where
    P: QuadraticProgram<S> + Send + Sync + 'static,
    S: FiniteScalar + MetricScalar + DivisibleScalar + Send + Sync + 'static,
{
    fn run_boxed(&self, ctx: &ClusterExecutionContext) -> BatchItemOutcome<S> {
        let mut x = self.initial.clone();
        let shape = match self.problem.shape() {
            Ok(shape) => shape,
            Err(error) => return BatchItemOutcome::Failed { error },
        };
        let mut workspace =
            match ClusterConstrainedWorkspace::new(shape.variables, shape.constraints) {
                Ok(w) => w,
                Err(error) => return BatchItemOutcome::Failed { error },
            };
        match solve_constrained_projected_first_order_dyn(
            &self.problem,
            self.step_scale,
            &mut x,
            &mut workspace,
            &self.config,
            ctx,
        ) {
            Ok(record) => BatchItemOutcome::Solved {
                solution: ClusterSolution::DenseVector(x),
                report: record.report,
            },
            Err(error) => BatchItemOutcome::Failed { error },
        }
    }
}

#[cfg(test)]
mod tests;
