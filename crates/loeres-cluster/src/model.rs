//! Typed dynamic projected first-order model (RFC 016).
//!
//! The dynamic-storage analog of the RFC 006 device kernel's problem/workspace
//! surface: a first-order oracle over box bounds ([`ClusterProjectedFirstOrderProblem`]),
//! a reusable single-scratch workspace ([`ClusterProjectedFirstOrderWorkspace`]),
//! the kernel numeric config ([`ProjectedFirstOrderConfig`]), and the typed solve
//! record ([`ProjectedFirstOrderSolveRecord`]). The typed surface is primary; the
//! erased [`ClusterJob`](crate::solve::ClusterJob) adapter lives in `solve`.
//!
//! Server-only (`loeres-cluster`); never reachable from edge crates (zero-bleed).

use loeres::validation::{TrustedByCaller, ValidationScope};
use loeres::{ContiguousVectorAccessMut, FiniteScalar, MetricScalar, SolveReport, SolverError};
use loeres_backend_std::DenseVector;

/// A box/bound-constrained first-order oracle over dynamic dense storage.
///
/// Mirrors the RFC 006 `ProjectedFirstOrderProblem` contract for runtime sizes.
/// `validate_boundary` is an optional problem-specific hook run *in addition to*
/// the kernel-enforced universal checks (it does not own dimension/bound safety).
///
/// **Stability invariants (N1).** For the duration of a single solve, `dimension()`,
/// `bounds()`, and `step_scale()` must return stable values, and `gradient_at`
/// must write all `dimension()` gradient entries. The kernel validates structure
/// once before the loop and re-checks bound/gradient/candidate finiteness every
/// iteration, but it does not re-check finite `lo <= hi` per iteration; an
/// implementation that uses interior mutability to change bounds mid-solve
/// violates this contract.
pub trait ClusterProjectedFirstOrderProblem<S>
where
    S: FiniteScalar + MetricScalar,
{
    /// Number of primal variables (runtime).
    fn dimension(&self) -> usize;

    /// Lower/upper box bounds for projection.
    fn bounds(&self) -> (&DenseVector<S>, &DenseVector<S>);

    /// Gradient `∇f(x)` written into `grad` (all `dimension()` entries) — the
    /// hot-loop oracle.
    fn gradient_at(&self, x: &DenseVector<S>, grad: &mut DenseVector<S>)
    -> Result<(), SolverError>;

    /// Problem-provided step scale `α` (validated finite and `> 0` before the loop).
    fn step_scale(&self) -> S;

    /// Optional problem-specific pre-loop validation hook. Default: `Ok(())`.
    fn validate_boundary(&self) -> Result<(), SolverError> {
        Ok(())
    }
}

/// Reusable single-scratch workspace for the typed solve entrypoint.
///
/// Holds one gradient-scratch vector sized to `dimension`, allocated at
/// construction and reused via [`reset_for_entry`](Self::reset_for_entry). The
/// candidate iterate is computed with scalar temporaries and written into `x` in
/// place, so no second vector and no per-iteration allocation are needed (C4).
#[derive(Clone, Debug)]
pub struct ClusterProjectedFirstOrderWorkspace<S> {
    gradient: DenseVector<S>,
}

impl<S: FiniteScalar + MetricScalar> ClusterProjectedFirstOrderWorkspace<S> {
    /// Allocate scratch sized to `dimension`.
    ///
    /// # Errors
    /// [`SolverError::InvalidDimension`] when `dimension == 0`.
    pub fn new(dimension: usize) -> Result<Self, SolverError> {
        if dimension == 0 {
            return Err(SolverError::InvalidDimension);
        }
        let gradient = DenseVector::from_vec(vec![S::zero(); dimension])?;
        Ok(Self { gradient })
    }

    /// The dimension this workspace was sized for.
    #[must_use]
    pub fn dimension(&self) -> usize {
        self.gradient.len()
    }

    /// Reset scratch for a fresh solve (overwrite-on-use; no allocation).
    pub fn reset_for_entry(&mut self) {
        if let Some(slice) = self.gradient.as_contiguous_mut() {
            for v in slice.iter_mut() {
                *v = S::zero();
            }
        }
    }

    /// Crate-internal mutable scratch access for the kernel oracle call.
    pub(crate) fn gradient_mut(&mut self) -> &mut DenseVector<S> {
        &mut self.gradient
    }

    /// Crate-internal scratch read access for the projected step.
    pub(crate) fn gradient(&self) -> &DenseVector<S> {
        &self.gradient
    }
}

/// Kernel numeric configuration (orthogonal to the RFC 008 orchestration config).
#[derive(Clone, Copy, Debug)]
pub struct ProjectedFirstOrderConfig<S> {
    /// Maximum projected-step iterations (`> 0`).
    pub max_iterations: u32,
    /// Convergence tolerance on the max coordinate step (finite, `> 0`).
    pub tolerance: S,
}

impl<S: FiniteScalar + MetricScalar> ProjectedFirstOrderConfig<S> {
    /// Validate the numeric configuration.
    ///
    /// # Errors
    /// [`SolverError::InvalidInput`] for `max_iterations == 0` or non-positive
    /// `tolerance`; [`SolverError::NonFiniteInput`] for non-finite `tolerance`.
    pub fn validate(&self) -> Result<(), SolverError> {
        if self.max_iterations == 0 {
            return Err(SolverError::InvalidInput);
        }
        if !self.tolerance.is_finite() {
            return Err(SolverError::NonFiniteInput);
        }
        if self.tolerance <= S::zero() {
            return Err(SolverError::InvalidInput);
        }
        Ok(())
    }
}

/// How the kernel's finite invariant was discharged this run (RFC 016 §7,
/// v0.14.1). The three states are named directly so the record never misencodes
/// a trusted-away scan as `FiniteCoverage::NotApplicable` (which RFC 012 reserves
/// for finite-incapable domains) nor as `Checked` (which would claim a scan that
/// did not run). Trust evidence stays RFC 012's `TrustedByCaller`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectedFirstOrderFiniteEvidence {
    /// The kernel ran the finite scans (bounds + initial iterate) and they passed.
    Scanned,
    /// The caller transferred responsibility for finiteness (RFC 012 evidence);
    /// the pre-loop finite scans for the asserted scope were skipped. The
    /// non-skippable hot-loop finiteness checks still ran.
    Trusted(TrustedByCaller),
    /// Reserved: non-finite values are impossible by the scalar's domain/type
    /// contract. Not produced by this `f64` / `FiniteScalar` kernel in v1.
    DomainInapplicable,
}

/// Typed solve outcome: the terminal report plus honest validation evidence
/// (RFC 016 §7, v0.14.1).
///
/// `checked_scope` always includes `PROBLEM_CONFIG` (the universal structural
/// checks that always run), and includes `FINITE` only when the kernel actually
/// scanned it. `finite` names how the finite invariant was discharged; caller
/// trust lives inside `finite` as RFC 012's `TrustedByCaller`, so there is no
/// parallel trust model. RFC 015 decides later what is cacheable.
#[derive(Clone, Copy, Debug)]
pub struct ProjectedFirstOrderSolveRecord {
    /// Terminal report (RFC 014).
    pub report: SolveReport,
    /// Structural/finite scopes the kernel verified this run.
    pub checked_scope: ValidationScope,
    /// How the finite invariant was discharged.
    pub finite: ProjectedFirstOrderFiniteEvidence,
}

#[cfg(test)]
mod tests;
