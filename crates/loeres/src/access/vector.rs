//! Vector access traits and the core borrowed vector views.
//!
//! Layout-agnostic fallible access ([`VectorAccess`] / [`VectorAccessMut`]), an
//! optional contiguous fast path ([`ContiguousVectorAccess`] /
//! [`ContiguousVectorAccessMut`]), and slice-backed reference views
//! ([`VectorView`] / [`VectorViewMut`]). RFC 002 §3.3, §3.5, §3.8.

use super::dim_u32;
use crate::dimension::DimensionKind;
use crate::error::SolverError;
use crate::scalar::BaseScalar;

/// Read access to a one-dimensional sequence of scalars, independent of storage.
///
/// Element access is fallible: an out-of-bounds index returns a structured
/// [`SolverError`] rather than panicking, so device kernels can fail closed.
// `len` without `is_empty` is intentional: RFC 002 §3.3 fixes this trait's
// surface to `len` / `dimension_kind` / `get`.
#[allow(clippy::len_without_is_empty)]
pub trait VectorAccess {
    /// The element type, constrained to the base scalar capability tier.
    type Scalar: BaseScalar;

    /// The number of elements.
    fn len(&self) -> usize;

    /// Whether the length is known at compile time or at run time.
    fn dimension_kind(&self) -> DimensionKind;

    /// The element at `index`, or [`SolverError::DimensionMismatch`] if `index`
    /// is out of bounds (payload: the requested index and the valid length).
    fn get(&self, index: usize) -> Result<Self::Scalar, SolverError>;
}

/// Mutable element access over a [`VectorAccess`] sequence.
pub trait VectorAccessMut: VectorAccess {
    /// Write `value` at `index`, or return [`SolverError::DimensionMismatch`] if
    /// `index` is out of bounds.
    fn set(&mut self, index: usize, value: Self::Scalar) -> Result<(), SolverError>;
}

/// Optional fast path: vector storage that is contiguous in memory.
///
/// A kernel calls [`as_contiguous`](ContiguousVectorAccess::as_contiguous) once
/// and, on `Some`, runs a tight branch-free loop; on `None` it falls back to
/// per-element [`VectorAccess::get`].
pub trait ContiguousVectorAccess: VectorAccess {
    /// The contiguous backing slice, or `None` if storage is not contiguous.
    fn as_contiguous(&self) -> Option<&[Self::Scalar]>;
}

/// Optional fast path for mutable contiguous vector storage.
pub trait ContiguousVectorAccessMut: ContiguousVectorAccess + VectorAccessMut {
    /// The contiguous mutable backing slice, or `None` if not contiguous.
    fn as_contiguous_mut(&mut self) -> Option<&mut [Self::Scalar]>;
}

/// A read-only vector view borrowing a contiguous slice.
///
/// Allocation-free; the borrow checker is the safety boundary. Reports
/// [`DimensionKind::Dynamic`], since a slice length is a run-time value.
#[derive(Copy, Clone, Debug)]
pub struct VectorView<'a, S: BaseScalar> {
    data: &'a [S],
}

impl<'a, S: BaseScalar> VectorView<'a, S> {
    /// Borrow `data` as a vector view.
    #[inline]
    pub fn from_slice(data: &'a [S]) -> Self {
        Self { data }
    }

    /// The borrowed backing slice.
    #[inline]
    pub fn as_slice(&self) -> &'a [S] {
        self.data
    }
}

impl<S: BaseScalar> VectorAccess for VectorView<'_, S> {
    type Scalar = S;

    #[inline]
    fn len(&self) -> usize {
        self.data.len()
    }

    #[inline]
    fn dimension_kind(&self) -> DimensionKind {
        DimensionKind::Dynamic
    }

    #[inline]
    fn get(&self, index: usize) -> Result<S, SolverError> {
        match self.data.get(index) {
            Some(&value) => Ok(value),
            None => Err(SolverError::DimensionMismatch {
                lhs: dim_u32(index)?,
                rhs: dim_u32(self.data.len())?,
            }),
        }
    }
}

impl<S: BaseScalar> ContiguousVectorAccess for VectorView<'_, S> {
    #[inline]
    fn as_contiguous(&self) -> Option<&[S]> {
        Some(self.data)
    }
}

/// A mutable vector view borrowing a contiguous slice.
///
/// Contiguous and therefore injective: every index maps to a distinct element,
/// so [`set`](VectorAccessMut::set) cannot alias. Reports
/// [`DimensionKind::Dynamic`].
#[derive(Debug)]
pub struct VectorViewMut<'a, S: BaseScalar> {
    data: &'a mut [S],
}

impl<'a, S: BaseScalar> VectorViewMut<'a, S> {
    /// Borrow `data` as a mutable vector view.
    #[inline]
    pub fn from_slice_mut(data: &'a mut [S]) -> Self {
        Self { data }
    }
}

impl<S: BaseScalar> VectorAccess for VectorViewMut<'_, S> {
    type Scalar = S;

    #[inline]
    fn len(&self) -> usize {
        self.data.len()
    }

    #[inline]
    fn dimension_kind(&self) -> DimensionKind {
        DimensionKind::Dynamic
    }

    #[inline]
    fn get(&self, index: usize) -> Result<S, SolverError> {
        match self.data.get(index) {
            Some(&value) => Ok(value),
            None => Err(SolverError::DimensionMismatch {
                lhs: dim_u32(index)?,
                rhs: dim_u32(self.data.len())?,
            }),
        }
    }
}

impl<S: BaseScalar> VectorAccessMut for VectorViewMut<'_, S> {
    #[inline]
    fn set(&mut self, index: usize, value: S) -> Result<(), SolverError> {
        let len = self.data.len();
        match self.data.get_mut(index) {
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
}

impl<S: BaseScalar> ContiguousVectorAccess for VectorViewMut<'_, S> {
    #[inline]
    fn as_contiguous(&self) -> Option<&[S]> {
        Some(&*self.data)
    }
}

impl<S: BaseScalar> ContiguousVectorAccessMut for VectorViewMut<'_, S> {
    #[inline]
    fn as_contiguous_mut(&mut self) -> Option<&mut [S]> {
        Some(&mut *self.data)
    }
}
