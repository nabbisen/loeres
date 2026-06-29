# RFC 005 — Caller-Owned Typed Workspace Mechanics and Poison-Free Reuse

**Status.** Implemented (v0.9.0) — two-crate workspace boundary. `loeres-backend-static::workspace` provides the `WorkspaceFootprint` byte-footprint contract (impls for the RFC 004 owned arrays behind `owned-arrays`); `loeres-device::workspace` provides the `DeviceWorkspace` / `DeviceWorkspaceDiagnostic` / `WorkspaceFor` lifecycle contracts; `loeres-device::config` provides `DeviceSolveConfig` / `TimingMode` (`#[non_exhaustive]`, feature-gated `ConstantIteration`) with structural validation. Implementation-decision pass (M1–M8) accepted; corrections P1 + the five pre-coding patches applied. Concrete solver workspaces, problem families, `DeviceSolveReport`, and the solve kernel remain RFC 006-owned.
**Tracks.** Phase 2 / Milestone 2 — Static Backend and Real-Time Kernel
**Touches.** `loeres-backend-static/src/workspace.rs`, `loeres-backend-static/src/lib.rs`, `loeres-device/src/workspace.rs`, `loeres-device/src/config.rs`, `loeres-device/src/lib.rs`

---

### Extended Metadata
* **Rust Edition Compliance:** Rust 2024 Baseline
* **Target Environment:** `loeres-backend-static` (storage-block primitives) and `loeres-device` (workspace lifecycle and runtime configuration); both `#![no_std]`, no `alloc`. Uses `loeres` and `loeres-backend-static`.

## 1. Executive Summary & Problem Statement

Device solvers cannot allocate hidden scratch buffers. The caller must own and pass all workspace memory explicitly. A workspace partially mutated by a failed solve must not become a poisoned object requiring expensive manual zeroing before the next control-loop tick.

This RFC defines the **two-crate workspace boundary** for Milestone 2:

* `loeres-backend-static` gains a minimal, generic **workspace storage-block layer** — `no_std`/no-`alloc` scratch-storage building blocks composed from the RFC 004 array/view primitives, plus footprint-reporting support and any marker contract device workspaces need.
* `loeres-device` gains the **caller-owned workspace lifecycle** (`DeviceWorkspace` / `WorkspaceFor`-style contracts, reset-on-entry, poison-free reuse) and the **runtime execution-configuration** primitives (`DeviceSolveConfig`, `TimingMode`).

This RFC deliberately does **not** define concrete solver workspaces (e.g. `DenseQpWorkspace`), problem-family contracts, the solve-kernel body, or solver report types. Those are owned by **RFC 006** (the deterministic device solver kernel). The boundary is fixed in §3.

## 2. Architectural Context & Dependency Alignment

This RFC depends on:

* [RFC 001](001-stratified-scalar.md) for scalar capabilities;
* [RFC 003](003-allocation-free-errors.md) for `WorkspaceTooSmall`, `DiagnosticSnapshot`, and numerical errors;
* [RFC 004](004-static-storage-engine.md) for static storage primitives (`FixedVector` / `FixedMatrix`, static views, `DimensionKind::Static`).

It forward-references **RFC 006** for the solver kernel, problem families, concrete workspaces, and the device report type.

Dependency rules:

| Crate | Role |
|---|---|
| `loeres-backend-static` | Provides the static **workspace storage-block primitives** (this RFC) plus the RFC 004 storage containers used inside them |
| `loeres-device` | Owns the typed **workspace lifecycle** and runtime configuration; deterministic entrypoints land in RFC 006 |
| `loeres` | Provides errors, `DiagnosticSnapshot`, scalar bounds, access traits |
| `loeres-backend-std` | Not permitted in either crate |
| `loeres-cluster` | Not permitted in either crate |

No `alloc`, no `std`, and no background runtime are allowed in either crate.

## 3. Crate Boundary and Module Ownership

This is the load-bearing structural contract of the RFC. The boundary is strict and is enforced by the dependency/feature gates in §10 and §12.

