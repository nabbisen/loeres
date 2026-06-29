use super::*;
use loeres::{
    ContiguousMatrixAccess, ContiguousVectorAccess, ContiguousVectorAccessMut, DimensionKind,
    MatrixAccess, MatrixAccessMut, SolverError, VectorAccess, VectorAccessMut,
};

#[test]
fn static_vector_view_reads() {
    let data = [1.0_f64, 2.0, 3.0];
    let v = StaticVectorView::from_array_ref(&data);
    assert_eq!(VectorAccess::len(&v), 3);
    assert_eq!(v.get(1).unwrap(), 2.0);
    assert!(matches!(v.dimension_kind(), DimensionKind::Static));
    assert_eq!(v.as_contiguous(), Some(&[1.0, 2.0, 3.0][..]));
    assert!(matches!(
        v.get(3),
        Err(SolverError::DimensionMismatch { lhs: 3, rhs: 3 })
    ));
}

#[test]
fn static_vector_view_mut_round_trip() {
    let mut data = [0.0_f64; 3];
    {
        let mut v = StaticVectorViewMut::from_array_mut(&mut data);
        v.set(2, 8.0).unwrap();
        assert_eq!(v.get(2).unwrap(), 8.0);
        assert_eq!(v.as_contiguous(), Some(&[0.0, 0.0, 8.0][..]));
        v.as_contiguous_mut().unwrap()[0] = 1.0;
        assert!(matches!(v.dimension_kind(), DimensionKind::Static));
    }
    assert_eq!(data[0], 1.0);
    assert_eq!(data[2], 8.0);
}

#[test]
fn static_matrix_view_reads() {
    let data = [1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0];
    let m = StaticMatrixView::<f64, 2, 3, 6>::from_row_major_ref(&data);
    let d = m.dims();
    assert_eq!(d.rows, 2);
    assert_eq!(d.cols, 3);
    assert!(matches!(m.dimension_kind(), DimensionKind::Static));
    assert_eq!(m.get(1, 2).unwrap(), 6.0);
    assert_eq!(m.as_row_major(), Some(&data[..]));
    assert!(matches!(
        m.get(2, 0),
        Err(SolverError::DimensionMismatch { lhs: 2, rhs: 2 })
    ));
}

#[test]
fn static_matrix_view_mut_round_trip() {
    let mut data = [0.0_f64; 6];
    {
        let mut m = StaticMatrixViewMut::<f64, 2, 3, 6>::from_row_major_mut(&mut data);
        m.set(0, 1, 4.0).unwrap();
        assert_eq!(m.get(0, 1).unwrap(), 4.0);
        assert!(matches!(m.dimension_kind(), DimensionKind::Static));
        assert!(matches!(
            m.set(9, 0, 0.0),
            Err(SolverError::DimensionMismatch { lhs: 9, rhs: 2 })
        ));
    }
    assert_eq!(data[1], 4.0);
}
