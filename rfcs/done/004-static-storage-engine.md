# RFC 004 — Const-Generic and Fixed-Size Static Storage Engine

**Status.** Implemented (v0.8.0) — `loeres-backend-static` `dimension` / `array` / `view` modules; owned `FixedVector` / `FixedMatrix` (feature `owned-arrays`) and the baseline contiguous static views, with const-assert dimension invariants (MSRV-validated, 1.85.0) and the RFC 002 access + contiguous fast-path traits reporting `DimensionKind::Static`. Implementation-decision pass (D1–D6) accepted. Advanced `static-views` deferred (§7.2).
**Tracks.** Phase 2 / Milestone 2 — Static Backend and Real-Time Kernel
**Touches.** `loeres-backend-static/src/array.rs`, `loeres-backend-static/src/dimension.rs`, `loeres-backend-static/src/view.rs`, `loeres-backend-static/src/lib.rs`

---

### Extended Metadata
* **Rust Edition Compliance:** Rust 2024 Baseline
* **Target Environment:** `loeres-backend-static`; consumed by `loeres-device`

## 1. Executive Summary & Problem Statement

`loeres-backend-static` provides the concrete allocation-free storage foundation for device-side optimization. It must represent fixed-size vectors and matrices, and borrowed views over caller memory, without `std`, without `alloc`, and without relying on unstable const-evaluation behaviour.

This RFC defines that storage engine: owned fixed-size arrays, borrowed static views, static dimension descriptors, and the trait wiring that makes them satisfy the implemented RFC 002 access contracts. It uses a fallback-first memory-flattening strategy that avoids complex generic const expressions (such as `R * C` in type-level array lengths) unless a target profile explicitly supports them, and it enforces type-level dimension invariants with compile-time assertions rather than runtime results.

This revision reconciles the original draft to decisions settled after it was written: the accepted external-design module layout and feature posture (ED §1.5/§1.6.2), and the now-implemented RFC 002 access surface (including the contiguous fast path and the `DimensionKind::Static` variant).

## 2. Architectural Context & Dependency Alignment

This RFC depends on:

* [RFC 001](001-stratified-scalar.md) for scalar bounds;
* [RFC 002](002-storage-agnostic-contracts.md) for access traits, the contiguous fast path, `Dim2`, and `DimensionKind`;
* [RFC 003](003-allocation-free-errors.md) for error categories.

Dependency rules:

| Crate | Rule |
|---|---|
| `loeres-backend-static` | `#![no_std]`, no `alloc`, no `std` |
| `loeres` | Only internal Loeres dependency |
| `loeres-device` | Consumes static backend but does not own storage primitives |
| `loeres-backend-std` | No dependency from static backend |
| `loeres-cluster` | No dependency from static backend baseline |

Optional dependencies such as `heapless` are not part of the baseline. They may be evaluated only by a later RFC if they do not weaken zero-bleed guarantees.

## 3. Feature Posture

The crate's public surface is feature-gated per external design §1.6.2. This RFC owns the static-backend feature semantics:

| Feature | Default | RFC 004 responsibility |
|---|---:|---|
| _none_ (baseline) | yes | Minimal **contiguous** borrowed static adapters and their trait implementations, plus the static dimension descriptors — enough to satisfy the core access contracts. No owned fixed-array wrappers. |
| `owned-arrays` | no | The owned `FixedVector` / `FixedMatrix` wrappers and their trait implementations. |
| `static-views` | no | Advanced borrowed views: row, column, sub-matrix, and strided. Design deferred (§7.2). |
| `diagnostic-snapshot` | no | Out of scope for RFC 004 unless a later revision needs compact static-backend diagnostic metadata. |

Policy:

* Owned fixed arrays are **not** unconditional baseline in v0.x; they require `owned-arrays`.
* Advanced views are **not** promoted to default in RFC 004. Whether any advanced view becomes default is **deferred** until there is evidence from device-solver implementation (RFC 006).
* `owned-arrays` and `static-views` may be enabled together.
* No feature of this crate may enable `std`, `alloc`, async, logging, or server backends (external design §1.6.2; requirements BSTATIC-001/002).

## 4. Module Layout

The crate uses the established external-design §1.5 layout. The owned/advanced surfaces are feature-gated, not split into separate baseline modules.

```rust
// lib.rs
pub mod array;      // owned fixed-size vector/matrix wrappers (feature = "owned-arrays")
pub mod dimension;  // static dimension markers / descriptors (baseline)
pub mod view;       // borrowed static views: baseline contiguous; advanced behind "static-views"
pub mod workspace;  // RFC 005-owned; left as a placeholder by this RFC
```