| Module | Owner | Contents |
|---|---|---|
| `loeres-backend-static::workspace` | **RFC 005** | Static workspace storage-block vocabulary; footprint-reporting support; `no_std`/no-`alloc` scratch-storage building blocks composed from RFC 004 array/view primitives; any marker contract device workspaces bind to. **No** concrete solver workspaces, entrypoints, problem contracts, report types, or timing-mode driver behavior. |
| `loeres-device::workspace` | **RFC 005** | The `DeviceWorkspace` / `WorkspaceFor`-style lifecycle contracts; reset-on-entry; poison-free / immediately reusable failure semantics; workspace byte-size reporting. |
| `loeres-device::config` | **RFC 005** | Runtime execution-policy primitives: `DeviceSolveConfig`, `TimingMode` (policy data, not const-generic solver identity). |
| `loeres-device::solve` | **RFC 006** | The actual `solve_device` body and deterministic entrypoints. |
| `loeres-device::problem` | **RFC 006** | The first-kernel problem family (`P`). |
| device report type (`DeviceSolveReport`) | **RFC 006** | Final device report shape, `AsCoreReport`-conforming (RFC 014). |
| concrete solver workspaces (`DenseQpWorkspace`, …) | **RFC 006** | Concrete per-solver workspace shapes built on the §4 primitives. |

RFC 005 is a two-crate *lifecycle/storage-boundary* RFC. It is not a solver-kernel RFC.

## 4. Backend-Static Workspace Storage-Block Layer

`loeres-backend-static::workspace` provides the minimal, generic vocabulary that device workspaces are built from. It is intentionally small: it standardizes how caller-owned scratch storage is described and measured, without owning any solver-specific shape.

Responsibilities:

1. **Scratch-storage building blocks** — typed bundles of RFC 004 fixed storage (`FixedVector` / `FixedMatrix`) that a device workspace composes. These are plain owned static storage; they carry no solver semantics.
2. **Footprint reporting** — a contract for reporting a block's byte footprint, computable from `core::mem::size_of` (see §8 / F9); no additional RFC 004 byte-size constant is introduced.
3. **Marker contract** — an optional marker that device workspace lifecycle bounds can name to require "static, caller-owned, footprint-known" storage.

The **concrete form** of this layer — whether the storage-block vocabulary is realized as a trait, a newtype wrapper, or a marker-only module, and whether footprint reporting is a trait method or a free function — is deferred to the implementation-decision pass (§4 of the review-response readiness list). This section fixes the *role and boundary*, not the API surface.

Feature posture: the storage-block primitives that embed owned RFC 004 arrays require `loeres-backend-static/owned-arrays` (§10). Pure marker/footprint contracts that name no owned array may live in the baseline; the implementation-decision pass settles which items sit on which side of that line.

## 5. Device Workspace Lifecycle

`loeres-device::workspace` owns the caller-owned lifecycle contract. A device solver workspace is a caller-owned struct, sized before solve, passed by unique `&mut`, and safe to discard or immediately reuse after any outcome.

### 5.1 Lifecycle contract

The lifecycle core is a single essential method; the compact diagnostic accessor is an always-available, ungated **extension trait** (M4):

```rust
pub trait DeviceWorkspace {
    fn reset_for_entry(&mut self);
}

pub trait DeviceWorkspaceDiagnostic {
    fn diagnostic(&self) -> DiagnosticSnapshot;
}
```

`reset_for_entry` is a logical initialization step. It must not require zeroing the whole buffer unless a specific field must be initialized for correctness; the default expectation is overwrite-on-use (§7).

`DeviceWorkspaceDiagnostic::diagnostic` returns the compact core `DiagnosticSnapshot` (RFC 003). It is **always available and never gated** by `diagnostic-snapshot` (§10, F6); the feature governs only richer/optional diagnostics. Keeping it on a separate trait holds `DeviceWorkspace` to the one essential lifecycle method while leaving the accessor ungated.

Trait method declarations carry **no** `#[inline]` (F4): `#[inline]` belongs on implementation bodies and on default methods that have bodies, never on a bodiless required signature.

### 5.2 Workspace sizing

Each solver family associates a workspace type with its problem dimensions and reports its footprint:

```rust
pub trait WorkspaceFor<P> {
    type Workspace: DeviceWorkspace;

    fn required_workspace_bytes() -> usize;
}
```

`required_workspace_bytes()` may compute directly from `core::mem::size_of::<Self::Workspace>()` (F9). The concrete `P` problem families and the concrete `Workspace` shapes are RFC 006-owned; RFC 005 fixes only the lifecycle/sizing contract.

### 5.3 Workspace entry contract

Every deterministic solve entrypoint (RFC 006) begins by invoking the equivalent of `workspace.reset_for_entry();` before reading scratch fields. The caller therefore never needs to manually clear or wipe the workspace between repeated solves.

