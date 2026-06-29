# RFC 006 — Baseline Deterministic Device Solver Kernel

**Status.** Implemented (v0.10.0; fail-safe hardening + closeout corrections in v0.10.1) — baseline box/bound-constrained projected first-order kernel. `ProjectedFirstOrderProblem` (problem.rs) + `DeviceSolveReport` / `ProjectedFirstOrderWorkspace` / `solve_projected_first_order` (solve.rs), behind `loeres-device` feature `owned-arrays`; outcomes via RFC 014 `SolveReport` + `AsCoreReport`. Implementation-decision pass I1–I10 applied (see §7).
**Tracks.** Phase 2 / Milestone 2 — Static Backend and Real-Time Kernel
**Touches.** `loeres-device/src/problem.rs`, `loeres-device/src/solve.rs`, `loeres-device/src/diagnostic.rs`, `loeres-device/src/lib.rs`

---

### Extended Metadata
* **Rust Edition Compliance:** Rust 2024 Baseline
* **Target Environment:** `loeres-device`; depends on `loeres` and `loeres-backend-static`

## 1. Executive Summary & Problem Statement

The first `loeres-device` solver must prove the project's core promise: bounded, panic-averse, allocation-free optimization fit for deterministic control-loop integration. To keep that proof small and verifiable, the baseline kernel is a **box/bound-constrained projected first-order solver** — not a general dense QP. It runs a bounded iteration loop, projects each step onto explicit lower/upper bounds, validates finiteness and tolerance, uses a caller-owned typed workspace, and maps outcomes through RFC 014 / RFC 003.

Dense convex QP, KKT/active-set, and interior-point methods are explicitly **out of scope** for RFC 006; they are left to a later, specialized solver RFC once the static-storage / workspace path is proven.

## 2. Architectural Context & Dependency Alignment

This RFC depends on:

* [RFC 001](001-stratified-scalar.md) for scalar bounds;
* [RFC 002](002-storage-agnostic-contracts.md) for access traits and the contiguous fast path;
* [RFC 003](003-allocation-free-errors.md) for fail-safe error categories;
* [RFC 004](004-static-storage-engine.md) for fixed storage;
* [RFC 005](005-typed-workspace-mechanics.md) for the workspace lifecycle, `WorkspaceFor`, and `DeviceSolveConfig` / `TimingMode`.

`loeres-device` must not depend on `loeres-backend-std`, `loeres-cluster`, `std`, `alloc`, threads, async runtimes, logging frameworks, or FFI gateways.

## 3. Concrete Technical Specification

### 3.1 Solver scope

The baseline kernel is a **box/bound-constrained projected first-order method** over fixed-size primal vectors: a bounded loop of gradient-driven steps, each projected onto `[lo, hi]`.

Explicitly **excluded** from RFC 006 (deferred to a later specialized solver RFC):

* dense convex QP, KKT / active-set, interior-point methods;
* mixed-integer programming; branch-and-bound;
* dynamic variable registration; string-based modeling DSLs;
* sparse dynamic assembly; unbounded adaptive loops.

### 3.2 Problem contract

The baseline problem is a **first-order oracle over box bounds**, not a matrix-exposing dense-QP contract. The following is a design sketch; exact trait / associated-type names and signatures are settled in the implementation-decision pass.

```rust
pub trait ProjectedFirstOrderProblem<S> {
    /// Caller-owned contiguous vector storage for primal / gradient / bound vectors.
    type Vector: ContiguousVectorAccess<Scalar = S>;

    /// Number of primal variables.
    fn dimension(&self) -> usize;

    /// Validate problem data before the loop (finite bounds, `lo <= hi`, …).
    fn validate_boundary(&self) -> Result<(), SolverError>;

    /// Lower and upper box bounds for projection.
    fn bounds(&self) -> (&Self::Vector, &Self::Vector);

    /// Objective value at `x` (for reporting; the step is gradient-driven).
    fn objective_at(&self, x: &Self::Vector) -> Result<S, SolverError>;

    /// Gradient `∇f(x)` written into `grad` — the hot-loop oracle.
    fn gradient_at(&self, x: &Self::Vector, grad: &mut Self::Vector) -> Result<(), SolverError>;
}
```

No `#[inline]` is placed on these bodiless declarations (F5); implementors place `#[inline]` on method bodies.

### 3.3 Solve entrypoint and scalar bounds

