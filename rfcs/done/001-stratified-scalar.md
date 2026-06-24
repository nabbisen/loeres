# RFC 001 — Stratified Scalar Capability Model

**Status.** Implemented (v0.6.0) — note: `loeres` package renamed to `loeres`, directory to `crates/loeres/` (v0.6.3).
**Tracks.** Phase 1 / Milestone 1 — Foundational Core Architecture
**Touches.** `loeres/src/scalar.rs`, `loeres/src/lib.rs`, public scalar trait namespace

---

### Extended Metadata
* **Rust Edition Compliance:** Rust 2024 Baseline
* **Target Environment:** `loeres`; consumed by `loeres-backend-static`, `loeres-backend-std`, `loeres-device`, and `loeres-cluster`

## 1. Executive Summary & Problem Statement

Loeres must support two incompatible numerical deployment worlds: dynamically allocated cluster workloads and allocation-free device workloads. A monolithic `Scalar` trait would over-constrain the device path by forcing every numeric type to expose operations such as ordering, division, square root, logarithm, and exponentiation even when a solver does not need them.

This RFC defines a six-tier scalar capability model for `loeres`:

1. `BaseScalar` — copyable arithmetic and identity values; no ordering.
2. `OrderedScalar` — ordering and Loeres-defined `min` / `max` / `clamp`.
3. `FiniteScalar` — boundary validation (`is_finite` / `is_nan` / `is_infinite`).
4. `DivisibleScalar` — checked division and inversion.
5. `MetricScalar` — magnitude, tolerance, and residual comparison.
6. `AdvancedNumericalScalar` — square root, logarithm, exponential, and similar.

The design goal is to let algorithms state the smallest numerical contract they actually require. This prevents hidden floating-point assumptions from leaking into fixed-point, integer-like, or custom deterministic numeric types. In particular, ordering is split out of the base tier so that a type which has arithmetic but no meaningful total order is not forced to fabricate one, and so that the precise behavior of `min` / `max` on floating-point types is pinned rather than inherited from the language default.

## 2. Architectural Context & Dependency Alignment

This RFC touches only `loeres`. It introduces no dependency on `std`, `alloc`, `num-traits`, `libm`, `micromath`, BLAS, or any backend crate.

Dependency alignment:

| Crate | Relationship to this RFC | Dependency impact |
|---|---|---|
| `loeres` | Owns the scalar traits | Remains `#![no_std]`, no `alloc` |
| `loeres-backend-static` | Implements traits for fixed-size storage scalar choices | Depends on `loeres` only |
| `loeres-backend-std` | Implements traits for primitive floats and dynamic storage scalar choices | May depend on `std`, but not through core |
| `loeres-device` | Requires minimal scalar bounds per deterministic solver | No `std`, no `alloc` |
| `loeres-cluster` | May request richer scalar bounds for high-level algorithms | `std` allowed only outside core |

The zero-pollution rule is mandatory: no scalar trait may mention `f32`, `f64`, heap allocation, formatting allocation, runtime logging, or OS behavior.

## 3. Concrete Technical Specification

### 3.1 Module layout

`loeres` must expose scalar traits from an explicit module:

```rust
pub mod scalar;

pub use scalar::{
    AdvancedNumericalScalar,
    BaseScalar,
    DivisibleScalar,
    FiniteScalar,
    MetricScalar,
    OrderedScalar,
};
```

### 3.2 Tier 1: `BaseScalar`

`BaseScalar` is the minimum algebraic vocabulary required to represent optimization data. It deliberately excludes ordering, division, and transcendental functions. It requires only equality (for zero-testing), not `PartialOrd`, and not `Debug`.

```rust
pub trait BaseScalar: Copy + Clone + PartialEq + Sized {
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
    fn is_zero(self) -> bool {
        self == Self::zero()
    }
}
```

The trait uses method-based arithmetic instead of requiring the full `core::ops` trait family as supertraits. Implementers may delegate to `Add`, `Sub`, `Mul`, and `Neg`, but the public contract remains under Loeres control. `is_zero` relies only on `PartialEq`; it does not require ordering.

