//! Per-item batch outcome contract (RFC 008 §3.2).
//!
//! One ill-conditioned model must never fail the whole batch: every item
//! carries its own structured outcome. The status/error split is preserved —
//! a bounded-terminus non-convergence is a *solved* item carrying
//! [`SolveStatus::NotConverged`](loeres::SolveStatus), never `Failed`. `Failed`
//! is reserved for fail-safe [`SolverError`] conditions; `Panicked` is an
//! executor-caught worker panic (RFC 008 §4.4); `Cancelled` is cooperative
//! cancellation or timeout (§3.6).

use loeres::{SolveReport, SolverError};
use loeres_backend_std::DenseVector;

/// A concrete, inspectable erased solution over the supported shapes
/// (RFC 008 §3.2 / F5).
///
/// Dense-vector first; sparse / dense-matrix variants are added (feature-gated)
/// only once a producer exists. `#[non_exhaustive]` so new variants are not a
/// breaking change.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum ClusterSolution<S> {
    /// A dense solution vector.
    DenseVector(DenseVector<S>),
}

/// The outcome of a single batch item.
///
/// `Solved` means the attempt reached a structured terminal report and produced
/// the declared [`ClusterSolution`]; it does **not** imply
/// [`SolveStatus::Converged`](loeres::SolveStatus). `Failed` carries a fail-safe
/// [`SolverError`]. `Cancelled` is cooperative cancellation or timeout.
/// `Panicked` is a worker-task panic caught at the item boundary under
/// `panic = "unwind"` (none is produced under `panic = "abort"`).
#[derive(Clone, Debug)]
pub enum BatchItemOutcome<S> {
    /// Reached a structured terminal report (converged or not) and produced a
    /// solution.
    Solved {
        /// The produced solution.
        solution: ClusterSolution<S>,
        /// The structured solve report (may be `NotConverged`).
        report: SolveReport,
    },
    /// A fail-safe solver error prevented a solution.
    Failed {
        /// The structured solver error.
        error: SolverError,
    },
    /// The item was cancelled (pre-dispatch, cooperatively mid-run, timeout, or
    /// an inner `SolverError::Cancelled` mapped here — RFC 008 §3.6).
    Cancelled,
    /// The worker task panicked and was contained at the item boundary
    /// (RFC 008 §4.4); only possible under `panic = "unwind"`.
    Panicked,
}

/// Explicit per-category counts so dashboards need not scan the outcome vector.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct BatchSummary {
    /// `Solved` items whose report is `Converged`.
    pub solved_converged: usize,
    /// `Solved` items whose report is `NotConverged`.
    pub solved_not_converged: usize,
    /// `Failed` items.
    pub failed: usize,
    /// `Cancelled` items.
    pub cancelled: usize,
    /// `Panicked` items.
    pub panicked: usize,
}

impl BatchSummary {
    /// Total number of items summarized.
    #[must_use]
    pub const fn total(&self) -> usize {
        self.solved_converged
            + self.solved_not_converged
            + self.failed
            + self.cancelled
            + self.panicked
    }
}

/// A completed batch: per-item outcomes plus a precomputed summary.
#[derive(Clone, Debug)]
pub struct BatchSolveReport<S> {
    /// Per-item outcomes, in submission order.
    pub outcomes: Vec<BatchItemOutcome<S>>,
    /// Precomputed per-category counts.
    pub summary: BatchSummary,
}

impl<S> BatchSolveReport<S> {
    /// An empty report (the result of an empty batch — RFC 008 T1).
    #[must_use]
    pub fn empty() -> Self {
        Self {
            outcomes: Vec::new(),
            summary: BatchSummary::default(),
        }
    }

    /// Build a report from outcomes, tallying the summary.
    #[must_use]
    pub fn from_outcomes(outcomes: Vec<BatchItemOutcome<S>>) -> Self {
        let mut summary = BatchSummary::default();
        for outcome in &outcomes {
            match outcome {
                BatchItemOutcome::Solved { report, .. } => {
                    if report.status().is_converged() {
                        summary.solved_converged += 1;
                    } else {
                        summary.solved_not_converged += 1;
                    }
                }
                BatchItemOutcome::Failed { .. } => summary.failed += 1,
                BatchItemOutcome::Cancelled => summary.cancelled += 1,
                BatchItemOutcome::Panicked => summary.panicked += 1,
            }
        }
        Self { outcomes, summary }
    }
}

#[cfg(test)]
mod tests;