`array` holds both `FixedVector` and `FixedMatrix` (an internal `array/vector.rs` + `array/matrix.rs` split is permitted later if the file grows, but the RFC-level public module shape stays `array` / `dimension` / `view`). `workspace` is owned by RFC 005 and is not specified here.

## 5. Static Dimension Reporting

Every owned fixed-storage type and every const-sized static view in this RFC reports `DimensionKind::Static` from its access-trait `dimension_kind()`. The core borrowed views in `loeres` report `Dynamic`; `loeres-backend-static` is the source of `Static`. A static type that reported `Dynamic` would defeat the purpose of the variant, so `Static` reporting is mandatory for all types defined here.

## 6. Owned Fixed Storage (`owned-arrays`)

### 6.1 Fixed vector

```rust
#[repr(transparent)]
pub struct FixedVector<S, const N: usize> {
    data: [S; N],
}
```

Inherent API (no `S: BaseScalar` bound — these methods perform no scalar operations):

```rust
impl<S, const N: usize> FixedVector<S, N> {
    pub const fn from_array(data: [S; N]) -> Self;   // compile-time asserts N > 0
    pub const fn len(&self) -> usize;
    pub fn as_slice(&self) -> &[S];
    pub fn as_mut_slice(&mut self) -> &mut [S];
}
```

The `N > 0` invariant is enforced by a compile-time assertion (§8), so `FixedVector<S, 0>` cannot be constructed through `from_array`. Memory footprint is exposed for review (§12).

Trait implementations (require `S: BaseScalar`):

* `VectorAccess`
* `VectorAccessMut`
* `ContiguousVectorAccess` — `as_contiguous` returns `Some(&self.data)`
* `ContiguousVectorAccessMut` — `as_contiguous_mut` returns `Some(&mut self.data)`

`dimension_kind()` returns `DimensionKind::Static`.

### 6.2 Fixed matrix (fallback-first design)

The baseline matrix shape avoids `where [(); R * C]:` and similar generic const-expression requirements by carrying the flattened length `N` as an explicit parameter.

```rust
#[repr(transparent)]
pub struct FixedMatrix<S, const R: usize, const C: usize, const N: usize> {
    data: [S; N],
}
```

Type-level invariant (compile-time asserted, §8):

```text
N == R * C
R > 0
C > 0
```

Inherent API (no `S: BaseScalar` bound):

```rust
impl<S, const R: usize, const C: usize, const N: usize> FixedMatrix<S, R, C, N> {
    pub const fn from_row_major_array(data: [S; N]) -> Self;  // compile-time asserts the invariant
    pub const fn rows(&self) -> usize;
    pub const fn cols(&self) -> usize;
    pub fn as_flat_slice(&self) -> &[S];
    pub fn as_flat_slice_mut(&mut self) -> &mut [S];
}
```

`from_row_major_array` returns `Self`, not `Result`: the `N == R * C` / `R,C > 0` relationships are type-level facts, so a mismatched **public construction** is a **compile error**, not a runtime outcome (§8). Element access computes `offset = row * C + col` with checked arithmetic and returns `SolverError` on an invalid index, never a panic (§9, §10).

Trait implementations (require `S: BaseScalar`):

* `MatrixAccess`
* `MatrixAccessMut`
* `ContiguousMatrixAccess` — `as_row_major` returns `Some(&self.data)` (length `N == R * C`)

RFC 002 ships no `ContiguousMatrixAccessMut`; this RFC does not invent one. `dimension_kind()` returns `DimensionKind::Static`.

Because `as_flat_slice()` has length exactly `N == R * C`, it composes cleanly with the core `MatrixView::from_row_major(slice, R, C)`, satisfying the ADR-020 exact-size precondition without copying.

### 6.3 Ergonomic alias (deferred)

If a future compiler profile reliably supports `{ R * C }` in type position, an ergonomic alias may be introduced:

```rust
pub type FixedMatrixRc<S, const R: usize, const C: usize> = FixedMatrix<S, R, C, { R * C }>;
```

This alias is **not** part of the baseline and must not be introduced until validated by CI for all supported targets.

## 7. Borrowed Static Views

### 7.1 Baseline contiguous views (default)

The featureless baseline provides const-sized contiguous borrowed views over caller-owned memory (peripheral buffers, DMA regions, RTOS-owned state), so embedded users need not copy into a Loeres-owned container.

