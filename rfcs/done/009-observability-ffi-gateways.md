# RFC 009 — Observability, Metrics, and FFI Gateway Interfacing

**Status.** Implemented (v0.15.0) — `loeres-cluster::observe` now exposes
metadata-only telemetry categories, the total `SolverError` / `BatchItemOutcome`
classifiers, explicit observer sinks, and the non-invasive observed batch
wrapper. `loeres-cluster::gateway` now exposes the safe gateway boundary
categories and a pure Rust mock gateway job; no concrete native solver adapter
ships in this RFC. RFC 008's `ClusterJob` seam and RFC 014's status/error split
remain unchanged, and the edge crates receive no observability, metrics, FFI, or
tracing dependency.
**Tracks.** Phase 3 / Milestone 3 — Dynamic Infrastructure and Cloud Cluster
**Touches.** `loeres-cluster/src/observe.rs`, `loeres-cluster/src/gateway.rs`,
`loeres-cluster/src/lib.rs`, optional cluster-only feature wiring in
`crates/loeres-cluster/Cargo.toml`, and, only for later concrete adapters,
cluster-only internals under `loeres-backend-std::adapter`.

---

### Extended Metadata
* **Rust Edition Compliance:** Rust 2024 Baseline
* **Target Environment:** `loeres-cluster` and `loeres-backend-std`; `std`,
  server-only, default-off integration features

## 1. Executive Summary & Problem Statement

Cluster users need two things that device users must never inherit:

1. metadata observability and aggregate metrics for service operation; and
2. an explicit, audited boundary for optional native / legacy solver gateways.

These capabilities are incompatible with the edge baseline. They may use `std`,
threads, host observability libraries, and eventually FFI, but they must not
reach `loeres`, `loeres-backend-static`, or `loeres-device`.

This RFC defines the **cluster-only boundary** for those capabilities. The first
slice is intentionally conservative:

* observability records are metadata-only and redacted by construction;
* metrics labels are bounded enum-like categories, never raw model data;
* optional `tracing` / metrics integrations are adapters behind current feature
  names, not baseline public dependencies;
* the FFI gateway module defines safe Rust boundary contracts and audit
  requirements, but no concrete third-party native solver is accepted without a
  follow-up adapter-specific design note.

## 2. Architectural Context & Dependency Alignment

This RFC depends on:

* [RFC 003](003-allocation-free-errors.md) for `SolverError`;
* [RFC 007](007-dynamic-sparse-adapters.md) for dynamic storage;
* [RFC 008](008-async-orchestration-budgets.md) for `ClusterJob`,
  `BatchItemOutcome`, `BatchSolveReport`, cancellation, and per-item semantics;
* [RFC 012](012-validation-state-and-trusted-input-policy.md) for the
  validation vocabulary that must not be fabricated by observability;
* [RFC 014](014-core-solver-outcome-state.md) for `SolveReport` and the
  status/error split;
* [RFC 016](016-std-side-projected-first-order-cluster-kernel.md) for
  the current std-side kernel and its honest validation evidence.

Dependency rules:

| Crate | Rule |
|---|---|
| `loeres-cluster` | Owns `observe` and `gateway`; may expose cluster-only adapters behind default-off features. |
| `loeres-backend-std` | May host adapter internals for gateway conversion, but only for server builds. |
| `loeres` | No dependency on telemetry, metrics, FFI, tracing, logging, or gateway types. |
| `loeres-backend-static` | No dependency on telemetry, metrics, FFI, tracing, logging, or gateway types. |
| `loeres-device` | No dependency on telemetry, metrics, FFI, tracing, logging, or gateway types. |
| `xtask` | Verifies dependency isolation, feature isolation, and unsafe audit policy. |

## 3. Concrete Technical Specification

### 3.1 Feature posture

The current cluster feature names are retained:

```toml
[features]
default = []
parallel-rayon = ["dep:rayon"]
async-tokio = ["dep:tokio"]
observability-tracing = []
observability-metrics = []
serde = []
ffi-gateway = []
```

Rules:

