//! Static dimension descriptors and shared dimension-checked access support.
//!
//! Re-exports the core dimension types, records that every type in this crate
//! reports [`DimensionKind::Static`], and provides the bounds-checked element
//! access primitives shared by the owned arrays ([`crate::array`]) and the
//! borrowed static views ([`crate::view`]). The access logic is written once
//! here so both surfaces satisfy the RFC 002 contracts identically, with the
//! same RFC 002 §5.1 / ADR-020 error mapping.

pub use loeres::{Dim2, DimensionKind};

use loeres::{BaseScalar, SolverError};

/// Every owned fixed-size type and const-sized static view in this crate has
/// compile-time extents and therefore reports [`DimensionKind::Static`]. The
/// core borrowed views report `Dynamic`; this crate is the source of `Static`.
pub const STATIC_KIND: DimensionKind = DimensionKind::Static;

/// The [`Dim2`] for a const-generic `R × C` shape.
#[inline]
pub const fn static_dim2<const R: usize, const C: usize>() -> Dim2 {
    Dim2::new(R, C)
}

/// Convert an extent or index into the `u32` error payload **without
/// truncation** (RFC 002 §5.1 / ADR-020): a value that does not fit maps to
/// [`SolverError::InvalidDimension`], never a wrapped payload.
#[inline]
pub(crate) fn dim_u32(value: usize) -> Result<u32, SolverError> {
    u32::try_from(value).map_err(|_| SolverError::InvalidDimension)
}

/// Bounds-checked read of a one-dimensional sequence.
#[inline]
pub(crate) fn vector_get<S: BaseScalar>(data: &[S], index: usize) -> Result<S, SolverError> {
    match data.get(index) {
        Some(&value) => Ok(value),
        None => Err(SolverError::DimensionMismatch {
            lhs: dim_u32(index)?,
            rhs: dim_u32(data.len())?,
        }),
    }
}

/// Bounds-checked write of a one-dimensional sequence.
#[inline]
pub(crate) fn vector_set<S: BaseScalar>(
    data: &mut [S],
    index: usize,
    value: S,
) -> Result<(), SolverError> {
    let len = data.len();
    match data.get_mut(index) {
        Some(slot) => {
            *slot = value;
            Ok(())
        }
        None => Err(SolverError::DimensionMismatch {
            lhs: dim_u32(index)?,
            rhs: dim_u32(len)?,
        }),
    }
}

/// Row-major offset for `(row, col)` under the RFC 002 §5.1 (B1) per-axis bounds
/// contract (row checked first, then column) with checked arithmetic.
///
/// Overflow cannot occur for a backing whose construction invariant holds, so it
/// is surfaced as [`SolverError::InternalInvariantViolation`] — failed closed,
/// never panicked.
#[inline]
pub(crate) fn checked_offset(
    row: usize,
    col: usize,
    rows: usize,
    cols: usize,
) -> Result<usize, SolverError> {
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

/// Bounds-checked row-major read.
#[inline]
pub(crate) fn matrix_get<S: BaseScalar>(
    data: &[S],
    row: usize,
    col: usize,
    rows: usize,
    cols: usize,
) -> Result<S, SolverError> {
    let offset = checked_offset(row, col, rows, cols)?;
    match data.get(offset) {
        Some(&value) => Ok(value),
        None => Err(SolverError::InternalInvariantViolation),
    }
}

/// Bounds-checked row-major write.
#[inline]
pub(crate) fn matrix_set<S: BaseScalar>(
    data: &mut [S],
    row: usize,
    col: usize,
    rows: usize,
    cols: usize,
    value: S,
) -> Result<(), SolverError> {
    let offset = checked_offset(row, col, rows, cols)?;
    match data.get_mut(offset) {
        Some(slot) => {
            *slot = value;
            Ok(())
        }
        None => Err(SolverError::InternalInvariantViolation),
    }
}

#[cfg(test)]
mod tests;