`BaseScalar` intentionally does not require `core::fmt::Debug`. `Debug` is `core` (so it would not break `no_std`), but it is not part of the baseline mathematical scalar contract, and requiring it would force every custom fixed-point or deterministic embedded scalar to participate in formatting machinery. Diagnostic, test, or reporting paths may add `S: core::fmt::Debug` as a local bound where they actually format scalar values.

`BaseScalar` represents signed or sign-capable optimization scalars, since it includes `neg`. Pure unsigned scalar domains are outside the baseline unless represented by a wrapper with defined negation semantics.

**Base arithmetic totality and overflow policy.** `BaseScalar` arithmetic is assumed to be panic-free and total over the scalar implementation's documented operating range. Implementations for bounded or fixed-point scalar families must document whether arithmetic is wrapping, saturating, range-proven by construction, or unavailable for Loeres baseline solvers. A scalar type whose ordinary addition, subtraction, multiplication, or negation may panic under valid solver input must not implement `BaseScalar` for device-facing use. Algorithms that require checked arithmetic beyond this contract must use a future specialized checked-arithmetic scalar tier rather than overloading `BaseScalar`.

### 3.3 Tier 2: `OrderedScalar`

`OrderedScalar` adds ordering and the Loeres-defined extrema and clamp operations. It is the bound a solver requires for projection, comparison, and box-constraint handling. Separating it from `BaseScalar` keeps order-free numeric types valid at the base tier and gives Loeres explicit control over floating-point `min` / `max` behavior.

```rust
pub trait OrderedScalar: BaseScalar + PartialOrd {
    /// The lesser of `self` and `rhs`. Required (no default body) so each
    /// implementation pins its NaN behavior; see the NaN contract below.
    fn min(self, rhs: Self) -> Self;

    /// The greater of `self` and `rhs`, under the same NaN contract.
    fn max(self, rhs: Self) -> Self;

    /// Clamp `self` into the interval `[lo, hi]`. The caller must guarantee
    /// `lo <= hi` (validated at the solve boundary). This method never panics.
    #[inline]
    fn clamp(self, lo: Self, hi: Self) -> Self {
        self.max(lo).min(hi)
    }
}
```

**NaN contract.** For floating-point scalars, `min` and `max` are **NaN-propagating**: if either operand is NaN, the result is NaN. This deliberately differs from the language default (`f64::min` / `f64::max`, which ignore NaN). Loeres solve entrypoints validate inputs with `FiniteScalar` before entering calculation loops, so a NaN reaching `min` / `max` is a contract violation; propagating it surfaces that violation to the next finite check or convergence test rather than silently masking it. For scalar families that have a total order and no NaN (fixed-point, bounded integer-like), `min` / `max` are the ordinary total-order extrema.

**Clamp precondition.** `clamp` assumes `lo <= hi`, which solve entrypoints validate at the boundary (returning a structured error otherwise). `clamp` never panics; if the precondition is violated, the default implementation returns `hi`. This fallback is panic-avoidance behavior, not a valid projection semantics that callers may rely on. With a valid `lo <= hi`, box projection — `clamp(x, lo, hi)` — is a closed-form, allocation-free, non-panicking operation suitable for the device path.

### 3.4 Tier 3: `FiniteScalar`

`FiniteScalar` is a boundary-validation trait. It must be implemented for any scalar type used by public solve entrypoints that need to reject non-finite inputs.

```rust
pub trait FiniteScalar: BaseScalar {
    fn is_finite(self) -> bool;

    fn is_nan(self) -> bool;

    fn is_infinite(self) -> bool;
}
```

For fixed-point or bounded integer-like scalars, these methods may be constant-time trivial implementations such as `true`, `false`, and `false`. This is allowed and expected.

### 3.5 Tier 4: `DivisibleScalar`

Division must never be represented as an unchecked baseline operation. Every public division path must return a structured error rather than panic or produce a silent undefined numerical state. `DivisibleScalar` guards only the exact-zero denominator and non-finite results; near-zero conditioning is a solver-level concern handled with `MetricScalar`, so `DivisibleScalar` depends on `BaseScalar` rather than `MetricScalar`.

