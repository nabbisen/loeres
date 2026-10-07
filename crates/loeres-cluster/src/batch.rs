//! Per-item batch outcome contract (RFC 008 §3.2).
//!
//! One ill-conditioned model must never fail the whole batch: every item
//! carries its own structured outcome. The status/error split is preserved —
//! a bounded-terminus non-convergence is a *solved* item carrying
//! [`SolveStatus::NotConverged`](loeres::SolveStatus), never `Failed`. `Failed`
//! is reserved for fail-safe [`SolverError`] conditions; `Panicked` is an
//! executor-caught worker panic (RFC 008 §4.4); `Cancelled` is cooperative
//! cancellation or timeout (§3.6).
//!
//! RFC 042: a constrained item's `Solved` outcome also carries what its job
//! computed beyond the report — `projection_cap_hits`, `max_constraint_violation`
//! and `infeasibility_evidence` — the same fields
//! [`ConstrainedSolveRecord`](crate::ConstrainedSolveRecord) gives a direct
//! caller, under the same warnings. `Solved` is `#[non_exhaustive]` so the
//! *next* field does not break a caller who already matches `Solved { .. }`
//! (RFC 042 §2.2's decision: enum-variant fields always share the variant's
//! own visibility — Rust has no per-field privacy there — so RFC 014 §311's
//! private-field pattern, which protects `SolveReport` nested inside this
//! very variant, cannot be reapplied to the variant's own field list; a
//! plain struct like `SolveReport` is the only shape that pattern fits).

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

/// What a constrained job computed beyond the report, for an item whose
/// problem had linear inequalities (RFC 042).
///
/// Carried only when there was a polyhedron to check: `Solved`'s
/// `constrained_detail()` is `None` for an item with **no** linear
/// inequalities at all, which is distinct from `Some` with a zero
/// `max_constraint_violation` — the latter means there were constraints and
/// they were satisfied. Conflating the two would let a reader draw "feasible"
/// from an absence that actually means "nothing to check".
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct ConstrainedBatchDetail<S> {
    /// Outer iterations whose Dykstra projection hit `projection_max_sweeps`
    /// without converging. Distinguishes RFC 039's two non-convergence causes
    /// from this value alone: nonzero means the projection's sweep cap bound;
    /// zero (with the item still `NotConverged`) means the outer iteration cap
    /// did, which the sweep cap cannot be raised to fix.
    pub projection_cap_hits: u32,
    /// `max(0, maxᵢ(aᵢᵀx − bᵢ))` at the returned iterate.
    pub max_constraint_violation: S,
    /// A **heuristic observation** that the polyhedron may be empty (RFC 034
    /// Amendment 2), **wrong in both directions** — it misses most weakly
    /// infeasible systems and can be set on a feasible problem whose
    /// projection needed more than the sweep cap. Not a status; never for
    /// control flow. See
    /// [`ConstrainedSolveRecord::infeasibility_evidence`](crate::ConstrainedSolveRecord)
    /// for the measured error rates this crosses the seam unweakened.
    pub infeasibility_evidence: bool,
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
    ///
    /// `#[non_exhaustive]`: a caller outside this crate must match with a
    /// trailing `..` and must not construct this variant by struct literal,
    /// so the next field this RFC's own `constrained` was added under does
    /// not break anyone (RFC 042 §2.2).
    #[non_exhaustive]
    Solved {
        /// The produced solution.
        solution: ClusterSolution<S>,
        /// The structured solve report (may be `NotConverged`).
        report: SolveReport,
        /// What a constrained job additionally computed, or `None` if the
        /// item's problem had no linear inequalities. Read through
        /// [`constrained_detail`](BatchItemOutcome::constrained_detail) for
        /// the "no constraints" documentation in one place.
        constrained: Option<ConstrainedBatchDetail<S>>,
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

impl<S> BatchItemOutcome<S> {
    /// What a constrained job computed beyond the report, for a `Solved` item
    /// whose problem had linear inequalities. `None` for every other variant,
    /// and for a `Solved` item whose problem had none (RFC 042 §2.1).
    #[must_use]
    pub const fn constrained_detail(&self) -> Option<&ConstrainedBatchDetail<S>> {
        match self {
            Self::Solved { constrained, .. } => constrained.as_ref(),
            Self::Failed { .. } | Self::Cancelled | Self::Panicked => None,
        }
    }
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
