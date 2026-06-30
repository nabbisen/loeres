use super::*;

#[test]
fn workspace_rejects_zero_dimension() {
    assert!(matches!(
        ClusterProjectedFirstOrderWorkspace::<f64>::new(0),
        Err(SolverError::InvalidDimension)
    ));
}

#[test]
fn workspace_sizes_to_dimension() {
    let ws = ClusterProjectedFirstOrderWorkspace::<f64>::new(3).unwrap();
    assert_eq!(ws.dimension(), 3);
}

#[test]
fn workspace_reset_preserves_dimension() {
    let mut ws = ClusterProjectedFirstOrderWorkspace::<f64>::new(2).unwrap();
    ws.reset_for_entry();
    assert_eq!(ws.dimension(), 2);
}

#[test]
fn config_rejects_zero_iterations() {
    let c = ProjectedFirstOrderConfig {
        max_iterations: 0,
        tolerance: 1e-6_f64,
    };
    assert!(matches!(c.validate(), Err(SolverError::InvalidInput)));
}

#[test]
fn config_rejects_non_finite_tolerance() {
    let c = ProjectedFirstOrderConfig {
        max_iterations: 10,
        tolerance: f64::NAN,
    };
    assert!(matches!(c.validate(), Err(SolverError::NonFiniteInput)));
}

#[test]
fn config_rejects_non_positive_tolerance() {
    let zero = ProjectedFirstOrderConfig {
        max_iterations: 10,
        tolerance: 0.0_f64,
    };
    assert!(matches!(zero.validate(), Err(SolverError::InvalidInput)));
    let neg = ProjectedFirstOrderConfig {
        max_iterations: 10,
        tolerance: -1.0_f64,
    };
    assert!(matches!(neg.validate(), Err(SolverError::InvalidInput)));
}

#[test]
fn config_accepts_valid() {
    let c = ProjectedFirstOrderConfig {
        max_iterations: 10,
        tolerance: 1e-6_f64,
    };
    assert!(c.validate().is_ok());
}