* `observability-tracing` may add an optional `tracing` dependency later, but
  the dependency version must be centralized in root `[workspace.dependencies]`.
* `observability-metrics` may add an optional metrics-export dependency later,
  likewise centralized at the workspace root.
* `ffi-gateway` is default-off forever unless a superseding RFC explicitly
  changes the project trust boundary.
* None of these features may be forwarded by `loeres`, `loeres-backend-static`,
  or `loeres-device`.
* No Tokio, Rayon, tracing, metrics, or native solver type may appear in the
  baseline public surface.

### 3.2 Observability module scope

`loeres_cluster::observe` owns small, inspectable metadata records and optional
adapter glue. It does not own raw logging macros in solver hot loops.

`SolveTelemetryEvent` is a **per-item** event. It is derived from
`BatchItemOutcome` and its contained `SolveReport` / `SolverError`, not from
batch-level orchestration failures. `ClusterError` remains a batch-level result
from `solve_batch`; if operators need visibility into `ClusterError`, RFC 009
models that as a separate batch-level event or metric family, never as one
item's `OutcomeKind`.

Baseline public categories:

```rust
#[non_exhaustive]
pub enum SolverFamilyId {
    ProjectedFirstOrder,
    Gateway,
    Unknown,
}

#[non_exhaustive]
pub enum ProblemClassId {
    BoxConstrainedFirstOrder,
    GatewayNative,
    Unknown,
}

#[non_exhaustive]
pub enum DimensionBucket {
    Unknown,
    Empty,
    Small,
    Medium,
    Large,
    Huge,
}

#[non_exhaustive]
pub enum IterationsBucket {
    None,
    Few,
    Moderate,
    Many,
    Maxed,
    Unknown,
}

#[non_exhaustive]
pub enum ElapsedBucket {
    Instant,
    Short,
    Medium,
    Long,
    TimeoutOrCancelled,
    Unknown,
}

#[non_exhaustive]
pub enum OutcomeKind {
    SolvedConverged,
    SolvedNotConverged,
    InvalidInput,
    NumericalFailure,
    Cancelled,
    Panicked,
    BackendFailure,
    InternalError,
}

pub struct SolveTelemetryEvent {
    pub solver_family: SolverFamilyId,
    pub problem_class: ProblemClassId,
    pub dimension_bucket: DimensionBucket,
    pub outcome: OutcomeKind,
    pub iterations_bucket: IterationsBucket,
    pub elapsed_bucket: ElapsedBucket,
}
```

The exact bucket thresholds are implementation-owned constants documented in the
module. They must be coarse enough that a telemetry record is not a model-size
fingerprint for ordinary tenant workloads. `DimensionBucket::Unknown` is used
when the observed entrypoint has no honest dimension metadata for an item; the
implementation must not fabricate dimensions.

`solver_family` and `problem_class` are set by the observed entrypoint or
adapter that knows which path ran: for example, the projected-first-order
cluster kernel or a gateway backend. They must not be inferred from an erased
`dyn ClusterJob`. `Unknown` is the honest default when an entrypoint does not
self-identify.

The implementation ships a total `SolverError` to `OutcomeKind` classifier:

| `SolverError` | `OutcomeKind` |
|---|---|
| `DimensionMismatch { .. }` | `InvalidInput` |
| `InvalidDimension` | `InvalidInput` |
| `InvalidInput` | `InvalidInput` |
| `NonFiniteInput` | `InvalidInput` |
| `UnsupportedProblemStructure` | `InvalidInput` |
| `SingularMatrix` | `NumericalFailure` |
| `IllConditioned` | `NumericalFailure` |
| `NumericalDomain` | `NumericalFailure` |
| `Overflow` | `NumericalFailure` |
| `WorkspaceTooSmall` | `InvalidInput` |
| `Cancelled` | `Cancelled` |
| `BackendUnavailable` | `BackendFailure` |
| `InternalInvariantViolation` | `InternalError` |
| future `SolverError` variants | `InternalError` until explicitly classified |

This table intentionally makes `InternalInvariantViolation` observable as
`InternalError`; it must not be laundered into `InvalidInput`.

