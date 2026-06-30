//! Tests for the validation-state vocabulary (RFC 012).

use super::{
    FiniteCoverage, TrustKind, TrustToken, TrustedByCaller, ValidationCoverage, ValidationScope,
    ValidationState,
};

#[test]
fn scope_bits_are_distinct() {
    assert_ne!(ValidationScope::FINITE, ValidationScope::PROBLEM_CONFIG);
    assert_ne!(ValidationScope::PROBLEM_CONFIG, ValidationScope::PRELOOP);
    assert_ne!(ValidationScope::FINITE, ValidationScope::PRELOOP);
    assert_eq!(ValidationScope::empty(), ValidationScope::EMPTY);
}

#[test]
fn scope_union_intersect_contains() {
    let fc = ValidationScope::FINITE.union(ValidationScope::PROBLEM_CONFIG);
    assert!(fc.contains(ValidationScope::FINITE));
    assert!(fc.contains(ValidationScope::PROBLEM_CONFIG));
    assert!(!fc.contains(ValidationScope::PRELOOP));
    assert_eq!(
        fc.intersect(ValidationScope::FINITE),
        ValidationScope::FINITE
    );
    assert!(!ValidationScope::empty().contains(ValidationScope::FINITE));
    // operator conveniences agree with the named methods
    assert_eq!(
        ValidationScope::FINITE | ValidationScope::PRELOOP,
        ValidationScope::FINITE.union(ValidationScope::PRELOOP)
    );
    assert_eq!(fc & ValidationScope::FINITE, ValidationScope::FINITE);
}

#[test]
fn all_is_release_local_composition() {
    // ALL is composed from the current bits, not assumed to be 0xff.
    assert_eq!(
        ValidationScope::ALL,
        ValidationScope::FINITE
            .union(ValidationScope::PROBLEM_CONFIG)
            .union(ValidationScope::PRELOOP)
    );
    assert!(ValidationScope::ALL.contains(ValidationScope::FINITE));
    assert!(ValidationScope::ALL.contains(ValidationScope::PROBLEM_CONFIG));
    assert!(ValidationScope::ALL.contains(ValidationScope::PRELOOP));
}

#[test]
fn finite_coverage_is_distinct() {
    assert_ne!(FiniteCoverage::Checked, FiniteCoverage::NotApplicable);
}

#[test]
fn trust_token_roundtrip() {
    let t = TrustToken::new(42);
    assert_eq!(t.value(), 42);
}

#[test]
fn coverage_records_scope_and_finite() {
    let c = ValidationCoverage::new(ValidationScope::ALL, FiniteCoverage::Checked);
    assert_eq!(c.scope(), ValidationScope::ALL);
    assert_eq!(c.finite(), FiniteCoverage::Checked);
    let na = ValidationCoverage::new(ValidationScope::PRELOOP, FiniteCoverage::NotApplicable);
    assert_eq!(na.finite(), FiniteCoverage::NotApplicable);
}

#[test]
fn coverage_normalizes_finite_into_scope() {
    // Even a scope without FINITE is normalized to include it, so the scope bit
    // and the `finite` field can never contradict.
    let c = ValidationCoverage::new(ValidationScope::PRELOOP, FiniteCoverage::Checked);
    assert!(c.scope().contains(ValidationScope::FINITE));
    assert!(c.scope().contains(ValidationScope::PRELOOP));
    let na = ValidationCoverage::new(ValidationScope::empty(), FiniteCoverage::NotApplicable);
    assert!(na.scope().contains(ValidationScope::FINITE));
}

#[test]
fn trusted_by_caller_makes_scope_visible() {
    let trust = TrustedByCaller::caller_assertion(
        ValidationScope::FINITE,
        TrustToken::new(7),
        Some("ingest-pipeline-a"),
    );
    assert_eq!(trust.kind, TrustKind::CallerAssertion);
    assert_eq!(trust.scope, ValidationScope::FINITE);
    assert_eq!(trust.token.value(), 7);
    assert_eq!(trust.label, Some("ingest-pipeline-a"));
}

#[test]
fn state_variants() {
    let unval = ValidationState::Unvalidated;
    let val = ValidationState::Validated(ValidationCoverage::new(
        ValidationScope::ALL,
        FiniteCoverage::Checked,
    ));
    let trusted = ValidationState::Trusted(TrustedByCaller::caller_assertion(
        ValidationScope::ALL,
        TrustToken::new(1),
        None,
    ));
    assert_ne!(unval, val);
    assert_ne!(val, trusted);
    match val {
        ValidationState::Validated(c) => assert_eq!(c.finite(), FiniteCoverage::Checked),
        _ => panic!("expected Validated"),
    }
}