```rust
pub struct StaticVectorView<'a, S, const N: usize> { data: &'a [S; N] }
pub struct StaticVectorViewMut<'a, S, const N: usize> { data: &'a mut [S; N] }
pub struct StaticMatrixView<'a, S, const R: usize, const C: usize, const N: usize> { data: &'a [S; N] }
pub struct StaticMatrixViewMut<'a, S, const R: usize, const C: usize, const N: usize> { data: &'a mut [S; N] }
```

These views **implement the access traits directly** and report `DimensionKind::Static` — they are not thin wrappers around the core `Dynamic`-reporting views. Specifically:

* `StaticVectorView`: `VectorAccess` + `ContiguousVectorAccess`;
* `StaticVectorViewMut`: `VectorAccess` + `VectorAccessMut` + `ContiguousVectorAccess` + `ContiguousVectorAccessMut`;
* `StaticMatrixView`: `MatrixAccess` + `ContiguousMatrixAccess`;
* `StaticMatrixViewMut`: `MatrixAccess` + `MatrixAccessMut` + `ContiguousMatrixAccess`.

The matrix views carry the same `N == R * C` (and `R,C > 0`) **public-construction** invariant as `FixedMatrix`, enforced by a compile-time assertion in the public constructors (§8). A mismatched instantiation such as `StaticMatrixView<_, 2, 3, 5>` may be nameable as a type, but it cannot be constructed through the public API: any public constructor for that instantiation fails at compile time where the assertion is supported by the validated toolchain profile (§8).

### 7.2 Advanced views (`static-views`)

Row, column, sub-matrix, and strided views are gated behind `static-views`. Their detailed design (stride descriptors, non-contiguous access, which — if any — implement the contiguous fast path) is deferred to a dedicated section or follow-up RFC, and they are **not** default in v0.x. A non-contiguous view returns `None` from the relevant `Contiguous*Access` method so kernels fall back to per-element access (RFC 002 contract).

## 8. Const Policy and Invariant Enforcement

Type-level dimension invariants are enforced at **compile time**; only genuinely runtime lengths use a runtime `Result`.

1. **Type-level shape errors are compile-time errors.** `N > 0` (vectors), `R,C > 0` and `N == R * C` (matrices and const-sized matrix views) are checked with an inline `const { assert!(...) }` in the public constructor, evaluated at monomorphization. A bad public construction fails to compile. This is value-level const evaluation, not the unstable type-level `where [(); R * C]:` constraint, and is permitted by BSTATIC-012 ("static assertions").
2. **No truncating conversions in `const` constructors.** A `const fn` must not build a `SolverError` payload by casting `usize` to `u32`, which would truncate (BSTATIC-006 / ADR-020). Because type-level invariants are compile-time, no error payload is constructed for them at all.
3. **Runtime `Result` only for runtime lengths.** Where a length is genuinely runtime (e.g. a future adapter from a runtime slice), use a non-`const` constructor with **checked** `usize → u32` conversion and the RFC 002 / ADR-020 mapping: length mismatch → `DimensionMismatch { lhs: actual, rhs: required }`; overflow / zero where positive is required → `InvalidDimension`. Exact size only.
4. **No unstable baseline.** No `generic_const_exprs`; no `where [(); R * C]:` baseline requirement; the `FixedMatrixRc` alias stays gated.
5. **MSRV validation precedes signature freeze.** Before implementation-decision approval, the selected inline/static `const` assertion pattern must be validated against the pinned MSRV and current toolchain. If it is not accepted, this RFC must be patched **before coding** to choose a runtime `Result` constructor shape for the affected APIs — changing a `const fn -> Self` into a `fn -> Result<Self, SolverError>` is a public-API decision, not an implementation fallback. The implementation must not silently change constructor return types, and must never silently cast `usize` to `u32`.

### 8.1 No separate const-eval probe crate

This RFC does **not** add a standalone `xtask` const-eval probe crate. Instead it documents the validated stable subset above (plain const params; simple `const fn` constructors where no runtime error payload is needed; inline/static const assertions for type invariants subject to MSRV; no `generic_const_exprs`; runtime `Result` only for runtime lengths). CI compiles the static backend under the pinned MSRV and current toolchain (§11). A probe may be introduced later only if RFC 004 or RFC 005 comes to depend on a wider const-eval profile.

**MSRV validation (recorded).** The chosen pattern — `const { assert!(N == R * C && R > 0 && C > 0) }` inside a `const fn … -> Self` constructor over const-generic parameters — was validated on the pinned **MSRV 1.85.0** under **edition 2024**: a valid instantiation compiles and runs, and a mismatched public construction (e.g. `FixedMatrix::<_, 2, 3, 5>`) fails to compile with `error[E0080]` at the assertion. The `const fn -> Self` signatures are therefore confirmed for MSRV; no runtime-`Result` fallback is required under §8.5.