The workspace is bound to the problem **at the type level** by the shared scalar `S` and const dimension `N`: the entrypoint takes a concrete `ProjectedFirstOrderWorkspace<S, N>`, so a workspace of the wrong dimension for the iterate is a compile error. RFC 005's `WorkspaceFor<P>` remains the **sizing/reporting** contract (`required_workspace_bytes`), implemented by the concrete problem family; it is not the entrypoint binding mechanism, because an opaque `<F as WorkspaceFor<P>>::Workspace` would prevent the kernel from reaching the gradient scratch. Implemented signature:

```rust
pub fn solve_projected_first_order<P, S, const N: usize>(
    problem: &P,
    x: &mut FixedVector<S, N>,
    workspace: &mut ProjectedFirstOrderWorkspace<S, N>,
    config: &DeviceSolveConfig<S>,
) -> Result<DeviceSolveReport, SolverError>
where
    P: ProjectedFirstOrderProblem<S, N>,
    S: FiniteScalar + MetricScalar,
{
    // 1. workspace.reset_for_entry()      (RFC 005 lifecycle)
    // 2. config.validate()                (RFC 005 structural validation)
    // 3. problem.validate_boundary()
    // 4. validate step scale (finite, > 0) and initial iterate (finite)
    // 5. for k in 0..config.max_iterations { projected gradient step }
    // 6. map step outcomes -> SolveReport (RFC 014 §3.5) -> DeviceSolveReport
}
```

`x` is the explicit in/out iterate (I2); the workspace is pure gradient scratch.

`config.max_iterations` is a runtime field, not a const generic.

**Scalar bounds (F7).** The baseline projected step is `x_{k+1} = clamp(x_k - α·∇f(x_k), lo, hi)`, using `BaseScalar::{sub, mul}`, `OrderedScalar::clamp`, and `MetricScalar::{abs, lte_tolerance}` for the convergence test. `DivisibleScalar` is added **only if** the step rule divides inside the solver. The step scale `α` is **problem-provided** (`step_scale`) and validated finite and strictly positive before the loop; the kernel performs no division, so the bound stays **`S: FiniteScalar + MetricScalar`** (each carries `BaseScalar`; neither implies the other) and `DeviceSolveConfig` (RFC 005) is unchanged.

### 3.4 Timing modes

`TimingMode::EarlyExitAllowed`: may return immediately after convergence; still capped at `max_iterations`.

`TimingMode::ConstantIteration`: executes exactly `max_iterations` iterations, records convergence as data, and does not early-return on success; it may still fail fast on boundary validation before the loop. This variant and its driver branch are compiled only under the `constant-iteration` feature (RFC 005 M5); `TimingMode` is `#[non_exhaustive]`, so the driver's match carries a wildcard arm. This is constant-iteration, not cryptographic constant-time.

### 3.5 Report type

The device report wraps RFC 014's `SolveReport` and implements `AsCoreReport` (RFC 014 §3.8 / §5.3). It carries no diagnostic field; the compact diagnostic is read from the workspace through RFC 005's always-available `DeviceWorkspaceDiagnostic::diagnostic()` accessor. Richer policy-gated diagnostic collection is deferred to a later diagnostics RFC; RFC 006 introduces no such policy type.

```rust
use loeres::solver::{AsCoreReport, SolveReport, SolveStatus};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct DeviceSolveReport {
    core: SolveReport,
}

impl DeviceSolveReport {
    #[inline]
    pub const fn from_core(core: SolveReport) -> Self {
        Self { core }
    }
    #[inline]
    pub const fn core(&self) -> SolveReport {
        self.core
    }
    #[inline]
    pub const fn status(&self) -> SolveStatus {
        self.core.status()
    }
    #[inline]
    pub const fn iterations_executed(&self) -> u32 {
        self.core.iterations_executed()
    }
}

impl AsCoreReport for DeviceSolveReport {
    #[inline]
    fn as_core_report(&self) -> SolveReport {
        self.core
    }
}
```

The report must remain allocation-free and size-budgeted, and must project losslessly onto the core `SolveReport` (RFC 014 §3.8).

### 3.6 Panic-aversion audit map

