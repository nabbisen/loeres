//! Batch executors (RFC 008 §3.5). The per-item engine is shared by the
//! sequential and (feature-gated) parallel paths so they produce identical
//! outcomes for the same batch.
//!
//! Panic containment (RFC 008 §4.4 / T2): each item runs inside
//! [`catch_unwind`](std::panic::catch_unwind), so a worker panic becomes
//! [`BatchItemOutcome::Panicked`] under `panic = "unwind"`. Under
//! `panic = "abort"` there is no catch boundary and the process aborts — no
//! `Panicked` outcome is produced, and none is promised.

use std::panic::{self, AssertUnwindSafe};
use std::time::Instant;

use loeres::SolverError;

use crate::batch::{BatchItemOutcome, BatchSolveReport};
use crate::runtime::ClusterCancellationToken;
use crate::solve::{ClusterExecutionContext, ClusterJob};

/// Map an inner `SolverError::Cancelled` to the batch `Cancelled` variant so the
/// two cancellation representations cannot diverge (RFC 008 §3.6 / F9).
fn normalize<S>(outcome: BatchItemOutcome<S>) -> BatchItemOutcome<S> {
    match outcome {
        BatchItemOutcome::Failed {
            error: SolverError::Cancelled,
        } => BatchItemOutcome::Cancelled,
        other => other,
    }
}

/// Run one item: observe cancellation and the timeout deadline at the boundary,
/// then run the job with panic containment.
fn run_item<S>(
    job: &dyn ClusterJob<S>,
    ctx: &ClusterExecutionContext,
    cancel: &ClusterCancellationToken,
    deadline: Option<Instant>,
) -> BatchItemOutcome<S> {
    if cancel.is_cancelled() {
        return BatchItemOutcome::Cancelled;
    }
    if let Some(deadline) = deadline {
        if Instant::now() >= deadline {
            return BatchItemOutcome::Cancelled;
        }
    }
    match panic::catch_unwind(AssertUnwindSafe(|| job.run_boxed(ctx))) {
        Ok(outcome) => normalize(outcome),
        Err(_) => BatchItemOutcome::Panicked,
    }
}

/// Run the batch sequentially on the calling thread, preserving submission order.
pub(crate) fn execute_sequential<S>(
    jobs: &[Box<dyn ClusterJob<S>>],
    ctx: &ClusterExecutionContext,
    cancel: &ClusterCancellationToken,
    deadline: Option<Instant>,
) -> BatchSolveReport<S> {
    let outcomes = jobs
        .iter()
        .map(|job| run_item(job.as_ref(), ctx, cancel, deadline))
        .collect();
    BatchSolveReport::from_outcomes(outcomes)
}

/// Run the batch across a bounded Rayon pool, preserving submission order.
///
/// # Errors
/// [`ClusterError::ExecutorInit`](crate::runtime::ClusterError) if the worker
/// pool cannot be built.
#[cfg(feature = "parallel-rayon")]
pub(crate) fn execute_parallel<S>(
    jobs: &[Box<dyn ClusterJob<S>>],
    ctx: &ClusterExecutionContext,
    cancel: &ClusterCancellationToken,
    deadline: Option<Instant>,
    max_parallelism: usize,
) -> Result<BatchSolveReport<S>, crate::runtime::ClusterError>
where
    S: Send,
{
    use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(max_parallelism)
        .build()
        .map_err(|_| crate::runtime::ClusterError::ExecutorInit)?;
    let outcomes = pool.install(|| {
        jobs.par_iter()
            .map(|job| run_item(job.as_ref(), ctx, cancel, deadline))
            .collect()
    });
    Ok(BatchSolveReport::from_outcomes(outcomes))
}

#[cfg(test)]
mod tests;
