# RFC 001 — Stratified Scalar Capability Model

**Status.** Proposed
**Tracks.** Phase 1 / Milestone 1 — Foundational Core Architecture
**Touches.** `loeres-core/src/scalar.rs`, `loeres-core/src/lib.rs`, public scalar trait namespace

---

### Extended Metadata
* **Rust Edition Compliance:** Rust 2024 Baseline
* **Target Environment:** `loeres-core`; consumed by `loeres-backend-static`, `loeres-backend-std`, `loeres-device`, and `loeres-cluster`

## 1. Executive Summary & Problem Statement

Loeres must support two incompatible numerical deployment worlds: dynamically allocated cluster workloads and allocation-free device workloads. A monolithic `Scalar` trait would over-constrain the device path by forcing every numeric type to expose operations such as division, square root, logarithm, and exponentiation even when a solver does not need them.

This RFC defines a five-tier scalar capability model for `loeres-core`:

1. `BaseScalar`
2. `FiniteScalar`
3. `DivisibleScalar`
4. `MetricScalar`
5. `AdvancedNumericalScalar`

The design goal is to let algorithms state the smallest numerical contract they actually require. This prevents hidden floating-point assumptions from leaking into fixed-point, integer-like, or custom deterministic numeric types.

## 2. Architectural Context & Dependency Alignment

This RFC touches only `loeres-core`. It introduces no dependency on `std`, `alloc`, `num-traits`, `libm`, `micromath`, BLAS, or any backend crate.

Dependency alignment:

| Crate | Relationship to this RFC | Dependency impact |
|---|---|---|
| `loeres-core` | Owns the scalar traits | Remains `#![no_std]`, no `alloc` |
| `loeres-backend-static` | Implements traits for fixed-size storage scalar choices | Depends on `loeres-core` only |
| `loeres-backend-std` | Implements traits for primitive floats and dynamic storage scalar choices | May depend on `std`, but not through core |
| `loeres-device` | Requires minimal scalar bounds per deterministic solver | No `std`, no `alloc` |
| `loeres-cluster` | May request richer scalar bounds for high-level algorithms | `std` allowed only outside core |

The zero-pollution rule is mandatory: no scalar trait may mention `f32`, `f64`, heap allocation, formatting allocation, runtime logging, or OS behavior.

## 3. Concrete Technical Specification

### 3.1 Module layout

`loeres-core` must expose scalar traits from an explicit module:

```rust
pub mod scalar;

pub use scalar::{
    AdvancedNumericalScalar,
    BaseScalar,
    DivisibleScalar,
    FiniteScalar,
    MetricScalar,
};
```

### 3.2 Tier 1: `BaseScalar`

`BaseScalar` is the minimum algebraic vocabulary required to represent optimization data. It deliberately excludes division and transcendental functions.

```rust
pub trait BaseScalar:
    Copy + Clone + PartialEq + PartialOrd + core::fmt::Debug + Sized
{
    #[inline]
    fn zero() -> Self;

    #[inline]
    fn one() -> Self;

    #[inline]
    fn add(self, rhs: Self) -> Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self;

    #[inline]
    fn mul(self, rhs: Self) -> Self;

    #[inline]
    fn neg(self) -> Self;

    #[inline]
    fn min(self, rhs: Self) -> Self;

    #[inline]
    fn max(self, rhs: Self) -> Self;

    #[inline]
    fn is_zero(self) -> bool {
        self == Self::zero()
    }
}
```

The trait uses method-based arithmetic instead of requiring the full `core::ops` trait family as supertraits. Implementers may delegate to `Add`, `Sub`, `Mul`, and `Neg`, but the public contract remains under Loeres control.

### 3.3 Tier 2: `FiniteScalar`

`FiniteScalar` is a boundary-validation trait. It must be implemented for any scalar type used by public solve entrypoints that need to reject non-finite inputs.

```rust
pub trait FiniteScalar: BaseScalar {
    #[inline]
    fn is_finite(self) -> bool;

    #[inline]
    fn is_nan(self) -> bool;

    #[inline]
    fn is_infinite(self) -> bool;
}
```

For fixed-point or bounded integer-like scalars, these methods may be constant-time trivial implementations such as `true`, `false`, and `false`. This is allowed and expected.

### 3.4 Tier 3: `DivisibleScalar`

Division must never be represented as an unchecked baseline operation. Every public division path must return a structured error rather than panic or produce a silent undefined numerical state.

```rust
use crate::error::SolverError;

pub trait DivisibleScalar: BaseScalar {
    #[inline]
    fn checked_div(self, rhs: Self) -> Result<Self, SolverError>;

    #[inline]
    fn checked_recip(self) -> Result<Self, SolverError> {
        Self::one().checked_div(self)
    }
}
```

The mandatory division-by-zero rule is:

* `rhs.is_zero()` must return `Err(SolverError::NumericalDomain)` or a more specific non-exhaustive error category.
* Primitive implementations must not use direct `/` before checking the denominator.
* Custom fixed-point implementations must also check representable range and return `Err(SolverError::Overflow)` or `Err(SolverError::NumericalDomain)` as appropriate.

### 3.5 Tier 4: `MetricScalar`

