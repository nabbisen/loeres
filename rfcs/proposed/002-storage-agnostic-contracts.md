# RFC 002 — Storage-Agnostic Matrix and Vector Access Contracts

**Status.** Proposed
**Tracks.** Phase 1 / Milestone 1 — Foundational Core Architecture
**Touches.** `loeres/src/access.rs`, `loeres/src/dimension.rs`, `loeres/src/lib.rs`, core vector/matrix access namespaces

---

### Extended Metadata
* **Rust Edition Compliance:** Rust 2024 Baseline
* **Target Environment:** `loeres`; consumed by static and dynamic backends

> **Revision note (v0.6.1, in response to the v0.6.0 architect review).** This RFC
> was patched before implementation: the module is `dimension`, not `dim` (B1);
> `DimensionKind` no longer carries `Borrowed`, which is an ownership property,
> not a dimension property (B6); core owns only simple contiguous row-major views,
> with strided/sub-matrix views deferred to `loeres-backend-static` / RFC 004
> (B2); an optional contiguous fast-path surface was added for RFC 006 kernels
> (B3); an explicit access-error mapping over the RFC 003 `SolverError` set was
> added, with a no-silent-`usize`→`u32`-truncation rule (B4); and core admits no
> overlapping mutable view (B5).

## 1. Executive Summary & Problem Statement

Loeres must allow the same mathematical solver contracts to operate over multiple storage families without forcing those storage families into one memory model. A cluster backend may use heap-backed dense and sparse matrices, while a device backend may use fixed arrays, borrowed DMA buffers, or statically sized views.

This RFC defines storage-agnostic access contracts for vectors and matrices. These contracts describe dimensions and fallible element access only. They deliberately do not define heavyweight linear algebra kernels such as matrix multiplication, Cholesky factorization, sparse assembly, or BLAS-style routines.

The public module name is `loeres::access`, not `loeres::linalg`, to avoid suggesting that core owns algorithmic linear algebra operations.

## 2. Architectural Context & Dependency Alignment

This RFC touches only `loeres`. It depends on [RFC 001](../done/001-stratified-scalar.md) for scalar bounds and [RFC 003](../done/003-allocation-free-errors.md) for error types.

Dependency alignment:

| Crate | Role |
|---|---|
| `loeres` | Defines dimension and access traits only |
| `loeres-backend-static` | Implements traits for fixed arrays and borrowed views |
| `loeres-backend-std` | Implements traits for heap-backed dense/sparse layouts |
| `loeres-device` | Consumes these traits via static dispatch only |
| `loeres-cluster` | Consumes these traits directly or via backend adapters |

No `std`, `alloc`, or backend type may appear in these core trait definitions.

## 3. Concrete Technical Specification

### 3.1 Module layout

```rust
pub mod access;
pub mod dimension;

pub use access::{
    ContiguousMatrixAccess,
    ContiguousVectorAccess,
    ContiguousVectorAccessMut,
    MatrixAccess,
    MatrixAccessMut,
    MatrixView,
    MatrixViewMut,
    VectorAccess,
    VectorAccessMut,
    VectorView,
    VectorViewMut,
};

pub use dimension::{Dim2, DimensionKind};
```

### 3.2 Dimension representation

Dimensions are runtime values even when backed by compile-time constants. This keeps trait signatures uniform across static and dynamic storage.

```rust
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Dim2 {
    pub rows: usize,
    pub cols: usize,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DimensionKind {
    Static,
    Dynamic,
}
```

`Dim2` is data-only and allocation-free. Static backends return constants as ordinary `usize` values.

`DimensionKind` describes **only** whether a dimension is known at compile time
(`Static`) or at run time (`Dynamic`). It deliberately does **not** carry a
`Borrowed` variant: borrowed-versus-owned is a storage/view-ownership property,
not a dimension property — a borrowed view may have static or dynamic dimensions.
If a future RFC needs to expose ownership at the type or value level, it does so
through a separate `StorageKind` / `AccessOrigin` concept, so that algorithms
branch on dimension or layout information rather than on ownership.

### 3.3 Vector access traits

```rust
use crate::error::SolverError;
use crate::scalar::BaseScalar;

pub trait VectorAccess {
    type Scalar: BaseScalar;

    #[inline]
    fn len(&self) -> usize;

    #[inline]
    fn dimension_kind(&self) -> DimensionKind;

    #[inline]
    fn get(&self, index: usize) -> Result<Self::Scalar, SolverError>;
}

pub trait VectorAccessMut: VectorAccess {
    #[inline]
    fn set(&mut self, index: usize, value: Self::Scalar) -> Result<(), SolverError>;
}
```

The `get` and `set` methods must be fallible. Out-of-bounds access must return a structured error.

### 3.4 Matrix access traits

```rust
pub trait MatrixAccess {
    type Scalar: BaseScalar;

    #[inline]
    fn dims(&self) -> Dim2;

    #[inline]
    fn dimension_kind(&self) -> DimensionKind;

    #[inline]
    fn get(&self, row: usize, col: usize) -> Result<Self::Scalar, SolverError>;
}

pub trait MatrixAccessMut: MatrixAccess {
    #[inline]
    fn set(&mut self, row: usize, col: usize, value: Self::Scalar) -> Result<(), SolverError>;
}
```

