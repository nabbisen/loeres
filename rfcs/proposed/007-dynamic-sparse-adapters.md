# RFC 007 — Dynamic Dense and Sparse Storage Adapters

**Status.** Proposed
**Tracks.** Phase 3 / Milestone 3 — Dynamic Infrastructure and Cloud Cluster
**Touches.** `loeres-backend-std/src/dense.rs`, `loeres-backend-std/src/sparse.rs`, `loeres-backend-std/src/ingest.rs`, `loeres-backend-std/src/lib.rs`

---

### Extended Metadata
* **Rust Edition Compliance:** Rust 2024 Baseline
* **Target Environment:** `loeres-backend-std`; consumed by `loeres-cluster`

## 1. Executive Summary & Problem Statement

Cluster workloads need dynamic dimensions, sparse data, heap allocation, and interoperability with high-throughput Rust numerical libraries. At the same time, these dynamic capabilities must not leak backward into `loeres-core`, `loeres-backend-static`, or `loeres-device`.

This RFC defines the external design for `loeres-backend-std` dense and sparse adapters. The adapters implement core access contracts over heap-backed layouts and provide allocation-minimizing ingestion paths for request-driven workloads.

## 2. Architectural Context & Dependency Alignment

This RFC depends on:

* [RFC 001](001-stratified-scalar.md);
* [RFC 002](002-storage-agnostic-contracts.md);
* [RFC 003](../done/003-allocation-free-errors.md).

Dependency rules:

| Crate | Rule |
|---|---|
| `loeres-backend-std` | May use `std`, heap allocation, third-party dense/sparse crates |
| `loeres-core` | Must not depend on this crate |
| `loeres-backend-static` | Must not depend on this crate |
| `loeres-device` | Must not depend on this crate |
| `loeres-cluster` | Consumes this crate |

`loeres-backend-std` is server-facing only.

## 3. Concrete Technical Specification

### 3.1 Module layout

```rust
pub mod dense;
pub mod ingest;
pub mod sparse;

pub use dense::DenseMatrix;
pub use dense::DenseVector;
pub use sparse::SparseMatrix;
pub use sparse::SparseVector;
pub use ingest::{DenseIngestPolicy, SparseIngestPolicy};
```

### 3.2 Dynamic dense adapters

Dense adapters wrap dynamic heap-backed storage while implementing `VectorAccess` and `MatrixAccess`.

```rust
pub struct DenseVector<S> {
    // private heap-backed layout
}

pub struct DenseMatrix<S> {
    // private heap-backed layout
}
```

Public construction patterns:

```rust
impl<S: BaseScalar> DenseVector<S> {
    pub fn from_vec(data: Vec<S>) -> Result<Self, SolverError>;
    pub fn len(&self) -> usize;
}

impl<S: BaseScalar> DenseMatrix<S> {
    pub fn from_row_major_vec(rows: usize, cols: usize, data: Vec<S>) -> Result<Self, SolverError>;
    pub fn dims(&self) -> Dim2;
}
```

Using `Vec` is permitted only in `loeres-backend-std` and cluster-facing APIs.

### 3.3 Dynamic sparse adapters

Sparse adapters must define explicit missing-entry semantics.

```rust
pub enum SparseMissingEntryPolicy {
    ImplicitZero,
    ErrorOnMissing,
}

pub struct SparseMatrix<S> {
    // private sparse layout
    missing_policy: SparseMissingEntryPolicy,
}
```

When `ImplicitZero` is selected, `get(row, col)` may return `S::zero()` for absent entries. When `ErrorOnMissing` is selected, absent entries return a structured error.

### 3.4 Backend mappings

The implementation may map to third-party dynamic libraries behind feature gates, for example:

| Feature | Purpose | Public leak allowed? |
|---|---|---|
| `ndarray-adapter` | dense dynamic matrix storage | No core leak |
| `sparse-adapter` | sparse matrix storage | No core leak |
| `serde-ingest` | request payload ingestion | No core/device leak |

Concrete third-party types must be hidden behind Loeres wrapper types unless a feature explicitly exposes conversion helpers.

### 3.5 Allocation-minimizing ingestion

The ingestion module must support direct construction from incoming payload structures with minimal copying.

```rust
pub enum DenseIngestPolicy {
    BorrowThenCopyOnce,
    TakeOwnership,
    ValidateThenTakeOwnership,
}

pub enum SparseIngestPolicy {
    TripletStream,
    CompressedOwned,
    ValidateThenCompress,
}
```

The target design is one allocation for final storage, not one allocation per row or per element.

### 3.6 Validation state

To avoid repeated large O(N) scans in trusted server pipelines, dynamic adapters may carry validation state:

```rust
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ValidationState {
    Unvalidated,
    FiniteChecked,
    TrustedByCaller,
}
```

`TrustedByCaller` must require an explicit API call and must be documented as a responsibility transfer. It is safe Rust, but it weakens Loeres validation guarantees by caller assertion and must be visible in code review.

## 4. Rust Systems-Level Nuances & Memory Safety

### 4.1 Heap allocation visibility

All heap allocation is confined to `loeres-backend-std` and `loeres-cluster`. No type from this RFC may appear in core/device public signatures.

### 4.2 Double allocation avoidance

Ingestion APIs should take ownership of already-collected buffers when possible. APIs that accept borrowed request data must document whether they copy once or retain references.

### 4.3 Sparse access cost

`MatrixAccess::get(row, col)` may be expensive for some sparse layouts. Algorithms requiring efficient sparse traversal must use backend-specific extension traits, not the core baseline access trait alone.

### 4.4 `unsafe` policy

Baseline wrappers use safe Rust. If a third-party backend requires unsafe conversions, those conversions must be isolated behind private functions with documented invariants and tests.

## 5. Algorithmic & Numerical Fail-Safe Guardrails

1. Invalid dimensions return structured errors.
2. Duplicate sparse entries must have a declared combine policy or be rejected.
3. Non-finite validation must be available but may be skipped only with explicit validation state.
4. Sparse missing entries must have explicit policy.
5. Ingestion must reject payloads that would exceed configured memory limits.
6. Cluster-facing construction must not panic on malformed user payloads.

## 6. Verification, Validation, and CI Gates

### 6.1 Access conformance tests

Dense and sparse adapters must pass RFC 002 access conformance tests.

### 6.2 Allocation behavior tests

Ingestion tests must count or otherwise profile representative allocation behavior for:

* dense row-major payload ingestion;
* sparse triplet ingestion;
* ownership-taking ingestion;
* validation failure cleanup.

### 6.3 Dependency direction check

`xtask zero-bleed` must prove no reverse dependency from core/static/device to backend-std.

### 6.4 Validation-state tests

Tests must verify that:

* `Unvalidated` triggers validation at public solve boundaries;
* `FiniteChecked` skips redundant finite scans where allowed;
* `TrustedByCaller` is explicit and auditable.

### 6.5 Acceptance criteria

RFC 007 may move to `done/` only when:

1. dynamic dense and sparse wrappers implement core access traits;
2. final storage can be constructed without per-element allocation;
3. validation-state policy is represented publicly;
4. malformed payloads return structured errors;
5. no dynamic backend type leaks into core/device signatures.