```rust
use crate::error::SolverError;

pub trait DivisibleScalar: BaseScalar {
    fn checked_div(self, rhs: Self) -> Result<Self, SolverError>;

    #[inline]
    fn checked_recip(self) -> Result<Self, SolverError> {
        Self::one().checked_div(self)
    }
}
```

The mandatory division rules are:

* `rhs.is_zero()` must return `Err(SolverError::NumericalDomain)` or a more specific non-exhaustive error category.
* Primitive implementations must not use direct `/` before checking the denominator.
* For primitive floating-point implementations, `checked_div` must not return `Ok` containing NaN or infinity: if finite operands produce a non-finite result (for example, finite division overflowing to infinity), return `Err(SolverError::Overflow)` or another RFC-approved numerical-domain category, by documented policy.
* If either operand is already non-finite, public solve-boundary validation should normally reject it earlier; primitive `checked_div` must still be panic-free and must not mask the condition.
* Custom fixed-point implementations must also check representable range and return `Err(SolverError::Overflow)` or `Err(SolverError::NumericalDomain)` as appropriate.

### 3.6 Tier 5: `MetricScalar`

`MetricScalar` supports convergence, tolerance, and residual comparisons. It is not required for all problem descriptions. Because tolerance comparison is inherently ordered (`self.abs() <= tolerance`), `MetricScalar` extends `OrderedScalar`; a `MetricScalar` bound therefore implies `OrderedScalar` (and `BaseScalar`), which is why a solver that already requires `MetricScalar` need not also name `OrderedScalar`.

```rust
pub trait MetricScalar: OrderedScalar {
    fn abs(self) -> Self;

    fn epsilon() -> Self;

    #[inline]
    fn lte_tolerance(self, tolerance: Self) -> bool {
        self.abs() <= tolerance
    }
}
```

`abs` must be panic-free. For bounded scalar families where the absolute value of the minimum negative value is not representable, the implementation must either define a documented saturating/domain-specific behavior or the scalar type must not implement `MetricScalar` for device-facing solvers until a checked-magnitude tier is introduced.

`epsilon()` is not necessarily the primitive machine epsilon. It is the scalar type's default numerical tolerance unit for Loeres algorithms. Individual solvers may require explicit tolerance configuration instead of using this default. RFC 001 accepts `epsilon()` only as a provisional name (a candidate replacement is `algorithmic_epsilon()`); if RFC 006 or RFC 013 finds that users confuse it with primitive machine epsilon, the name must be changed before the first public release.

`lte_tolerance` assumes the supplied `tolerance` has already been validated as finite and nonnegative, or as valid under the scalar family in use. Solver configuration validation, not `MetricScalar`, owns that check; `MetricScalar` remains a minimal comparison capability and does not return `Result`. Public solver entrypoints must reject negative or non-finite tolerance values before calling `lte_tolerance`.

### 3.7 Tier 6: `AdvancedNumericalScalar`

`AdvancedNumericalScalar` is optional and solver-specific. It is forbidden as a baseline bound for `loeres` access traits, problem representations, or device entrypoints unless a concrete algorithm explicitly requires it.

```rust
pub trait AdvancedNumericalScalar: DivisibleScalar + MetricScalar {
    fn checked_sqrt(self) -> Result<Self, SolverError>;

    fn checked_ln(self) -> Result<Self, SolverError>;

    fn checked_exp(self) -> Result<Self, SolverError>;
}
```

This trait exists for algorithms such as barrier methods or algorithms that need square roots for norm evaluation. It must not become an implicit requirement of the whole ecosystem.

Domain and overflow failures for the primitive case:

| Method | Domain failure | Overflow failure |
|---|---|---|
| `checked_sqrt(x)` | `x < 0` → `NumericalDomain` | implementation-specific |
| `checked_ln(x)` | `x <= 0` → `NumericalDomain` | implementation-specific |
| `checked_exp(x)` | none for finite real input | overflow → `Overflow` |

Baseline primitive implementations in `loeres` do **not** imply baseline `AdvancedNumericalScalar` implementations. Advanced functions for primitive floats may require the `libm` feature or a later backend/adapter RFC, because `no_std` targets have no built-in transcendental functions. This keeps transcendental math out of the baseline core.

