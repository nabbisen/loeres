use super::*;
use crate::batch::ClusterSolution;
use crate::runtime::{BatchExecutionPolicy, ClusterValidationPolicy};
use loeres::validation::{FiniteCoverage, ValidationCoverage, ValidationScope, ValidationState};
use loeres::{SolveReport, SolverError};
use loeres_backend_std::DenseVector;

fn solved(converged: bool) -> BatchItemOutcome<f64> {
    let report = if converged {
        SolveReport::converged_early(2)
    } else {
        SolveReport::not_converged_cap(9)
    };
    BatchItemOutcome::Solved {
        solution: ClusterSolution::DenseVector(DenseVector::from_vec(vec![1.0, 2.0]).unwrap()),
        report,
    }
}

fn kinds(outcomes: &[BatchItemOutcome<f64>]) -> Vec<u8> {
    outcomes
        .iter()
        .map(|o| match o {
            BatchItemOutcome::Solved { report, .. } => {
                if report.status().is_converged() {
                    0
                } else {
                    1
                }
            }
            BatchItemOutcome::Failed { .. } => 2,
            BatchItemOutcome::Cancelled => 3,
            BatchItemOutcome::Panicked => 4,
        })
        .collect()
}

struct FixedJob(fn() -> BatchItemOutcome<f64>);
impl ClusterJob<f64> for FixedJob {
    fn run_boxed(&self, _ctx: &ClusterExecutionContext) -> BatchItemOutcome<f64> {
        (self.0)()
    }
}

struct PanicJob;
impl ClusterJob<f64> for PanicJob {
    fn run_boxed(&self, _ctx: &ClusterExecutionContext) -> BatchItemOutcome<f64> {
        panic!("worker boom");
    }
}

struct CancelTriggerJob(ClusterCancellationToken);
impl ClusterJob<f64> for CancelTriggerJob {
    fn run_boxed(&self, _ctx: &ClusterExecutionContext) -> BatchItemOutcome<f64> {
        self.0.cancel();
        solved(true)
    }
}

struct ValidatingJob {
    required: ValidationScope,
    provided: Option<ValidationState>,
}
impl ClusterJob<f64> for ValidatingJob {
    fn run_boxed(&self, ctx: &ClusterExecutionContext) -> BatchItemOutcome<f64> {
        match ctx
            .validation_policy()
            .resolve(self.required, self.provided)
        {
            Ok(_) => solved(true),
            Err(_) => BatchItemOutcome::Failed {
                error: SolverError::InvalidInput,
            },
        }
    }
}

#[test]
fn empty_batch_is_valid_and_empty() {
    let report = solve_batch::<f64>(
        Vec::new(),
        ClusterSolveConfig::default(),
        ClusterCancellationToken::new(),
    )
    .unwrap();
    assert!(report.outcomes.is_empty());
    assert_eq!(report.summary.total(), 0);
}

#[test]
fn zero_parallelism_is_orchestration_error() {
    let config = ClusterSolveConfig {
        max_parallelism: 0,
        ..ClusterSolveConfig::default()
    };
    let jobs: Vec<Box<dyn ClusterJob<f64>>> = vec![Box::new(FixedJob(|| solved(true)))];
    assert!(matches!(
        solve_batch(jobs, config, ClusterCancellationToken::new()),
        Err(ClusterError::InvalidConfig)
    ));
}

#[test]
fn mixed_batch_isolates_each_item() {
    let jobs: Vec<Box<dyn ClusterJob<f64>>> = vec![
        Box::new(FixedJob(|| solved(true))),
        Box::new(FixedJob(|| solved(false))),
        Box::new(FixedJob(|| BatchItemOutcome::Failed {
            error: SolverError::SingularMatrix,
        })),
        Box::new(PanicJob),
        Box::new(FixedJob(|| BatchItemOutcome::Failed {
            error: SolverError::Cancelled,
        })),
    ];
    let report = solve_batch(
        jobs,
        ClusterSolveConfig::default(),
        ClusterCancellationToken::new(),
    )
    .unwrap();
    assert_eq!(report.summary.solved_converged, 1);
    assert_eq!(report.summary.solved_not_converged, 1);
    assert_eq!(report.summary.failed, 1);
    assert_eq!(report.summary.panicked, 1);
    assert_eq!(report.summary.cancelled, 1);
    // Valid items are unaffected by sibling failure/panic.
    assert!(matches!(
        report.outcomes[0],
        BatchItemOutcome::Solved { .. }
    ));
    assert!(matches!(report.outcomes[4], BatchItemOutcome::Cancelled));
}

