# RFC 004 — Const-Generic and Fixed-Size Static Storage Engine

**Status.** Proposed
**Tracks.** Phase 2 / Milestone 2 — Static Backend and Real-Time Kernel
**Touches.** `loeres-backend-static/src/vector.rs`, `loeres-backend-static/src/matrix.rs`, `loeres-backend-static/src/view.rs`, `loeres-backend-static/src/lib.rs`

---

### Extended Metadata
* **Rust Edition Compliance:** Rust 2024 Baseline
* **Target Environment:** `loeres-backend-static`; consumed by `loeres-device`

## 1. Executive Summary & Problem Statement

`loeres-backend-static` provides the concrete allocation-free storage foundation for device-side optimization. It must represent fixed-size vectors and matrices without `std`, without `alloc`, and without relying on unstable const-evaluation behavior.

This RFC defines a static storage engine based on owned arrays, borrowed views, and a fallback memory-flattening strategy that avoids complex generic const expressions such as `R * C` in type-level array lengths unless the target compiler profile explicitly supports them.

## 2. Architectural Context & Dependency Alignment

This RFC depends on:

* [RFC 001](../done/001-stratified-scalar.md) for scalar bounds;
* [RFC 002](002-storage-agnostic-contracts.md) for access traits;
* [RFC 003](../done/003-allocation-free-errors.md) for error categories.

Dependency rules:

| Crate | Rule |
|---|---|
| `loeres-backend-static` | `#![no_std]`, no `alloc`, no `std` |
| `loeres-core` | Only internal Loeres dependency |
| `loeres-device` | Consumes static backend but does not own storage primitives |
| `loeres-backend-std` | No dependency from static backend |
| `loeres-cluster` | No dependency from static backend baseline |

Optional dependencies such as `heapless` are not part of the baseline. They may be evaluated only by later RFC if they do not weaken zero-bleed guarantees.

## 3. Concrete Technical Specification

### 3.1 Module layout

```rust
pub mod matrix;
pub mod vector;
pub mod view;

pub use matrix::FixedMatrix;
pub use vector::FixedVector;
pub use view::{StaticMatrixView, StaticMatrixViewMut, StaticVectorView, StaticVectorViewMut};
```

### 3.2 Fixed vector

```rust
#[repr(transparent)]
pub struct FixedVector<S, const N: usize> {
    data: [S; N],
}
```

Public API:

```rust
impl<S: BaseScalar, const N: usize> FixedVector<S, N> {
    #[inline]
    pub const fn from_array(data: [S; N]) -> Self;

    #[inline]
    pub const fn len(&self) -> usize;

    #[inline]
    pub fn as_slice(&self) -> &[S];

    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [S];
}
```

`FixedVector<S, 0>` is rejected by constructor-level validation or compile-time probes. If stable Rust cannot enforce `N > 0` in the type system for every target profile, CI must include a targeted compile-fail or runtime constructor validation gate.

### 3.3 Fixed matrix fallback-first design

The baseline matrix shape avoids `where [(); R * C]:` and similar generic const expression requirements.

```rust
pub struct FixedMatrix<S, const R: usize, const C: usize, const N: usize> {
    data: [S; N],
}
```

Invariant:

```text
N == R * C
R > 0
C > 0
```

This design forces users or type aliases to supply `N` explicitly. It avoids relying on type-level multiplication support in stable target compilers.

Public API:

```rust
impl<S: BaseScalar, const R: usize, const C: usize, const N: usize> FixedMatrix<S, R, C, N> {
    #[inline]
    pub const fn from_row_major_array(data: [S; N]) -> Result<Self, SolverError>;

    #[inline]
    pub const fn rows(&self) -> usize;

    #[inline]
    pub const fn cols(&self) -> usize;

    #[inline]
    pub fn as_flat_slice(&self) -> &[S];

    #[inline]
    pub fn as_flat_slice_mut(&mut self) -> &mut [S];
}
```

If a future compiler profile supports reliable generic const expression use for `R * C`, an ergonomic type alias may be introduced:

