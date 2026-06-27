//! Tests for RFC 002 storage-agnostic access contracts.
//!
//! Validates the design specification (RFC 002 §6.2) and the implementation
//! decisions A1 (exact-size row-major views) and B1 (per-axis 2-D bounds).

use crate::access::{
    ContiguousMatrixAccess, ContiguousVectorAccess, ContiguousVectorAccessMut, MatrixAccess,
    MatrixAccessMut, MatrixView, MatrixViewMut, VectorAccess, VectorAccessMut, VectorView,
    VectorViewMut,
};
use crate::dimension::{Dim2, DimensionKind};
use crate::error::SolverError;

// ---------------------------------------------------------------------------
// Vector views
// ---------------------------------------------------------------------------

#[test]
fn vector_view_valid_access() {
    let data = [1.0_f64, 2.0, 3.0];
    let v = VectorView::from_slice(&data);
    assert_eq!(v.len(), 3);
    assert_eq!(v.get(0), Ok(1.0));
    assert_eq!(v.get(2), Ok(3.0));
    assert_eq!(v.as_slice(), &data);
}

#[test]
fn vector_view_dimension_kind_is_dynamic() {
    let data = [0.0_f64; 4];
    let v = VectorView::from_slice(&data);
    assert_eq!(v.dimension_kind(), DimensionKind::Dynamic);
}

#[test]
fn vector_view_index_out_of_bounds_is_dimension_mismatch() {
    let data = [1.0_f64, 2.0, 3.0];
    let v = VectorView::from_slice(&data);
    assert_eq!(
        v.get(3),
        Err(SolverError::DimensionMismatch { lhs: 3, rhs: 3 })
    );
    assert_eq!(
        v.get(10),
        Err(SolverError::DimensionMismatch { lhs: 10, rhs: 3 })
    );
}

#[test]
fn vector_view_as_contiguous_returns_backing() {
    let data = [4.0_f64, 5.0, 6.0];
    let v = VectorView::from_slice(&data);
    assert_eq!(v.as_contiguous(), Some(&data[..]));
}

#[test]
fn vector_view_mut_set_then_get() {
    let mut data = [1.0_f64, 2.0, 3.0];
    let mut v = VectorViewMut::from_slice_mut(&mut data);
    assert_eq!(v.set(1, 9.0), Ok(()));
    assert_eq!(v.get(1), Ok(9.0));
}

#[test]
fn vector_view_mut_set_out_of_bounds() {
    let mut data = [1.0_f64, 2.0];
    let mut v = VectorViewMut::from_slice_mut(&mut data);
    assert_eq!(
        v.set(5, 0.0),
        Err(SolverError::DimensionMismatch { lhs: 5, rhs: 2 })
    );
}

#[test]
fn vector_view_mut_contiguous_mut_allows_mutation() {
    let mut data = [1.0_f64, 2.0, 3.0];
    let mut v = VectorViewMut::from_slice_mut(&mut data);
    let slice = v.as_contiguous_mut().expect("contiguous");
    slice[0] = 7.0;
    assert_eq!(v.get(0), Ok(7.0));
}

#[test]
fn vector_view_no_panic_on_invalid_index() {
    let data = [1.0_f64];
    let v = VectorView::from_slice(&data);
    // Returns an error rather than panicking; reaching the assert proves it.
    assert!(v.get(usize::MAX).is_err());
}

#[cfg(target_pointer_width = "64")]
#[test]
fn vector_index_exceeding_u32_is_not_truncated() {
    // An index larger than u32::MAX must map to InvalidDimension, never wrap to
    // a misleading DimensionMismatch payload (RFC 002 §5.1, patch B4).
    let data = [1.0_f64, 2.0, 3.0];
    let v = VectorView::from_slice(&data);
    let oversized = (u32::MAX as usize) + 1;
    assert_eq!(v.get(oversized), Err(SolverError::InvalidDimension));
}

// ---------------------------------------------------------------------------
// Matrix views — construction (A1: exact size)
// ---------------------------------------------------------------------------

