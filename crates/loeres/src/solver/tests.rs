//! Tests for the RFC 014 solver outcome/status taxonomy.
//!
//! These validate the **design specification**: the status/error split, the
//! size & representation budgets (§4.1), the four valid status/termination
//! combinations and the unconstructability of the invalid ones (§3.3/§6.2), the
//! `AsCoreReport` round-trip (§6.4), and the RFC 003 reconciliation (§6.6).

use crate::error::{SolverError, error_code_to_str};
use crate::solver::{
    AsCoreReport, IterationReport, SolveReport, SolveStatus, StepOutcome, TerminationReason,
};

#[test]
fn size_budgets() {
    use core::mem::size_of;
    assert!(size_of::<StepOutcome>() <= 2);
    assert!(size_of::<SolveStatus>() <= 2);
    assert!(size_of::<TerminationReason>() <= 2);
    assert!(size_of::<IterationReport>() <= 12);
    assert!(size_of::<SolveReport>() <= 16);
}

#[test]
fn data_free_enums_are_one_byte_under_repr_u8() {
    use core::mem::size_of;
    assert_eq!(size_of::<StepOutcome>(), 1);
    assert_eq!(size_of::<SolveStatus>(), 1);
    assert_eq!(size_of::<TerminationReason>(), 1);
}

#[test]
fn step_outcomes_are_distinct() {
    assert_ne!(StepOutcome::Continue, StepOutcome::Converged);
    assert_ne!(StepOutcome::Converged, StepOutcome::NoProgress);
    assert_ne!(StepOutcome::Continue, StepOutcome::NoProgress);
}

#[test]
fn is_converged_reflects_status() {
    assert!(SolveStatus::Converged.is_converged());
    assert!(!SolveStatus::NotConverged.is_converged());
}

/// Each constructor yields exactly one of the four valid (status, termination)
/// pairs from §3.3; there are exactly four, and the two invalid pairs
/// (`Converged + NoProgress`, `NotConverged + ConvergenceCriterion`) have no
/// constructor.
#[test]
fn constructors_cover_the_four_valid_combinations() {
    let pairs = [
        SolveReport::converged_early(5),
        SolveReport::converged_at_cap(100),
        SolveReport::not_converged_cap(100),
        SolveReport::not_converged_stalled(7),
    ]
    .map(|r| (r.status(), r.termination()));

    use SolveStatus::*;
    use TerminationReason::*;
    assert_eq!(pairs[0], (Converged, ConvergenceCriterion));
    assert_eq!(pairs[1], (Converged, IterationCap));
    assert_eq!(pairs[2], (NotConverged, IterationCap));
    assert_eq!(pairs[3], (NotConverged, NoProgress));

    // No constructor produces an invalid combination.
    for (status, termination) in pairs {
        let valid = matches!(
            (status, termination),
            (Converged, ConvergenceCriterion)
                | (Converged, IterationCap)
                | (NotConverged, IterationCap)
                | (NotConverged, NoProgress)
        );
        assert!(valid, "invalid pair produced: {status:?} + {termination:?}");
    }
}

#[test]
fn report_accessors_round_trip_fields() {
    let r = SolveReport::not_converged_cap(42);
    assert_eq!(r.status(), SolveStatus::NotConverged);
    assert_eq!(r.termination(), TerminationReason::IterationCap);
    assert_eq!(r.iterations_executed(), 42);
    assert_eq!(
        r.iteration(),
        IterationReport::new(42, TerminationReason::IterationCap)
    );
    assert_eq!(r.iteration().iterations_executed(), 42);
    assert_eq!(r.iteration().termination(), TerminationReason::IterationCap);
}

/// A reference report type that wraps the core report — stands in for the device
/// (`DeviceSolveReport`, RFC 006) and cluster (RFC 008) reports, which land with
/// their crates. Proves `AsCoreReport` is implementable and lossless (§6.4).
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
struct WrapReport {
    core: SolveReport,
}

impl AsCoreReport for WrapReport {
    fn as_core_report(&self) -> SolveReport {
        self.core
    }
}

#[test]
fn as_core_report_is_lossless_for_every_valid_report() {
    let reports = [
        SolveReport::converged_early(3),
        SolveReport::converged_at_cap(64),
        SolveReport::not_converged_cap(64),
        SolveReport::not_converged_stalled(9),
    ];
    for core in reports {
        let wrapped = WrapReport { core };
        assert_eq!(wrapped.as_core_report(), core, "status/termination lost");
        assert_eq!(wrapped.core_status(), core.status());
    }
}

#[test]
fn converged_at_cap_separates_status_from_termination() {
    // Constant-iteration mode: converged early but ran to the cap.
    let r = SolveReport::converged_at_cap(100);
    assert!(r.status().is_converged());
    assert_eq!(r.termination(), TerminationReason::IterationCap);
}

#[test]
fn non_convergence_is_a_status_not_an_error() {
    // RFC 014 §5.2 / §6.6: a max-iteration run is `Ok(not_converged_cap)`,
    // never an `Err`. The taxonomy expresses it as a report, not a SolverError.
    let report = SolveReport::not_converged_cap(256);
    assert_eq!(report.status(), SolveStatus::NotConverged);

    // And the canonical error set carries no non-convergence category.
    let error_codes: Vec<&str> = [
        SolverError::DimensionMismatch { lhs: 1, rhs: 2 },
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
    .iter()
    .copied()
    .map(error_code_to_str)
    .collect();
    assert!(!error_codes.iter().any(|c| c.contains("converged")));
    assert!(!error_codes.iter().any(|c| c.contains("iteration")));
    assert!(!error_codes.contains(&"panic_gate_violation"));
}
