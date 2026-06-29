//! Owned fixed-size storage (feature `owned-arrays`).
//!
//! [`FixedVector`] and [`FixedMatrix`] are allocation-free owned wrappers over
//! `[S; N]`. Type-level dimension invariants are enforced by compile-time
//! assertions, so a mismatched **public construction** fails to compile (RFC 004
//! §8; validated on MSRV 1.85). Element access is fallible and panic-averse, and
//! both types implement the RFC 002 access and contiguous fast-path traits and
//! report [`DimensionKind::Static`].

use loeres::{
    BaseScalar, ContiguousMatrixAccess, ContiguousVectorAccess, ContiguousVectorAccessMut, Dim2,
    DimensionKind, MatrixAccess, MatrixAccessMut, SolverError, VectorAccess, VectorAccessMut,
};

use crate::dimension;

/// An owned, fixed-size column of `N` scalars (`N > 0`).
///
/// `FixedVector<S, 0>` cannot be built through [`from_array`](FixedVector::from_array):
/// the `N > 0` invariant is a compile-time assertion, so such a construction
/// fails to compile.
#[repr(transparent)]
pub struct FixedVector<S, const N: usize> {
    data: [S; N],
}

#[allow(clippy::len_without_is_empty)] // N > 0 is a construction invariant; is_empty would be trivially false.
impl<S, const N: usize> FixedVector<S, N> {
    /// The element count, exposed for footprint review (BSTATIC-008).
    pub const ELEMENTS: usize = N;

    /// Wrap an array as a fixed vector. Fails to compile when `N == 0`.
    #[inline]
    pub const fn from_array(data: [S; N]) -> Self {
        const { assert!(N > 0, "FixedVector requires N > 0") }
        Self { data }
    }

    /// The number of elements (`N`).
    #[inline]
    pub const fn len(&self) -> usize {
        N
    }

    /// The backing slice.
    #[inline]
    pub fn as_slice(&self) -> &[S] {
        &self.data
    }

    /// The mutable backing slice.
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [S] {
        &mut self.data
    }
}

impl<S: BaseScalar, const N: usize> VectorAccess for FixedVector<S, N> {
    type Scalar = S;

    #[inline]
    fn len(&self) -> usize {
        N
    }

    #[inline]
    fn dimension_kind(&self) -> DimensionKind {
        dimension::STATIC_KIND
    }

    #[inline]
    fn get(&self, index: usize) -> Result<S, SolverError> {
        dimension::vector_get(&self.data, index)
    }
}

impl<S: BaseScalar, const N: usize> VectorAccessMut for FixedVector<S, N> {
    #[inline]
    fn set(&mut self, index: usize, value: S) -> Result<(), SolverError> {
        dimension::vector_set(&mut self.data, index, value)
    }
}

impl<S: BaseScalar, const N: usize> ContiguousVectorAccess for FixedVector<S, N> {
    #[inline]
    fn as_contiguous(&self) -> Option<&[S]> {
        Some(&self.data)
    }
}

impl<S: BaseScalar, const N: usize> ContiguousVectorAccessMut for FixedVector<S, N> {
    #[inline]
    fn as_contiguous_mut(&mut self) -> Option<&mut [S]> {
        Some(&mut self.data)
    }
}

/// An owned, fixed-size row-major matrix of `R × C` scalars, flattened into
/// `[S; N]` with the invariant `N == R * C` and `R, C > 0`.
///
/// The flattened length `N` is an explicit parameter (fallback-first design),
/// avoiding unstable generic const expressions. A mismatched public construction
/// — for example `FixedMatrix::<_, 2, 3, 5>` — fails to compile (RFC 004 §8).
#[repr(transparent)]
pub struct FixedMatrix<S, const R: usize, const C: usize, const N: usize> {
    data: [S; N],
}

impl<S, const R: usize, const C: usize, const N: usize> FixedMatrix<S, R, C, N> {
    /// Row count (`R`), exposed for footprint review (BSTATIC-008).
    pub const ROWS: usize = R;
    /// Column count (`C`).
    pub const COLS: usize = C;
    /// Element count (`N == R * C`).
    pub const ELEMENTS: usize = N;

    /// Wrap a row-major array as a fixed matrix. Fails to compile unless
    /// `N == R * C` and `R, C > 0`.
    #[inline]
    pub const fn from_row_major_array(data: [S; N]) -> Self {
        const {
            assert!(
                N == R * C && R > 0 && C > 0,
                "FixedMatrix requires N == R*C and R,C > 0"
            )
        }
        Self { data }
    }

    /// Row count (`R`).
    #[inline]
    pub const fn rows(&self) -> usize {
        R
    }

    /// Column count (`C`).
    #[inline]
    pub const fn cols(&self) -> usize {
        C
    }

    /// The flattened row-major backing slice (length `N == R * C`).
    #[inline]
    pub fn as_flat_slice(&self) -> &[S] {
        &self.data
    }

    /// The mutable flattened row-major backing slice.
    #[inline]
    pub fn as_flat_slice_mut(&mut self) -> &mut [S] {
        &mut self.data
    }
}

impl<S: BaseScalar, const R: usize, const C: usize, const N: usize> MatrixAccess
    for FixedMatrix<S, R, C, N>
{
    type Scalar = S;

    #[inline]
    fn dims(&self) -> Dim2 {
        dimension::static_dim2::<R, C>()
    }

    #[inline]
    fn dimension_kind(&self) -> DimensionKind {
        dimension::STATIC_KIND
    }

    #[inline]
    fn get(&self, row: usize, col: usize) -> Result<S, SolverError> {
        dimension::matrix_get(&self.data, row, col, R, C)
    }
}

impl<S: BaseScalar, const R: usize, const C: usize, const N: usize> MatrixAccessMut
    for FixedMatrix<S, R, C, N>
{
    #[inline]
    fn set(&mut self, row: usize, col: usize, value: S) -> Result<(), SolverError> {
        dimension::matrix_set(&mut self.data, row, col, R, C, value)
    }
}

impl<S: BaseScalar, const R: usize, const C: usize, const N: usize> ContiguousMatrixAccess
    for FixedMatrix<S, R, C, N>
{
    #[inline]
    fn as_row_major(&self) -> Option<&[S]> {
        Some(&self.data)
    }
}

#[cfg(test)]
mod tests;
