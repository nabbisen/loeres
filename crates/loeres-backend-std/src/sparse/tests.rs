//! Tests for the dynamic sparse adapters (RFC 007).

use super::{SparseIngestOptions, SparseMatrix};
use loeres::{DimensionKind, MatrixAccess, SolverError};

fn sample() -> SparseMatrix<f64> {
    // 3x3 with stored entries at (0,0)=1, (0,2)=2, (2,1)=3, plus an explicit
    // stored zero at (1,1)=0 to exercise stored-vs-implicit.
    SparseMatrix::from_triplets(
        3,
        3,
        &[(0, 0, 1.0), (0, 2, 2.0), (2, 1, 3.0), (1, 1, 0.0)],
        SparseIngestOptions::default(),
    )
    .unwrap()
}

#[test]
fn sparse_dims_nnz_and_kind() {
    let m = sample();
    assert_eq!(m.dims().rows, 3);
    assert_eq!(m.dims().cols, 3);
    assert_eq!(m.nnz(), 4);
    assert_eq!(m.dimension_kind(), DimensionKind::Dynamic);
}

#[test]
fn sparse_get_stored_and_implicit_zero() {
    let m = sample();
    assert_eq!(m.get(0, 0).unwrap(), 1.0);
    assert_eq!(m.get(0, 2).unwrap(), 2.0);
    assert_eq!(m.get(2, 1).unwrap(), 3.0);
    // absent in-bounds entry reads as implicit zero
    assert_eq!(m.get(1, 0).unwrap(), 0.0);
    assert_eq!(m.get(2, 2).unwrap(), 0.0);
}

#[test]
fn sparse_get_out_of_bounds() {
    let m = sample();
    assert_eq!(
        m.get(3, 0).unwrap_err(),
        SolverError::DimensionMismatch { lhs: 3, rhs: 3 }
    );
    assert_eq!(
        m.get(0, 7).unwrap_err(),
        SolverError::DimensionMismatch { lhs: 7, rhs: 3 }
    );
}

#[test]
fn sparse_try_get_stored_distinguishes_stored_zero() {
    let m = sample();
    // explicitly stored zero at (1,1)
    assert_eq!(m.try_get_stored(1, 1).unwrap(), Some(0.0));
    // absent (implicit zero) at (1,0)
    assert_eq!(m.try_get_stored(1, 0).unwrap(), None);
    // stored nonzero
    assert_eq!(m.try_get_stored(0, 0).unwrap(), Some(1.0));
    // out-of-bounds
    assert_eq!(
        m.try_get_stored(9, 0).unwrap_err(),
        SolverError::DimensionMismatch { lhs: 9, rhs: 3 }
    );
}

#[test]
fn sparse_rejects_duplicates() {
    let err = SparseMatrix::from_triplets(
        2,
        2,
        &[(0, 0, 1.0), (0, 0, 2.0)],
        SparseIngestOptions::default(),
    )
    .unwrap_err();
    assert_eq!(err, SolverError::InvalidInput);
}

#[test]
fn sparse_rejects_out_of_bounds_triplet() {
    let err = SparseMatrix::from_triplets(
        2,
        2,
        &[(0, 0, 1.0), (5, 0, 2.0)],
        SparseIngestOptions::default(),
    )
    .unwrap_err();
    assert_eq!(err, SolverError::DimensionMismatch { lhs: 5, rhs: 2 });
}

#[test]
fn sparse_rejects_zero_dimensions() {
    assert_eq!(
        SparseMatrix::<f64>::from_triplets(0, 2, &[], SparseIngestOptions::default()).unwrap_err(),
        SolverError::InvalidDimension
    );
}

#[test]
fn sparse_memory_limit() {
    let opts = SparseIngestOptions {
        max_entries: Some(1),
    };
    let err = SparseMatrix::from_triplets(2, 2, &[(0, 0, 1.0), (1, 1, 2.0)], opts).unwrap_err();
    assert_eq!(err, SolverError::InvalidInput);
    assert!(SparseMatrix::from_triplets(2, 2, &[(0, 0, 1.0)], opts).is_ok());
}

#[test]
fn sparse_validate_finite_scans_stored_only() {
    let ok = sample();
    assert!(ok.validate_finite().is_ok());
    let bad =
        SparseMatrix::from_triplets(2, 2, &[(0, 0, f64::NAN)], SparseIngestOptions::default())
            .unwrap();
    assert_eq!(
        bad.validate_finite().unwrap_err(),
        SolverError::NonFiniteInput
    );
}
