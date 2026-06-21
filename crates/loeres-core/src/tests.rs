//! Tests for the RFC 003 error topology.
//!
//! These validate the **design specification** — the canonical variant set,
//! the size budgets, the stable code mapping, and the classification contract —
//! not merely the current code shape.

use crate::diagnostic::{DiagnosticCode, DiagnosticSnapshot};
use crate::error::{SolverError, error_code_to_str};

/// The canonical RFC 003 error set. Pins the public surface: a stray variant
/// (e.g. a `MaxIterationsReached`) would have to be added here and would also
/// fail the `error_code_to_str` exhaustiveness check inside the crate.
fn all_errors() -> [SolverError; 13] {
    [
        SolverError::DimensionMismatch { lhs: 3, rhs: 4 },
        SolverError::InvalidDimension,
        SolverError::InvalidInput,
        SolverError::NonFiniteInput,
        SolverError::UnsupportedProblemStructure,
        SolverError::SingularMatrix,
        SolverError::IllConditioned,
        SolverError::NumericalDomain,
        SolverError::Overflow,
        SolverError::WorkspaceTooSmall,
        SolverError::Cancelled,
        SolverError::BackendUnavailable,
        SolverError::InternalInvariantViolation,
    ]
}

#[test]
fn solver_error_within_size_budget() {
    // RFC 003 §3.3
    assert!(core::mem::size_of::<SolverError>() <= 16);
}

#[test]
fn diagnostic_snapshot_within_size_budget() {
    // RFC 003 §3.4
    assert!(core::mem::size_of::<DiagnosticSnapshot>() <= 16);
}

#[test]
fn every_error_maps_to_nonempty_snake_case_code() {
    for e in all_errors() {
        let s = error_code_to_str(e);
        assert!(!s.is_empty(), "empty code for {e:?}");
        assert!(
            s.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
            "non snake_case code {s:?} for {e:?}"
        );
    }
}

#[test]
fn error_codes_are_unique() {
    let codes: Vec<&str> = all_errors()
        .iter()
        .copied()
        .map(error_code_to_str)
        .collect();
    let mut deduped = codes.clone();
    deduped.sort_unstable();
    deduped.dedup();
    assert_eq!(
        deduped.len(),
        codes.len(),
        "duplicate error codes: {codes:?}"
    );
}

#[test]
fn error_codes_are_stable() {
    // Pin representative strings so a rename is a conscious, reviewed change.
    assert_eq!(
        error_code_to_str(SolverError::NonFiniteInput),
        "non_finite_input"
    );
    assert_eq!(
        error_code_to_str(SolverError::DimensionMismatch { lhs: 1, rhs: 2 }),
        "dimension_mismatch"
    );
    assert_eq!(
        error_code_to_str(SolverError::InternalInvariantViolation),
        "internal_invariant_violation"
    );
}

#[test]
fn dimension_mismatch_preserves_payload() {
    let e = SolverError::DimensionMismatch { lhs: 7, rhs: 9 };
    let SolverError::DimensionMismatch { lhs, rhs } = e else {
        panic!("unexpected variant: {e:?}");
    };
    assert_eq!((lhs, rhs), (7, 9));
}

#[test]
fn classification_helpers_are_mutually_exclusive() {
    for e in all_errors() {
        let groups = [
            e.is_input_error(),
            e.is_numerical_error(),
            e.is_resource_error(),
        ]
        .into_iter()
        .filter(|b| *b)
        .count();
        assert!(groups <= 1, "{e:?} classified into multiple groups");
    }
}

#[test]
fn classification_groups_match_spec_intent() {
    assert!(SolverError::NonFiniteInput.is_input_error());
    assert!(SolverError::DimensionMismatch { lhs: 0, rhs: 1 }.is_input_error());
    assert!(SolverError::SingularMatrix.is_numerical_error());
    assert!(SolverError::Overflow.is_numerical_error());
    assert!(SolverError::WorkspaceTooSmall.is_resource_error());
    assert!(SolverError::Cancelled.is_resource_error());
    // Capability mismatch and internal bugs belong to no caller-facing group.
    assert!(!SolverError::UnsupportedProblemStructure.is_input_error());
    assert!(!SolverError::InternalInvariantViolation.is_resource_error());
}

#[test]
fn error_implements_debug() {
    // `Debug` is required (RFC 003 §6.5); confirm it renders the variant name.
    assert_eq!(format!("{:?}", SolverError::Overflow), "Overflow");
}

#[test]
fn non_convergence_is_not_an_error() {
    // RFC 014: non-convergence is `Ok(SolveReport { NotConverged, .. })`, never
    // a `SolverError`. Pin the set so such a variant cannot creep in silently.
    let codes: std::collections::BTreeSet<&str> = all_errors()
        .iter()
        .copied()
        .map(error_code_to_str)
        .collect();
    assert_eq!(codes.len(), 13);
    assert!(!codes.contains("max_iterations_reached"));
    assert!(!codes.contains("not_converged"));
}

#[test]
fn diagnostic_default_is_empty() {
    let d = DiagnosticSnapshot::default();
    assert_eq!(d, DiagnosticSnapshot::EMPTY);
    assert_eq!(d.code, DiagnosticCode::None);
    assert_eq!((d.iteration, d.primary_index, d.secondary_index), (0, 0, 0));
}

#[test]
fn diagnostic_snapshot_is_constructible_and_copy() {
    let d = DiagnosticSnapshot {
        code: DiagnosticCode::IterationLimit,
        iteration: 128,
        primary_index: 4,
        secondary_index: 0,
    };
    let copied = d;
    assert_eq!(copied, d);
    assert_eq!(copied.code, DiagnosticCode::IterationLimit);
}