### 3.8 Supertrait summary

The capability graph is acyclic and minimal:

```text
BaseScalar : Copy + Clone + PartialEq + Sized
OrderedScalar : BaseScalar + PartialOrd
FiniteScalar : BaseScalar
DivisibleScalar : BaseScalar
MetricScalar : OrderedScalar
AdvancedNumericalScalar : DivisibleScalar + MetricScalar
```

`OrderedScalar`, `FiniteScalar`, and `DivisibleScalar` are independent extensions of `BaseScalar`. `MetricScalar` is the only tier that pulls in ordering by supertrait.

### 3.9 Inlining policy

`#[inline]` is required on default method bodies in `loeres` and on Loeres-owned impl methods in backend crates. On required trait methods without a body, the annotation is a documentation and lint target only; actual code-generation behavior depends on the concrete impl and the compiler's optimization decisions, so `#[inline]` does not by itself guarantee a zero-cost abstraction.

The default policy is `#[inline]`, not `#[inline(always)]`, because excessive forced inlining can inflate `.text` size on device targets. `#[inline(always)]` is reserved for extremely small accessors validated by size profiling.

### 3.10 Primitive scalar implementations

Primitive scalar implementations are allowed in `loeres` only if they do not introduce additional dependencies or target-specific behavior. Initial support should include:

* `f32`
* `f64`

Floating-point `OrderedScalar` implementations must implement the NaN-propagating contract of §3.3 explicitly (they may use the concrete `is_nan` of the primitive type; they must not delegate to `f32::min` / `f64::min`, which ignore NaN). Because `is_zero` uses `PartialEq`, `-0.0 == 0.0` holds for primitive floats, so `is_zero(-0.0)` is `true`; this is the intended behavior, and implementations must not invent bitwise-zero semantics. Fixed-point implementations may live in backend crates or optional adapter crates. The core traits must not assume that primitive floats are the only scalar family.

Which primitive trait implementations are baseline core work:

| Primitive trait impl | Baseline? | Notes |
|---|---:|---|
| `BaseScalar` for `f32`/`f64` | yes | no extra dependency |
| `OrderedScalar` for `f32`/`f64` | yes | NaN-propagating extrema |
| `FiniteScalar` for `f32`/`f64` | yes | inherent float methods |
| `DivisibleScalar` for `f32`/`f64` | yes | checked denominator and non-finite result policy |
| `MetricScalar` for `f32`/`f64` | yes | `abs`, default algorithmic epsilon |
| `AdvancedNumericalScalar` for `f32`/`f64` | no baseline | requires `libm` or a later adapter decision |

Implementers must not treat advanced float operations as baseline core work.

## 4. Rust Systems-Level Nuances & Memory Safety

### 4.1 Static dispatch

Scalar traits are intended for monomorphized generic use. They must not be used behind `dyn` in `loeres-device` or `loeres` mathematical kernels. Cluster orchestration may use dynamic dispatch at a higher boundary, but not to define the scalar semantics in core.

### 4.2 No hidden panics

Trait methods must not require implementers to panic for invalid numerical domains. Domain failure is represented as `Result<_, SolverError>` on operations where invalid domains are possible. `min`, `max`, and `clamp` never panic; they follow the §3.3 NaN and precondition contracts instead.

### 4.3 Binary size

The tiered design prevents algorithms from pulling unnecessary method bodies into a device image. RFC 008 will define the cluster monomorphization budget, but this RFC already requires that device solvers select the narrowest scalar bounds they need.

### 4.4 No `unsafe`

This RFC introduces no `unsafe` API and does not permit `unsafe` in scalar trait implementations unless a later backend-specific RFC justifies it. Primitive implementations should use ordinary safe arithmetic and checked guards.

## 5. Algorithmic & Numerical Fail-Safe Guardrails

