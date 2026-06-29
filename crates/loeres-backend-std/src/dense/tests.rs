//! Tests for the dynamic dense adapters (RFC 007).

use super::{DenseIngestOptions, DenseMatrix, DenseVector};
use loeres::{
    ContiguousMatrixAccess, ContiguousVectorAccess, ContiguousVectorAccessMut, DimensionKind,
    MatrixAccess, MatrixAccessMut, SolverError, VectorAccess, VectorAccessMut,
};

#[test]
fn dense_vector_reads_and_kind() {
    let v = DenseVector::from_vec(vec![1.0, 2.0, 3.0]).unwrap();
    assert_eq!(VectorAccess::len(&v), 3);
    assert_eq!(v.dimension_kind(), DimensionKind::Dynamic);
    assert_eq!(v.get(0).unwrap(), 1.0);
    assert_eq!(v.get(2).unwrap(), 3.0);
}

#[test]
fn dense_vector_out_of_bounds_is_dimension_mismatch() {
    let v = DenseVector::from_vec(vec![1.0, 2.0]).unwrap();
    assert_eq!(
        v.get(5).unwrap_err(),
        SolverError::DimensionMismatch { lhs: 5, rhs: 2 }
    );
}

#[test]
fn dense_vector_mutable_write() {
    let mut v = DenseVector::from_vec(vec![0.0, 0.0]).unwrap();
    v.set(1, 9.0).unwrap();
    assert_eq!(v.get(1).unwrap(), 9.0);
    assert_eq!(
        v.set(2, 1.0).unwrap_err(),
        SolverError::DimensionMismatch { lhs: 2, rhs: 2 }
    );
}

#[test]
fn dense_vector_contiguous_fast_paths() {
    let mut v = DenseVector::from_vec(vec![1.0, 2.0, 3.0]).unwrap();
    assert_eq!(v.as_contiguous().unwrap().len(), 3);
    {
        let slice = v.as_contiguous_mut().unwrap();
        assert_eq!(slice.len(), 3);
        slice[0] = 7.0;
    }
    assert_eq!(v.get(0).unwrap(), 7.0);
}

#[test]
fn dense_vector_validate_finite() {
    let ok = DenseVector::from_vec(vec![1.0, 2.0]).unwrap();
    assert!(ok.validate_finite().is_ok());
    let bad = DenseVector::from_vec(vec![1.0, f64::NAN]).unwrap();
    assert_eq!(
        bad.validate_finite().unwrap_err(),
        SolverError::NonFiniteInput
    );
}

#[test]
fn dense_matrix_reads_and_dims() {
    let m = DenseMatrix::from_row_major_vec(2, 3, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]).unwrap();
    assert_eq!(m.dims().rows, 2);
    assert_eq!(m.dims().cols, 3);
    assert_eq!(m.dimension_kind(), DimensionKind::Dynamic);
    assert_eq!(m.get(0, 0).unwrap(), 1.0);
    assert_eq!(m.get(1, 2).unwrap(), 6.0);
    assert_eq!(m.as_row_major().unwrap().len(), 6);
}

#[test]
fn dense_matrix_out_of_bounds() {
    let m = DenseMatrix::from_row_major_vec(2, 2, vec![1.0, 2.0, 3.0, 4.0]).unwrap();
    assert_eq!(
        m.get(2, 0).unwrap_err(),
        SolverError::DimensionMismatch { lhs: 2, rhs: 2 }
    );
    assert_eq!(
        m.get(0, 9).unwrap_err(),
        SolverError::DimensionMismatch { lhs: 9, rhs: 2 }
    );
}

#[test]
fn dense_matrix_length_mismatch() {
    let err = DenseMatrix::from_row_major_vec(2, 3, vec![1.0; 5]).unwrap_err();
    assert_eq!(err, SolverError::DimensionMismatch { lhs: 5, rhs: 6 });
}

#[test]
fn dense_matrix_zero_and_overflow_dimensions() {
    assert_eq!(
        DenseMatrix::<f64>::from_row_major_vec(0, 3, vec![]).unwrap_err(),
        SolverError::InvalidDimension
    );
    assert_eq!(
        DenseMatrix::<f64>::from_row_major_vec(usize::MAX, 2, vec![]).unwrap_err(),
        SolverError::InvalidDimension
    );
}

#[test]
fn dense_matrix_mutable_write() {
    let mut m = DenseMatrix::from_row_major_vec(2, 2, vec![0.0; 4]).unwrap();
    m.set(1, 0, 5.0).unwrap();
    assert_eq!(m.get(1, 0).unwrap(), 5.0);
}

#[test]
fn dense_memory_limit() {
    let opts = DenseIngestOptions {
        max_elements: Some(2),
    };
    assert_eq!(
        DenseVector::from_vec_with_options(vec![1.0, 2.0, 3.0], opts).unwrap_err(),
        SolverError::InvalidInput
    );
    assert!(DenseVector::from_vec_with_options(vec![1.0, 2.0], opts).is_ok());
    assert_eq!(
        DenseMatrix::from_row_major_vec_with_options(2, 2, vec![0.0; 4], opts).unwrap_err(),
        SolverError::InvalidInput
    );
}