```rust
pub type FixedMatrixRc<S, const R: usize, const C: usize> = FixedMatrix<S, R, C, { R * C }>;
```

This alias must not be part of the baseline until validated by CI for all supported targets.

### 3.4 Manual stride math

Matrix element access computes:

```text
offset = row * C + col
```

using checked arithmetic. Invalid index or overflow returns `SolverError`, never panic.

### 3.5 Borrowed static views

The static backend may provide dimension-specialized borrowed views on top of core borrowed views.

```rust
pub struct StaticVectorView<'a, S, const N: usize> {
    data: &'a [S; N],
}

pub struct StaticVectorViewMut<'a, S, const N: usize> {
    data: &'a mut [S; N],
}

pub struct StaticMatrixView<'a, S, const R: usize, const C: usize, const N: usize> {
    data: &'a [S; N],
}

pub struct StaticMatrixViewMut<'a, S, const R: usize, const C: usize, const N: usize> {
    data: &'a mut [S; N],
}
```

These types allow users to adapt statically allocated memory without copying it into a Loeres-owned container.

### 3.6 Trait implementations

`FixedVector` implements:

* `VectorAccess`
* `VectorAccessMut`

`FixedMatrix` implements:

* `MatrixAccess`
* `MatrixAccessMut`

No implementation may require allocation or dynamic dispatch.

## 4. Rust Systems-Level Nuances & Memory Safety

### 4.1 Const evaluation check

Before accepting this RFC as implemented, the project must run compiler probes for:

* explicit `FixedMatrix<S, R, C, N>` baseline;
* rejected or gated `FixedMatrixRc<S, R, C>` alias using `{ R * C }`;
* `const fn` constructor feasibility for dimension invariant checks;
* device target compilation.

The baseline must not depend on unstable complex generic const expressions.

### 4.2 Stack utilization

Large fixed arrays must not be passed by value in solver APIs. Public APIs must accept references:

```rust
&FixedMatrix<S, R, C, N>
&mut FixedMatrix<S, R, C, N>
```

or borrowed views. Consuming APIs may exist only for small helper construction paths, not hot solver entrypoints.

### 4.3 Zero-size and overflow handling

Zero dimensions are invalid for solver storage. Multiplication overflow in `R * C` invariant checks returns or triggers a compile-time failure depending on constructor context.

### 4.4 `unsafe` policy

Baseline storage access uses safe Rust. Any future unchecked accessors must be explicitly marked unsafe and isolated behind a separate RFC. No unsafe baseline indexing is accepted.

## 5. Algorithmic & Numerical Fail-Safe Guardrails

1. Storage constructors validate dimensions and flattening invariants before use.
2. Element access methods return `SolverError` on invalid indices.
3. Matrix access does not assume row-major semantics outside `FixedMatrix` itself.
4. Borrowed views do not copy data and therefore cannot hide memory use.
5. No API silently truncates, resizes, or reallocates storage.

## 6. Verification, Validation, and CI Gates

### 6.1 Zero-bleed dependency gate

CI must prove that `loeres-backend-static` has no transitive `std` or `alloc` dependency in baseline feature configuration.

### 6.2 Target compile matrix

At minimum, compile checks must include:

* host `no_std` check;
* a reference embedded target profile selected by the project;
* tests for baseline array and borrowed-view APIs.

### 6.3 Const-evaluation probes

`xtask` must compile a probe crate that verifies which const expression patterns are accepted. The result must be recorded in generated CI output so later RFCs do not assume unsupported compiler behavior.

### 6.4 Access conformance tests

Static backend storage must pass the RFC 002 access conformance suite.

### 6.5 Size tests

Tests or examples must include at least one non-trivial matrix size large enough to catch accidental by-value copies in API review.

### 6.6 Acceptance criteria

RFC 004 may move to `done/` only when:

1. static vectors and matrices compile with no `std` and no `alloc`;
2. matrix layout does not rely on unvalidated generic const expressions;
3. borrowed static views require no copying;
4. element access is fallible and panic-averse;
5. all static backend conformance tests pass.
