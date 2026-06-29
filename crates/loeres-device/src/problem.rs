//! Fixed-size problem wrappers.
//!
//! The baseline projected first-order problem contract (RFC 006): a
//! box/bound-constrained family exposing a first-order oracle plus static box
//! bounds, built over fixed static storage. Available under the `owned-arrays`
//! feature, since the primal iterate and gradient scratch are fixed-size owned
//! vectors (`FixedVector<S, N>`).

#[cfg(feature = "owned-arrays")]
use loeres::{ContiguousVectorAccess, SolverError};
#[cfg(feature = "owned-arrays")]
use loeres_backend_static::array::FixedVector;

/// A box/bound-constrained problem for the baseline projected first-order device
/// kernel (RFC 006).
///
/// Each iteration needs only a first-order oracle ([`gradient_at`]) and the
/// static box bounds; the kernel performs `x <- clamp(x - alpha * grad, lo, hi)`
/// and stops on small iterate change. The primal iterate and gradient scratch
/// are fixed-size owned vectors (`FixedVector<S, N>`), distinct in type from the
/// read-only bound storage ([`Bounds`](ProjectedFirstOrderProblem::Bounds)) per
/// the implementation-decision pass (I3).
///
/// [`gradient_at`]: ProjectedFirstOrderProblem::gradient_at
#[cfg(feature = "owned-arrays")]
pub trait ProjectedFirstOrderProblem<S, const N: usize> {
    /// Read-only contiguous storage for the lower/upper box bounds.
    type Bounds: ContiguousVectorAccess<Scalar = S>;

    /// Validate problem data before the iteration loop: finite, correctly
    /// dimensioned bounds with `lower <= upper` elementwise. Returns the
    /// appropriate [`SolverError`] (e.g. [`SolverError::InvalidInput`] for an
    /// inverted bound) on failure.
    fn validate_boundary(&self) -> Result<(), SolverError>;

    /// The lower box bound `lo`.
    fn lower_bound(&self) -> &Self::Bounds;

    /// The upper box bound `hi`.
    fn upper_bound(&self) -> &Self::Bounds;

    /// The problem-provided step scale `alpha`.
    ///
    /// Problem-provided rather than config-provided (I6), so the kernel performs
    /// no internal division and stays at the `FiniteScalar + MetricScalar` bound.
    fn step_scale(&self) -> S;

    /// Write the gradient `grad f(x)` into `grad`.
    ///
    /// `x` is the read-only current iterate; `grad` is caller-owned scratch
    /// (the workspace gradient buffer). Returns a [`SolverError`] on a numerical
    /// failure in the oracle.
    fn gradient_at(
        &self,
        x: &FixedVector<S, N>,
        grad: &mut FixedVector<S, N>,
    ) -> Result<(), SolverError>;

    /// The objective value `f(x)`.
    ///
    /// Reporting-only: the baseline iterate-change kernel (I7) does not call this
    /// in the hot loop, and [`DeviceSolveReport`] carries no objective field. It
    /// is part of the contract for callers and for future objective-based
    /// criteria.
    ///
    /// [`DeviceSolveReport`]: crate::solve::DeviceSolveReport
    fn objective_at(&self, x: &FixedVector<S, N>) -> Result<S, SolverError>;
}
