//! Problem-family contracts (RFC 027 §11.1).
//!
//! A quadratic program
//!
//! ```text
//! minimize    f(x) = ½ xᵀQx + cᵀx
//! subject to  lo ≤ x ≤ hi
//!             A x ≤ b
//! ```
//!
//! expressed through the RFC 002 access traits. The contract names *what* a
//! problem is, never *how* it is stored: every datum is reached through
//! [`MatrixAccess`] / [`VectorAccess`], so a fixed-size static backend and a
//! dynamic heap backend satisfy the same traits without either importing the
//! other, and nothing here commits to a layout, allocates, or dispatches
//! dynamically.
//!
//! Three component traits carry the three parts of the program, and
//! [`QuadraticProgram`] is implemented automatically for any type that
//! implements all three. A problem with no inequality constraints supplies a
//! constraint matrix with zero rows.
//!
//! What the contract does **not** carry:
//!
//! - **`Q` symmetry and positive semidefiniteness are caller preconditions**, not
//!   checked. Verifying them needs a factorization (RFC 027 §11.5).
//! - **Numeric validation** — finiteness scans of `Q`, `c`, `A`, `b`, and the
//!   bounds, and the row norms a projection needs — belongs to the solving kernel,
//!   where RFC 012's trust policy decides what may be skipped. [`QuadraticProgram::shape`]
//!   checks structure only.
//! - **LP is expressible (`Q = 0`) but not solved** by the projected kernels this
//!   contract feeds: projected gradient on a linear objective has no curvature to
//!   converge against (RFC 027 §11.6).
//!
//! Contracts only — no modeling DSL, no expression parsing (external design §2.7).

use crate::access::{MatrixAccess, VectorAccess, VectorAccessMut, dim_u32};
use crate::error::SolverError;
use crate::scalar::BaseScalar;

/// The objective `f(x) = ½ xᵀQx + cᵀx`.
///
/// `Q` is `n × n` and `c` has length `n`, where `n` is the number of variables.
/// `Q` must be symmetric positive semidefinite; that is the caller's
/// responsibility and is not verified.
pub trait QuadraticObjective<S: BaseScalar> {
    /// Storage for the quadratic term `Q`.
    type Hessian: MatrixAccess<Scalar = S>;
    /// Storage for the linear term `c`.
    type Linear: VectorAccess<Scalar = S>;

    /// The quadratic term `Q` (`n × n`).
    fn hessian(&self) -> &Self::Hessian;

    /// The linear term `c` (length `n`).
    fn linear_term(&self) -> &Self::Linear;

    /// The first-order oracle `∇f(x) = Qx + c`, written into `grad`.
    ///
    /// This is what lets the existing projected-first-order kernels consume a
    /// quadratic program: the gradient is fully determined by `Q` and `c`.
    ///
    /// Each coordinate is accumulated in a fixed order — `cᵢ`, then
    /// `Qᵢⱼ·xⱼ` for ascending `j` — so the result is reproducible for a given
    /// backend and target. It uses only [`BaseScalar`] arithmetic: no division,
    /// no allocation, and fallible element access throughout.
    ///
    /// The oracle computes; it does not judge. A non-finite result is left for the
    /// kernel's hot-loop finiteness check, which RFC 012 never lets a caller skip.
    ///
    /// An implementation with structure to exploit — a sparse `Q`, say — may
    /// override this, and must keep the same result.
    ///
    /// # Errors
    ///
    /// [`SolverError::DimensionMismatch`] when `Q` is not `n × n`, or when `x` or
    /// `grad` does not have length `n`, with `n` taken from `c`. Element-access
    /// errors from the backends propagate unchanged.
    fn gradient_into<X, G>(&self, x: &X, grad: &mut G) -> Result<(), SolverError>
    where
        X: VectorAccess<Scalar = S>,
        G: VectorAccessMut<Scalar = S>,
    {
        let hessian = self.hessian();
        let linear = self.linear_term();
        let n = linear.len();
        let dims = hessian.dims();
        require_len(dims.rows, n)?;
        require_len(dims.cols, n)?;
        require_len(x.len(), n)?;
        require_len(grad.len(), n)?;

        for i in 0..n {
            let mut accumulated = linear.get(i)?;
            for j in 0..n {
                accumulated = accumulated.add(hessian.get(i, j)?.mul(x.get(j)?));
            }
            grad.set(i, accumulated)?;
        }
        Ok(())
    }
}

