use std::sync::Mutex;

use super::*;
use crate::batch::ClusterSolution;
use loeres::SolveReport;
use loeres_backend_std::DenseVector;

fn dense(v: &[f64]) -> ClusterSolution<f64> {
    ClusterSolution::DenseVector(DenseVector::from_vec(v.to_vec()).unwrap())
}

fn solved(iterations: u32, converged: bool) -> BatchItemOutcome<f64> {
    let report = if converged {
        SolveReport::converged_early(iterations)
    } else {
        SolveReport::not_converged_cap(iterations)
    };
    BatchItemOutcome::Solved {
        solution: dense(&[1.0, 2.0]),
        report,
    }
}

struct FixedJob(fn() -> BatchItemOutcome<f64>);

impl ClusterJob<f64> for FixedJob {
    fn run_boxed(&self, _ctx: &crate::ClusterExecutionContext) -> BatchItemOutcome<f64> {
        (self.0)()
    }
}

#[derive(Default)]
struct RecordingObserver {
    events: Mutex<Vec<SolveTelemetryEvent>>,
}

impl RecordingObserver {
    fn events(&self) -> Vec<SolveTelemetryEvent> {
        self.events.lock().unwrap().clone()
    }
}

impl SolveObserver for RecordingObserver {
    fn observe(&self, event: &SolveTelemetryEvent) {
        self.events.lock().unwrap().push(*event);
    }
}

#[test]
fn solver_error_mapping_is_total_for_current_variants() {
    let cases = [
        (
            SolverError::DimensionMismatch { lhs: 1, rhs: 2 },
            OutcomeKind::InvalidInput,
        ),
        (SolverError::InvalidDimension, OutcomeKind::InvalidInput),
        (SolverError::InvalidInput, OutcomeKind::InvalidInput),
        (SolverError::NonFiniteInput, OutcomeKind::InvalidInput),
        (
            SolverError::UnsupportedProblemStructure,
            OutcomeKind::InvalidInput,
        ),
        (SolverError::SingularMatrix, OutcomeKind::NumericalFailure),
        (SolverError::IllConditioned, OutcomeKind::NumericalFailure),
        (SolverError::NumericalDomain, OutcomeKind::NumericalFailure),
        (SolverError::Overflow, OutcomeKind::NumericalFailure),
        (SolverError::WorkspaceTooSmall, OutcomeKind::InvalidInput),
        (SolverError::Cancelled, OutcomeKind::Cancelled),
        (SolverError::BackendUnavailable, OutcomeKind::BackendFailure),
        (
            SolverError::InternalInvariantViolation,
            OutcomeKind::InternalError,
        ),
    ];
    for (error, expected) in cases {
        assert_eq!(outcome_kind_from_solver_error(error), expected);
    }
}

#[test]
fn batch_item_mapping_preserves_status_error_split() {
    assert_eq!(
        outcome_kind_from_item(&solved(2, true)),
        OutcomeKind::SolvedConverged
    );
    assert_eq!(
        outcome_kind_from_item(&solved(9, false)),
        OutcomeKind::SolvedNotConverged
    );
    assert_eq!(
        outcome_kind_from_item::<f64>(&BatchItemOutcome::Failed {
            error: SolverError::SingularMatrix,
        }),
        OutcomeKind::NumericalFailure
    );
    assert_eq!(
        outcome_kind_from_item::<f64>(&BatchItemOutcome::Cancelled),
        OutcomeKind::Cancelled
    );
    assert_eq!(
        outcome_kind_from_item::<f64>(&BatchItemOutcome::Panicked),
        OutcomeKind::Panicked
    );
}

#[test]
fn bucket_helpers_are_coarse_and_have_unknown_fallbacks() {
    assert_eq!(DimensionBucket::from_len(0), DimensionBucket::Empty);
    assert_eq!(DimensionBucket::from_len(32), DimensionBucket::Small);
    assert_eq!(DimensionBucket::from_len(1024), DimensionBucket::Medium);
    assert_eq!(DimensionBucket::from_len(65_536), DimensionBucket::Large);
    assert_eq!(DimensionBucket::from_len(65_537), DimensionBucket::Huge);
    assert_eq!(IterationsBucket::from_count(0), IterationsBucket::None);
    assert_eq!(IterationsBucket::from_count(10), IterationsBucket::Few);
    assert_eq!(
        IterationsBucket::from_count(100),
        IterationsBucket::Moderate
    );
    assert_eq!(IterationsBucket::from_count(1_000), IterationsBucket::Many);
    assert_eq!(IterationsBucket::from_count(1_001), IterationsBucket::Maxed);
}

