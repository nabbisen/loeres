# RFC 006 — Baseline Deterministic Device Solver Kernel

**Status.** Proposed
**Tracks.** Phase 2 / Milestone 2 — Static Backend and Real-Time Kernel
**Touches.** `loeres-device/src/qp.rs`, `loeres-device/src/kernel.rs`, `loeres-device/src/report.rs`, deterministic solver entrypoints

---

### Extended Metadata
* **Rust Edition Compliance:** Rust 2024 Baseline
* **Target Environment:** `loeres-device`; depends on `loeres` and `loeres-backend-static`

## 1. Executive Summary & Problem Statement

The first `loeres-device` solver must demonstrate the project's core promise: bounded, panic-averse, allocation-free optimization suitable for deterministic control-loop integration.

This RFC defines the external and systems-level design for the baseline deterministic solver kernel. The initial solver family is a dense, structured, first-order or QP-oriented kernel with bounded iteration count. It is intentionally narrower than the cluster solver family.

## 2. Architectural Context & Dependency Alignment

This RFC depends on:

* [RFC 001](../done/001-stratified-scalar.md) for scalar bounds;
* [RFC 002](../done/002-storage-agnostic-contracts.md) for access traits;
* [RFC 003](../done/003-allocation-free-errors.md) for fail-safe error categories;
* [RFC 004](../done/004-static-storage-engine.md) for fixed storage;
* [RFC 005](005-typed-workspace-mechanics.md) for workspace lifecycle.

`loeres-device` must not depend on `loeres-backend-std`, `loeres-cluster`, `std`, `alloc`, threads, async runtimes, logging frameworks, or FFI gateways.

## 3. Concrete Technical Specification

### 3.1 Solver scope

The baseline kernel supports a constrained dense problem family suitable for deterministic execution. The exact mathematical family may be finalized as:

* dense convex QP with fixed dimensions; or
* capped projected-gradient / first-order method for structured convex problems.

The design explicitly excludes:

* mixed-integer programming;
* branch-and-bound;
* dynamic variable registration;
* string-based modeling DSLs;
* sparse dynamic assembly;
* unbounded adaptive loops.

### 3.2 Problem shape

```rust
pub trait DeviceProblem<S> {
    type Primal: VectorAccess<Scalar = S>;
    type Constraints: VectorAccess<Scalar = S>;

    #[inline]
    fn validate_boundary(&self) -> Result<(), SolverError>;

    #[inline]
    fn objective_at(&self, x: &Self::Primal) -> Result<S, SolverError>;

    #[inline]
    fn constraints_at(
        &self,
        x: &Self::Primal,
        residual: &mut Self::Constraints,
    ) -> Result<(), SolverError>;
}
```

Concrete kernels may specialize this trait into QP-specific matrix forms after RFC acceptance.

### 3.3 Constant-iteration loop pattern

The baseline kernel must use a bounded `for` loop. An unstructured convergence-controlled `while` loop is forbidden.

```rust
pub fn solve_dense_device<P, W, S>(
    problem: &P,
    workspace: &mut W,
    config: &DeviceSolveConfig<S>,
) -> Result<DeviceSolveReport, SolverError>
where
    P: DeviceProblem<S>,
    W: DeviceWorkspace,
    S: FiniteScalar + DivisibleScalar + MetricScalar,
{
    // public design pattern only
    // 1. workspace.reset_for_entry()
    // 2. problem.validate_boundary()
    // 3. for iteration in 0..config.max_iterations { bounded step }
    // 4. return structured report or SolverError
}
```

`config.max_iterations` is a runtime field. It is not a const generic parameter.

### 3.4 Timing modes

`TimingMode::EarlyExitAllowed`:

* may return immediately after convergence;
* still has a maximum iteration cap.

`TimingMode::ConstantIteration`:

* executes exactly `max_iterations` iterations;
* records convergence as data;
* does not early return for success;
* may still fail fast for boundary validation errors before entering the loop.

This is constant-iteration, not cryptographic constant-time.

### 3.5 Report type

