# RFC 009 - Implementation-Decision Memo

**Artifact.** `rfcs/proposed/009-observability-ffi-gateways.md`
**Decision pass.** v1, after architect design review patch.
**Target release.** v0.15.0.
**Scope.** Settle the RFC 009 implementation choices deferred by the architect
review: total outcome classification, observer integration shape, and source of
solver/problem metadata.

---

## Verdict

Approved for implementation after the RFC 009 freeze-blocker patch.

The implementation must stay design-first and non-invasive: cluster
observability is derived from completed outcomes, not threaded through the
solver hot path; `ClusterJob`, `ClusterSolveConfig`, `SolveReport`, and
`BatchItemOutcome` stay unchanged.

## I1 - Event granularity

`SolveTelemetryEvent` is a per-item event. It is emitted only for a
`BatchItemOutcome` item and never for a whole-batch `ClusterError`.

Whole-batch failures from `solve_batch` remain outside `OutcomeKind`:

| Source | Telemetry handling |
|---|---|
| `Ok(BatchSolveReport)` | classify each `BatchItemOutcome` item |
| `Err(ClusterError::InvalidConfig)` | optional separate batch-level metric/event |
| `Err(ClusterError::ExecutorInit)` | optional separate batch-level metric/event |
| `Err(ClusterError::Shutdown)` | optional separate batch-level metric/event |

No `OrchestrationFailure` outcome is implemented in RFC 009.

## I2 - Total `SolverError` to `OutcomeKind` mapping

The classifier must be total over the current `SolverError` variants and use a
wildcard arm for future variants because `SolverError` is `#[non_exhaustive]`.
Future variants map to `InternalError` until an RFC or reviewed patch assigns a
more precise category.

| `SolverError` | `OutcomeKind` | Rationale |
|---|---|---|
| `DimensionMismatch { .. }` | `InvalidInput` | caller/model shape contract failed |
| `InvalidDimension` | `InvalidInput` | caller/model shape contract failed |
| `InvalidInput` | `InvalidInput` | direct invalid-input contract |
| `NonFiniteInput` | `InvalidInput` | caller/model value contract failed |
| `UnsupportedProblemStructure` | `InvalidInput` | selected solver/profile cannot accept this model |
| `SingularMatrix` | `NumericalFailure` | numerical solve failure |
| `IllConditioned` | `NumericalFailure` | numerical solve failure |
| `NumericalDomain` | `NumericalFailure` | numerical solve failure |
| `Overflow` | `NumericalFailure` | numerical solve failure |
| `WorkspaceTooSmall` | `InvalidInput` | workspace/profile capacity contract failed |
| `Cancelled` | `Cancelled` | cooperative cancellation or timeout |
| `BackendUnavailable` | `BackendFailure` | optional backend/resource unavailable |
| `InternalInvariantViolation` | `InternalError` | library invariant breach; do not blame caller |
| future variants | `InternalError` | fail closed until explicitly classified |

Implementation note: `WorkspaceTooSmall` is allowed to map to `InvalidInput`
only when it represents the public workspace/profile capacity contract. If a
Loeres-managed internal workspace produces this condition unexpectedly, that
path should surface `InternalInvariantViolation` instead so telemetry reports
`InternalError`.

`InternalError` covers two v1 cases: known Loeres library invariant breaches and
future `SolverError` variants that have not yet been classified. That fail-closed
default avoids blaming callers for unknown conditions. A future reviewed change
may split out a narrower category if operators need that distinction.

## I3 - `BatchItemOutcome` classification

The v1 classifier shape is pure and allocation-free apart from existing report
storage:

```rust
pub fn outcome_kind_from_item<S>(item: &BatchItemOutcome<S>) -> OutcomeKind;

pub fn outcome_kind_from_solver_error(error: SolverError) -> OutcomeKind;
```

Classification rules:

| `BatchItemOutcome` | `OutcomeKind` |
|---|---|
| `Solved { report, .. }` where `report.status().is_converged()` | `SolvedConverged` |
| `Solved { report, .. }` where status is not converged | `SolvedNotConverged` |
| `Failed { error }` | `outcome_kind_from_solver_error(error)` |
| `Cancelled` | `Cancelled` |
| `Panicked` | `Panicked` |

Non-convergence remains a solved status, not a failure.

## I4 - Observation context source

`solver_family` and `problem_class` are not inferred from erased jobs. They are
provided by an explicit cluster-only context that the observed entrypoint or
adapter supplies:

```rust
pub struct SolveObservationContext {
    pub solver_family: SolverFamilyId,
    pub problem_class: ProblemClassId,
}
```

Required sources:

| Entrypoint/adapter | `solver_family` | `problem_class` |
|---|---|---|
| RFC 016 projected-first-order cluster entrypoint | `ProjectedFirstOrder` | `BoxConstrainedFirstOrder` |
| RFC 009 mock gateway | `Gateway` | `GatewayNative` |
| unknown/custom erased job | `Unknown` | `Unknown` |

The implementation may add constructor helpers such as
`SolveObservationContext::projected_first_order()` and
`SolveObservationContext::gateway_native()`. These helpers must return bounded
enum values only and must not accept caller-provided strings.

## I5 - Integration shape

The v1 public integration is post-hoc and non-invasive:

1. a helper that observes an already completed `BatchSolveReport`;
2. a thin `solve_batch_observed` convenience wrapper that calls the existing
   `solve_batch`, then invokes the helper on success.

Expected shape:

```rust
pub fn observe_batch_report<S>(
    report: &BatchSolveReport<S>,
    context: SolveObservationContext,
    observer: &dyn SolveObserver,
);

pub fn solve_batch_observed<S>(
    jobs: Vec<Box<dyn ClusterJob<S>>>,
    config: ClusterSolveConfig,
    cancel: ClusterCancellationToken,
    context: SolveObservationContext,
    observer: &dyn SolveObserver,
) -> Result<BatchSolveReport<S>, ClusterError>
where
    S: Send;
```

The exact generic bounds must match the existing `solve_batch` signature during
implementation. The wrapper must not add a field to `ClusterSolveConfig`, must
not change `ClusterJob`, must not require global logging state, and must forward
the caller-provided `ClusterCancellationToken` verbatim.

`solve_batch_observed` applies one `SolveObservationContext` to the whole batch.
That is the v1 homogeneous-batch contract: a batch is assumed to represent one
known kernel/backend path. Mixed batches should use `Unknown` family/problem
values; per-item observation context is a future extension, not part of RFC 009.

For `Err(ClusterError)`, `solve_batch_observed` returns the error unchanged. A
separate batch-level observer or metric helper may be added only if it remains
bounded-category and does not reuse `OutcomeKind`.

## I6 - Bucket derivation

Bucket thresholds are implementation-owned constants inside
`loeres_cluster::observe`. The first implementation should prefer conservative,
coarse buckets:

| Field | Source |
|---|---|
| `DimensionBucket` | model/adapter metadata supplied by the observed entrypoint, otherwise `Unknown` |
| `IterationsBucket` | `SolveReport` iteration count when present, otherwise `Unknown` |
| `ElapsedBucket` | wrapper-measured elapsed time when using `solve_batch_observed`, otherwise `Unknown` |

The helper over an already completed `BatchSolveReport` emits
`DimensionBucket::Unknown` unless entrypoint-supplied per-item dimension
metadata is available. `Failed`, `Cancelled`, and `Panicked` items also emit
`DimensionBucket::Unknown` absent such metadata. A `Solved` item may derive a
coarse bucket from solution length when that count is available and
redaction-safe, but this is optional and never applied to items without a
solution.

The helper may produce `ElapsedBucket::Unknown` unless elapsed metadata is
explicitly supplied by a separate helper variant. It must not infer elapsed time
from wall-clock time after the fact.

## I7 - Redaction and labels

All emitted event fields are bounded enums. The implementation must not add
strings, raw dimensions, objective values, residuals, final solution values,
tenant IDs, request IDs, file paths, or `TrustedByCaller` labels to
`SolveTelemetryEvent` or metrics labels.

`TrustedByCaller.label` is `Option<&'static str>` today, but RFC 009 still
excludes it from telemetry as a stable defense-in-depth rule.

## Implementation checklist

- Add `loeres_cluster::observe` data types and pure classifiers first.
- Add tests pinning the total `SolverError` mapping table.
- Add tests pinning `BatchItemOutcome` status/error classification.
- Add redaction tests proving no caller strings or raw model values can enter
  event fields or labels.
- Add `observe_batch_report` before `solve_batch_observed`.
- Keep all new observability and metrics integrations cluster-only and
  default-off.
- Do not implement orchestrator scheduling by gateway thread-safety class.

## Gates for this memo

The RFC remains proposed until implementation ships. Before moving RFC 009 to
`done/`, the implementation must pass the acceptance gates already listed in
RFC 009 section 6.6, including `cargo xtask release-gate`.