#[test]
fn matrix_view_valid_row_major_access() {
    // 2x3 row-major: [[1,2,3],[4,5,6]]
    let data = [1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0];
    let m = MatrixView::from_row_major(&data, 2, 3).expect("valid");
    assert_eq!(m.dims(), Dim2::new(2, 3));
    assert_eq!(m.get(0, 0), Ok(1.0));
    assert_eq!(m.get(0, 2), Ok(3.0));
    assert_eq!(m.get(1, 0), Ok(4.0));
    assert_eq!(m.get(1, 2), Ok(6.0));
}

#[test]
fn matrix_view_dimension_kind_is_dynamic() {
    let data = [0.0_f64; 6];
    let m = MatrixView::from_row_major(&data, 2, 3).expect("valid");
    assert_eq!(m.dimension_kind(), DimensionKind::Dynamic);
}

#[test]
fn matrix_view_construction_rejects_too_small_slice() {
    let data = [1.0_f64; 5]; // needs 6
    assert_eq!(
        MatrixView::from_row_major(&data, 2, 3).unwrap_err(),
        SolverError::DimensionMismatch { lhs: 5, rhs: 6 }
    );
}

#[test]
fn matrix_view_construction_rejects_too_large_slice() {
    // A1: an oversized backing is rejected, not silently treated as a prefix.
    let data = [1.0_f64; 7]; // needs 6
    assert_eq!(
        MatrixView::from_row_major(&data, 2, 3).unwrap_err(),
        SolverError::DimensionMismatch { lhs: 7, rhs: 6 }
    );
}

#[test]
fn matrix_view_construction_rejects_dimension_overflow() {
    // rows * cols overflows usize -> InvalidDimension, checked before any
    // indexing or slicing. Backing slice content is irrelevant.
    let data = [1.0_f64; 1];
    assert_eq!(
        MatrixView::from_row_major(&data, usize::MAX, 2).unwrap_err(),
        SolverError::InvalidDimension
    );
}

#[test]
fn matrix_view_accepts_exact_size() {
    let data = [1.0_f64; 6];
    assert!(MatrixView::from_row_major(&data, 2, 3).is_ok());
    assert!(MatrixView::from_row_major(&data, 3, 2).is_ok());
    assert!(MatrixView::from_row_major(&data, 6, 1).is_ok());
}

// ---------------------------------------------------------------------------
// Matrix views — bounds (B1: per-axis, row then column)
// ---------------------------------------------------------------------------

#[test]
fn matrix_view_row_out_of_range_reports_row_axis() {
    let data = [1.0_f64; 6];
    let m = MatrixView::from_row_major(&data, 2, 3).expect("valid");
    assert_eq!(
        m.get(2, 0),
        Err(SolverError::DimensionMismatch { lhs: 2, rhs: 2 })
    );
}

#[test]
fn matrix_view_col_out_of_range_reports_col_axis() {
    let data = [1.0_f64; 6];
    let m = MatrixView::from_row_major(&data, 2, 3).expect("valid");
    assert_eq!(
        m.get(0, 3),
        Err(SolverError::DimensionMismatch { lhs: 3, rhs: 3 })
    );
}

#[test]
fn matrix_view_both_out_of_range_reports_row_first() {
    let data = [1.0_f64; 6];
    let m = MatrixView::from_row_major(&data, 2, 3).expect("valid");
    // row=5 (bound 2) and col=9 (bound 3) both invalid; row is reported.
    assert_eq!(
        m.get(5, 9),
        Err(SolverError::DimensionMismatch { lhs: 5, rhs: 2 })
    );
}

#[test]
fn matrix_view_square_payload_does_not_encode_axis() {
    // Documented B1 limitation: on a square matrix a row and a column violation
    // can produce identical payloads; the axis is distinguished by context.
    let data = [1.0_f64; 4];
    let m = MatrixView::from_row_major(&data, 2, 2).expect("valid");
    let row_violation = m.get(5, 0);
    let col_violation = m.get(0, 5);
    assert_eq!(
        row_violation,
        Err(SolverError::DimensionMismatch { lhs: 5, rhs: 2 })
    );
    assert_eq!(row_violation, col_violation);
}

