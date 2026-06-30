use super::*;

#[test]
fn non_finite_initial_x_under_validate_all_inputs() {
    let p = Quadratic {
        weights: vec![1.0],
        centers: vec![0.0],
        lo: dv(&[-1.0]),
        hi: dv(&[1.0]),
        alpha: 0.5,
    };
    let mut x = dv(&[f64::NAN]);
    let mut ws = ClusterProjectedFirstOrderWorkspace::new(1).unwrap();
    let err = solve_projected_first_order_dyn(
        &p,
        &mut x,
        &mut ws,
        &cfg(10, 1e-9),
        &ctx(ClusterValidationPolicy::ValidateAllInputs),
    )
    .unwrap_err();
    assert!(matches!(err, SolverError::NonFiniteInput));
}

#[test]
fn non_finite_bound_under_validate_all_inputs() {
    let p = Quadratic {
        weights: vec![1.0],
        centers: vec![0.0],
        lo: dv(&[-1.0]),
        hi: dv(&[f64::NAN]),
        alpha: 0.5,
    };
    let mut x = dv(&[0.0]);
    let mut ws = ClusterProjectedFirstOrderWorkspace::new(1).unwrap();
    let err = solve_projected_first_order_dyn(
        &p,
        &mut x,
        &mut ws,
        &cfg(10, 1e-9),
        &ctx(ClusterValidationPolicy::ValidateAllInputs),
    )
    .unwrap_err();
    assert!(matches!(err, SolverError::NonFiniteInput));
}

#[test]
fn non_finite_bound_under_trust_is_numerical_domain() {
    // FINITE trusted away -> pre-loop scan skipped; the non-finite bound is caught
    // in the hot loop as NumericalDomain, never Solved (C3).
    let p = Quadratic {
        weights: vec![1.0],
        centers: vec![0.0],
        lo: dv(&[-1.0]),
        hi: dv(&[f64::NAN]),
        alpha: 0.5,
    };
    let trust = TrustedByCaller::caller_assertion(ValidationScope::ALL, TrustToken::new(3), None);
    let mut x = dv(&[0.0]);
    let mut ws = ClusterProjectedFirstOrderWorkspace::new(1).unwrap();
    let err = solve_projected_first_order_dyn(
        &p,
        &mut x,
        &mut ws,
        &cfg(10, 1e-9),
        &ctx(ClusterValidationPolicy::TrustedByCaller(trust)),
    )
    .unwrap_err();
    assert!(matches!(err, SolverError::NumericalDomain));
}

#[test]
fn non_finite_step_scale_is_non_finite_input() {
    let p = Quadratic {
        weights: vec![1.0],
        centers: vec![0.0],
        lo: dv(&[-1.0]),
        hi: dv(&[1.0]),
        alpha: f64::NAN,
    };
    let mut x = dv(&[0.0]);
    let mut ws = ClusterProjectedFirstOrderWorkspace::new(1).unwrap();
    let err = solve_projected_first_order_dyn(
        &p,
        &mut x,
        &mut ws,
        &cfg(10, 1e-9),
        &ctx(ClusterValidationPolicy::ValidateAllInputs),
    )
    .unwrap_err();
    assert!(matches!(err, SolverError::NonFiniteInput));
}

#[test]
fn non_positive_step_scale_is_invalid_input() {
    let p = Quadratic {
        weights: vec![1.0],
        centers: vec![0.0],
        lo: dv(&[-1.0]),
        hi: dv(&[1.0]),
        alpha: -0.5,
    };
    let mut x = dv(&[0.0]);
    let mut ws = ClusterProjectedFirstOrderWorkspace::new(1).unwrap();
    let err = solve_projected_first_order_dyn(
        &p,
        &mut x,
        &mut ws,
        &cfg(10, 1e-9),
        &ctx(ClusterValidationPolicy::ValidateAllInputs),
    )
    .unwrap_err();
    assert!(matches!(err, SolverError::InvalidInput));
}

#[test]
fn respect_backend_validation_state_scans_here() {
    // v1: no provided-state channel, so this policy scans here and a valid problem
    // solves with finite evidence Scanned (B2; provided/cached state is RFC 015-owned).
    let p = Quadratic {
        weights: vec![1.0],
        centers: vec![2.0],
        lo: dv(&[-10.0]),
        hi: dv(&[10.0]),
        alpha: 0.5,
    };
    let mut x = dv(&[0.0]);
    let mut ws = ClusterProjectedFirstOrderWorkspace::new(1).unwrap();
    let rec = solve_projected_first_order_dyn(
        &p,
        &mut x,
        &mut ws,
        &cfg(1000, 1e-9),
        &ctx(ClusterValidationPolicy::RespectBackendValidationState),
    )
    .unwrap();
    assert!(rec.report.status().is_converged());
    assert_eq!(rec.finite, ProjectedFirstOrderFiniteEvidence::Scanned);
}
