use super::*;
use loeres::SolveReport;

fn dense(v: &[f64]) -> ClusterSolution<f64> {
    ClusterSolution::DenseVector(DenseVector::from_vec(v.to_vec()).unwrap())
}

fn solved(converged: bool) -> BatchItemOutcome<f64> {
    let report = if converged {
        SolveReport::converged_early(3)
    } else {
        SolveReport::not_converged_cap(10)
    };
    BatchItemOutcome::Solved {
        solution: dense(&[1.0, 2.0]),
        report,
        constrained: None,
    }
}

#[test]
fn empty_report_has_zero_summary() {
    let report = BatchSolveReport::<f64>::empty();
    assert!(report.outcomes.is_empty());
    assert_eq!(report.summary, BatchSummary::default());
    assert_eq!(report.summary.total(), 0);
}

#[test]
fn from_outcomes_tallies_each_category() {
    let outcomes = vec![
        solved(true),
        solved(false),
        solved(true),
        BatchItemOutcome::Failed {
            error: loeres::SolverError::SingularMatrix,
        },
        BatchItemOutcome::Cancelled,
        BatchItemOutcome::Panicked,
    ];
    let report = BatchSolveReport::from_outcomes(outcomes);
    assert_eq!(report.summary.solved_converged, 2);
    assert_eq!(report.summary.solved_not_converged, 1);
    assert_eq!(report.summary.failed, 1);
    assert_eq!(report.summary.cancelled, 1);
    assert_eq!(report.summary.panicked, 1);
    assert_eq!(report.summary.total(), 6);
}

#[test]
fn outcomes_preserve_submission_order() {
    let outcomes = vec![solved(true), BatchItemOutcome::Cancelled, solved(false)];
    let report = BatchSolveReport::from_outcomes(outcomes);
    assert!(matches!(
        report.outcomes[0],
        BatchItemOutcome::Solved { .. }
    ));
    assert!(matches!(report.outcomes[1], BatchItemOutcome::Cancelled));
    assert!(matches!(
        report.outcomes[2],
        BatchItemOutcome::Solved { .. }
    ));
}

// RFC 042: the batch seam carries the constrained detail too.

#[test]
fn constrained_detail_is_none_for_an_item_with_no_linear_inequalities() {
    // `solved` builds a box-only outcome: `constrained_detail()` must be
    // `None`, not `Some` with a zero violation — there was no polyhedron to
    // check at all.
    assert!(solved(true).constrained_detail().is_none());
}

#[test]
fn constrained_detail_distinguishes_no_constraints_from_constraints_satisfied() {
    let no_constraints = solved(true);
    let satisfied = BatchItemOutcome::Solved {
        solution: dense(&[1.0, 2.0]),
        report: SolveReport::converged_early(3),
        constrained: Some(ConstrainedBatchDetail {
            projection_cap_hits: 0,
            max_constraint_violation: 0.0,
            infeasibility_evidence: false,
        }),
    };
    assert!(no_constraints.constrained_detail().is_none());
    let detail = satisfied.constrained_detail().expect("had constraints");
    assert_eq!(detail.max_constraint_violation, 0.0);
    assert_eq!(detail.projection_cap_hits, 0);
    assert!(!detail.infeasibility_evidence);
}

#[test]
fn constrained_detail_is_none_for_every_non_solved_variant() {
    let failed = BatchItemOutcome::<f64>::Failed {
        error: loeres::SolverError::SingularMatrix,
    };
    assert!(failed.constrained_detail().is_none());
    assert!(
        BatchItemOutcome::<f64>::Cancelled
            .constrained_detail()
            .is_none()
    );
    assert!(
        BatchItemOutcome::<f64>::Panicked
            .constrained_detail()
            .is_none()
    );
}

#[test]
fn constrained_detail_distinguishes_rfc_039s_two_non_convergence_mechanisms() {
    // Exit criterion 1a: a batch caller must be able to tell RFC 039's
    // projection-cap-bound non-convergence (nonzero cap hits) apart from the
    // outer-iteration-cap-bound one (zero cap hits) from the outcome alone.
    let projection_cap_bound = BatchItemOutcome::Solved {
        solution: dense(&[0.5]),
        report: SolveReport::not_converged_stalled(7),
        constrained: Some(ConstrainedBatchDetail {
            projection_cap_hits: 3,
            max_constraint_violation: 2.6e-5,
            infeasibility_evidence: false,
        }),
    };
    let iteration_cap_bound = BatchItemOutcome::Solved {
        solution: dense(&[1.5e4]),
        report: SolveReport::not_converged_cap(50_000),
        constrained: Some(ConstrainedBatchDetail {
            projection_cap_hits: 0,
            max_constraint_violation: 0.0,
            infeasibility_evidence: false,
        }),
    };
    assert_eq!(
        projection_cap_bound
            .constrained_detail()
            .unwrap()
            .projection_cap_hits,
        3
    );
    assert_eq!(
        iteration_cap_bound
            .constrained_detail()
            .unwrap()
            .projection_cap_hits,
        0
    );
}
