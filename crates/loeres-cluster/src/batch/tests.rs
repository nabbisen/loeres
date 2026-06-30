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
