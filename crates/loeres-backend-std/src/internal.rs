//! Internal helpers shared by the dynamic storage adapters.

use loeres::SolverError;

/// The dimension error for a mismatch between `lhs` and `rhs`, applying the
/// checked `u32` payload-fallback rule (RFC 007 §3.2, Correction 2): if both
/// values fit `u32`, `DimensionMismatch { lhs, rhs }`; otherwise the non-payload
/// `InvalidDimension`. Never truncates.
pub(crate) fn dimension_mismatch(lhs: usize, rhs: usize) -> SolverError {
    match (u32::try_from(lhs), u32::try_from(rhs)) {
        (Ok(lhs), Ok(rhs)) => SolverError::DimensionMismatch { lhs, rhs },
        _ => SolverError::InvalidDimension,
    }
}
