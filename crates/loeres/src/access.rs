//! Storage-agnostic vector and matrix access contracts.
//!
//! These traits describe **dimensions and fallible element access only**. They
//! do not define heavy linear-algebra kernels (matrix multiplication,
//! factorization, sparse assembly, BLAS-style routines) and commit to no memory
//! layout: a conforming backend may be row-major, column-major, strided, block,
//! or sparse, and a caller must not infer layout from trait conformance. The
//! module is named `access`, not `linalg`, to make that boundary explicit.
//!
//! Reference implementations live here as borrowed views over caller memory:
//! [`VectorView`] / [`VectorViewMut`] and a simple contiguous row-major
//! [`MatrixView`] / [`MatrixViewMut`]. Advanced views — column-major, strided,
//! and sub-matrix — are not part of this core baseline; they belong to
//! `loeres-backend-static` (RFC 004) and `loeres-backend-std`. An optional
//! contiguous fast path ([`ContiguousVectorAccess`], [`ContiguousMatrixAccess`],
//! and the mutable vector variant) lets a kernel drop into a tight branch-free
//! loop when a backend exposes contiguous storage, and fall back to fallible
//! per-element access when it does not.
//!
//! Defined by RFC 002 (Storage-Agnostic Matrix and Vector Access Contracts).

mod matrix;
mod vector;

#[cfg(test)]
mod tests;

pub use matrix::{
    ContiguousMatrixAccess, MatrixAccess, MatrixAccessMut, MatrixView, MatrixViewMut,
};
pub use vector::{
    ContiguousVectorAccess, ContiguousVectorAccessMut, VectorAccess, VectorAccessMut, VectorView,
    VectorViewMut,
};

use crate::error::SolverError;

/// Convert a `usize` extent or index into the `u32` payload that RFC 003 error
/// variants carry, **without silent truncation** (RFC 002 §5.1, patch B4).
///
/// A value that does not fit in `u32` maps to [`SolverError::InvalidDimension`]
/// rather than wrapping to a misleading payload. This path is reachable only
/// from caller-supplied indices or dimensions; library-internal overflow uses
/// [`SolverError::InternalInvariantViolation`] at its own call site.
#[inline]
pub(crate) fn dim_u32(value: usize) -> Result<u32, SolverError> {
    u32::try_from(value).map_err(|_| SolverError::InvalidDimension)
}
