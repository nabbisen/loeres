# RFC 007 — Dynamic Dense and Sparse Storage Adapters

**Status.** Implemented (v0.11.0). Storage-first dynamic dense/sparse adapters in `loeres-backend-std`; canonical validation-state ownership deferred to RFC 012.
**Tracks.** Phase 3 / Milestone 3 — Dynamic Infrastructure and Cloud Cluster
**Touches.** `loeres-backend-std/src/dense.rs` (+ `dense/ingest.rs`), `loeres-backend-std/src/sparse.rs` (+ `sparse/ingest.rs`), `loeres-backend-std/src/lib.rs`

---

### Extended Metadata
* **Rust Edition Compliance:** Rust 2024 Baseline
* **Target Environment:** `loeres-backend-std`; consumed by `loeres-cluster`

## 1. Executive Summary & Problem Statement

Cluster workloads need dynamic dimensions, sparse data, heap allocation, and interoperability with high-throughput Rust numerical libraries. At the same time, these dynamic capabilities must not leak backward into `loeres`, `loeres-backend-static`, or `loeres-device`.

This RFC defines the external design for `loeres-backend-std` dense and sparse adapters. The adapters implement core access contracts over heap-backed layouts and provide allocation-minimizing ingestion paths for request-driven workloads.

## 2. Architectural Context & Dependency Alignment

This RFC depends on:

* [RFC 001](001-stratified-scalar.md);
* [RFC 002](002-storage-agnostic-contracts.md);
* [RFC 003](003-allocation-free-errors.md).

Dependency rules:

| Crate | Rule |
|---|---|
| `loeres-backend-std` | May use `std`, heap allocation, third-party dense/sparse crates |
| `loeres` | Must not depend on this crate |
| `loeres-backend-static` | Must not depend on this crate |
| `loeres-device` | Must not depend on this crate |
| `loeres-cluster` | Consumes this crate |

`loeres-backend-std` is server-facing only.

**Validation-state ownership (F3).** This RFC is **storage-first**. Canonical validation-state and trusted-input types are owned by [RFC 012](../proposed/012-validation-state-and-trusted-input-policy.md) and live in `loeres` core; RFC 007 does **not** define them. RFC 007 provides only ordinary construction checks (dimension, duplicate, memory-limit). Sequencing: RFC 007 lands the dynamic storage foundation; RFC 012 follows and integrates shared validation-state markers into backend-std / cluster APIs **before** RFC 008 relies on validated/trusted-input semantics.

## 3. Concrete Technical Specification

### 3.1 Module layout

RFC 007 keeps the accepted `loeres-backend-std` topography (external-design §1.5) unchanged — `dense`, `sparse`, `view`, `batch`, `adapter` — and populates `dense` and `sparse`. Ingestion lives as constructors/builders and nested implementation submodules (`dense::ingest`, `sparse::ingest`), not as a new top-level public module.

```rust
// dense and sparse are populated by this RFC; view/batch/adapter are unchanged.
pub use dense::{DenseMatrix, DenseVector};
pub use sparse::SparseMatrix;
```

`SparseVector` is **out of scope** for the RFC 007 baseline (F5); it may be added by a later RFC that fully specifies its layout, duplicate/missing-entry semantics, construction, and access-trait impls.

### 3.2 Dynamic dense adapters

Dense adapters wrap dynamic heap-backed (row-major `Vec<S>`) storage. **Trait coverage (P2):**

- `DenseVector<S>` implements `VectorAccess`, `VectorAccessMut`, `ContiguousVectorAccess`, and `ContiguousVectorAccessMut`.
- `DenseMatrix<S>` implements `MatrixAccess`, `MatrixAccessMut`, and `ContiguousMatrixAccess`. RFC 002 provides no `ContiguousMatrixAccessMut`, so RFC 007 does not invent one.

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

**Construction error mapping (P3).** Constructors map failures precisely (no loose "`InvalidInput` or `DimensionMismatch`"):

| Failure | Error |
|---|---|
| zero vector length, zero rows, or zero columns | `SolverError::InvalidDimension` |
| `rows * cols` checked-multiply overflow | `SolverError::InvalidDimension` |
| row-major dense data length mismatch | `SolverError::DimensionMismatch { lhs: actual_len, rhs: required_len }` if both fit `u32`; otherwise `SolverError::InvalidDimension` |
| sparse out-of-bounds triplet coordinate | `SolverError::DimensionMismatch { lhs: index, rhs: bound }` if both fit `u32`; otherwise `SolverError::InvalidDimension` |
| sparse duplicate `(row, col)` coordinate | `SolverError::InvalidInput` |
| ingestion memory limit exceeded | `SolverError::InvalidInput` |

