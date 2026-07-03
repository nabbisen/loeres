//! Metadata-only observability integration points (RFC 009).
//!
//! Events are cluster-only, redacted by construction, and made from bounded
//! enum categories. They never carry raw model data, caller strings, objective
//! values, residuals, final solution values, paths, tenant identifiers, or
//! validation trust labels.

use std::time::Duration;

use loeres::{SolveReport, SolverError};

use crate::batch::{BatchItemOutcome, BatchSolveReport};
use crate::runtime::{ClusterCancellationToken, ClusterError, ClusterSolveConfig};
use crate::solve::{ClusterJob, solve_batch};

/// Coarse solver family category.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum SolverFamilyId {
    /// The std-side projected-first-order cluster kernel.
    ProjectedFirstOrder,
    /// A cluster gateway backend.
    Gateway,
    /// The observed entrypoint did not self-identify.
    Unknown,
}

impl SolverFamilyId {
    /// Stable bounded metrics/tracing label.
    #[must_use]
    pub const fn as_label(self) -> &'static str {
        match self {
            Self::ProjectedFirstOrder => "projected_first_order",
            Self::Gateway => "gateway",
            Self::Unknown => "unknown",
        }
    }
}

/// Coarse problem class category.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ProblemClassId {
    /// Box/bound-constrained first-order problem.
    BoxConstrainedFirstOrder,
    /// Native/gateway-owned problem shape.
    GatewayNative,
    /// The observed entrypoint did not self-identify.
    Unknown,
}

impl ProblemClassId {
    /// Stable bounded metrics/tracing label.
    #[must_use]
    pub const fn as_label(self) -> &'static str {
        match self {
            Self::BoxConstrainedFirstOrder => "box_constrained_first_order",
            Self::GatewayNative => "gateway_native",
            Self::Unknown => "unknown",
        }
    }
}

/// Redacted dimension bucket.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum DimensionBucket {
    /// No honest item dimension was supplied.
    Unknown,
    /// Empty input or output.
    Empty,
    /// Small enough for local debugging without exposing exact size.
    Small,
    /// Medium-sized model.
    Medium,
    /// Large model.
    Large,
    /// Huge model.
    Huge,
}

impl DimensionBucket {
    /// Classify an exact dimension into a coarse bucket.
    #[must_use]
    pub const fn from_len(len: usize) -> Self {
        match len {
            0 => Self::Empty,
            1..=32 => Self::Small,
            33..=1024 => Self::Medium,
            1025..=65_536 => Self::Large,
            _ => Self::Huge,
        }
    }

    /// Stable bounded metrics/tracing label.
    #[must_use]
    pub const fn as_label(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Empty => "empty",
            Self::Small => "small",
            Self::Medium => "medium",
            Self::Large => "large",
            Self::Huge => "huge",
        }
    }
}

/// Redacted iteration-count bucket.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum IterationsBucket {
    /// Zero iterations.
    None,
    /// A small bounded number of iterations.
    Few,
    /// A moderate bounded number of iterations.
    Moderate,
    /// Many iterations.
    Many,
    /// Very high or capped iteration count.
    Maxed,
    /// No honest iteration count was supplied.
    Unknown,
}

impl IterationsBucket {
    /// Classify an exact iteration count into a coarse bucket.
    #[must_use]
    pub const fn from_count(iterations: u32) -> Self {
        match iterations {
            0 => Self::None,
            1..=10 => Self::Few,
            11..=100 => Self::Moderate,
            101..=1_000 => Self::Many,
            _ => Self::Maxed,
        }
    }

    /// Stable bounded metrics/tracing label.
    #[must_use]
    pub const fn as_label(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Few => "few",
            Self::Moderate => "moderate",
            Self::Many => "many",
            Self::Maxed => "maxed",
            Self::Unknown => "unknown",
        }
    }
}

/// Redacted elapsed-time bucket.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ElapsedBucket {
    /// Zero-duration observation.
    Instant,
    /// Short elapsed time.
    Short,
    /// Medium elapsed time.
    Medium,
    /// Long elapsed time.
    Long,
    /// Timeout or cancellation path.
    TimeoutOrCancelled,
    /// No honest elapsed time was supplied.
    Unknown,
}

impl ElapsedBucket {
    /// Classify an elapsed duration into a coarse bucket.
    #[must_use]
    pub const fn from_duration(duration: Duration) -> Self {
        if duration.as_nanos() == 0 {
            Self::Instant
        } else if duration.as_millis() <= 10 {
            Self::Short
        } else if duration.as_secs() < 1 {
            Self::Medium
        } else {
            Self::Long
        }
    }

