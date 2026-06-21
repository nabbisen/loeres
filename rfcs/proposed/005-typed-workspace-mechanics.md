# RFC 005 — Caller-Owned Typed Workspace Mechanics and Poison-Free Reuse

**Status.** Proposed
**Tracks.** Phase 2 / Milestone 2 — Static Backend and Real-Time Kernel
**Touches.** `loeres-device/src/workspace.rs`, `loeres-device/src/config.rs`, `loeres-device/src/lib.rs`, solver workspace modules

---

### Extended Metadata
* **Rust Edition Compliance:** Rust 2024 Baseline
* **Target Environment:** `loeres-device`; uses `loeres-core` and `loeres-backend-static`

## 1. Executive Summary & Problem Statement

Device solvers cannot allocate hidden scratch buffers. The caller must own and pass all workspace memory explicitly. However, a workspace that is partially mutated by a failed solve must not become a poisoned object requiring expensive manual zeroing before the next control-loop tick.

This RFC defines typed, caller-owned workspace mechanics for `loeres-device`. The workspace must be sized before solve execution, passed by unique mutable reference, and safe to discard or immediately reuse after any solver outcome.

## 2. Architectural Context & Dependency Alignment

This RFC depends on:

* [RFC 001](../done/001-stratified-scalar.md) for scalar capabilities;
* [RFC 003](../done/003-allocation-free-errors.md) for `WorkspaceTooSmall` and numerical errors;
* [RFC 004](004-static-storage-engine.md) for static storage primitives.

Dependency rules:

| Crate | Role |
|---|---|
| `loeres-device` | Owns typed workspace lifecycle and deterministic entrypoints |
| `loeres-backend-static` | Provides static storage containers used inside workspace structs |
| `loeres-core` | Provides errors, scalar bounds, access traits |
| `loeres-backend-std` | Not permitted |
| `loeres-cluster` | Not permitted |

No `alloc`, no `std`, and no background runtime are allowed.

## 3. Concrete Technical Specification

### 3.1 Workspace design principle

Every device solver must define a solver-specific workspace type. A generic raw byte buffer is not the baseline public API.

```rust
pub trait DeviceWorkspace {
    #[inline]
    fn reset_for_entry(&mut self);

    #[inline]
    fn diagnostic(&self) -> DiagnosticSnapshot;
}
```

`reset_for_entry` is a logical initialization step. It must not require zeroing the whole buffer unless a specific field must be initialized for correctness. The default expectation is overwrite-on-use.

### 3.2 Compile-time footprint computation

Each solver family defines an associated workspace type from problem dimensions:

```rust
pub trait WorkspaceFor<P> {
    type Workspace: DeviceWorkspace;

    #[inline]
    fn required_workspace_bytes() -> usize;
}
```

For a dense QP solver, an example shape is:

```rust
pub struct DenseQpWorkspace<S, const N: usize, const M: usize> {
    pub primal_delta: FixedVector<S, N>,
    pub residual: FixedVector<S, M>,
    pub scratch_n: FixedVector<S, N>,
    pub diagnostic: DiagnosticSnapshot,
}
```

This is a design shape, not an implementation lock. RFC 006 may refine the exact buffers for the first solver kernel.

### 3.3 Workspace entry contract

Every deterministic solve entrypoint must begin by calling the equivalent of:

```rust
workspace.reset_for_entry();
```

before using scratch fields. Therefore, the caller does not need to manually clear or wipe the workspace between repeated solves.

### 3.4 Poison-free failure semantics

A workspace has three possible logical conditions:

| Condition | Meaning | Caller action required |
|---|---|---|
| Ready | Workspace can be passed to solver | None |
| Dirty | Workspace contains arbitrary scratch data from a completed or failed run | None; next solve resets required entry fields |
| Discarded | Caller chooses to stop using it | Drop or ignore |

The public API must not expose a poisoned state that blocks reuse. Any solver failure must leave the workspace memory safe-to-discard and immediately reusable by the next solver call.

