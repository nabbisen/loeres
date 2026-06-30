use super::*;
use loeres::validation::{
    FiniteCoverage, TrustToken, TrustedByCaller, ValidationCoverage, ValidationScope,
    ValidationState,
};

#[test]
fn zero_parallelism_is_invalid_config() {
    let config = ClusterSolveConfig {
        max_parallelism: 0,
        ..ClusterSolveConfig::default()
    };
    assert_eq!(config.validate(), Err(ClusterError::InvalidConfig));
}

#[test]
fn nonzero_parallelism_is_valid() {
    assert_eq!(ClusterSolveConfig::default().validate(), Ok(()));
}

#[test]
fn parallel_falls_back_to_sequential_without_feature() {
    let config = ClusterSolveConfig {
        execution_policy: BatchExecutionPolicy::Parallel,
        ..ClusterSolveConfig::default()
    };
    let expected = if cfg!(feature = "parallel-rayon") {
        BatchExecutionPolicy::Parallel
    } else {
        BatchExecutionPolicy::Sequential
    };
    assert_eq!(config.effective_execution(), expected);
}

#[test]
fn validate_all_inputs_passes_recorded_coverage() {
    let policy = ClusterValidationPolicy::ValidateAllInputs;
    let provided = ValidationState::Validated(ValidationCoverage::new(
        ValidationScope::ALL,
        FiniteCoverage::Checked,
    ));
    let resolved = policy
        .resolve(ValidationScope::PROBLEM_CONFIG, Some(provided))
        .unwrap();
    match resolved {
        ValidationState::Validated(cov) => {
            assert!(cov.scope().contains(ValidationScope::PROBLEM_CONFIG));
            assert!(cov.scope().contains(ValidationScope::FINITE));
            assert_eq!(cov.finite(), FiniteCoverage::Checked);
        }
        other => panic!("expected Validated, got {other:?}"),
    }
}

#[test]
fn validate_all_inputs_rejects_absent_validation() {
    // The resolver runs no scans; with nothing recorded it must reject rather
    // than fabricate a Validated state (B1).
    let policy = ClusterValidationPolicy::ValidateAllInputs;
    let err = policy
        .resolve(ValidationScope::PROBLEM_CONFIG, None)
        .unwrap_err();
    assert_eq!(err.covered, ValidationScope::EMPTY);
}

#[test]
fn validate_all_inputs_rejects_trust_in_lieu_of_validation() {
    let trust = TrustedByCaller::caller_assertion(ValidationScope::ALL, TrustToken::new(3), None);
    let policy = ClusterValidationPolicy::ValidateAllInputs;
    let err = policy
        .resolve(
            ValidationScope::PROBLEM_CONFIG,
            Some(ValidationState::Trusted(trust)),
        )
        .unwrap_err();
    assert_eq!(err.required, ValidationScope::PROBLEM_CONFIG);
}

#[test]
fn respect_backend_accepts_sufficient_coverage() {
    let policy = ClusterValidationPolicy::RespectBackendValidationState;
    let provided = ValidationState::Validated(ValidationCoverage::new(
        ValidationScope::ALL,
        FiniteCoverage::Checked,
    ));
    let resolved = policy
        .resolve(ValidationScope::PROBLEM_CONFIG, Some(provided))
        .unwrap();
    assert!(matches!(resolved, ValidationState::Validated(_)));
}

#[test]
fn respect_backend_rejects_missing_coverage() {
    let policy = ClusterValidationPolicy::RespectBackendValidationState;
    let provided = ValidationState::Validated(ValidationCoverage::new(
        ValidationScope::FINITE,
        FiniteCoverage::Checked,
    ));
    let err = policy
        .resolve(ValidationScope::PRELOOP, Some(provided))
        .unwrap_err();
    assert_eq!(err.required, ValidationScope::PRELOOP);
    assert!(!err.covered.contains(ValidationScope::PRELOOP));
}

#[test]
fn respect_backend_rejects_absent_state() {
    let policy = ClusterValidationPolicy::RespectBackendValidationState;
    let err = policy.resolve(ValidationScope::FINITE, None).unwrap_err();
    assert_eq!(err.covered, ValidationScope::EMPTY);
}

#[test]
fn trusted_by_caller_covers_asserted_scope() {
    let trust =
        TrustedByCaller::caller_assertion(ValidationScope::ALL, TrustToken::new(7), Some("test"));
    let policy = ClusterValidationPolicy::TrustedByCaller(trust);
    let resolved = policy
        .resolve(ValidationScope::PROBLEM_CONFIG, None)
        .unwrap();
    match resolved {
        ValidationState::Trusted(t) => assert_eq!(t.token, TrustToken::new(7)),
        other => panic!("expected Trusted, got {other:?}"),
    }
}

#[test]
fn trusted_by_caller_rejects_scope_beyond_assertion() {
    let trust =
        TrustedByCaller::caller_assertion(ValidationScope::FINITE, TrustToken::new(1), None);
    let policy = ClusterValidationPolicy::TrustedByCaller(trust);
    let err = policy.resolve(ValidationScope::PRELOOP, None).unwrap_err();
    assert_eq!(err.covered, ValidationScope::FINITE);
}