```rust
use loeres::solver::{AsCoreReport, SolveReport, SolveStatus};

// Status comes from the core taxonomy (RFC 014). The baseline device report
// carries no diagnostic field; the compact diagnostic is read from the
// workspace (RFC 005) only when DiagnosticPolicy enables it. A diagnostic-
// bearing variant, if needed, is a separate explicitly policy-gated type
// (RFC 014 §3.8).
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct DeviceSolveReport {
    core: SolveReport,
}

impl DeviceSolveReport {
    #[inline]
    pub const fn from_core(core: SolveReport) -> Self { Self { core } }
    #[inline]
    pub const fn core(&self) -> SolveReport { self.core }
    #[inline]
    pub const fn status(&self) -> SolveStatus { self.core.status() }
    #[inline]
    pub const fn iterations_executed(&self) -> u32 { self.core.iterations_executed() }
}

impl AsCoreReport for DeviceSolveReport {
    #[inline]
    fn as_core_report(&self) -> SolveReport { self.core }
}
```

The report must remain allocation-free and size-budgeted, and must project losslessly onto the core `SolveReport` (RFC 014 §3.8).

### 3.6 Panic-aversion audit map

| Risk | Required handling |
|---|---|
| Out-of-bounds access | fallible access from RFC 002 |
| Division by zero | `DivisibleScalar::checked_div` |
| Non-finite input | boundary validation using `FiniteScalar` |
| Overflow | checked arithmetic where applicable, `SolverError::Overflow` |
| Singular update | `SolverError::SingularMatrix` |
| Ill-conditioned data | `SolverError::IllConditioned` |
| Iteration non-convergence | `Ok(SolveReport)` with `SolveStatus::NotConverged` (RFC 014 §3.5); never a `SolverError` |
| Workspace mismatch | type-level match or pre-loop error |

No `unwrap`, `expect`, panic-based assertion, or unchecked index is accepted in the kernel path.

## 4. Rust Systems-Level Nuances & Memory Safety

### 4.1 Monomorphization control

Problem dimensions may be const generics because they define storage. Policy values such as max iterations and timing mode remain runtime fields to avoid one copy of the solver per policy combination.

### 4.2 Stack behavior

All large data is held in caller-owned workspace. The kernel must not create large local arrays inside the solve function.

### 4.3 Instruction predictability

The kernel should prefer straight-line bounded loops and small helper functions. Helper functions may use `#[inline]`, but excessive `#[inline(always)]` must be justified by size and WCET profiling.

### 4.4 No `unsafe`

The baseline deterministic kernel must use safe Rust only. Any later unsafe optimization must include a proof obligation and an unsafe containment boundary.

## 5. Algorithmic & Numerical Fail-Safe Guardrails

1. Boundary validation runs before the iteration loop unless the caller supplies a validated-input state accepted by a later RFC.
2. Every step returns `Result<StepOutcome, SolverError>` using `StepOutcome` from `loeres::solver` (RFC 014 §3.2); the bounded driver maps step outcomes onto the terminal `SolveReport` exactly as in RFC 014 §3.5, and must test the `converged_at_cap` cap-equality invariant per timing mode.
3. The solver cannot loop forever because iteration count is bounded.
4. In constant-iteration mode, convergence does not change the number of loop iterations.
5. In early-exit mode, convergence may return early but never exceeds the cap.
6. Numerical domain violations are errors, not panics.
7. The solver's supported tolerance range must be validated before the loop.

## 6. Verification, Validation, and CI Gates

### 6.1 Device target execution

CI must include a device reference target build and, where available, emulated execution for the baseline kernel.

### 6.2 Panic-path static audit

`xtask` must scan the kernel and device hot path for:

* `unwrap`;
* `expect`;
* direct indexing in solver loops;
* `panic!`;
* `todo!`;
* `unimplemented!`;
* allocation APIs;
* logging macros.

### 6.3 Adversarial numerical tests

Tests must include:

* zero denominator;
* non-finite input for primitive floats;
* dimension mismatch;
* deliberately ill-conditioned matrix/problem shape;
* max-iteration non-convergence reported as `Ok(SolveStatus::NotConverged)`, not an error;
* valid convergence case;
* workspace reuse after each error path and after bounded non-convergence status.

### 6.4 Size and timing gates

The first implemented kernel must publish:

* compiled binary size contribution;
* workspace byte footprint;
* maximum iteration cap used in tests;
* representative iteration timing on the reference target profile.

### 6.5 Numerical parity gate

The same problem corpus must run against the cluster path when RFC 007 and RFC 008 are available. Success is convergence agreement within `epsilon = 1e-5`, not bitwise equality.

### 6.6 Acceptance criteria

RFC 006 may move to `done/` only when:

1. the baseline device solver has a bounded iteration loop;
2. it uses caller-owned typed workspace;
3. it passes panic-aversion audits;
4. it compiles without `std` or `alloc`;
5. it passes adversarial numerical tests;
6. it reports size and workspace budgets.