    /// Stable bounded metrics/tracing label.
    #[must_use]
    pub const fn as_label(self) -> &'static str {
        match self {
            Self::Instant => "instant",
            Self::Short => "short",
            Self::Medium => "medium",
            Self::Long => "long",
            Self::TimeoutOrCancelled => "timeout_or_cancelled",
            Self::Unknown => "unknown",
        }
    }
}

/// Per-item solve outcome category.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum OutcomeKind {
    /// Structured terminal report with converged status.
    SolvedConverged,
    /// Structured terminal report with non-converged status.
    SolvedNotConverged,
    /// Caller/model input contract failed.
    InvalidInput,
    /// Numerical solve failure.
    NumericalFailure,
    /// Cooperative cancellation or timeout.
    Cancelled,
    /// Worker panic caught by cluster orchestration.
    Panicked,
    /// Optional backend/resource failure.
    BackendFailure,
    /// Loeres invariant breach or future unclassified error.
    InternalError,
}

impl OutcomeKind {
    /// Stable bounded metrics/tracing label.
    #[must_use]
    pub const fn as_label(self) -> &'static str {
        match self {
            Self::SolvedConverged => "solved_converged",
            Self::SolvedNotConverged => "solved_not_converged",
            Self::InvalidInput => "invalid_input",
            Self::NumericalFailure => "numerical_failure",
            Self::Cancelled => "cancelled",
            Self::Panicked => "panicked",
            Self::BackendFailure => "backend_failure",
            Self::InternalError => "internal_error",
        }
    }
}

/// Metadata-only per-item telemetry event.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct SolveTelemetryEvent {
    /// Solver family supplied by the observed entrypoint.
    pub solver_family: SolverFamilyId,
    /// Problem class supplied by the observed entrypoint.
    pub problem_class: ProblemClassId,
    /// Coarse dimension bucket.
    pub dimension_bucket: DimensionBucket,
    /// Per-item outcome category.
    pub outcome: OutcomeKind,
    /// Coarse iteration-count bucket.
    pub iterations_bucket: IterationsBucket,
    /// Coarse elapsed-time bucket.
    pub elapsed_bucket: ElapsedBucket,
}

/// Batch-wide observation context supplied by the observed entrypoint.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct SolveObservationContext {
    /// Solver family for this homogeneous observed batch.
    pub solver_family: SolverFamilyId,
    /// Problem class for this homogeneous observed batch.
    pub problem_class: ProblemClassId,
}

impl SolveObservationContext {
    /// Context for RFC 016 projected-first-order cluster solves.
    #[must_use]
    pub const fn projected_first_order() -> Self {
        Self {
            solver_family: SolverFamilyId::ProjectedFirstOrder,
            problem_class: ProblemClassId::BoxConstrainedFirstOrder,
        }
    }

    /// Context for RFC 009 gateway-native solves.
    #[must_use]
    pub const fn gateway_native() -> Self {
        Self {
            solver_family: SolverFamilyId::Gateway,
            problem_class: ProblemClassId::GatewayNative,
        }
    }

    /// Honest fallback for custom or mixed erased batches.
    #[must_use]
    pub const fn unknown() -> Self {
        Self {
            solver_family: SolverFamilyId::Unknown,
            problem_class: ProblemClassId::Unknown,
        }
    }
}

/// Explicit redacted sink for per-item telemetry events.
pub trait SolveObserver: Send + Sync {
    /// Observe one already-redacted metadata event.
    fn observe(&self, event: &SolveTelemetryEvent);
}

/// Observer that drops all events.
#[derive(Copy, Clone, Debug, Default)]
pub struct NoopObserver;

impl SolveObserver for NoopObserver {
    fn observe(&self, _event: &SolveTelemetryEvent) {}
}

/// Classify a `SolverError` into the RFC 009 per-item outcome vocabulary.
///
/// `InternalError` is the fail-closed v1 category for both known Loeres
/// invariant breaches and future unclassified `SolverError` variants.
#[must_use]
pub const fn outcome_kind_from_solver_error(error: SolverError) -> OutcomeKind {
    match error {
        SolverError::DimensionMismatch { .. }
        | SolverError::InvalidDimension
        | SolverError::InvalidInput
        | SolverError::NonFiniteInput
        | SolverError::UnsupportedProblemStructure
        | SolverError::WorkspaceTooSmall => OutcomeKind::InvalidInput,
        SolverError::SingularMatrix
        | SolverError::IllConditioned
        | SolverError::NumericalDomain
        | SolverError::Overflow => OutcomeKind::NumericalFailure,
        SolverError::Cancelled => OutcomeKind::Cancelled,
        SolverError::BackendUnavailable => OutcomeKind::BackendFailure,
        SolverError::InternalInvariantViolation => OutcomeKind::InternalError,
        _ => OutcomeKind::InternalError,
    }
}