1. A solver requiring only addition and multiplication must bound itself by `BaseScalar`, not `OrderedScalar` or `DivisibleScalar`.
2. A solver performing projection, clamping, or order comparison must require `OrderedScalar` (or `MetricScalar`, which implies it).
3. A solver reading public floating-point input must require `FiniteScalar` at its public validation boundary.
4. A solver performing division must call `checked_div` or `checked_recip` (and require `DivisibleScalar`).
5. A solver computing residual magnitudes or tolerance comparisons must require `MetricScalar`.
6. A solver using square roots, logarithms, exponentials, or other domain-sensitive advanced operations must require `AdvancedNumericalScalar` only on that solver path.
7. A scalar operation that cannot produce a meaningful value must return a structured error, not a sentinel scalar value.

For reference, the first deterministic device solver (a box/bound-constrained projected first-order kernel) most likely requires `FiniteScalar + MetricScalar` — `OrderedScalar` is implied by `MetricScalar`, which supplies the `clamp` used for box projection — plus `DivisibleScalar` only if the solver estimates its own step size or uses a relative tolerance internally. The exact bound is finalized by the RFC 006 kernel-scope decision.

## 6. Verification, Validation, and CI Gates

### 6.1 Compile gates

CI must verify that `loeres` compiles as:

```text
#![no_std]
```

with no `std` or `alloc` dependency.

### 6.2 Trait dependency gates

A static check must reject:

* `std::` imports inside `loeres/src/scalar.rs`;
* `alloc::` imports inside `loeres/src/scalar.rs`;
* dependencies such as `num-traits` from `loeres` baseline;
* direct primitive-float assumptions in trait definitions beyond optional primitive impl blocks.

### 6.3 Division guard tests

Primitive `DivisibleScalar` implementations must include tests that validate:

* division by zero returns an error;
* reciprocal of zero returns an error;
* finite non-zero division succeeds;
* finite operands whose quotient is non-finite (overflow to infinity) return an error rather than `Ok(inf)`;
* no `unwrap`, `expect`, or indexing panic is used in implementation bodies.

### 6.4 Ordering and NaN tests

Primitive `OrderedScalar` implementations must include tests that validate:

* `min` / `max` on ordinary finite operands match the total order;
* `min` / `max` propagate NaN for floating-point scalars (NaN with any operand yields NaN);
* `clamp` returns a value within `[lo, hi]` for `lo <= hi` and never panics;
* `clamp` with `lo > hi` returns `hi` without panicking;
* `clamp` propagates NaN when `self`, `lo`, or `hi` is NaN for primitive floating scalars (tested explicitly, since a backend may override `clamp`).

### 6.5 Scalar-law tests

Loeres-owned scalar implementations must include tests validating at least the following laws (these catch implementation drift, not formal algebraic proofs):

* `is_zero(zero())` is `true`; `one()` is not zero for primitive floats and for fixed-point types where applicable;
* `FiniteScalar`: for any value, `is_finite` is mutually exclusive with `is_nan` and `is_infinite`;
* `OrderedScalar`: for finite non-NaN operands, `min` and `max` are commutative and idempotent;
* floating `OrderedScalar`: `min` and `max` propagate NaN;
* `MetricScalar::abs(x)` is nonnegative under `OrderedScalar` for finite values;
* `MetricScalar::lte_tolerance(x, tolerance)` is exercised only with finite, nonnegative `tolerance`;
* `DivisibleScalar::checked_recip(x)` and `checked_div(one(), x)` agree for nonzero finite values.

### 6.6 Inlining audit

The inlining annotation check is a source-level hygiene gate ensuring Loeres-owned scalar methods that have bodies are annotated with `#[inline]` or `#[inline(always)]`. It may be implemented as a subcheck of `cargo xtask panic-audit` or as a separate source-lint command; RFC 010 owns the exact command placement. This is a project hygiene gate, not a formal proof of LLVM behavior.

### 6.7 Acceptance criteria

RFC 001 may move to `done/` only when:

1. all six scalar trait tiers are defined in `loeres`, with the §3.8 supertrait graph;
2. `BaseScalar` requires neither `PartialOrd` nor `core::fmt::Debug`, and ordering / `min` / `max` / `clamp` live on `OrderedScalar`;
3. no public scalar trait imports `std`, `alloc`, or backend types;
4. primitive implementations pass the division-domain, ordering/NaN, and scalar-law tests;
5. device-target compilation succeeds with `loeres` only;
6. downstream RFCs 002 and 003 can refer to these traits without circular dependency.