/// Elementwise box bounds `lo ≤ x ≤ hi`.
///
/// The same shape the RFC 006 and RFC 016 projected-first-order problems
/// already use. One storage type serves both bounds.
pub trait BoxBounds<S: BaseScalar> {
    /// Storage for the lower and upper bounds.
    type Bound: VectorAccess<Scalar = S>;

    /// The lower bound `lo` (length `n`).
    fn lower_bounds(&self) -> &Self::Bound;

    /// The upper bound `hi` (length `n`).
    fn upper_bounds(&self) -> &Self::Bound;
}

/// Linear inequality constraints `A x ≤ b`.
///
/// `A` is `m × n` and `b` has length `m`. A problem with no inequality
/// constraints supplies `m = 0`: a matrix with zero rows and an empty `b`.
pub trait LinearInequalities<S: BaseScalar> {
    /// Storage for the constraint matrix `A`.
    type Constraints: MatrixAccess<Scalar = S>;
    /// Storage for the right-hand side `b`.
    type Rhs: VectorAccess<Scalar = S>;

    /// The constraint matrix `A` (`m × n`).
    fn constraint_matrix(&self) -> &Self::Constraints;

    /// The right-hand side `b` (length `m`).
    fn constraint_rhs(&self) -> &Self::Rhs;
}

/// The size of a quadratic program, from [`QuadraticProgram::shape`].
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ProgramShape {
    /// The number of variables `n`.
    pub variables: usize,
    /// The number of linear inequality constraints `m` (possibly zero).
    pub constraints: usize,
}

/// A complete quadratic program: objective, box bounds, and linear inequalities.
///
/// Implemented automatically for every type that implements
/// [`QuadraticObjective`], [`BoxBounds`], and [`LinearInequalities`]; it is never
/// implemented by hand.
pub trait QuadraticProgram<S: BaseScalar>:
    QuadraticObjective<S> + BoxBounds<S> + LinearInequalities<S>
{
    /// Check that every part agrees on the program's size, and return it.
    ///
    /// The number of variables `n` is taken from the linear term `c`, and must be
    /// non-zero. Then `Q` is `n × n`, both bounds have length `n`, and `b` has
    /// length `m` where `m` is the row count of `A`. `A` must have `n` columns
    /// unless it has no rows: an empty constraint set has no column count worth
    /// checking.
    ///
    /// Structure only. No element is read, so this is cheap and never skippable;
    /// finiteness, `lo ≤ hi`, and row norms are the kernel's to validate under
    /// RFC 012's trust policy.
    ///
    /// Because the trait is implemented automatically, this cannot be overridden.
    ///
    /// # Errors
    ///
    /// [`SolverError::InvalidDimension`] when `n` is zero;
    /// [`SolverError::DimensionMismatch`] when any other extent disagrees, with the
    /// actual extent as `lhs` and the expected one as `rhs`.
    fn shape(&self) -> Result<ProgramShape, SolverError> {
        let variables = self.linear_term().len();
        if variables == 0 {
            return Err(SolverError::InvalidDimension);
        }
        let hessian = self.hessian().dims();
        require_len(hessian.rows, variables)?;
        require_len(hessian.cols, variables)?;
        require_len(self.lower_bounds().len(), variables)?;
        require_len(self.upper_bounds().len(), variables)?;

        let constraints = self.constraint_matrix().dims();
        if constraints.rows > 0 {
            require_len(constraints.cols, variables)?;
        }
        require_len(self.constraint_rhs().len(), constraints.rows)?;

        Ok(ProgramShape {
            variables,
            constraints: constraints.rows,
        })
    }
}

impl<S, T> QuadraticProgram<S> for T
where
    S: BaseScalar,
    T: QuadraticObjective<S> + BoxBounds<S> + LinearInequalities<S>,
{
}

/// `actual == expected`, or a [`SolverError::DimensionMismatch`] carrying both.
#[inline]
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

#[cfg(test)]
mod tests;