#[test]
fn labels_are_bounded_static_categories() {
    let labels = [
        SolverFamilyId::ProjectedFirstOrder.as_label(),
        ProblemClassId::BoxConstrainedFirstOrder.as_label(),
        DimensionBucket::Unknown.as_label(),
        IterationsBucket::Unknown.as_label(),
        ElapsedBucket::Unknown.as_label(),
        OutcomeKind::InternalError.as_label(),
    ];
    assert_eq!(
        labels,
        [
            "projected_first_order",
            "box_constrained_first_order",
            "unknown",
            "unknown",
            "unknown",
            "internal_error"
        ]
    );
    for label in labels {
        assert!(!label.contains("tenant"));
        assert!(!label.contains('/'));
        assert!(!label.contains("request"));
    }
}

#[test]
fn observe_batch_report_emits_redacted_events_in_order() {
    let report = BatchSolveReport::from_outcomes(vec![
        solved(3, true),
        BatchItemOutcome::Failed {
            error: SolverError::InternalInvariantViolation,
        },
        BatchItemOutcome::Cancelled,
    ]);
    let observer = RecordingObserver::default();
    observe_batch_report_with_metadata(
        &report,
        SolveObservationContext::projected_first_order(),
        &[DimensionBucket::Small],
        ElapsedBucket::Unknown,
        &observer,
    );
    let events = observer.events();
    assert_eq!(events.len(), 3);
    assert_eq!(events[0].solver_family, SolverFamilyId::ProjectedFirstOrder);
    assert_eq!(
        events[0].problem_class,
        ProblemClassId::BoxConstrainedFirstOrder
    );
    assert_eq!(events[0].dimension_bucket, DimensionBucket::Small);
    assert_eq!(events[0].outcome, OutcomeKind::SolvedConverged);
    assert_eq!(events[0].iterations_bucket, IterationsBucket::Few);
    assert_eq!(events[1].dimension_bucket, DimensionBucket::Unknown);
    assert_eq!(events[1].outcome, OutcomeKind::InternalError);
    assert_eq!(events[1].iterations_bucket, IterationsBucket::Unknown);
    assert_eq!(events[2].outcome, OutcomeKind::Cancelled);
    assert_eq!(events[2].elapsed_bucket, ElapsedBucket::TimeoutOrCancelled);
}

#[test]
fn solve_batch_observed_forwards_cancellation_and_emits_events_on_success() {
    let token = ClusterCancellationToken::new();
    token.cancel();
    let jobs: Vec<Box<dyn ClusterJob<f64>>> = vec![Box::new(FixedJob(|| solved(1, true)))];
    let observer = RecordingObserver::default();
    let report = solve_batch_observed(
        jobs,
        ClusterSolveConfig::default(),
        token,
        SolveObservationContext::unknown(),
        &observer,
    )
    .unwrap();
    assert_eq!(report.summary.cancelled, 1);
    let events = observer.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].solver_family, SolverFamilyId::Unknown);
    assert_eq!(events[0].outcome, OutcomeKind::Cancelled);
    assert_eq!(events[0].elapsed_bucket, ElapsedBucket::TimeoutOrCancelled);
}

#[test]
fn solve_batch_observed_does_not_emit_per_item_event_on_cluster_error() {
    let jobs: Vec<Box<dyn ClusterJob<f64>>> = vec![Box::new(FixedJob(|| solved(1, true)))];
    let config = ClusterSolveConfig {
        max_parallelism: 0,
        ..ClusterSolveConfig::default()
    };
    let observer = RecordingObserver::default();
    let err = solve_batch_observed(
        jobs,
        config,
        ClusterCancellationToken::new(),
        SolveObservationContext::unknown(),
        &observer,
    )
    .unwrap_err();
    assert_eq!(err, ClusterError::InvalidConfig);
    assert!(observer.events().is_empty());
}