| Risk | Required handling |
|---|---|
| Out-of-bounds access | fallible access from RFC 002 |
| Division by zero | `DivisibleScalar::checked_div` (only if the step rule divides) |
| Non-finite input | boundary validation using `FiniteScalar::is_finite` |
| Overflow | checked arithmetic where applicable, `SolverError::Overflow` |
| Ill-conditioned / invalid bounds | `SolverError::IllConditioned` / `SolverError::InvalidInput` |
| Iteration non-convergence | `Ok(SolveReport)` with `SolveStatus::NotConverged` (RFC 014 §3.5); never a `SolverError` |
| Workspace mismatch | type-level binding via shared `S, N` and `ProjectedFirstOrderWorkspace<S, N>` (§3.3); `WorkspaceFor<P>` is the sizing contract |

No `unwrap`, `expect`, panic-based assertion, or unchecked index is accepted in the kernel path.

### 3.7 Contiguous fast-path usage

This is the kernel RFC 002 scoped its contiguous fast path for. The hot loop reads and writes the primal, gradient, bound, and scratch vectors through `ContiguousVectorAccess` / `ContiguousVectorAccessMut` (`as_contiguous` / `as_contiguous_mut`) where the storage is contiguous, falling back to per-element `VectorAccess` only when it is not. The baseline projected first-order kernel is **vector-dominant** and requires no matrix operator; `ContiguousMatrixAccess` (RFC 002 / 004) remains available for a later RFC 006 extension or a follow-up dense-QP RFC, but is not part of this kernel.

### 3.8 Concrete workspace ownership

The concrete projected-first-order workspace type is RFC 006-owned (per the RFC 005 §3 boundary), built on RFC 004 fixed storage, implementing the RFC 005 `DeviceWorkspace` / `DeviceWorkspaceDiagnostic` lifecycle. It is `ProjectedFirstOrderWorkspace<S, N>` in `solve`, holding the gradient scratch; the entrypoint binds it concretely by shared `S, N`. `WorkspaceFor<P>` is implemented by the problem family as the sizing contract (`required_workspace_bytes`), not the entrypoint binding.

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
3. The solver cannot loop forever because the iteration count is bounded.
4. In constant-iteration mode, convergence does not change the number of loop iterations.
5. In early-exit mode, convergence may return early but never exceeds the cap.
6. Box projection uses `OrderedScalar` / `MetricScalar` ordering and clamp; numerical-domain violations are errors, not panics.
7. The solver's supported tolerance range is validated before the loop (RFC 005 structural validation rejects non-finite / negative tolerance; any solver-specific tolerance rule is RFC 006's).

## 6. Verification, Validation, and CI Gates

### 6.1 Device target execution

CI must include a device reference target build and, where available, emulated execution for the baseline kernel.

### 6.2 Panic-path static audit

`xtask` must scan the kernel and device hot path for `unwrap`, `expect`, direct indexing in solver loops, `panic!`, `todo!`, `unimplemented!`, allocation APIs, and logging macros.

### 6.3 Adversarial numerical tests

Tests must include:

* non-finite input for primitive floats;
* dimension mismatch;
* invalid bounds (`lo > hi`) and deliberately ill-conditioned problem shape;
* max-iteration non-convergence returns `Ok(DeviceSolveReport)` wrapping a core `SolveReport` whose status is `SolveStatus::NotConverged` and whose termination reason is `TerminationReason::IterationCap` — not a `SolverError`;
* a valid convergence case;
* a zero denominator, only if the accepted step rule divides;
* workspace reuse after each error path and after bounded non-convergence status.

### 6.4 Size and timing gates

The first implemented kernel must publish: compiled binary size contribution; workspace byte footprint (`WorkspaceFor::required_workspace_bytes`); maximum iteration cap used in tests; and representative iteration timing on the reference target profile.

### 6.5 Numerical parity gate

The same problem corpus must run against the cluster path when RFC 007 and RFC 008 are available. Success is convergence agreement within `epsilon = 1e-5`, not bitwise equality.

### 6.6 Acceptance criteria

RFC 006 may move to `done/` only when:

1. the baseline device solver is a bounded-iteration box/bound-constrained projected first-order kernel;
2. it uses a caller-owned typed gradient-scratch workspace bound concretely by shared `S, N` (`ProjectedFirstOrderWorkspace<S, N>`), with `WorkspaceFor<P>` as the sizing contract;
3. it passes the panic-aversion audits;
4. it compiles without `std` or `alloc`;
5. it passes the adversarial numerical tests;
6. `DeviceSolveReport` wraps `SolveReport` and implements `AsCoreReport`;
7. it reports size and workspace budgets.

## 7. Implementation Decisions and Departures (v0.10.0)

The narrow implementation-decision pass resolved the following before coding.