These traits are layout-independent. A backend may be row-major, column-major, strided, block-structured, or sparse. Core callers must not infer memory layout from trait conformance.

### 3.5 Borrowed vector views

Borrowed views provide allocation-free adapters for existing memory. They live in `loeres` only if they can be represented with slices and no backend assumptions.

```rust
pub struct VectorView<'a, S: BaseScalar> {
    data: &'a [S],
}

pub struct VectorViewMut<'a, S: BaseScalar> {
    data: &'a mut [S],
}
```

Public constructors:

```rust
impl<'a, S: BaseScalar> VectorView<'a, S> {
    #[inline]
    pub fn from_slice(data: &'a [S]) -> Self;

    #[inline]
    pub fn as_slice(&self) -> &'a [S];
}

impl<'a, S: BaseScalar> VectorViewMut<'a, S> {
    #[inline]
    pub fn from_slice_mut(data: &'a mut [S]) -> Self;
}
```

A `VectorViewMut` must not expose APIs that allow aliasing mutable access. The Rust lifetime system is the safety boundary.

### 3.6 Borrowed matrix views

`loeres` owns only a **simple contiguous, row-major** dense matrix view.
This keeps core small and avoids turning RFC 002 into a storage-layout RFC.
Advanced views — column-major, arbitrary strided, and sub-matrix views — are
**not** in the core baseline; they belong to `loeres-backend-static` behind the
`static-views` feature (RFC 004), and to `loeres-backend-std` for dynamic
storage. A backend's own views still implement the layout-agnostic
[`MatrixAccess`] traits above, so core kernels consume any backend uniformly;
core simply does not provide the strided concrete type.

```rust
pub struct MatrixView<'a, S: BaseScalar> {
    data: &'a [S],
    rows: usize,
    cols: usize,
}

pub struct MatrixViewMut<'a, S: BaseScalar> {
    data: &'a mut [S],
    rows: usize,
    cols: usize,
}
```

Element `(row, col)` maps to `data[row * cols + col]`. Constructors must validate
that the slice is exactly large enough for the declared dimensions
(`rows * cols`, with the multiplication checked for overflow). Invalid
construction returns `SolverError` (see the access-error mapping in §5).

```rust
impl<'a, S: BaseScalar> MatrixView<'a, S> {
    #[inline]
    pub fn from_row_major(data: &'a [S], rows: usize, cols: usize) -> Result<Self, SolverError>;
}
```

### 3.7 Static dispatch rule

Core access traits are not object-safe targets for device mathematics. The public design explicitly bans:

```rust
&dyn MatrixAccess<Scalar = S>
&dyn VectorAccess<Scalar = S>
```

inside `loeres`, `loeres-backend-static`, and `loeres-device` solver kernels.

Allowed pattern:

```rust
pub fn residual_norm<V, S>(v: &V) -> Result<S, SolverError>
where
    V: VectorAccess<Scalar = S>,
    S: MetricScalar,
{
    /* generic static dispatch */
}
```

Dynamic dispatch may appear in `loeres-cluster` orchestration boundaries only, as specified by RFC 008.

### 3.8 No mandatory operations; optional contiguous fast path

The access traits must not require:

* iteration allocation;
* row extraction into temporary vectors;
* matrix multiplication;
* factorization;
* sparse traversal APIs;
* `Iterator` return types that require complex associated lifetimes unless separately accepted by RFC.

The fallible per-element traits in §3.3–§3.4 are the **baseline**. They are safe
but carry a bounds check and `Result` branch on every scalar access, which is
expensive in a tight inner loop. So core also defines a small, **optional**
fast-path surface that a kernel can branch into once when a backend exposes
contiguous storage. These are extension traits: a backend implements them only
when it can, and a kernel falls back to fallible access when it cannot.

```rust
/// Implemented by vector storage that is contiguous in memory.
pub trait ContiguousVectorAccess: VectorAccess {
    /// The contiguous backing slice, or `None` if the storage is not contiguous.
    fn as_contiguous(&self) -> Option<&[Self::Scalar]>;
}

pub trait ContiguousVectorAccessMut: ContiguousVectorAccess + VectorAccessMut {
    fn as_contiguous_mut(&mut self) -> Option<&mut [Self::Scalar]>;
}

/// Implemented by dense matrix storage with a row-major contiguous backing,
/// for kernels such as a Hessian mat-vec.
pub trait ContiguousMatrixAccess: MatrixAccess {
    /// The row-major contiguous backing (length `rows * cols`), or `None`.
    fn as_row_major(&self) -> Option<&[Self::Scalar]>;
}
```

Scope is deliberately narrow — exactly what the first device kernel (RFC 006)
needs. Sparse traversal is **not** part of this fast path; it belongs to RFC 007.
The exact fast-path surface is finalized by the RFC 006 kernel-scope decision; a
kernel uses it as:

```rust
match v.as_contiguous() {
    Some(slice) => { /* tight, branch-free loop */ }
    None => { /* fall back to fallible per-element access */ }
}
```