## 6. Device Runtime Configuration

`loeres-device::config` owns execution-policy values as **runtime data**, not type-level const generics — this keeps policy out of solver identity and prevents monomorphization bloat.

```rust
#[non_exhaustive]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum TimingMode {
    EarlyExitAllowed,
    #[cfg(feature = "constant-iteration")]
    ConstantIteration,
}

pub struct DeviceSolveConfig<S> {
    pub max_iterations: u32,
    pub tolerance: S,
    pub timing_mode: TimingMode,
}
```

Const generics are reserved for memory dimensions; tolerance, iteration caps, and timing mode are runtime fields.

**Timing mode vs the `constant-iteration` feature (M5).** Both exist with distinct jobs: the `constant-iteration` *feature* controls whether constant-iteration support is compiled into `loeres-device`; the runtime `TimingMode` enum selects among the modes available in the compiled feature set. `EarlyExitAllowed` is always available; the `ConstantIteration` **variant is feature-gated**, so requesting constant-iteration without the feature fails at compile time. `TimingMode` is `#[non_exhaustive]`, so downstream `match`es must carry a wildcard arm and remain robust regardless of which features compiled the variant in. RFC 006 defines the timing-mode *driver behavior*; RFC 005 fixes only the policy data and this architecture rule.

**Structural config validation (M6).** RFC 005 owns *structural/feature* validation of `DeviceSolveConfig`; *solver-specific* validation (e.g. whether a particular kernel forbids zero tolerance) belongs to RFC 006. The RFC 005 check covers: `max_iterations > 0`; tolerance is finite (where `S: FiniteScalar`); and tolerance is not negative (where ordering is available). It returns a structured `SolverError`, never a panic. Because the `ConstantIteration` variant is feature-gated (M5), no runtime "timing mode supported" check is needed. RFC 005 validation rejects **negative or non-finite** tolerance; it does **not** reject zero tolerance — whether a concrete solver forbids zero is RFC 006's decision.

## 7. Poison-Free Failure Semantics

A workspace has three logical conditions; none of them blocks reuse:

| Condition | Meaning | Caller action required |
|---|---|---|
| Ready | Can be passed to a solver | None |
| Dirty | Holds arbitrary scratch data from a completed or failed run | None; the next solve resets required entry fields |
| Discarded | Caller stops using it | Drop or ignore |

The public API must not expose a poisoned state that blocks reuse. Any solver failure must leave the workspace memory safe-to-discard and immediately reusable by the next call.

**Zero-clearing elimination.** The API must not require a `clear_all_to_zero()`-style call between consecutive loop cycles. Full zeroing may exist as a debug/test helper, but never as part of the correctness contract.

## 8. Solve Entrypoint — RFC 006-Owned (non-normative sketch)

The deterministic solve entrypoint, its problem type `P`, its report, and its final scalar bounds are **owned by RFC 006**. The following is a **non-normative sketch** retained only to show how the §5/§6 contracts compose; its final shape is RFC 006's:

```rust
// RFC 006-owned final shape; non-normative here.
pub fn solve_device<P, W, S>(
    problem: &P,
    workspace: &mut W,
    config: &DeviceSolveConfig<S>,
) -> Result<DeviceSolveReport, SolverError>
where
    W: DeviceWorkspace,
    S: MetricScalar + FiniteScalar;
```

* `DeviceSolveReport` is **defined by RFC 006** and must implement RFC 014's `AsCoreReport` rule; the baseline carries or wraps the core `SolveReport`, with diagnostics retrieved from the workspace unless RFC 006 deliberately introduces a diagnostic-bearing report type (F3). RFC 005 does not define it.
* The bound `S: MetricScalar + FiniteScalar` is **illustrative only** (F8): it is not redundant — `MetricScalar` (supertrait `OrderedScalar`) does not imply `FiniteScalar` (supertrait `BaseScalar`), so both are meaningful — but the final first-kernel bounds are RFC 006's.
* The entrypoint must retain caller-owned `&mut` workspace semantics (§9).

## 9. Rust Systems-Level Nuances & Memory Safety