| Item | Resolution |
|---|---|
| I1 | `DeviceSolveReport` lives in `solve.rs`; `diagnostic.rs` reserved for diagnostics. |
| I2 | Explicit in/out `&mut x`; the workspace is pure scratch. |
| I3 | Read-only `Bounds` is a distinct associated type from the work vectors. |
| I4 | Bounds read-only on the problem; primal/gradient mutable via `&mut x` and scratch. |
| I5 | Concrete `ProjectedFirstOrderWorkspace<S, N>` scratch in `solve`. |
| I6 | Problem-provided `step_scale` (no division); bound stays `FiniteScalar + MetricScalar`. RFC 005 config untouched. |
| I7 | Convergence on iterate change, `max_i |x_next[i] - x[i]| <= tolerance`. |
| I8 | `EarlyExitAllowed` → `converged_early`/`not_converged_cap`; `ConstantIteration` runs the full count → `converged_at_cap`/`not_converged_cap` (`iterations_executed == max_iterations`). |
| I9 | Objective is reporting-only; not evaluated in the hot loop. |
| I10 | Test corpus: convergence, box projection, cap non-convergence, inverted bounds, non-finite tolerance, dimension mismatch, workspace reuse after error and after non-convergence, sizing, objective, diagnostic, `AsCoreReport`, and (feature-gated) constant-iteration. |

Departures from earlier sketches, recorded for review:

1. **Concrete workspace binding (refines §3.8 / §3.5).** The kernel takes `workspace: &mut ProjectedFirstOrderWorkspace<S, N>` directly. The shared const `N` type-pins the workspace to the iterate and problem, so a wrong-sized workspace is a compile error — the same "wrong workspace is impossible by construction" guarantee the `WorkspaceFor<P>` sketch aimed for. An *opaque* `<F as WorkspaceFor<P>>::Workspace` in the signature would prevent the kernel from reaching the gradient scratch; the concrete binding is what makes scratch access possible. `WorkspaceFor<P>` remains the RFC 005 sizing contract (`required_workspace_bytes`), implemented by the concrete problem family and exercised in the tests.
2. **Concrete work vectors.** The primal and gradient are `FixedVector<S, N>` (the device static storage), not fully generic over `ContiguousVectorAccess`. The read-only `Bounds` stay a distinct associated type, so the bounds-vs-work distinction (I3) holds; full work-vector genericity is a later extension.
3. **Fast-path scope (refines §3.7).** Primal and gradient always use fixed-size slices; bounds use the contiguous slice when available and fall back to per-element `get()` otherwise.
4. **`objective_at` reporting-only.** Present in the contract (§3.2) but not called by the iterate-change driver, since `DeviceSolveReport` carries no objective field; it is covered by a direct test and is the natural hook for future objective-based criteria.

### 7.1 Measured evidence (v0.10.1, §6.4)

Size and timing evidence for the first implemented kernel:

- **Workspace footprint.** `ProjectedFirstOrderWorkspace<S, N>` is `8N + 16` bytes for `S = f64` (measured: `N = 2 → 32`, `N = 4 → 48`, `N = 8 → 80`): the `FixedVector<f64, N>` gradient scratch plus a 12-byte `DiagnosticSnapshot` with 4 bytes alignment padding. `DeviceSolveReport` is 12 bytes. The footprint is queryable at compile time through `WorkspaceFor::required_workspace_bytes`.
- **Iteration cap.** The test corpus uses caps up to 200; the 2-D box-quadratic at `α = 0.5` converges in roughly 30 iterations (the `0.5^k` contraction reaching `1e-9`).
- **Per-iteration cost.** One oracle gradient evaluation plus one `O(N)` projection pass; total work is bounded by `max_iterations × O(N)`, and is exactly `max_iterations` passes under `ConstantIteration` (deterministic).
- **Binary size.** The optimized `thumbv7em-none-eabihf` `loeres-device` rlib (`owned-arrays`) is ~73 KiB *including archive metadata and symbols* — a rough upper-bound proxy, not a per-symbol code contribution.

The `panic-audit` gate (RFC 006 §6.2) is implemented in `xtask` and runs in `release-gate`, scanning the `no_std` production crates for `unwrap` / `expect` / `panic!` / `todo!` / `unimplemented!` / logging macros (tests excluded). Automated per-symbol binary-size budgeting and reference-target wall-clock timing remain the `size-budget` gate's scope, owned by RFC 010 (xtask verification governance) / RFC 011 (target profiles).
