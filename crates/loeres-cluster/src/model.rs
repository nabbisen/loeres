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

use loeres::validation::{TrustedByCaller, ValidationCoverage};
use loeres::{ContiguousVectorAccessMut, FiniteScalar, MetricScalar, SolveReport, SolverError};
use loeres_backend_std::DenseVector;

/// A box/bound-constrained first-order oracle over dynamic dense storage.
///
/// Mirrors the RFC 006 `ProjectedFirstOrderProblem` contract for runtime sizes.
/// `validate_boundary` is an optional problem-specific hook run *in addition to*
/// the kernel-enforced universal checks (it does not own dimension/bound safety).
pub trait ClusterProjectedFirstOrderProblem<S>
where
    S: FiniteScalar + MetricScalar,
{
    /// Number of primal variables (runtime).
    fn dimension(&self) -> usize;

    /// Lower/upper box bounds for projection.
    fn bounds(&self) -> (&DenseVector<S>, &DenseVector<S>);

    /// Gradient `∇f(x)` written into `grad` — the hot-loop oracle.
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

/// Typed solve outcome: the terminal report plus the validation evidence.
///
/// `checked` records what the kernel actually verified this run (always the
/// structural `PROBLEM_CONFIG` scope; `FINITE` when scanned). `trust` records a
/// caller responsibility transfer, if any. RFC 012 vocabulary only; RFC 015
/// decides later what is cacheable (C1).
#[derive(Clone, Copy, Debug)]
pub struct ProjectedFirstOrderSolveRecord {
    /// Terminal report (RFC 014).
    pub report: SolveReport,
    /// Coverage the kernel verified this run.
    pub checked: ValidationCoverage,
    /// Caller trust transfer, if any.
    pub trust: Option<TrustedByCaller>,
}

#[cfg(test)]
mod tests;