#[test]
fn mid_batch_cancellation_cancels_remaining() {
    let token = ClusterCancellationToken::new();
    let jobs: Vec<Box<dyn ClusterJob<f64>>> = vec![
        Box::new(CancelTriggerJob(token.clone())),
        Box::new(FixedJob(|| solved(true))),
        Box::new(FixedJob(|| solved(true))),
    ];
    let report = solve_batch(jobs, ClusterSolveConfig::default(), token).unwrap();
    assert!(matches!(
        report.outcomes[0],
        BatchItemOutcome::Solved { .. }
    ));
    assert_eq!(report.summary.cancelled, 2);
}

#[test]
fn panic_isolation_leaves_siblings_solved() {
    let jobs: Vec<Box<dyn ClusterJob<f64>>> = vec![
        Box::new(FixedJob(|| solved(true))),
        Box::new(PanicJob),
        Box::new(FixedJob(|| solved(true))),
    ];
    let report = solve_batch(
        jobs,
        ClusterSolveConfig::default(),
        ClusterCancellationToken::new(),
    )
    .unwrap();
    assert_eq!(report.summary.solved_converged, 2);
    assert_eq!(report.summary.panicked, 1);
    assert!(matches!(report.outcomes[1], BatchItemOutcome::Panicked));
}

#[test]
fn validate_all_inputs_policy_admits_item() {
    let config = ClusterSolveConfig {
        validation_policy: ClusterValidationPolicy::ValidateAllInputs,
        ..ClusterSolveConfig::default()
    };
    let jobs: Vec<Box<dyn ClusterJob<f64>>> = vec![Box::new(ValidatingJob {
        required: ValidationScope::PROBLEM_CONFIG,
        // The validating job ran its scan and recorded coverage; the resolver
        // verifies (it does not scan).
        provided: Some(ValidationState::Validated(ValidationCoverage::new(
            ValidationScope::PROBLEM_CONFIG,
            FiniteCoverage::Checked,
        ))),
    })];
    let report = solve_batch(jobs, config, ClusterCancellationToken::new()).unwrap();
    assert_eq!(report.summary.solved_converged, 1);
}

#[test]
fn respect_backend_policy_rejects_uncovered_item() {
    let config = ClusterSolveConfig {
        validation_policy: ClusterValidationPolicy::RespectBackendValidationState,
        ..ClusterSolveConfig::default()
    };
    let jobs: Vec<Box<dyn ClusterJob<f64>>> = vec![Box::new(ValidatingJob {
        required: ValidationScope::PRELOOP,
        provided: None,
    })];
    let report = solve_batch(jobs, config, ClusterCancellationToken::new()).unwrap();
    assert_eq!(report.summary.failed, 1);
    assert!(matches!(
        report.outcomes[0],
        BatchItemOutcome::Failed {
            error: SolverError::InvalidInput
        }
    ));
}

#[cfg(feature = "parallel-rayon")]
#[test]
fn parallel_matches_sequential() {
    let build = || -> Vec<Box<dyn ClusterJob<f64>>> {
        vec![
            Box::new(FixedJob(|| solved(true))),
            Box::new(FixedJob(|| solved(false))),
            Box::new(FixedJob(|| BatchItemOutcome::Failed {
                error: SolverError::SingularMatrix,
            })),
        ]
    };
    let seq = solve_batch(
        build(),
        ClusterSolveConfig::default(),
        ClusterCancellationToken::new(),
    )
    .unwrap();
    let par_config = ClusterSolveConfig {
        execution_policy: BatchExecutionPolicy::Parallel,
        max_parallelism: 4,
        ..ClusterSolveConfig::default()
    };
    let par = solve_batch(build(), par_config, ClusterCancellationToken::new()).unwrap();
    assert_eq!(kinds(&seq.outcomes), kinds(&par.outcomes));
    assert_eq!(seq.summary, par.summary);
}

#[cfg(feature = "async-tokio")]
#[test]
fn async_matches_sync() {
    let build = || -> Vec<Box<dyn ClusterJob<f64>>> {
        vec![
            Box::new(FixedJob(|| solved(true))),
            Box::new(FixedJob(|| solved(false))),
        ]
    };
    let sync = solve_batch(
        build(),
        ClusterSolveConfig::default(),
        ClusterCancellationToken::new(),
    )
    .unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let asynced = runtime
        .block_on(solve_batch_async(
            build(),
            ClusterSolveConfig::default(),
            ClusterCancellationToken::new(),
        ))
        .unwrap();
    assert_eq!(kinds(&sync.outcomes), kinds(&asynced.outcomes));
    assert_eq!(sync.summary, asynced.summary);
}