## 9. Rust Systems-Level Nuances & Memory Safety

### 9.1 Stack utilization

Large fixed arrays must not be passed by value in solver APIs. Public APIs accept references or borrowed views:

```rust
&FixedMatrix<S, R, C, N>
&mut FixedMatrix<S, R, C, N>
```

Consuming/by-value APIs may exist only for small helper construction paths, never hot solver entrypoints (requirements BSTATIC-009; §6.5 size test).

### 9.2 Zero-size and overflow handling

Zero dimensions are invalid for solver storage and are rejected at compile time via the §8 assertions. `R * C` is a type-level relationship validated by the same assertions; it is never computed into an unchecked runtime offset.

### 9.3 `unsafe` policy

Baseline storage access uses safe Rust; no unchecked baseline indexing is accepted. Any future unchecked accessor must be explicitly `unsafe`, isolated, and introduced behind a separate RFC.

## 10. Algorithmic & Numerical Fail-Safe Guardrails

1. Constructors validate dimension invariants (compile-time) before any use.
2. Element access returns `SolverError` on invalid indices; it never panics.
3. Matrix access does not assume row-major semantics outside `FixedMatrix` / the contiguous matrix views themselves.
4. Borrowed views do not copy data and therefore cannot hide memory use.
5. No API silently truncates, resizes, reallocates, or drops failed writes (BSTATIC-006).

## 11. Verification, Validation, and CI Gates

### 11.1 Zero-bleed dependency gate

CI must prove `loeres-backend-static` has no transitive `std` or `alloc` dependency in any feature configuration (`xtask zero-bleed`).

### 11.2 Target compile matrix

* host `no_std` check;
* the bare-metal reference target (`thumbv7em-none-eabihf`, via `xtask no-std`);
* feature combinations: baseline, `owned-arrays`, `static-views`, and `owned-arrays + static-views`;
* `--no-default-features` build (requirements CI-004).

### 11.3 Access tests (RFC 004-owned)

RFC 004 does **not** rely on a reusable "RFC 002 conformance suite" — none exists; the cross-backend corpus is RFC 013's, and RFC 002 shipped in-crate unit tests. This RFC defines its own tests for its types, mirroring the RFC 002 §6.2 behaviour:

* valid vector/matrix reads return correct values;
* out-of-bounds access returns the RFC 002 policy error (`DimensionMismatch`, per-axis for matrices);
* mutate-then-read for mutable owned types and mutable static views;
* the contiguous fast path returns `Some` with exact length (`N`, and `N == R * C` for matrices);
* `dimension_kind() == DimensionKind::Static`;
* bad public construction of an invalid type-level shape fails to compile, where a compile-fail harness is practical;
* any runtime-length constructor rejects invalid lengths using checked payload conversion (no truncation);
* zero-bleed and `no_std`/no-`alloc` target builds pass.

RFC 013 remains the owner of the future cross-backend conformance corpus and numerical-parity policy.

### 11.4 Size test

At least one non-trivial matrix size must appear in tests or examples, large enough to catch accidental by-value copies in API review (BSTATIC-009).

### 11.5 Acceptance criteria

RFC 004 may move to `done/` only when:

1. baseline, `owned-arrays`, and `static-views` configurations compile with no `std` and no `alloc`, including the bare-metal target;
2. matrix layout relies on no unvalidated generic const expressions, and type-level invariants are compile-time enforced;
3. owned arrays and const-sized static views report `DimensionKind::Static` and implement the RFC 002 access and contiguous fast-path traits as specified (§6, §7.1);
4. borrowed static views require no copying;
5. element access is fallible and panic-averse;
6. the RFC 004-owned access tests (§11.3) pass.

## 12. Memory-Footprint Exposure

Owned types expose their footprint for review without inspection of internals (requirements BSTATIC-008), via associated constants and/or documented size guidance — for example an element count (`ELEMENTS`), matrix dimensions (`ROWS`, `COLS`), and a byte size (or documented `core::mem::size_of::<Self>()` guidance). Exact constant names and set are an implementation-decision detail (§13).

## 13. Deferred to the Implementation-Decision Review

The following are intentionally left for a narrow implementation-decision pass before coding:

* exact constructor and accessor names;
* exact feature-gate wiring in code (`#[cfg(feature = "...")]` placement);
* the compile-fail test mechanism (e.g. `trybuild`) and which invariants get compile-fail coverage;
* the exact associated-constant set for footprint exposure (§12);
* whether to split `array.rs` internally into `array/vector.rs` + `array/matrix.rs`;
* the detailed `static-views` advanced-view design (§7.2).
