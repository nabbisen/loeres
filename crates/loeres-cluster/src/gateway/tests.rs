use super::*;
use crate::runtime::{ClusterCancellationToken, ClusterSolveConfig};
use crate::solve::solve_batch;
use loeres::SolveReport;

fn solution() -> DenseVector<f64> {
    DenseVector::from_vec(vec![1.0, 2.0]).unwrap()
}

fn job(
    response: MockGatewayResponse<f64>,
    thread_safety: GatewayThreadSafety,
) -> MockGatewayJob<f64> {
    MockGatewayJob::try_new(response, thread_safety).unwrap()
}

#[test]
fn gateway_failure_mapping_uses_existing_solver_errors() {
    assert_eq!(
        solver_error_from_gateway_failure(GatewayFailureKind::BackendUnavailable),
        SolverError::BackendUnavailable
    );
    assert_eq!(
        solver_error_from_gateway_failure(GatewayFailureKind::InvalidModel),
        SolverError::InvalidInput
    );
    assert_eq!(
        solver_error_from_gateway_failure(GatewayFailureKind::NumericalFailure),
        SolverError::NumericalDomain
    );
    assert_eq!(
        solver_error_from_gateway_failure(GatewayFailureKind::Timeout),
        SolverError::Cancelled
    );
    assert_eq!(
        solver_error_from_gateway_failure(GatewayFailureKind::Cancelled),
        SolverError::Cancelled
    );
    assert_eq!(
        solver_error_from_gateway_failure(GatewayFailureKind::ForeignPanicOrAbort),
        SolverError::BackendUnavailable
    );
    assert_eq!(
        solver_error_from_gateway_failure(GatewayFailureKind::ContractViolation),
        SolverError::InternalInvariantViolation
    );
}

#[test]
fn mock_gateway_reports_solved_converged_and_not_converged() {
    let jobs: Vec<Box<dyn ClusterJob<f64>>> = vec![
        Box::new(job(
            MockGatewayResponse::Solved {
                solution: solution(),
                report: SolveReport::converged_early(2),
            },
            GatewayThreadSafety::Reentrant,
        )),
        Box::new(job(
            MockGatewayResponse::Solved {
                solution: solution(),
                report: SolveReport::not_converged_cap(9),
            },
            GatewayThreadSafety::IndependentWorkspaceOnly,
        )),
    ];
    let report = solve_batch(
        jobs,
        ClusterSolveConfig::default(),
        ClusterCancellationToken::new(),
    )
    .unwrap();
    assert_eq!(report.summary.solved_converged, 1);
    assert_eq!(report.summary.solved_not_converged, 1);
}

#[test]
fn mock_gateway_maps_failures_and_cancellation() {
    let jobs: Vec<Box<dyn ClusterJob<f64>>> = vec![
        Box::new(job(
            MockGatewayResponse::Failed(GatewayFailureKind::BackendUnavailable),
            GatewayThreadSafety::Reentrant,
        )),
        Box::new(job(
            MockGatewayResponse::Failed(GatewayFailureKind::InvalidModel),
            GatewayThreadSafety::Reentrant,
        )),
        Box::new(job(
            MockGatewayResponse::Failed(GatewayFailureKind::NumericalFailure),
            GatewayThreadSafety::Reentrant,
        )),
        Box::new(job(
            MockGatewayResponse::Failed(GatewayFailureKind::Timeout),
            GatewayThreadSafety::Reentrant,
        )),
    ];
    let report = solve_batch(
        jobs,
        ClusterSolveConfig::default(),
        ClusterCancellationToken::new(),
    )
    .unwrap();
    assert_eq!(report.summary.failed, 3);
    assert_eq!(report.summary.cancelled, 1);
    assert!(matches!(
        report.outcomes[0],
        BatchItemOutcome::Failed {
            error: SolverError::BackendUnavailable
        }
    ));
    assert!(matches!(
        report.outcomes[1],
        BatchItemOutcome::Failed {
            error: SolverError::InvalidInput
        }
    ));
    assert!(matches!(
        report.outcomes[2],
        BatchItemOutcome::Failed {
            error: SolverError::NumericalDomain
        }
    ));
    assert!(matches!(report.outcomes[3], BatchItemOutcome::Cancelled));
}

#[test]
fn mock_gateway_rejects_unwrapped_single_thread_backend() {
    let err = MockGatewayJob::try_new(
        MockGatewayResponse::Failed::<f64>(GatewayFailureKind::BackendUnavailable),
        GatewayThreadSafety::SingleThreadOnly,
    )
    .unwrap_err();
    assert_eq!(err, GatewayFailureKind::ContractViolation);
}

#[test]
fn mock_gateway_accepts_self_synchronized_backend() {
    let gateway = job(
        MockGatewayResponse::Solved {
            solution: solution(),
            report: SolveReport::converged_early(1),
        },
        GatewayThreadSafety::GloballySynchronized,
    );
    assert_eq!(
        gateway.thread_safety(),
        GatewayThreadSafety::GloballySynchronized
    );
    assert_eq!(gateway.backend_kind(), GatewayBackendKind::Mock);
}
