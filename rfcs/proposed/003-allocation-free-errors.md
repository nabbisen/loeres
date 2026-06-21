# RFC 003 — Allocation-Free Error Topology and Formatting Restrictions

**Status.** Proposed
**Tracks.** Phase 1 / Milestone 1 — Foundational Core Architecture
**Touches.** `loeres-core/src/error.rs`, `loeres-core/src/diagnostic.rs`, public error namespace

---

### Extended Metadata
* **Rust Edition Compliance:** Rust 2024 Baseline
* **Target Environment:** `loeres-core`; consumed by all Loeres crates

## 1. Executive Summary & Problem Statement

Loeres error values cross every crate boundary. They must be useful enough to express numerical and structural failure, but small enough not to inflate `Result<T, E>` in device hot paths. They must also be semver-extensible because future solvers will need new error categories.

This RFC defines the allocation-free error topology for `loeres-core`. It forbids `Display` in core, requires `Debug`, mandates static string lookup, and establishes compile-time size limits for errors and diagnostics.

## 2. Architectural Context & Dependency Alignment

This RFC touches only `loeres-core`. It is referenced by [RFC 001](001-stratified-scalar.md), [RFC 002](002-storage-agnostic-contracts.md), and all later RFCs.

Dependency constraints:

* no `std::error::Error` implementation in `loeres-core`;
* no `core::fmt::Display` implementation in `loeres-core` baseline;
* no heap-allocated strings;
* no formatting paths required for normal error translation;
* no backend-specific error payloads in core error variants.

Cluster crates may wrap `SolverError` in richer `std` errors, but that wrapper must not be part of `loeres-core`.

## 3. Concrete Technical Specification

### 3.1 Module layout

```rust
pub mod diagnostic;
pub mod error;

pub use diagnostic::{DiagnosticCode, DiagnosticSnapshot};
pub use error::{error_code_to_str, SolverError};
```

### 3.2 Public error enum

`SolverError` must be semver-extensible and compact.

```rust
#[non_exhaustive]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum SolverError {
    DimensionMismatch { lhs: u32, rhs: u32 },
    InvalidDimension,
    InvalidInput,
    NonFiniteInput,
    UnsupportedProblemStructure,
    SingularMatrix,
    IllConditioned,
    NumericalDomain,
    Overflow,
    WorkspaceTooSmall,
    Cancelled,
    BackendUnavailable,
    InternalInvariantViolation,
}
```

The exact variant list may be refined during implementation, but this RFC mandates the categories above. Any payload must remain compact, copyable, and allocation-free. Non-convergence at the iteration cap is not an error; it is reported as `loeres_core::solver::SolveStatus::NotConverged` (RFC 014). `PanicGateViolation` is not a runtime error; it is a CI/release-gate result owned by RFC 010.

### 3.3 Fixed-size maximum bounds

The maximum accepted size for `SolverError` is 16 bytes on all supported targets.

The codebase must include a compile-time assertion equivalent to:

```rust
const _: () = {
    assert!(core::mem::size_of::<SolverError>() <= 16);
};
```

If a supported stable compiler cannot evaluate this assertion in all target profiles, an equivalent `xtask` compile probe must enforce the same limit.

### 3.4 Diagnostic snapshot

Diagnostics provide optional data-only context without logging.

```rust
#[non_exhaustive]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DiagnosticCode {
    None,
    BoundaryValidationFailed,
    IterationLimit,
    ConditioningWarning,
    WorkspaceReinitialized,
    CancellationObserved,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct DiagnosticSnapshot {
    pub code: DiagnosticCode,
    pub iteration: u32,
    pub primary_index: u16,
    pub secondary_index: u16,
}
```

`DiagnosticSnapshot` must also be size-budgeted. The initial maximum is 16 bytes unless RFC 006 or RFC 008 demonstrates that a larger size is essential and harmless for target devices.

### 3.5 `Display` prohibition

`loeres-core` must not implement:

```rust
impl core::fmt::Display for SolverError
```

The reason is not that `Display` always allocates, but that formatting support encourages string paths and increases binary-size pressure in device builds. Human-facing formatting belongs in `loeres-cluster`, diagnostic tools, or optional host-side helper crates.

### 3.6 Static lookup translation

Core must expose a static string mapping for error categories:

```rust
#[inline]
pub const fn error_code_to_str(err: SolverError) -> &'static str {
    match err {
        SolverError::DimensionMismatch { .. } => "dimension_mismatch",
        SolverError::InvalidDimension => "invalid_dimension",
        SolverError::InvalidInput => "invalid_input",
        SolverError::NonFiniteInput => "non_finite_input",
        SolverError::NumericalDomain => "numerical_domain",
        SolverError::Overflow => "overflow",
        SolverError::SingularMatrix => "singular_matrix",
        SolverError::IllConditioned => "ill_conditioned",
        SolverError::UnsupportedProblemStructure => "unsupported_problem_structure",
        SolverError::WorkspaceTooSmall => "workspace_too_small",
        SolverError::Cancelled => "cancelled",
        SolverError::BackendUnavailable => "backend_unavailable",
        SolverError::InternalInvariantViolation => "internal_invariant_violation",
    }
}
```

If `#[non_exhaustive]` prevents exhaustive matching in some context, the implementation must be adjusted inside the defining crate while keeping the public signature:

```rust
pub const fn error_code_to_str(err: SolverError) -> &'static str;
```

### 3.7 Error code classification helpers

Optional helper methods may exist if they remain small:

```rust
impl SolverError {
    #[inline]
    pub const fn is_input_error(self) -> bool;

    #[inline]
    pub const fn is_numerical_error(self) -> bool;

    #[inline]
    pub const fn is_resource_error(self) -> bool;
}
```

These helpers must not allocate or format.

## 4. Rust Systems-Level Nuances & Memory Safety

### 4.1 `Result<T, E>` size control

Every core fallible method uses `Result<T, SolverError>`. A bloated `SolverError` would bloat return paths, spill registers, and increase stack pressure in device loops. The 16-byte hard limit exists to prevent error payload creep.

### 4.2 Semver compatibility

`#[non_exhaustive]` is mandatory for public error and diagnostic enums. Downstream users must match with a wildcard arm. This protects future Loeres versions from making every new solver-specific error category a breaking change.

### 4.3 No `unsafe`

This RFC requires no `unsafe`. Error values are plain data.

### 4.4 Cluster wrapping

`loeres-cluster` may define a separate error type that implements `std::error::Error` and `Display`, but it must wrap or translate `SolverError` at the cluster boundary. That richer error must never be returned by `loeres-core`, `loeres-backend-static`, or `loeres-device` baseline APIs.

## 5. Algorithmic & Numerical Fail-Safe Guardrails

The following mapping is mandatory:

| Failure condition | Required error category |
|---|---|
| Invalid vector/matrix dimension | `InvalidDimension` or `DimensionMismatch` |
| Non-finite public input | `NonFiniteInput` |
| Division by zero, sqrt of negative, log of invalid value | `NumericalDomain` |
| Arithmetic overflow in checked scalar/storage operation | `Overflow` |
| Singular matrix detected by solver | `SingularMatrix` |
| Conditioning exceeds solver policy | `IllConditioned` |
| Bounded loop finishes without convergence | not an error — `Ok(SolveReport)` with `SolveStatus::NotConverged` (RFC 014) |
| Valid problem unsupported by selected solver/profile | `UnsupportedProblemStructure` |
| Library bug / impossible state reached | `InternalInvariantViolation` |
| Provided workspace cannot fit required scratch state | `WorkspaceTooSmall` |
| Cluster cancellation token observed | `Cancelled` |
| Optional backend unavailable | `BackendUnavailable` |

Device solvers must not panic for any condition in this table.

## 6. Verification, Validation, and CI Gates

### 6.1 Compile-time size assertions

CI must compile assertions for:

```rust
size_of::<SolverError>() <= 16
size_of::<DiagnosticSnapshot>() <= 16
```

on every supported target profile.

### 6.2 Formatting audit

`xtask check-rfcs` must reject:

* `impl Display for SolverError` in `loeres-core`;
* `std::error::Error` implementation in `loeres-core`;
* heap-allocated strings in core error paths;
* `format!`, `String`, `Vec`, or `Box` in core error and diagnostic modules.

### 6.3 Static lookup tests

Tests must validate that every public `SolverError` category maps to a non-empty static string and that the mapping does not allocate.

### 6.4 Semver lint

A source-level lint must ensure public error and diagnostic enums carry `#[non_exhaustive]`.

### 6.5 Acceptance criteria

RFC 003 may move to `done/` only when:

1. `SolverError` and `DiagnosticSnapshot` satisfy the size budget;
2. `loeres-core` implements `Debug` but not `Display` for core errors;
3. `error_code_to_str(err: SolverError) -> &'static str` exists;
4. all access/scalar failure cases use structured errors;
5. no baseline core error path imports `std` or `alloc`.
