use super::*;
use loeres::SolverError;

#[test]
fn dim_u32_converts_small_values() {
    assert_eq!(dim_u32(0).unwrap(), 0);
    assert_eq!(dim_u32(7).unwrap(), 7);
}

#[test]
fn static_dim2_carries_shape() {
    let d = static_dim2::<2, 3>();
    assert_eq!(d.rows, 2);
    assert_eq!(d.cols, 3);
}

#[test]
fn static_kind_is_static() {
    assert!(matches!(STATIC_KIND, DimensionKind::Static));
}

#[test]
fn vector_get_in_bounds() {
    let data = [1.0_f64, 2.0, 3.0];
    assert_eq!(vector_get(&data, 1).unwrap(), 2.0);
}

#[test]
fn vector_get_oob_maps_dimension_mismatch() {
    let data = [1.0_f64, 2.0, 3.0];
    assert!(matches!(
        vector_get(&data, 3),
        Err(SolverError::DimensionMismatch { lhs: 3, rhs: 3 })
    ));
}

#[test]
fn vector_set_round_trip_and_oob() {
    let mut data = [1.0_f64, 2.0, 3.0];
    vector_set(&mut data, 0, 9.0).unwrap();
    assert_eq!(data[0], 9.0);
    assert!(matches!(
        vector_set(&mut data, 5, 0.0),
        Err(SolverError::DimensionMismatch { lhs: 5, rhs: 3 })
    ));
}

#[test]
fn checked_offset_row_major() {
    assert_eq!(checked_offset(1, 2, 2, 3).unwrap(), 5);
    assert_eq!(checked_offset(0, 0, 2, 3).unwrap(), 0);
}

#[test]
fn checked_offset_row_checked_before_col() {
    // RFC 002 B1: row axis is checked first; a bad row reports {row, rows}
    // even when the column is also out of range.
    assert!(matches!(
        checked_offset(2, 9, 2, 3),
        Err(SolverError::DimensionMismatch { lhs: 2, rhs: 2 })
    ));
    assert!(matches!(
        checked_offset(0, 3, 2, 3),
        Err(SolverError::DimensionMismatch { lhs: 3, rhs: 3 })
    ));
}

#[test]
fn matrix_get_set_round_trip() {
    let mut data = [0.0_f64; 6]; // 2x3 row-major
    matrix_set(&mut data, 1, 2, 2, 3, 7.0).unwrap();
    assert_eq!(matrix_get(&data, 1, 2, 2, 3).unwrap(), 7.0);
    assert_eq!(data[5], 7.0);
}
