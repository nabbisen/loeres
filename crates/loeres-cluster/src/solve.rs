//! Public solve entrypoints and the hybrid dispatch barrier (RFC 008 §3.3 / §3.5).
//!
//! [`ClusterJob`] is the stable seam where a future std-side numerical kernel
//! plugs in. In v0.13.0 there is no production kernel for dynamic storage (core
//! exposes only the outcome vocabulary; the device solver is edge-only), so the
//! orchestration machinery here is exercised by deterministic test jobs. A job
//! produces a per-item [`BatchItemOutcome`]; solver-domain failures belong in
//! that outcome, never in a top-level [`ClusterError`].

use crate::batch::{BatchItemOutcome, BatchSolveReport};
use crate::runtime::executor;
use crate::runtime::{
    BatchExecutionPolicy, ClusterCancellationToken, ClusterError, ClusterSolveConfig,
    ClusterValidationPolicy,
};

/// Per-item runtime context handed to each [`ClusterJob`].
///
/// Runtime-agnostic and shareable across workers: it carries the cooperative
/// cancellation handle, the cancellation poll interval, and the validation
/// policy (RFC 012 vocabulary) the job should apply to its input.
#[derive(Clone, Debug)]
pub struct ClusterExecutionContext {
    cancel: ClusterCancellationToken,
    poll_interval: u32,
    validation_policy: ClusterValidationPolicy,
}

impl ClusterExecutionContext {
    /// Build a context from the shared cancellation handle, poll interval, and
    /// validation policy.
    #[must_use]
    pub fn new(
        cancel: ClusterCancellationToken,
        poll_interval: u32,
        validation_policy: ClusterValidationPolicy,
    ) -> Self {
        Self {
            cancel,
            poll_interval,
            validation_policy,
        }
    }

    /// Whether cooperative cancellation has been requested.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancel.is_cancelled()
    }

    /// How often (in items) to poll cancellation; `0` means every item.
    #[must_use]
    pub fn poll_interval(&self) -> u32 {
        self.poll_interval
    }

    /// The validation policy to apply to this item's input.
    #[must_use]
    pub fn validation_policy(&self) -> ClusterValidationPolicy {
        self.validation_policy
    }
}

/// A unit of cluster work behind the dynamic dispatch barrier (RFC 008 §3.3).
///
/// Erasing a monomorphized kernel into `dyn ClusterJob<S>` bounds code bloat at
/// the orchestration boundary while the output stays a concrete
/// [`ClusterSolution`](crate::batch::ClusterSolution). Not part of `loeres`;
/// never used by `loeres-device` (zero-bleed).
pub trait ClusterJob<S>: Send + Sync {
    /// Run the job to a per-item outcome. Solver-domain failures are returned as
    /// `Failed`/`Cancelled` here, not as a top-level error.
    fn run_boxed(&self, ctx: &ClusterExecutionContext) -> BatchItemOutcome<S>;
}

/// Solve a batch, returning a per-item report.
///
/// An empty batch is valid and returns an empty report (RFC 008 T1).
///
/// # Errors
/// [`ClusterError`] only for orchestration-level failures (invalid global
/// config, worker-pool init, runtime shutdown) — never for per-item solver
/// failures, which are carried in the returned outcomes.
pub fn solve_batch<S>(
    jobs: Vec<Box<dyn ClusterJob<S>>>,
    config: ClusterSolveConfig,
    cancel: ClusterCancellationToken,
) -> Result<BatchSolveReport<S>, ClusterError>
where
    S: Send,
{
    config.validate()?;
    if jobs.is_empty() {
        return Ok(BatchSolveReport::empty());
    }
    let ctx = ClusterExecutionContext::new(
        cancel.clone(),
        config.cancellation_poll_interval,
        config.validation_policy,
    );
    match config.effective_execution() {
        BatchExecutionPolicy::Sequential => {
            Ok(executor::execute_sequential(&jobs, &ctx, &cancel, &config))
        }
        #[cfg(feature = "parallel-rayon")]
        BatchExecutionPolicy::Parallel => executor::execute_parallel(&jobs, &ctx, &cancel, &config),
        // `effective_execution` only yields `Parallel` when `parallel-rayon` is
        // enabled; this arm keeps the match exhaustive otherwise.
        #[cfg(not(feature = "parallel-rayon"))]
        BatchExecutionPolicy::Parallel => {
            Ok(executor::execute_sequential(&jobs, &ctx, &cancel, &config))
        }
    }
}

/// Async wrapper over [`solve_batch`], offloading the blocking batch onto Tokio's
/// blocking pool. Runtime-agnostic in its public types — only the internals use
/// Tokio.
///
/// # Errors
/// As [`solve_batch`], plus [`ClusterError::Shutdown`] if the blocking task
/// cannot complete (e.g. runtime shutdown).
#[cfg(feature = "async-tokio")]
pub async fn solve_batch_async<S>(
    jobs: Vec<Box<dyn ClusterJob<S>>>,
    config: ClusterSolveConfig,
    cancel: ClusterCancellationToken,
) -> Result<BatchSolveReport<S>, ClusterError>
where
    S: Send + 'static,
{
    tokio::task::spawn_blocking(move || solve_batch(jobs, config, cancel))
        .await
        .map_err(|_| ClusterError::Shutdown)?
}

#[cfg(test)]
mod tests;