* **Unique mutable borrowing.** The workspace is passed by `&mut`, so the compiler guarantees exclusive access during execution; no shared mutable global state is needed.
* **No by-value workspace transfer.** Entrypoints must not take the workspace by value; moving large workspace structs risks stack pressure and accidental copies.
* **Dirty-but-valid memory.** After an error, scratch fields may hold arbitrary intermediate values. This is acceptable because every subsequent solve overwrites or logically initializes all fields before reading them (§5.3, §7).
* **No `unsafe`.** Baseline workspace mechanics require no `unsafe` in either crate. Both crates retain their `forbid`/no-`unsafe` posture. Any future optimized scratch view requiring unsafe aliasing must be isolated in a later RFC.

## 10. Feature Wiring

* **`owned-arrays` (F5).** Baseline lifecycle and config contracts in `loeres-device` exist without `owned-arrays`. Storage-block primitives (§4) and concrete first-kernel workspaces (RFC 006) that embed owned `FixedVector` / `FixedMatrix` require `loeres-backend-static/owned-arrays`; `loeres-device` forwards or enables that feature for those workspace implementations. The backend-static default feature posture (RFC 004) is unchanged — `owned-arrays` stays opt-in — and this RFC documents the wiring so the requirement is explicit rather than a compile-time surprise.
* **`diagnostic-snapshot` (F6).** The compact diagnostic accessor `DeviceWorkspaceDiagnostic::diagnostic()` and the core `DiagnosticSnapshot` type are **always available and ungated**. This feature governs only *richer or optional* device diagnostics (additional helpers, policy-gated collection beyond the compact baseline, optional diagnostic modules/sinks, and solver-specific metadata introduced by RFC 006). It must **not** gate the compact accessor or the `DiagnosticSnapshot` type.
* **`constant-iteration` (F7).** Controls whether constant-iteration support is compiled into `loeres-device`, as described in §6.

## 11. Algorithmic & Numerical Fail-Safe Guardrails

1. A bounded run that reaches the iteration cap without convergence returns an `Ok` report whose core projection is `SolveStatus::NotConverged` with `TerminationReason::IterationCap` (RFC 014). It leaves the workspace dirty but reusable; it is a terminal status, not a `SolverError`.
2. `SingularMatrix` leaves the workspace dirty but reusable.
3. `IllConditioned` leaves the workspace dirty but reusable.
4. `NumericalDomain` leaves the workspace dirty but reusable.
5. A panic must not be used to signal workspace misuse.
6. If a workspace type does not match the problem dimensions, this must be impossible by type construction or rejected before the solver loop begins.

The key rule: failure may invalidate mathematical contents, but it must not invalidate the workspace object as a reusable memory region.

## 12. Verification, Validation, and CI Gates

### 12.1 Reuse tests

For every device solver (RFC 006), tests must execute: (1) a valid solve using workspace `W`; (2) an adversarial solve returning an error; (3) a valid solve again on the same `W` without manual clearing. Step 3 must succeed or return only a problem-dependent error, never a workspace-poisoning error.

### 12.2 Zero-clearing audit

`xtask` must reject device solver implementations that require public `clear_all` / `zeroize` calls between normal loop cycles. Security wiping for sensitive data may exist in a future RFC, but not as a mathematical correctness requirement.

### 12.3 Size reporting

Each workspace type must expose `required_workspace_bytes()`, and CI must record representative sizes in build output.

### 12.4 Dependency and boundary gate

No workspace module in either crate may import `std`, `alloc`, or the cluster/backend-std crates. The zero-bleed gate additionally enforces the §3 boundary: `loeres-backend-static::workspace` must not reference device entrypoints, problem contracts, report types, or timing-mode behavior; `loeres-device::workspace`/`config` must not embed concrete solver workspaces or kernel logic (those are RFC 006).

### 12.5 Acceptance criteria

RFC 005 may move to `done/` only when:

1. the backend-static workspace storage-block layer is defined and `no_std`/no-`alloc`, composed from RFC 004 primitives;
2. device workspace lifecycle traits are defined, caller-owned, and passed by `&mut`;
3. failures leave the workspace immediately reusable, with no full-buffer zeroing in the correctness path;
4. runtime configuration (`DeviceSolveConfig`, `TimingMode`) is policy data, not const-generic solver identity, with the `constant-iteration` feature controlling compiled support;
5. the §3 crate boundary holds — no concrete solver workspace, problem, report, or solve body appears in RFC 005 surface (those are RFC 006);
6. `owned-arrays` feature wiring is explicit; the compact diagnostic accessor is an always-available extension trait (`DeviceWorkspaceDiagnostic`), with richer diagnostics feature-gated behind `diagnostic-snapshot`.