### 3.3 Redaction policy

Telemetry emitted by Loeres must not contain:

* raw vector entries;
* raw matrix entries;
* objective values;
* residual values;
* final solution values;
* tenant identifiers;
* file paths;
* arbitrary caller strings;
* serialized model payloads;
* `TrustedByCaller` labels.

Allowed data is coarse metadata:

* solver family;
* broad problem class;
* dimension bucket;
* outcome category;
* iteration bucket;
* elapsed-time bucket;
* backend family category;
* feature/config category where it is an enum, not a caller string.

Loeres users may log their own model data outside Loeres. Loeres defaults and
adapters must stay metadata-only.

`TrustedByCaller.label` is currently `Option<&'static str>`, so it cannot carry
runtime request data by construction. It is still excluded from Loeres telemetry
labels as defense in depth and to keep the redaction rule stable if the policy
shape changes later.

### 3.4 Observer and sink shape

The baseline should use explicit sinks rather than global mutable logging state.
An observer receives already-redacted metadata:

```rust
pub trait SolveObserver: Send + Sync {
    fn observe(&self, event: &SolveTelemetryEvent);
}

pub struct NoopObserver;
```

This trait is cluster-only and object-safe by design. It must not enter `loeres`
or device APIs. A `NoopObserver` keeps the baseline cheap and avoids mandatory
logging dependencies.

RFC 009 does **not** change `ClusterJob` or `solve_batch` signatures. Initial
integration shipped as:

1. explicit helper functions that classify a `BatchSolveReport`;
2. wrapper API `solve_batch_observed`;
3. no observer field in cluster config.

This v1 path is post-hoc and non-invasive. `observe_batch_report` classifies an
already completed `BatchSolveReport`, and `solve_batch_observed` calls the
existing `solve_batch`, forwards the caller's `ClusterCancellationToken`
verbatim, and observes the completed report on success. It does not add a field
to `ClusterSolveConfig`, does not change `ClusterJob`, and does not require
global logging state. Adding an observer field to cluster config remains a
future config-revision shape.

### 3.5 Metrics model

Metrics are derived from `SolveTelemetryEvent`. Labels must be controlled
categories, not caller-provided strings. Per-item metrics use `OutcomeKind`;
batch-level `ClusterError` visibility, if added, uses a separate metric family
with a bounded cluster-error category label.

Required aggregate families:

* solve attempts by solver family and outcome kind;
* solved-converged vs solved-not-converged counts;
* failure counts by coarse category;
* cancellation and panic counts;
* elapsed-time buckets;
* iteration buckets.

Forbidden labels:

* tenant IDs;
* request IDs;
* arbitrary model names;
* raw dimensions;
* objective or solution values;
* file paths;
* trust labels.

The `observability-metrics` feature may provide an adapter to a metrics crate,
but the baseline module must be usable without that external dependency.

### 3.6 Gateway module scope

`loeres_cluster::gateway` owns the safe Rust boundary for optional native or
legacy solver adapters. RFC 009 does not accept any concrete third-party solver
or native ABI by itself.

Baseline public categories:

```rust
#[non_exhaustive]
pub enum GatewayBackendKind {
    Mock,
    Native,
    ExternalService,
    Unknown,
}

#[non_exhaustive]
pub enum GatewayThreadSafety {
    Reentrant,
    IndependentWorkspaceOnly,
    GloballySynchronized,
    SingleThreadOnly,
}

#[non_exhaustive]
pub enum GatewayFailureKind {
    BackendUnavailable,
    InvalidModel,
    NumericalFailure,
    Timeout,
    Cancelled,
    ForeignPanicOrAbort,
    ContractViolation,
}
```

Gateway adapters must normalize successful solves into existing cluster outcome
types where possible and failures into `SolverError` or a cluster-owned gateway
error that maps cleanly to `BatchItemOutcome::Failed`. Non-convergence from a
foreign solver remains a **status** when the foreign backend can provide a
bounded terminal report; it must not be collapsed into `SolverError`.

### 3.7 Concrete FFI adapters are follow-up work

