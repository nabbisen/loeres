//! Matrix access traits and the core borrowed row-major matrix views.
//!
//! Layout-agnostic fallible access ([`MatrixAccess`] / [`MatrixAccessMut`]), an
//! optional contiguous row-major fast path ([`ContiguousMatrixAccess`]), and
//! slice-backed reference views ([`MatrixView`] / [`MatrixViewMut`]). The views
//! are **simple contiguous row-major** only; column-major, strided, and
//! sub-matrix views are deferred to the backends (RFC 004 / 007). RFC 002 §3.4,
//! §3.6, §3.8.

use super::dim_u32;
use crate::dimension::{Dim2, DimensionKind};
use crate::error::SolverError;
use crate::scalar::BaseScalar;

/// Read access to a two-dimensional array of scalars, independent of layout.
///
/// A conforming backend may be row-major, column-major, strided, block, or
/// sparse; callers must not infer layout from conformance. Element access is
/// fallible.
pub trait MatrixAccess {
    /// The element type, constrained to the base scalar capability tier.
    type Scalar: BaseScalar;

    /// The shape as a row/column pair.
    fn dims(&self) -> Dim2;

    /// Whether the shape is known at compile time or at run time.
    fn dimension_kind(&self) -> DimensionKind;

    /// The element at `(row, col)`, or [`SolverError::DimensionMismatch`] if the
    /// coordinate is out of range. Bounds are checked row first, then column: a
    /// row violation reports `{ lhs: row, rhs: rows }`, a column violation
    /// reports `{ lhs: col, rhs: cols }`. The payload does not encode which axis
    /// failed; the caller distinguishes that from the access context.
    fn get(&self, row: usize, col: usize) -> Result<Self::Scalar, SolverError>;
}

/// Mutable element access over a [`MatrixAccess`] array.
pub trait MatrixAccessMut: MatrixAccess {
    /// Write `value` at `(row, col)`, under the same bounds contract as
    /// [`MatrixAccess::get`].
    fn set(&mut self, row: usize, col: usize, value: Self::Scalar) -> Result<(), SolverError>;
}

/// Optional fast path: dense matrix storage with a row-major contiguous backing
/// (length `rows * cols`), for kernels such as a Hessian mat-vec.
pub trait ContiguousMatrixAccess: MatrixAccess {
    /// The row-major contiguous backing of length `rows * cols`, or `None` if
    /// storage is not row-major contiguous.
    fn as_row_major(&self) -> Option<&[Self::Scalar]>;
}

/// Validate a row-major backing for the declared shape (RFC 002 §3.6, A1).
///
/// `rows * cols` is computed with overflow checking; an overflow is an invalid
/// declared shape. The slice length must equal that product **exactly** — both
/// undersized and oversized slices are rejected, so a wrong-size backing fails
/// at construction rather than silently becoming a prefix window.
#[inline]
fn validate_row_major(len: usize, rows: usize, cols: usize) -> Result<(), SolverError> {
    let required = rows
        .checked_mul(cols)
        .ok_or(SolverError::InvalidDimension)?;
    if len != required {
        return Err(SolverError::DimensionMismatch {
            lhs: dim_u32(len)?,
            rhs: dim_u32(required)?,
        });
    }
    Ok(())
}

/// Compute the row-major offset for `(row, col)`, applying the §3.4/§5.1 bounds
/// contract (row then column) and checked arithmetic.
///
/// Out-of-range coordinates return [`SolverError::DimensionMismatch`]. Arithmetic
/// overflow cannot occur for a view whose construction invariant holds
/// (`rows * cols` did not overflow and `row < rows`, `col < cols`), so it is
/// surfaced as [`SolverError::InternalInvariantViolation`] — a library bug,
/// failed closed rather than panicked.
#[inline]
fn checked_offset(row: usize, col: usize, rows: usize, cols: usize) -> Result<usize, SolverError> {
    if row >= rows {
        return Err(SolverError::DimensionMismatch {
            lhs: dim_u32(row)?,
            rhs: dim_u32(rows)?,
        });
    }
    if col >= cols {
        return Err(SolverError::DimensionMismatch {
            lhs: dim_u32(col)?,
            rhs: dim_u32(cols)?,
        });
    }
    row.checked_mul(cols)
        .and_then(|base| base.checked_add(col))
        .ok_or(SolverError::InternalInvariantViolation)
}

