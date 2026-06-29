use super::WorkspaceFootprint;
use crate::array::{FixedMatrix, FixedVector};

#[test]
fn fixed_vector_footprint_is_size_of() {
    assert_eq!(
        <FixedVector<f64, 4> as WorkspaceFootprint>::footprint_bytes(),
        core::mem::size_of::<FixedVector<f64, 4>>()
    );
    assert_eq!(
        <FixedVector<f64, 4> as WorkspaceFootprint>::footprint_bytes(),
        4 * 8
    );
}

#[test]
fn fixed_matrix_footprint_is_size_of() {
    assert_eq!(
        <FixedMatrix<f64, 2, 3, 6> as WorkspaceFootprint>::footprint_bytes(),
        core::mem::size_of::<FixedMatrix<f64, 2, 3, 6>>()
    );
    assert_eq!(
        <FixedMatrix<f64, 2, 3, 6> as WorkspaceFootprint>::footprint_bytes(),
        6 * 8
    );
}
