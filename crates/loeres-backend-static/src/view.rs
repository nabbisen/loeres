//! Borrowed static views over caller-owned memory.
//!
//! Baseline (default feature set): const-sized **contiguous** views, so embedded
//! callers can hand peripheral buffers, DMA regions, or RTOS-owned state to the
//! solver without copying into a Loeres-owned container. These views implement
//! the RFC 002 access traits **directly** (not as wrappers around the core
//! `Dynamic`-reporting views) and report [`DimensionKind::Static`].
//!
//! Advanced row / column / sub-matrix / strided views are gated behind the
//! `static-views` feature; their design is deferred (RFC 004 §7.2) and they are
//! not part of this baseline.

use loeres::{
    BaseScalar, ContiguousMatrixAccess, ContiguousVectorAccess, ContiguousVectorAccessMut, Dim2,
    DimensionKind, MatrixAccess, MatrixAccessMut, SolverError, VectorAccess, VectorAccessMut,
};

use crate::dimension;

/// A read-only contiguous view over a borrowed `&[S; N]`.
pub struct StaticVectorView<'a, S, const N: usize> {
    data: &'a [S; N],
}

impl<'a, S, const N: usize> StaticVectorView<'a, S, N> {
    /// Borrow an array reference as a static vector view.
    #[inline]
    pub const fn from_array_ref(data: &'a [S; N]) -> Self {
        Self { data }
    }
}

impl<S: BaseScalar, const N: usize> VectorAccess for StaticVectorView<'_, S, N> {
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
        dimension::vector_get(&self.data[..], index)
    }
}

impl<S: BaseScalar, const N: usize> ContiguousVectorAccess for StaticVectorView<'_, S, N> {
    #[inline]
    fn as_contiguous(&self) -> Option<&[S]> {
        Some(&self.data[..])
    }
}

/// A mutable contiguous view over a borrowed `&mut [S; N]`.
pub struct StaticVectorViewMut<'a, S, const N: usize> {
    data: &'a mut [S; N],
}

impl<'a, S, const N: usize> StaticVectorViewMut<'a, S, N> {
    /// Borrow a mutable array reference as a static vector view.
    #[inline]
    pub const fn from_array_mut(data: &'a mut [S; N]) -> Self {
        Self { data }
    }
}

impl<S: BaseScalar, const N: usize> VectorAccess for StaticVectorViewMut<'_, S, N> {
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
        dimension::vector_get(&self.data[..], index)
    }
}

impl<S: BaseScalar, const N: usize> VectorAccessMut for StaticVectorViewMut<'_, S, N> {
    #[inline]
    fn set(&mut self, index: usize, value: S) -> Result<(), SolverError> {
        dimension::vector_set(&mut self.data[..], index, value)
    }
}

impl<S: BaseScalar, const N: usize> ContiguousVectorAccess for StaticVectorViewMut<'_, S, N> {
    #[inline]
    fn as_contiguous(&self) -> Option<&[S]> {
        Some(&self.data[..])
    }
}

impl<S: BaseScalar, const N: usize> ContiguousVectorAccessMut for StaticVectorViewMut<'_, S, N> {
    #[inline]
    fn as_contiguous_mut(&mut self) -> Option<&mut [S]> {
        Some(&mut self.data[..])
    }
}

/// A read-only contiguous row-major view over a borrowed `&[S; N]`,
/// `N == R * C`.
pub struct StaticMatrixView<'a, S, const R: usize, const C: usize, const N: usize> {
    data: &'a [S; N],
}

impl<'a, S, const R: usize, const C: usize, const N: usize> StaticMatrixView<'a, S, R, C, N> {
    /// Borrow a row-major array reference as a static matrix view. Fails to
    /// compile unless `N == R * C` and `R, C > 0`.
    #[inline]
    pub const fn from_row_major_ref(data: &'a [S; N]) -> Self {
        const {
            assert!(
                N == R * C && R > 0 && C > 0,
                "StaticMatrixView requires N == R*C and R,C > 0"
            )
        }
        Self { data }
    }
}

impl<S: BaseScalar, const R: usize, const C: usize, const N: usize> MatrixAccess
    for StaticMatrixView<'_, S, R, C, N>
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
        dimension::matrix_get(&self.data[..], row, col, R, C)
    }
}

impl<S: BaseScalar, const R: usize, const C: usize, const N: usize> ContiguousMatrixAccess
    for StaticMatrixView<'_, S, R, C, N>
{
    #[inline]
    fn as_row_major(&self) -> Option<&[S]> {
        Some(&self.data[..])
    }
}

/// A mutable contiguous row-major view over a borrowed `&mut [S; N]`,
/// `N == R * C`.
pub struct StaticMatrixViewMut<'a, S, const R: usize, const C: usize, const N: usize> {
    data: &'a mut [S; N],
}

impl<'a, S, const R: usize, const C: usize, const N: usize> StaticMatrixViewMut<'a, S, R, C, N> {
    /// Borrow a mutable row-major array reference as a static matrix view. Fails
    /// to compile unless `N == R * C` and `R, C > 0`.
    #[inline]
    pub const fn from_row_major_mut(data: &'a mut [S; N]) -> Self {
        const {
            assert!(
                N == R * C && R > 0 && C > 0,
                "StaticMatrixViewMut requires N == R*C and R,C > 0"
            )
        }
        Self { data }
    }
}

impl<S: BaseScalar, const R: usize, const C: usize, const N: usize> MatrixAccess
    for StaticMatrixViewMut<'_, S, R, C, N>
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
        dimension::matrix_get(&self.data[..], row, col, R, C)
    }
}

impl<S: BaseScalar, const R: usize, const C: usize, const N: usize> MatrixAccessMut
    for StaticMatrixViewMut<'_, S, R, C, N>
{
    #[inline]
    fn set(&mut self, row: usize, col: usize, value: S) -> Result<(), SolverError> {
        dimension::matrix_set(&mut self.data[..], row, col, R, C, value)
    }
}

impl<S: BaseScalar, const R: usize, const C: usize, const N: usize> ContiguousMatrixAccess
    for StaticMatrixViewMut<'_, S, R, C, N>
{
    #[inline]
    fn as_row_major(&self) -> Option<&[S]> {
        Some(&self.data[..])
    }
}

#[cfg(test)]
mod tests;