**`u32` payload-fallback rule (Correction 2).** `DimensionMismatch { lhs, rhs }` carries `u32` payloads. The conversion is checked, never truncating: if **both** diagnostic values fit `u32`, return `DimensionMismatch { lhs, rhs }`; if **either** exceeds `u32`, return the non-payload `SolverError::InvalidDimension`. This applies to dense length mismatch and sparse out-of-bounds coordinate/bound diagnostics alike.

### 3.3 Dynamic sparse adapters

`SparseMatrix` implements `MatrixAccess` over a heap-backed sparse layout. **Trait coverage (P2):** `SparseMatrix<S>` implements `MatrixAccess` only; mutable sparse editing (`MatrixAccessMut`) and efficient sparse traversal are deferred to sparse-specific extension APIs and later RFCs. Per the RFC 002 access contract, an **in-bounds** `get(row, col)` always succeeds; an absent (unstored) in-bounds entry returns `Ok(S::zero())` (implicit zero). Out-of-bounds indices return `SolverError::DimensionMismatch` per RFC 002. The baseline does **not** change the core access contract's in-bounds success semantics (F4).

```rust
pub struct SparseMatrix<S> {
    // private sparse layout (e.g. compressed sparse row)
}
```

Callers that must distinguish a *stored* value from an *implicit* zero use a sparse-specific extension, not the core access trait:

```rust
impl<S: BaseScalar> SparseMatrix<S> {
    /// `Some(value)` if `(row, col)` is explicitly stored, `None` if absent
    /// (implicit zero). Out-of-bounds returns `Err(DimensionMismatch)`.
    pub fn try_get_stored(&self, row: usize, col: usize) -> Result<Option<S>, SolverError>;
}
```

**Duplicate entries (F7).** Sparse constructors reject duplicate `(row, col)` coordinates with `SolverError::InvalidInput`. No `Sum` / `KeepLast` combine policy is introduced in the baseline; an explicit combine policy is a later-RFC extension. Efficient sparse traversal beyond `MatrixAccess::get` belongs to backend-specific extension traits (§4.3), not the core baseline access trait.

### 3.4 Backend mappings

The implementation may map to third-party dynamic libraries behind feature gates, for example:

| Feature | Purpose | Public leak allowed? |
|---|---|---|
| `adapter-ndarray` | dense dynamic matrix storage via an `ndarray`-style backend | No core leak |
| `sparse` | sparse matrix storage | No core leak |
| `serde` | request payload (de)serialization for ingestion | No core/device leak |

These match the accepted `loeres-backend-std` feature matrix (external-design §1.6.4). A distinct `serde-ingest` feature is not introduced unless a later RFC proves ingestion serialization must be separable from the general server-side `serde` feature.

Concrete third-party types must be hidden behind Loeres wrapper types unless a feature explicitly exposes conversion helpers.

### 3.5 Allocation-minimizing ingestion

Ingestion constructors (in `dense::ingest` / `sparse::ingest`) support direct construction from incoming payloads with minimal copying. The copy/ownership strategy is:

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

The target is a bounded, layout-level allocation count for final storage, not one allocation per row or per element. Dense storage is one flat allocation. CSR sparse storage may use a small fixed number of final buffers such as `row_ptr`, `col_idx`, and `values`.

Coordinate validation and duplicate detection may require sorting / temporary preparation, but ingestion must avoid per-entry heap churn and must not allocate final storage after a known limit failure (the `max_elements` / `max_entries` check precedes final-storage allocation).

**Memory-limit options (F8).** Because the dynamic storage layer is where heap allocation happens, ingestion owns a local pre-allocation memory limit rather than deferring all of it to the cluster RFCs:

```rust
pub struct DenseIngestOptions {
    pub max_elements: Option<usize>,
}

pub struct SparseIngestOptions {
    pub max_entries: Option<usize>,
}
```

When `Some(limit)` is set, ingestion checks the payload size **before** allocating final storage and returns `SolverError::InvalidInput` if it would be exceeded; `None` imposes no limit. Cluster RFCs (008/009) may later pass service-level policy into these options; RFC 007 provides the local enforcement hook. No new error variant is introduced.

### 3.6 Validation state and scalar bounds