#[test]
fn matrix_view_no_linearized_index_overflow() {
    // An extreme row coordinate is caught by the per-axis check before any
    // row*cols+col arithmetic, so there is no overflow and no panic.
    let data = [1.0_f64; 6];
    let m = MatrixView::from_row_major(&data, 2, 3).expect("valid");
    assert!(m.get(usize::MAX, 0).is_err());
}

// ---------------------------------------------------------------------------
// Matrix views — mutation + fast path
// ---------------------------------------------------------------------------

#[test]
fn matrix_view_mut_set_then_get() {
    let mut data = [0.0_f64; 6];
    let mut m = MatrixViewMut::from_row_major_mut(&mut data, 2, 3).expect("valid");
    assert_eq!(m.set(1, 2, 42.0), Ok(()));
    assert_eq!(m.get(1, 2), Ok(42.0));
}

#[test]
fn matrix_view_mut_set_out_of_bounds() {
    let mut data = [0.0_f64; 6];
    let mut m = MatrixViewMut::from_row_major_mut(&mut data, 2, 3).expect("valid");
    assert_eq!(
        m.set(2, 0, 1.0),
        Err(SolverError::DimensionMismatch { lhs: 2, rhs: 2 })
    );
}

#[test]
fn matrix_view_as_row_major_has_product_length() {
    let data = [1.0_f64; 6];
    let m = MatrixView::from_row_major(&data, 2, 3).expect("valid");
    let flat = m.as_row_major().expect("contiguous");
    assert_eq!(flat.len(), 2 * 3);
    assert_eq!(flat, &data[..]);
}

#[test]
fn matrix_view_mut_as_row_major_has_product_length() {
    let mut data = [1.0_f64; 6];
    let m = MatrixViewMut::from_row_major_mut(&mut data, 3, 2).expect("valid");
    assert_eq!(m.as_row_major().expect("contiguous").len(), 3 * 2);
}

// ---------------------------------------------------------------------------
// Contiguous fast path: Some on core views, graceful None fallback
// ---------------------------------------------------------------------------

/// A non-contiguous vector that still satisfies [`VectorAccess`]; its
/// `as_contiguous` returns `None`, exercising the kernel fallback path.
struct NonContiguousVec<'a> {
    data: &'a [f64],
}

impl VectorAccess for NonContiguousVec<'_> {
    type Scalar = f64;
    fn len(&self) -> usize {
        self.data.len()
    }
    fn dimension_kind(&self) -> DimensionKind {
        DimensionKind::Dynamic
    }
    fn get(&self, index: usize) -> Result<f64, SolverError> {
        match self.data.get(index) {
            Some(&v) => Ok(v),
            None => Err(SolverError::DimensionMismatch { lhs: 0, rhs: 0 }),
        }
    }
}

impl ContiguousVectorAccess for NonContiguousVec<'_> {
    fn as_contiguous(&self) -> Option<&[f64]> {
        None
    }
}

/// Kernel-style sum: fast branch-free loop when contiguous, fallible
/// per-element access otherwise.
fn sum<V: ContiguousVectorAccess<Scalar = f64>>(v: &V) -> Result<f64, SolverError> {
    match v.as_contiguous() {
        Some(slice) => Ok(slice.iter().copied().sum()),
        None => {
            let mut acc = 0.0;
            for i in 0..v.len() {
                acc += v.get(i)?;
            }
            Ok(acc)
        }
    }
}

#[test]
fn contiguous_fast_path_and_fallback_agree() {
    let data = [1.0_f64, 2.0, 3.0, 4.0];
    let fast = VectorView::from_slice(&data);
    let slow = NonContiguousVec { data: &data };
    assert!(fast.as_contiguous().is_some());
    assert!(slow.as_contiguous().is_none());
    assert_eq!(sum(&fast), Ok(10.0));
    assert_eq!(sum(&slow), Ok(10.0));
}
