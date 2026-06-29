use super::*;
use loeres::{
    ContiguousMatrixAccess, ContiguousVectorAccess, ContiguousVectorAccessMut, DimensionKind,
    MatrixAccess, MatrixAccessMut, SolverError, VectorAccess, VectorAccessMut,
};

#[test]
fn fixed_vector_reads_and_consts() {
    let v = FixedVector::from_array([1.0_f64, 2.0, 3.0]);
    assert_eq!(v.len(), 3);
    assert_eq!(VectorAccess::len(&v), 3);
    assert_eq!(FixedVector::<f64, 3>::ELEMENTS, 3);
    assert_eq!(v.get(0).unwrap(), 1.0);
    assert_eq!(v.get(2).unwrap(), 3.0);
    assert!(matches!(v.dimension_kind(), DimensionKind::Static));
}

#[test]
fn fixed_vector_oob_is_dimension_mismatch() {
    let v = FixedVector::from_array([1.0_f64, 2.0]);
    assert!(matches!(
        v.get(2),
        Err(SolverError::DimensionMismatch { lhs: 2, rhs: 2 })
    ));
}

#[test]
fn fixed_vector_mutate_then_read() {
    let mut v = FixedVector::from_array([0.0_f64; 3]);
    v.set(1, 5.0).unwrap();
    assert_eq!(v.get(1).unwrap(), 5.0);
    assert!(matches!(
        v.set(9, 0.0),
        Err(SolverError::DimensionMismatch { lhs: 9, rhs: 3 })
    ));
}

#[test]
fn fixed_vector_contiguous_fast_path() {
    let mut v = FixedVector::from_array([1.0_f64, 2.0, 3.0]);
    assert_eq!(v.as_contiguous(), Some(&[1.0, 2.0, 3.0][..]));
    v.as_contiguous_mut().unwrap()[0] = 9.0;
    assert_eq!(v.get(0).unwrap(), 9.0);
}

#[test]
fn fixed_matrix_reads_and_consts() {
    let m = FixedMatrix::<f64, 2, 3, 6>::from_row_major_array([1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
    assert_eq!(m.rows(), 2);
    assert_eq!(m.cols(), 3);
    assert_eq!(FixedMatrix::<f64, 2, 3, 6>::ROWS, 2);
    assert_eq!(FixedMatrix::<f64, 2, 3, 6>::COLS, 3);
    assert_eq!(FixedMatrix::<f64, 2, 3, 6>::ELEMENTS, 6);
    let d = m.dims();
    assert_eq!(d.rows, 2);
    assert_eq!(d.cols, 3);
    assert!(matches!(m.dimension_kind(), DimensionKind::Static));
    assert_eq!(m.get(0, 0).unwrap(), 1.0);
    assert_eq!(m.get(1, 2).unwrap(), 6.0);
}

#[test]
fn fixed_matrix_per_axis_bounds() {
    let m = FixedMatrix::<f64, 2, 3, 6>::from_row_major_array([0.0; 6]);
    assert!(matches!(
        m.get(2, 9),
        Err(SolverError::DimensionMismatch { lhs: 2, rhs: 2 })
    ));
    assert!(matches!(
        m.get(0, 3),
        Err(SolverError::DimensionMismatch { lhs: 3, rhs: 3 })
    ));
}

#[test]
fn fixed_matrix_mutate_and_contiguous() {
    let mut m = FixedMatrix::<f64, 2, 3, 6>::from_row_major_array([0.0; 6]);
    m.set(1, 2, 7.0).unwrap();
    assert_eq!(m.get(1, 2).unwrap(), 7.0);
    assert_eq!(m.as_row_major(), Some(&[0.0, 0.0, 0.0, 0.0, 0.0, 7.0][..]));
    assert!(matches!(
        m.set(5, 5, 0.0),
        Err(SolverError::DimensionMismatch { lhs: 5, rhs: 2 })
    ));
}