**Canonical validation state is RFC 012-owned (F3).** RFC 007 does not define a `ValidationState` type or a trusted-input policy; those are owned by RFC 012 and live in `loeres` core (allocation-free, shared by device and cluster). The dynamic adapters expose only **ordinary construction checks**: dimension validity, sparse duplicate rejection (§3.3), and ingestion memory limits (§3.5). When RFC 012 lands, it may attach canonical validated/trusted markers to the result of a finite-validation pass over these adapters; RFC 007 does not pre-empt that representation (external-design open question 13).

**Scalar bounds (F6).** Construction and access bound `S: BaseScalar` (sufficient for `VectorAccess` / `MatrixAccess` and `SparseMatrix`'s implicit-zero `S::zero()`). Finite-validation helpers — a non-finite scan over a constructed adapter — bound `S: FiniteScalar` (`is_finite`) and return `Result<(), SolverError>` (`NonFiniteInput` on the first non-finite element). For sparse, the finite scan covers **stored values only**; absent entries are implicit zero and need no scan. The representation of validated/trusted *state* is RFC 012-owned, not part of this RFC.

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

1. Invalid dimensions return errors per the §3.2 construction error mapping: `SolverError::InvalidDimension` for zero or overflowing extents, `SolverError::DimensionMismatch` for length / coordinate disagreements.
2. Duplicate sparse `(row, col)` entries are rejected at construction with `SolverError::InvalidInput` (no baseline combine policy; §3.3).
3. Non-finite validation is available via `FiniteScalar` helpers (§3.6). Skipping validation under a trusted/validated state is RFC 012's domain, not this RFC's.
4. Sparse missing in-bounds entries read as implicit zero through `MatrixAccess::get`; the stored-vs-implicit distinction is via the sparse extension (`try_get_stored`, §3.3).
5. Ingestion rejects payloads exceeding the configured `max_elements` / `max_entries` limit before allocating final storage (§3.5).
6. Cluster-facing construction must not panic on malformed user payloads; it returns structured errors.

## 6. Verification, Validation, and CI Gates

### 6.1 Access conformance tests

There is not yet a shared reusable cross-backend conformance suite; **RFC 013 owns the future shared conformance corpus**. Until then RFC 007 ships its own access tests mirroring RFC 002 behavior for the dynamic adapters:

* dense vector / matrix valid in-bounds reads;
* dense mutable writes (`VectorAccessMut` / `MatrixAccessMut`);
* dynamic row-major exact-length validation at construction;
* sparse valid reads;
* sparse missing in-bounds entry returns `S::zero()`;
* sparse `try_get_stored` distinguishes a stored zero from an implicit zero;
* out-of-bounds maps to the RFC 002 `DimensionMismatch` policy;
* dense contiguous fast paths (`as_contiguous` / `as_contiguous_mut`) return `Some` with exact lengths;
* sparse does not claim a dense-like contiguous fast path unless actually supported.

These tests are RFC-007-local; they are superseded by the shared corpus when RFC 013 lands.

### 6.2 Allocation behavior tests

Ingestion tests must count or otherwise profile representative allocation behavior for:

* dense row-major payload ingestion;
* sparse triplet ingestion;
* ownership-taking ingestion;
* validation failure cleanup.

### 6.3 Dependency direction check

`xtask zero-bleed` must prove no reverse dependency from core/static/device to backend-std.

### 6.4 Construction-check tests

Tests must verify the ordinary construction checks RFC 007 owns:

* invalid dimensions return structured errors;
* duplicate sparse `(row, col)` coordinates are rejected with `SolverError::InvalidInput`;
* ingestion rejects payloads exceeding `max_elements` / `max_entries` before allocation;
* the `FiniteScalar` finite-validation helper flags non-finite entries;
* sparse `MatrixAccess::get` returns implicit zero for missing in-bounds entries, and `try_get_stored` distinguishes stored from implicit.

Canonical validated/trusted-input state tests are RFC 012-owned.

### 6.5 Acceptance criteria

RFC 007 may move to `done/` only when:

1. dynamic dense and sparse wrappers (`DenseVector`, `DenseMatrix`, `SparseMatrix`) implement the RFC 002 access traits;
2. final storage can be constructed without per-element allocation;
3. sparse `get` reads implicit zero for missing in-bounds entries, duplicates are rejected, and the stored-vs-implicit extension (`try_get_stored`) exists;
4. ingestion enforces the configured memory limit before allocating, and malformed payloads return structured errors without panicking;
5. no dynamic backend type leaks into core/device signatures;
6. canonical validation-state ownership remains with RFC 012 (no `ValidationState` type is defined in this RFC).