/// A read-only, contiguous row-major matrix view borrowing a slice.
///
/// Element `(row, col)` maps to `data[row * cols + col]`. Reports
/// [`DimensionKind::Dynamic`].
#[derive(Copy, Clone, Debug)]
pub struct MatrixView<'a, S: BaseScalar> {
    data: &'a [S],
    rows: usize,
    cols: usize,
}

impl<'a, S: BaseScalar> MatrixView<'a, S> {
    /// Borrow `data` as a `rows`×`cols` row-major matrix view.
    ///
    /// The slice length must equal `rows * cols` exactly (overflow-checked);
    /// see [`validate_row_major`]. To view a prefix of a larger buffer, pass
    /// `&data[..rows * cols]` explicitly.
    #[inline]
    pub fn from_row_major(data: &'a [S], rows: usize, cols: usize) -> Result<Self, SolverError> {
        validate_row_major(data.len(), rows, cols)?;
        Ok(Self { data, rows, cols })
    }
}

impl<S: BaseScalar> MatrixAccess for MatrixView<'_, S> {
    type Scalar = S;

    #[inline]
    fn dims(&self) -> Dim2 {
        Dim2::new(self.rows, self.cols)
    }

    #[inline]
    fn dimension_kind(&self) -> DimensionKind {
        DimensionKind::Dynamic
    }

    #[inline]
    fn get(&self, row: usize, col: usize) -> Result<S, SolverError> {
        let offset = checked_offset(row, col, self.rows, self.cols)?;
        match self.data.get(offset) {
            Some(&value) => Ok(value),
            None => Err(SolverError::InternalInvariantViolation),
        }
    }
}

impl<S: BaseScalar> ContiguousMatrixAccess for MatrixView<'_, S> {
    #[inline]
    fn as_row_major(&self) -> Option<&[S]> {
        Some(self.data)
    }
}

/// A mutable, contiguous row-major matrix view borrowing a slice.
///
/// Contiguous and therefore injective: every `(row, col)` maps to a distinct
/// element, so [`set`](MatrixAccessMut::set) cannot alias. Overlapping or
/// broadcast mutable layouts are not part of the core baseline (RFC 002 §4.5).
#[derive(Debug)]
pub struct MatrixViewMut<'a, S: BaseScalar> {
    data: &'a mut [S],
    rows: usize,
    cols: usize,
}

impl<'a, S: BaseScalar> MatrixViewMut<'a, S> {
    /// Borrow `data` as a `rows`×`cols` row-major mutable matrix view, under the
    /// same exact-size validation as [`MatrixView::from_row_major`].
    #[inline]
    pub fn from_row_major_mut(
        data: &'a mut [S],
        rows: usize,
        cols: usize,
    ) -> Result<Self, SolverError> {
        validate_row_major(data.len(), rows, cols)?;
        Ok(Self { data, rows, cols })
    }
}

impl<S: BaseScalar> MatrixAccess for MatrixViewMut<'_, S> {
    type Scalar = S;

    #[inline]
    fn dims(&self) -> Dim2 {
        Dim2::new(self.rows, self.cols)
    }

    #[inline]
    fn dimension_kind(&self) -> DimensionKind {
        DimensionKind::Dynamic
    }

    #[inline]
    fn get(&self, row: usize, col: usize) -> Result<S, SolverError> {
        let offset = checked_offset(row, col, self.rows, self.cols)?;
        match self.data.get(offset) {
            Some(&value) => Ok(value),
            None => Err(SolverError::InternalInvariantViolation),
        }
    }
}

impl<S: BaseScalar> MatrixAccessMut for MatrixViewMut<'_, S> {
    #[inline]
    fn set(&mut self, row: usize, col: usize, value: S) -> Result<(), SolverError> {
        let offset = checked_offset(row, col, self.rows, self.cols)?;
        match self.data.get_mut(offset) {
            Some(slot) => {
                *slot = value;
                Ok(())
            }
            None => Err(SolverError::InternalInvariantViolation),
        }
    }
}

impl<S: BaseScalar> ContiguousMatrixAccess for MatrixViewMut<'_, S> {
    #[inline]
    fn as_row_major(&self) -> Option<&[S]> {
        Some(&*self.data)
    }
}