A concrete native adapter must not be merged merely because the generic gateway
module exists. Each concrete adapter requires an adapter-specific design note or
follow-up RFC that records:

1. native library identity and version policy;
2. license compatibility;
3. ownership of every input and output buffer;
4. pointer lifetime and retention rules;
5. aliasing and alignment assumptions;
6. thread-safety classification;
7. cancellation/timeout behavior;
8. error-code translation table;
9. partial-result policy;
10. build/linking configuration;
11. cleanup behavior on partial failure;
12. how telemetry redaction is preserved.

## 4. Rust Systems-Level Nuances & Memory Safety

### 4.1 Unsafe containment

`loeres-cluster` currently does not forbid unsafe code, but RFC 009 makes FFI
unsafe blocks review-gated:

* all unsafe blocks for a native adapter live in private adapter modules;
* each unsafe block carries a short invariant comment;
* safe public functions validate dimensions, nullability, lengths, and buffer
  ownership before reaching unsafe;
* no Rust panic may unwind across a foreign ABI boundary;
* `xtask unsafe-audit` must list unsafe blocks before a concrete native adapter
  can ship.

### 4.2 Panic and abort boundary

Rust panics are caught on the Rust side where the adapter controls the call
boundary. Foreign panics, signals, process aborts, and memory corruption cannot
be reliably recovered. Each concrete adapter must state which of these can
occur and must not overstate recovery guarantees.

### 4.3 Thread safety

Gateway adapters must declare their thread-safety class. Cluster orchestration
does not schedule by this class in v1 because `ClusterJob<S>: Send + Sync`
already forms the shipped RFC 008 seam and exposes no per-job classification
channel. Enforcement is adapter-side, by construction:

* `Reentrant` adapters may run concurrently without extra adapter
  synchronization;
* `IndependentWorkspaceOnly` adapters make each `run_boxed` use its own native
  workspace;
* `GloballySynchronized` adapters hold an internal lock or equivalent
  synchronization;
* `SingleThreadOnly` adapters must be wrapped to present as
  `GloballySynchronized` or rejected at adapter construction. A raw
  single-thread backend that cannot be made `Send + Sync` never reaches
  `solve_batch`.

Orchestrator-level scheduling by gateway thread-safety class is out of scope for
v1 and requires a future seam RFC.

### 4.4 Validation and trust

Observability must never synthesize validation coverage. Gateway adapters must
either validate their converted model inputs or return a structured failure.
Trusted/cached validation over gateway models is not part of RFC 009; RFC 015
owns model identity, mutation epochs, and cacheability.

## 5. Algorithmic & Numerical Fail-Safe Guardrails

1. Observability must not affect numerical results.
2. Redaction happens before data reaches tracing or metrics adapters.
3. A telemetry event never proves validation coverage.
4. Gateway non-convergence remains a status when a bounded terminal report is
   available.
5. Gateway numerical and backend failures are structured failures, not panics.
6. Foreign partial results are not exposed as trusted Loeres solutions unless
   the adapter validates the result shape and status.
7. A third-party solver crash may be classified only according to the concrete
   adapter's documented recovery model.
8. Device crates remain unreachable from observability and gateway code.

## 6. Verification, Validation, and CI Gates

### 6.1 Redaction tests

Tests must prove that `SolveTelemetryEvent` construction and metrics/tracing
adapters do not include raw vector, matrix, objective, solution, tenant, path,
or trust-label data.

### 6.2 Feature isolation tests

The release gate must keep proving:

* `loeres`, `loeres-backend-static`, and `loeres-device` build for the
  bare-metal target with no `std` / `alloc`;
* enabling cluster observability/gateway features does not create forbidden
  dependency paths into edge crates;
* tracing/metrics/native dependencies are absent from the baseline feature set.

### 6.3 Unsafe audit

Before any concrete native adapter ships, `cargo xtask unsafe-audit` must report
all unsafe blocks and fail any unsafe block lacking an invariant comment. While
that RFC 010 command is scaffolded, concrete native adapters remain blocked.

### 6.4 Mock gateway tests