`MetricScalar` supports convergence, tolerance, and residual comparisons. It is not required for all problem descriptions.

```rust
pub trait MetricScalar: BaseScalar {
    #[inline]
    fn abs(self) -> Self;

    #[inline]
    fn epsilon() -> Self;

    #[inline]
    fn lte_tolerance(self, tolerance: Self) -> bool {
        self.abs() <= tolerance
    }
}
```

`epsilon()` is not necessarily the primitive machine epsilon. It is the scalar type's default numerical tolerance unit for Loeres algorithms. Individual solvers may require explicit tolerance configuration instead of using this default.

### 3.6 Tier 5: `AdvancedNumericalScalar`

`AdvancedNumericalScalar` is optional and solver-specific. It is forbidden as a baseline bound for `loeres-core` access traits, problem representations, or device entrypoints unless a concrete algorithm explicitly requires it.

```rust
pub trait AdvancedNumericalScalar: DivisibleScalar + MetricScalar {
    #[inline]
    fn checked_sqrt(self) -> Result<Self, SolverError>;

    #[inline]
    fn checked_ln(self) -> Result<Self, SolverError>;

    #[inline]
    fn checked_exp(self) -> Result<Self, SolverError>;
}
```

This trait exists for algorithms such as barrier methods or algorithms that need square roots for norm evaluation. It must not become an implicit requirement of the whole ecosystem.

### 3.7 Inlining mandate

Every scalar method with a body in `loeres-core` must use `#[inline]` unless a later RFC justifies `#[inline(always)]` for a specific low-level function. Every scalar implementation in Loeres-owned backend crates must also mark method implementations with `#[inline]` or `#[inline(always)]`.

The default policy is `#[inline]`, not `#[inline(always)]`, because excessive forced inlining can inflate `.text` size on device targets. `#[inline(always)]` is reserved for extremely small accessors validated by size profiling.

### 3.8 Primitive scalar implementations

Primitive scalar implementations are allowed in `loeres-core` only if they do not introduce additional dependencies or target-specific behavior. Initial support should include:

* `f32`
* `f64`

Fixed-point implementations may live in backend crates or optional adapter crates. The core traits must not assume that primitive floats are the only scalar family.

## 4. Rust Systems-Level Nuances & Memory Safety

### 4.1 Static dispatch

Scalar traits are intended for monomorphized generic use. They must not be used behind `dyn` in `loeres-device` or `loeres-core` mathematical kernels. Cluster orchestration may use dynamic dispatch at a higher boundary, but not to define the scalar semantics in core.

### 4.2 No hidden panics

Trait methods must not require implementers to panic for invalid numerical domains. Domain failure is represented as `Result<_, SolverError>` on operations where invalid domains are possible.

### 4.3 Binary size

The tiered design prevents algorithms from pulling unnecessary method bodies into a device image. RFC 008 will define the cluster monomorphization budget, but this RFC already requires that device solvers select the narrowest scalar bounds they need.

### 4.4 No `unsafe`

This RFC introduces no `unsafe` API and does not permit `unsafe` in scalar trait implementations unless a later backend-specific RFC justifies it. Primitive implementations should use ordinary safe arithmetic and checked guards.

## 5. Algorithmic & Numerical Fail-Safe Guardrails

1. A solver requiring only addition and multiplication must bound itself by `BaseScalar`, not `DivisibleScalar`.
2. A solver reading public floating-point input must require `FiniteScalar` at its public validation boundary.
3. A solver performing division must call `checked_div` or `checked_recip`.
4. A solver computing residual magnitudes must require `MetricScalar`.
5. A solver using square roots, logarithms, exponentials, or other domain-sensitive advanced operations must require `AdvancedNumericalScalar` only on that solver path.
6. A scalar operation that cannot produce a meaningful value must return a structured error, not a sentinel scalar value.

## 6. Verification, Validation, and CI Gates

### 6.1 Compile gates

CI must verify that `loeres-core` compiles as:

```text
#![no_std]
```

with no `std` or `alloc` dependency.

### 6.2 Trait dependency gates

A static check must reject:

* `std::` imports inside `loeres-core/src/scalar.rs`;
* `alloc::` imports inside `loeres-core/src/scalar.rs`;
* dependencies such as `num-traits` from `loeres-core` baseline;
* direct primitive-float assumptions in trait definitions beyond optional primitive impl blocks.

### 6.3 Division guard tests

Primitive `DivisibleScalar` implementations must include tests that validate:

* division by zero returns an error;
* reciprocal of zero returns an error;
* finite non-zero division succeeds;
* no `unwrap`, `expect`, or indexing panic is used in implementation bodies.

### 6.4 Inlining audit

`xtask check-rfcs` must include a source-level audit ensuring Loeres-owned scalar methods are annotated with `#[inline]` or `#[inline(always)]`. This is a project hygiene gate, not a formal proof of LLVM behavior.

### 6.5 Acceptance criteria

RFC 001 may move to `done/` only when:

1. all five scalar trait tiers are defined in `loeres-core`;
2. no public scalar trait imports `std`, `alloc`, or backend types;
3. primitive implementations pass division-domain tests;
4. device-target compilation succeeds with `loeres-core` only;
5. downstream RFCs 002 and 003 can refer to these traits without circular dependency.
