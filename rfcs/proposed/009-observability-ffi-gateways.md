# RFC 009 — Observability, Metrics, and FFI Gateway Interfacing

**Status.** Proposed
**Tracks.** Phase 3 / Milestone 3 — Dynamic Infrastructure and Cloud Cluster
**Touches.** `loeres-cluster/src/observability.rs`, `loeres-cluster/src/ffi.rs`, `loeres-backend-std/src/ffi_adapter.rs`, cluster-only telemetry and FFI modules

---

### Extended Metadata
* **Rust Edition Compliance:** Rust 2024 Baseline
* **Target Environment:** `loeres-cluster` and `loeres-backend-std`; cluster-only optional features

## 1. Executive Summary & Problem Statement

Cluster users need observability, metrics, and optional access to legacy numerical engines. These needs are incompatible with device constraints and must be isolated behind cluster-only features.

This RFC defines public observability hooks, redaction rules, and FFI gateway safety fencing for the Loeres cloud ecosystem. It ensures that telemetry never leaks tenant model data and that unsafe FFI code cannot contaminate core or device crates.

## 2. Architectural Context & Dependency Alignment

This RFC depends on:

* [RFC 003](../done/003-allocation-free-errors.md) for shared error categories;
* [RFC 007](007-dynamic-sparse-adapters.md) for dynamic backend wrappers;
* [RFC 008](008-async-orchestration-budgets.md) for orchestration boundaries.

Dependency rules:

| Crate | Rule |
|---|---|
| `loeres-cluster` | Owns observability and FFI gateway features |
| `loeres-backend-std` | May expose cluster-only FFI adapter internals |
| `loeres` | No dependency on telemetry or FFI |
| `loeres-backend-static` | No telemetry or FFI dependency |
| `loeres-device` | No telemetry or FFI dependency |

The `ffi-gateway` feature is default-off.

## 3. Concrete Technical Specification

### 3.1 Feature gates

```toml
[features]
default = []
observability = ["tracing"]
metrics = ["observability"]
ffi-gateway = []
```

The exact dependency names may change, but all observability and FFI features remain cluster-only.

### 3.2 Observability event model

Telemetry events must be metadata-only.

```rust
pub struct SolveTelemetryEvent {
    pub solver_family: SolverFamilyId,
    pub problem_class: ProblemClassId,
    pub dimension_bucket: DimensionBucket,
    pub outcome: OutcomeKind,
    pub iterations_bucket: IterationsBucket,
    pub elapsed_bucket: ElapsedBucket,
}
```

The event must not contain:

* raw vectors;
* raw matrices;
* objective values;
* final solution values;
* tenant identifiers unless already pseudonymized by the caller;
* serialized problem payloads.

### 3.3 Metrics model

Metrics are aggregate counters, histograms, and gauges. Metric labels must be controlled enums or sanitized bounded strings.

Allowed labels:

* solver family;
* problem class;
* coarse dimension bucket;
* outcome category;
* backend family.

Forbidden labels:

* raw user IDs;
* matrix entries;
* vector entries;
* objective values;
* arbitrary request strings;
* file paths containing tenant data.

### 3.4 FFI gateway boundary

FFI gateways are explicit adapters around C/Fortran numerical libraries. They must not be transitively enabled by `loeres-cluster` default features.

```rust
pub trait LegacySolverGateway {
    fn solve_legacy(
        &self,
        model: &ClusterModel,
        config: &LegacyGatewayConfig,
    ) -> Result<ClusterSolution, ClusterError>;
}
```

The public trait is safe. Unsafe calls live in private adapter modules.

### 3.5 Unsafe mitigation parameters

Every FFI adapter must document:

1. ownership of input buffers;
2. ownership of output buffers;
3. aliasing assumptions;
4. alignment requirements;
5. lifetime of foreign pointers;
6. whether the foreign library may retain pointers after the call;
7. thread-safety of the foreign library;
8. panic-unwinding boundary;
9. error-code translation table;
10. cleanup behavior on partial failure.

### 3.6 Panic-unwinding boundary

Rust panics must not unwind across FFI boundaries. FFI adapter entrypoints must catch Rust panics at the Rust side where applicable and translate them into structured errors.

Foreign panics, signals, or aborts cannot always be recovered from. The gateway documentation must explicitly classify the failure model of each third-party library.

### 3.7 Data-hygiene redaction policy

Before emitting telemetry, cluster code must convert raw dimensions and values into coarse categories.

Example buckets:

```rust
pub enum DimensionBucket {
    Small,
    Medium,
    Large,
    Huge,
}

pub enum OutcomeKind {
    Solved,
    InvalidInput,
    NumericalFailure,
    Timeout,
    Cancelled,
    BackendFailure,
}
```

Raw problem data must never be logged by Loeres itself. Users may log their own data outside Loeres, but Loeres defaults must be privacy-preserving.

## 4. Rust Systems-Level Nuances & Memory Safety

### 4.1 Unsafe containment

All `unsafe` blocks for FFI must be private, small, and surrounded by checked safe wrappers. Each unsafe block must include a comment stating the invariants being relied upon.

### 4.2 Thread-safety

FFI adapters must declare whether the foreign solver is:

* reentrant;
* thread-safe with independent workspaces;
* globally synchronized;
* single-thread only.

Cluster orchestration must honor this declaration.

### 4.3 Memory ownership

Buffers passed to FFI must remain alive for the entire call. If the foreign library retains pointers, the adapter must reject the call unless ownership transfer is explicitly modeled.

### 4.4 Observability cost

Telemetry hooks must be lightweight when enabled and compile away or become no-ops when disabled. Device crates do not include them at all.

## 5. Algorithmic & Numerical Fail-Safe Guardrails

1. FFI solver failures translate to structured cluster errors.
2. FFI adapters must not expose foreign partial results as trusted Loeres solutions unless validation succeeds.
3. Metrics must not leak tenant data.
4. Observability must not affect numerical results.
5. Legacy backends must be opt-in and clearly marked as outside the panic-averse device trust boundary.
6. A third-party solver crash may be classified as `BackendFailure`; recovery guarantees depend on the foreign library and must not be overstated.

## 6. Verification, Validation, and CI Gates

### 6.1 Redaction tests

Tests must verify that telemetry events do not contain raw vector, matrix, objective, or solution values.

### 6.2 Feature isolation tests

CI must prove that enabling `observability`, `metrics`, or `ffi-gateway` affects only cluster/std crates and never core/static/device baseline builds.

### 6.3 Unsafe audit

`xtask unsafe-audit` must list every unsafe block in FFI modules and require an invariant comment.

### 6.4 FFI mock tests

A mock C-compatible backend should be used to test:

* successful solve;
* error-code translation;
* invalid pointer rejection;
* foreign backend unavailable;
* panic boundary behavior on the Rust side;
* thread-safety policy enforcement.

### 6.5 Telemetry no-op tests

When observability features are disabled, the code path must compile without telemetry dependencies and without runtime logging calls.

### 6.6 Acceptance criteria

RFC 009 may move to `done/` only when:

1. observability emits metadata-only events;
2. metrics labels are bounded and redacted;
3. FFI gateways are default-off and cluster-only;
4. unsafe blocks are isolated and audited;
5. no observability or FFI dependency leaks into core/static/device builds.