The shipped gateway implementation uses a pure Rust mock backend to test the
safe boundary without linking native code. Covered cases:

* successful solved-converged report;
* solved-not-converged report;
* backend unavailable;
* invalid model rejection;
* numerical failure;
* cancellation/timeout mapping;
* adapter self-synchronization and construction-time rejection for unsafe
  thread-safety classes.

### 6.5 Telemetry no-op tests

With observability features disabled, batch solving and RFC 016 kernel solving
must compile and run without telemetry dependencies and without runtime logging
calls.

### 6.6 Acceptance criteria

RFC 009 moved to `done/` in v0.15.0 after these criteria were met:

1. `loeres_cluster::observe` exposes metadata-only event/bucket/sink types.
2. `OutcomeKind` is per-item, includes `InternalError`, and is backed by a total
   `SolverError` classifier.
3. Metrics and tracing adapters, if any, are default-off and redacted by
   construction.
4. `loeres_cluster::gateway` exposes safe gateway boundary types without
   accepting a concrete native solver by accident.
5. `ffi-gateway` is default-off and cluster-only.
6. No observability, metrics, or gateway dependency leaks into core/static/device
   builds.
7. Gateway thread-safety policy is enforced adapter-side; orchestrator
   scheduling by class is not introduced.
8. `ClusterJob`, `BatchItemOutcome`, and the RFC 014 status/error split remain
   intact.
9. `cargo fmt --all --check`, `cargo clippy --workspace --all-features
   --all-targets -- -D warnings`, `cargo test --workspace --all-features`, and
   `cargo xtask release-gate` pass.

## 7. Rejected Alternatives

### 7.1 Use raw tracing spans as the public API

Rejected. It would put a concrete observability framework into the public
surface and make redaction depend on caller discipline. Loeres-owned event types
keep the policy enforceable.

### 7.2 Add telemetry fields to `SolveReport`

Rejected. `SolveReport` is core, no-alloc, scalar-agnostic status vocabulary.
Telemetry is cluster-only and must not change core report size or device APIs.

### 7.3 Change `ClusterJob` to accept an observer

Rejected for this RFC. RFC 008's seam is already shipped and used by RFC 016.
Observation can be layered with wrappers or helper APIs without changing the
job trait. A later RFC may revise the seam if there is strong evidence.

### 7.4 Accept a concrete native solver in the generic gateway RFC

Rejected. Each native solver has unique ABI, license, threading, and failure
semantics. The generic RFC defines the boundary; concrete adapters need their
own evidence.

### 7.5 Let users provide arbitrary metric labels

Rejected. Arbitrary labels are a privacy and cardinality risk. Labels must be
bounded categories.

## 8. Implementation Closeout

| Sprint | Work |
|---|---|
| S0 Design freeze | Completed after architect review and implementation-decision memo. |
| S1 Skeleton | `observe.rs` and `gateway.rs` populated with cluster-only public types and no external dependencies. |
| S2 Redaction tests | Added tests proving event construction and label conversion cannot include raw model data. |
| S3 Integration wrapper | Added `observe_batch_report`, `observe_batch_report_with_metadata`, and `solve_batch_observed` without changing `ClusterJob`. |
| S4 Gateway mock | Added a safe mock gateway boundary exercising status/error mapping, adapter self-synchronization, and construction-time rejection. |
| S5 Verification | `cargo fmt --all --check`, clippy, workspace tests, and `cargo xtask release-gate` observed green on the working tree; `cargo xtask release-gate` also passed on a clean copy. |
| S6 Closeout | Updated README/ROADMAP/CHANGELOG/RFC index and moved RFC 009 to `done/` with v0.15.0 evidence. |

## 9. Exit Criteria

1. Observability is usable for cluster operators without raw data leakage.
2. Metrics labels are bounded and redacted.
3. The gateway module makes FFI safety obligations explicit but does not smuggle
   an unaudited native solver into the workspace.
4. The server/device zero-bleed boundary remains mechanically verified.
5. Existing RFC 008/RFC 016 APIs keep their status/error semantics.