### 3.5 Zero-clearing elimination

The API must not require:

```rust
workspace.clear_all_to_zero();
```

between consecutive loop cycles. Full zeroing may be available as a debug helper or test utility, but it must not be part of the correctness contract.

### 3.6 Runtime solver configuration

Execution policy values must be runtime configuration fields, not type-level const generics.

```rust
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum TimingMode {
    EarlyExitAllowed,
    ConstantIteration,
}

pub struct DeviceSolveConfig<S> {
    pub max_iterations: u32,
    pub tolerance: S,
    pub timing_mode: TimingMode,
}
```

Const generics are reserved for memory dimensions, not for policy values such as tolerance, max iterations, or timing mode. This prevents monomorphization bloat.

### 3.7 Solve entrypoint pattern

```rust
pub fn solve_device<P, W, S>(
    problem: &P,
    workspace: &mut W,
    config: &DeviceSolveConfig<S>,
) -> Result<DeviceSolveReport, SolverError>
where
    W: DeviceWorkspace,
    S: MetricScalar + FiniteScalar;
```

Concrete solvers may use more specific bounds and types, but they must retain caller-owned `&mut` workspace semantics.

## 4. Rust Systems-Level Nuances & Memory Safety

### 4.1 Unique mutable borrowing

The workspace is passed by `&mut` so Rust guarantees exclusive access during solve execution. No shared mutable global state is needed.

### 4.2 No by-value workspace transfer

Solver entrypoints must not take workspace by value. Moving large workspace structs risks stack pressure and accidental copies.

### 4.3 Dirty-but-valid memory

After an error, scratch fields may contain arbitrary intermediate numeric values. This is acceptable because every subsequent solve must overwrite or logically initialize all fields before reading them.

### 4.4 No `unsafe`

Baseline workspace mechanics require no `unsafe`. If a future solver kernel proposes optimized scratch views requiring unsafe aliasing assumptions, it must be isolated in a later RFC.

## 5. Algorithmic & Numerical Fail-Safe Guardrails

1. A bounded run that reaches the iteration cap without convergence (`Ok(SolveStatus::NotConverged)`, RFC 014) leaves the workspace dirty but reusable; it is a terminal status, not a `SolverError`.
2. `SingularMatrix` leaves the workspace dirty but reusable.
3. `IllConditioned` leaves the workspace dirty but reusable.
4. `NumericalDomain` leaves the workspace dirty but reusable.
5. A panic must not be used to signal workspace misuse.
6. If a workspace type does not match the problem dimensions, this must be impossible by type construction or rejected before the solver loop begins.

The key rule is: failure may invalidate mathematical contents, but it must not invalidate the workspace object as a reusable memory region.

## 6. Verification, Validation, and CI Gates

### 6.1 Reuse tests

For every device solver, tests must execute:

1. valid solve using workspace `W`;
2. adversarial solve that returns an error;
3. valid solve again using the same workspace `W` without manual clearing.

The third step must succeed or return only a problem-dependent error, never a workspace-poisoning error.

### 6.2 Zero-clearing audit

`xtask` must reject device solver implementations that require public calls to `clear_all`, `zeroize`, or equivalent methods between normal loop cycles. Security wiping may exist for sensitive data in future RFCs, but not as a normal mathematical correctness requirement.

### 6.3 Size reporting

Each workspace type must expose `required_workspace_bytes()` and CI must record representative sizes in build output.

### 6.4 Dependency gate

No workspace module may import `std`, `alloc`, or cluster/backend-std crates.

### 6.5 Acceptance criteria

RFC 005 may move to `done/` only when:

1. typed workspace patterns are defined for device solvers;
2. workspace memory is caller-owned and passed by `&mut`;
3. failures leave workspace immediately reusable;
4. normal reuse requires no full-buffer zeroing;
5. execution policy values are runtime config fields, not const generic solver identity.