## 4. Rust Systems-Level Nuances & Memory Safety

### 4.1 Bounds checks and panic aversion

All direct indexing inside view implementations must use checked index calculations. Implementations must not use unchecked indexing or `unwrap` in core baseline.

Matrix offset computation must be checked for:

1. row out of range;
2. column out of range;
3. stride multiplication overflow;
4. offset addition overflow;
5. offset beyond slice length.

### 4.2 Lifetime constraints

`VectorView<'a>` and `MatrixView<'a>` must not extend or store references beyond `'a`. No self-referential structure is permitted. Mutable view constructors require `&'a mut [S]` and must not provide cloning.

### 4.3 Result size pressure

Because access operations may appear in tight loops, `SolverError` size is constrained by RFC 003. RFC 002 must not introduce large access-specific error payloads.

### 4.4 `unsafe` policy

Core view implementations must use safe Rust only. If a future performance RFC proposes unchecked accessors, they must be separate extension APIs that are unavailable in the safe baseline.

### 4.5 No overlapping mutable views in core

The core `VectorViewMut` and `MatrixViewMut` are contiguous and therefore
**injective**: every logical `(row, col)` maps to a distinct backing element, so
`set` has no surprising aliasing. Overlapping or broadcast-style mutable layouts —
where a custom stride could map several logical cells onto the same element — are
**not** part of the core baseline. Such a layout is not Rust memory-unsafety when
implemented with checked indexing, but it is a mathematical aliasing bug that
makes `set(row, col)` non-deterministic. Any future custom-strided *mutable* view
(a backend concern, RFC 004) must either be injective and reject overlapping
layouts, or be exposed as read-only. For v0.x, core admits no overlapping mutable
view.

## 5. Algorithmic & Numerical Fail-Safe Guardrails

1. Invalid dimensions fail during view construction where possible.
2. Out-of-bounds element access fails at the access boundary.
3. Solver kernels must not assume that `.get()` cannot fail.
4. Sparse backend adapters may return zero for implicit missing entries only if their public storage semantics define that behavior.
5. Core access traits do not perform finite-value validation. Public solve entrypoints own input validation policy.

### 5.1 Access error mapping

Because RFC 003 is implemented, RFC 002 reuses its canonical `SolverError` set
rather than inventing access-specific variants. The mapping is fixed:

| Access failure | `SolverError` |
|---|---|
| Declared dimension invalid (e.g. zero where positive is required) | `InvalidDimension` |
| Shape mismatch between two objects (lengths/shapes that had to agree) | `DimensionMismatch { lhs, rhs }` |
| Element index out of bounds | `DimensionMismatch { lhs: index, rhs: len }` (the requested index and the valid length) |
| Backing slice too small / `rows * cols` overflow at view construction | `InvalidDimension` |
| A dimension or index that does not fit a `u32` diagnostic payload | `InvalidDimension` if it came from caller input; `InternalInvariantViolation` if from library logic |

`DimensionMismatch` and the index payloads are `u32` (RFC 003). Conversions from
`usize` to `u32` for these payloads must be **checked**: a value exceeding
`u32::MAX` returns `InvalidDimension` (caller input) or
`InternalInvariantViolation` (library logic), and **must never** be silently
truncated. The choice of `DimensionMismatch { lhs: index, rhs: len }` for an
out-of-bounds index is deliberate (it preserves both the bad index and the
bound); a diagnostic consumer distinguishes index-vs-shape mismatches by context.

## 6. Verification, Validation, and CI Gates

### 6.1 Core compile gates

CI must verify that `loeres::access` compiles under `no_std` without `alloc`.

### 6.2 View safety tests

Tests must cover:

* valid vector view access;
* invalid vector index error;
* valid row-major matrix view access;
* invalid matrix dimensions (backing slice too small);
* `rows * cols` overflow at construction;
* checked `usize` → `u32` conversion (oversized index/dimension yields a structured error, never a truncated payload);
* mutable set followed by get;
* no panic on invalid indices;
* contiguous fast path: `as_contiguous` / `as_row_major` return `Some` for the core contiguous views, and a kernel falls back correctly when they return `None`.

### 6.3 Static dispatch audit

`xtask check-public-api` (RFC 010) must reject `dyn MatrixAccess`, `dyn VectorAccess`, `Box<dyn MatrixAccess>`, and equivalent virtual-dispatch forms inside edge-facing crates. (This is the same `check-public-api` gate RFC 014 uses for `dyn AsCoreReport`.)

### 6.4 Backend conformance tests

Each backend must provide conformance tests that run the same access corpus against:

* `VectorView` / `MatrixView`;
* fixed static storage from RFC 004;
* dynamic dense storage from RFC 007;
* dynamic sparse storage from RFC 007 where applicable.

### 6.5 Acceptance criteria

RFC 002 may move to `done/` only when:

1. `loeres::access` exposes access-only traits and borrowed views;
2. view constructors validate dimensions without panics;
3. no heavy linalg operations exist in core baseline traits;
4. static dispatch is enforced for core/device paths;
5. all view tests pass on a `no_std` target build profile.
