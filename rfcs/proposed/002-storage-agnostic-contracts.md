# RFC 002 — Storage-Agnostic Matrix and Vector Access Contracts

**Status.** Proposed
**Tracks.** Phase 1 / Milestone 1 — Foundational Core Architecture
**Touches.** `loeres-core/src/access.rs`, `loeres-core/src/dim.rs`, `loeres-core/src/lib.rs`, core vector/matrix access namespaces

---

### Extended Metadata
* **Rust Edition Compliance:** Rust 2024 Baseline
* **Target Environment:** `loeres-core`; consumed by static and dynamic backends

## 1. Executive Summary & Problem Statement

Loeres must allow the same mathematical solver contracts to operate over multiple storage families without forcing those storage families into one memory model. A cluster backend may use heap-backed dense and sparse matrices, while a device backend may use fixed arrays, borrowed DMA buffers, or statically sized views.

This RFC defines storage-agnostic access contracts for vectors and matrices. These contracts describe dimensions and fallible element access only. They deliberately do not define heavyweight linear algebra kernels such as matrix multiplication, Cholesky factorization, sparse assembly, or BLAS-style routines.

The public module name is `loeres_core::access`, not `loeres_core::linalg`, to avoid suggesting that core owns algorithmic linear algebra operations.

## 2. Architectural Context & Dependency Alignment

This RFC touches only `loeres-core`. It depends on [RFC 001](001-stratified-scalar.md) for scalar bounds and [RFC 003](../done/003-allocation-free-errors.md) for error types.

Dependency alignment:

| Crate | Role |
|---|---|
| `loeres-core` | Defines dimension and access traits only |
| `loeres-backend-static` | Implements traits for fixed arrays and borrowed views |
| `loeres-backend-std` | Implements traits for heap-backed dense/sparse layouts |
| `loeres-device` | Consumes these traits via static dispatch only |
| `loeres-cluster` | Consumes these traits directly or via backend adapters |

No `std`, `alloc`, or backend type may appear in these core trait definitions.

## 3. Concrete Technical Specification

### 3.1 Module layout

```rust
pub mod access;
pub mod dim;

pub use access::{
    MatrixAccess,
    MatrixAccessMut,
    MatrixView,
    MatrixViewMut,
    VectorAccess,
    VectorAccessMut,
    VectorView,
    VectorViewMut,
};

pub use dim::{Dim2, DimensionKind};
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
    Borrowed,
}
```

`Dim2` is data-only and allocation-free. Static backends return constants as ordinary `usize` values.

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

Borrowed views provide allocation-free adapters for existing memory. They live in `loeres-core` only if they can be represented with slices and no backend assumptions.

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

Core matrix views support compact strided dense views. They do not imply ownership or layout of all backend matrices.

```rust
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum StrideKind {
    RowMajor,
    ColumnMajor,
    Custom { row_stride: usize, col_stride: usize },
}

pub struct MatrixView<'a, S: BaseScalar> {
    data: &'a [S],
    rows: usize,
    cols: usize,
    stride: StrideKind,
}

pub struct MatrixViewMut<'a, S: BaseScalar> {
    data: &'a mut [S],
    rows: usize,
    cols: usize,
    stride: StrideKind,
}
```

Constructors must validate that the slice is large enough for the declared dimensions and stride. Invalid construction returns `SolverError`.

```rust
impl<'a, S: BaseScalar> MatrixView<'a, S> {
    #[inline]
    pub fn from_slice(
        data: &'a [S],
        rows: usize,
        cols: usize,
        stride: StrideKind,
    ) -> Result<Self, SolverError>;
}
```

### 3.7 Static dispatch rule

Core access traits are not object-safe targets for device mathematics. The public design explicitly bans:

```rust
&dyn MatrixAccess<Scalar = S>
&dyn VectorAccess<Scalar = S>
```

inside `loeres-core`, `loeres-backend-static`, and `loeres-device` solver kernels.

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

### 3.8 No mandatory operations

The access traits must not require:

* iteration allocation;
* row extraction into temporary vectors;
* matrix multiplication;
* factorization;
* sparse traversal APIs;
* `Iterator` return types that require complex associated lifetimes unless separately accepted by RFC.

Backends may provide extension traits for efficient traversal, but the core baseline remains access-only.

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

## 5. Algorithmic & Numerical Fail-Safe Guardrails

1. Invalid dimensions fail during view construction where possible.
2. Out-of-bounds element access fails at the access boundary.
3. Solver kernels must not assume that `.get()` cannot fail.
4. Sparse backend adapters may return zero for implicit missing entries only if their public storage semantics define that behavior.
5. Core access traits do not perform finite-value validation. Public solve entrypoints own input validation policy.

## 6. Verification, Validation, and CI Gates

### 6.1 Core compile gates

CI must verify that `loeres-core::access` compiles under `no_std` without `alloc`.

### 6.2 View safety tests

Tests must cover:

* valid vector view access;
* invalid vector index error;
* valid matrix view access for row-major and column-major stride;
* invalid matrix dimensions;
* stride overflow paths;
* mutable set followed by get;
* no panic on invalid indices.

### 6.3 Static dispatch audit

`xtask check-rfcs` must reject `dyn MatrixAccess`, `dyn VectorAccess`, `Box<dyn MatrixAccess>`, and equivalent virtual-dispatch forms inside edge-facing crates.

### 6.4 Backend conformance tests

Each backend must provide conformance tests that run the same access corpus against:

* `VectorView` / `MatrixView`;
* fixed static storage from RFC 004;
* dynamic dense storage from RFC 007;
* dynamic sparse storage from RFC 007 where applicable.

### 6.5 Acceptance criteria

RFC 002 may move to `done/` only when:

1. `loeres_core::access` exposes access-only traits and borrowed views;
2. view constructors validate dimensions without panics;
3. no heavy linalg operations exist in core baseline traits;
4. static dispatch is enforced for core/device paths;
5. all view tests pass on a `no_std` target build profile.