/// Classify one batch item into the RFC 009 per-item outcome vocabulary.
#[must_use]
pub fn outcome_kind_from_item<S>(item: &BatchItemOutcome<S>) -> OutcomeKind {
    match item {
        BatchItemOutcome::Solved { report, .. } => {
            if report.status().is_converged() {
                OutcomeKind::SolvedConverged
            } else {
                OutcomeKind::SolvedNotConverged
            }
        }
        BatchItemOutcome::Failed { error } => outcome_kind_from_solver_error(*error),
        BatchItemOutcome::Cancelled => OutcomeKind::Cancelled,
        BatchItemOutcome::Panicked => OutcomeKind::Panicked,
    }
}

/// Classify a solve report's iteration count.
#[must_use]
pub const fn iterations_bucket_from_report(report: SolveReport) -> IterationsBucket {
    IterationsBucket::from_count(report.iterations_executed())
}

fn iterations_bucket_from_item<S>(item: &BatchItemOutcome<S>) -> IterationsBucket {
    match item {
        BatchItemOutcome::Solved { report, .. } => iterations_bucket_from_report(*report),
        _ => IterationsBucket::Unknown,
    }
}

fn elapsed_bucket_from_item<S>(
    item: &BatchItemOutcome<S>,
    elapsed_bucket: ElapsedBucket,
) -> ElapsedBucket {
    match item {
        BatchItemOutcome::Cancelled => ElapsedBucket::TimeoutOrCancelled,
        BatchItemOutcome::Failed {
            error: SolverError::Cancelled,
        } => ElapsedBucket::TimeoutOrCancelled,
        _ => elapsed_bucket,
    }
}

/// Build one telemetry event from already redacted metadata.
#[must_use]
pub fn telemetry_event_from_item<S>(
    item: &BatchItemOutcome<S>,
    context: SolveObservationContext,
    dimension_bucket: DimensionBucket,
    elapsed_bucket: ElapsedBucket,
) -> SolveTelemetryEvent {
    SolveTelemetryEvent {
        solver_family: context.solver_family,
        problem_class: context.problem_class,
        dimension_bucket,
        outcome: outcome_kind_from_item(item),
        iterations_bucket: iterations_bucket_from_item(item),
        elapsed_bucket: elapsed_bucket_from_item(item, elapsed_bucket),
    }
}

/// Observe a completed batch report with unknown per-item dimensions and
/// elapsed times.
pub fn observe_batch_report<S>(
    report: &BatchSolveReport<S>,
    context: SolveObservationContext,
    observer: &dyn SolveObserver,
) {
    observe_batch_report_with_metadata(report, context, &[], ElapsedBucket::Unknown, observer);
}

/// Observe a completed batch report with optional per-item dimension metadata.
///
/// Missing dimension entries are classified as [`DimensionBucket::Unknown`].
pub fn observe_batch_report_with_metadata<S>(
    report: &BatchSolveReport<S>,
    context: SolveObservationContext,
    dimension_buckets: &[DimensionBucket],
    elapsed_bucket: ElapsedBucket,
    observer: &dyn SolveObserver,
) {
    for (index, item) in report.outcomes.iter().enumerate() {
        let dimension_bucket = dimension_buckets
            .get(index)
            .copied()
            .unwrap_or(DimensionBucket::Unknown);
        let event = telemetry_event_from_item(item, context, dimension_bucket, elapsed_bucket);
        observer.observe(&event);
    }
}

/// Thin observed wrapper over [`solve_batch`].
///
/// The cancellation token is forwarded verbatim. On orchestration-level
/// [`ClusterError`], no per-item events are emitted because no
/// [`BatchSolveReport`] exists.
///
/// # Errors
/// As [`solve_batch`].
pub fn solve_batch_observed<S>(
    jobs: Vec<Box<dyn ClusterJob<S>>>,
    config: ClusterSolveConfig,
    cancel: ClusterCancellationToken,
    context: SolveObservationContext,
    observer: &dyn SolveObserver,
) -> Result<BatchSolveReport<S>, ClusterError>
where
    S: Send,
{
    let started = std::time::Instant::now();
    let report = solve_batch(jobs, config, cancel)?;
    observe_batch_report_with_metadata(
        &report,
        context,
        &[],
        ElapsedBucket::from_duration(started.elapsed()),
        observer,
    );
    Ok(report)
}

#[cfg(test)]
mod tests;
