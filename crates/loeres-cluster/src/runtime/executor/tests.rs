use super::*;
use crate::batch::ClusterSolution;
use crate::runtime::{ClusterCancellationToken, ClusterSolveConfig, ClusterValidationPolicy};
use crate::solve::{ClusterExecutionContext, ClusterJob};
use loeres::{SolveReport, SolverError};
use loeres_backend_std::DenseVector;
use std::time::Duration;

struct FixedJob(BatchItemOutcome<f64>);
impl ClusterJob<f64> for FixedJob {
    fn run_boxed(&self, _ctx: &ClusterExecutionContext) -> BatchItemOutcome<f64> {
        self.0.clone()
    }
}

struct PanicJob;
impl ClusterJob<f64> for PanicJob {
    fn run_boxed(&self, _ctx: &ClusterExecutionContext) -> BatchItemOutcome<f64> {
        panic!("worker boom");
    }
}

fn ctx(token: &ClusterCancellationToken) -> ClusterExecutionContext {
    ClusterExecutionContext::new(token.clone(), 0, ClusterValidationPolicy::ValidateAllInputs)
}

fn solved() -> BatchItemOutcome<f64> {
    BatchItemOutcome::Solved {
        solution: ClusterSolution::DenseVector(DenseVector::from_vec(vec![1.0]).unwrap()),
        report: SolveReport::converged_early(1),
    }
}

#[test]
fn panic_is_contained_as_panicked() {
    let token = ClusterCancellationToken::new();
    let jobs: Vec<Box<dyn ClusterJob<f64>>> = vec![Box::new(PanicJob)];
    let report = execute_sequential(&jobs, &ctx(&token), &token, &ClusterSolveConfig::default());
    assert!(matches!(report.outcomes[0], BatchItemOutcome::Panicked));
    assert_eq!(report.summary.panicked, 1);
}

#[test]
fn pre_cancelled_token_yields_all_cancelled() {
    let token = ClusterCancellationToken::new();
    token.cancel();
    let jobs: Vec<Box<dyn ClusterJob<f64>>> =
        vec![Box::new(FixedJob(solved())), Box::new(FixedJob(solved()))];
    let report = execute_sequential(&jobs, &ctx(&token), &token, &ClusterSolveConfig::default());
    assert_eq!(report.summary.cancelled, 2);
}

#[test]
fn inner_solver_cancelled_is_normalized_to_cancelled() {
    let token = ClusterCancellationToken::new();
    let jobs: Vec<Box<dyn ClusterJob<f64>>> = vec![Box::new(FixedJob(BatchItemOutcome::Failed {
        error: SolverError::Cancelled,
    }))];
    let report = execute_sequential(&jobs, &ctx(&token), &token, &ClusterSolveConfig::default());
    assert!(matches!(report.outcomes[0], BatchItemOutcome::Cancelled));
    assert_eq!(report.summary.failed, 0);
}

#[test]
fn other_solver_error_stays_failed() {
    let token = ClusterCancellationToken::new();
    let jobs: Vec<Box<dyn ClusterJob<f64>>> = vec![Box::new(FixedJob(BatchItemOutcome::Failed {
        error: SolverError::SingularMatrix,
    }))];
    let report = execute_sequential(&jobs, &ctx(&token), &token, &ClusterSolveConfig::default());
    assert!(matches!(
        report.outcomes[0],
        BatchItemOutcome::Failed { .. }
    ));
}

#[test]
fn elapsed_timeout_cancels_items() {
    let token = ClusterCancellationToken::new();
    let config = ClusterSolveConfig {
        timeout: Some(Duration::ZERO),
        ..ClusterSolveConfig::default()
    };
    let jobs: Vec<Box<dyn ClusterJob<f64>>> = vec![Box::new(FixedJob(solved()))];
    let report = execute_sequential(&jobs, &ctx(&token), &token, &config);
    assert!(matches!(report.outcomes[0], BatchItemOutcome::Cancelled));
}
