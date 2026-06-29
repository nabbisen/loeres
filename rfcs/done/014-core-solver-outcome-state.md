# RFC 014 — Core Solver Outcome and Status Taxonomy

**Status.** Implemented (v0.5.0) — core `loeres::solver` taxonomy and `AsCoreReport` complete; the device (RFC 006) and cluster (RFC 008) report derivations land with those crates. Note: `loeres` package renamed to `loeres`, directory to `crates/loeres/` (v0.6.3).
**Tracks.** Phase 1 / Milestone 1 — Foundational Core Architecture (sequenced immediately after RFC 003)
**Touches.** `loeres/src/solver.rs`, `loeres/src/lib.rs`, the public solver-status namespace; reconciles `loeres/src/error.rs` (RFC 003), `loeres-device` report types (RFC 006), `loeres-cluster` report types (RFC 008), conformance status categories (RFC 013), and `xtask check-public-api` (RFC 010)

---

### Extended Metadata
* **Rust Edition Compliance:** Rust 2024 Baseline
* **Target Environment:** `loeres`; consumed by `loeres-device`, `loeres-cluster`, and host-side conformance tooling

## 1. Executive Summary & Problem Statement

The external design (§2.10) and the roadmap (RFC 1.3 scope) both call for a shared `loeres::solver` taxonomy. No RFC in the 001–013 set owns it. As a result [RFC 006](../proposed/006-deterministic-solver-kernel.md) §5 already references `Result<StepOutcome, SolverError>` with `StepOutcome` undefined, and separately invents a device-local `ConvergenceStatus`. Left unowned, every execution crate grows its own parallel outcome taxonomy — the same fragmentation that occurred with validation state across RFCs 007, 008, and 012.

This RFC defines the single core outcome/status taxonomy for `loeres` and the rule by which device and cluster report types derive from it. Its central principle is a clean status/error split:

> **Status** is expected, bounded solver progress and is returned in `Ok`. **Error** is a boundary-validation rejection or a fail-safe condition and is returned in `Err`. The same condition must never be representable as both.

The headline consequence is that **non-convergence at the configured iteration cap is a status, not an error**. `MaxIterationsReached` therefore moves out of `SolverError` (RFC 003) and becomes `SolveStatus::NotConverged` carried in an `Ok(SolveReport)`.

## 2. Architectural Context & Dependency Alignment

This RFC touches only `loeres`. It depends on [RFC 003](003-allocation-free-errors.md) for `SolverError` and `DiagnosticSnapshot`. It is consumed by [RFC 005](005-typed-workspace-mechanics.md), [RFC 006](../proposed/006-deterministic-solver-kernel.md), [RFC 008](../proposed/008-async-orchestration-budgets.md), and [RFC 013](../proposed/013-conformance-corpus-and-numerical-parity.md). Although numbered 014, it is implemented in Milestone 1 directly after RFC 003, because device and cluster solve entrypoints cannot be designed until the shared outcome vocabulary is frozen.

| Crate | Relationship to this RFC |
|---|---|
| `loeres` | Owns `loeres::solver` outcome/status types and the derivation trait |
| `loeres-backend-static` | Not affected; defines no solver outcomes |
| `loeres-device` | `DeviceSolveReport` must derive from the core `SolveReport` |
| `loeres-backend-std` | Not affected; defines no solver outcomes |
| `loeres-cluster` | Batch/solve report types must derive from the core `SolveReport` |

No `std`, no `alloc`, no backend type, and no scalar generic appears in these definitions. Every type in this RFC is `Copy` plain data.

## 3. Concrete Technical Specification

### 3.1 Module layout

```rust
pub mod solver;

pub use solver::{
    AsCoreReport,
    IterationReport,
    SolveReport,
    SolveStatus,
    StepOutcome,
    TerminationReason,
};
```

### 3.2 Step-level outcome

A single solver step returns `Result<StepOutcome, SolverError>`. A successful step yields a `StepOutcome`; a fail-safe condition discovered during the step yields `Err(SolverError)`. The driver loop — not the step — decides terminal solve status.

```rust
#[repr(u8)]
#[non_exhaustive]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum StepOutcome {
    /// The step made usable progress; the driver may continue iterating.
    Continue,
    /// The convergence criterion was satisfied at this step.
    Converged,
    /// The step produced no usable progress (below the configured progress floor).
    /// The driver decides whether this is terminal under the active timing policy.
    NoProgress,
}
```

A step never returns `MaxIterationsReached`; reaching the cap is a property of the bounded driver loop, not of any individual step.

### 3.3 Terminal status and termination reason

`SolveStatus` and `TerminationReason` are orthogonal in concept but constrained in combination. Status answers *did the solver meet its convergence criterion?* Termination reason answers *why did the bounded loop stop?* In constant-iteration mode these genuinely diverge: a solver may be `Converged` yet still terminate by `IterationCap` because it ran the full configured count after detecting convergence early.

```rust
#[repr(u8)]
#[non_exhaustive]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum SolveStatus {
    /// The convergence criterion was satisfied within the iteration cap.
    Converged,
    /// The solver terminated without meeting the convergence criterion.
    /// This is bounded, expected progress information — never an error.
    NotConverged,
}

#[repr(u8)]
#[non_exhaustive]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum TerminationReason {
    /// Stopped because the convergence criterion was met (early-exit mode).
    ConvergenceCriterion,
    /// Stopped because the configured iteration cap was reached.
    IterationCap,
    /// Stopped early because the solver detected no usable progress.
    NoProgress,
}

impl SolveStatus {
    #[inline]
    pub const fn is_converged(self) -> bool {
        matches!(self, SolveStatus::Converged)
    }
}
```

Only the following status/termination combinations are valid. The report constructors in §3.4 make all other combinations unconstructable.

| `SolveStatus` | Valid `TerminationReason` values |
|---|---|
| `Converged` | `ConvergenceCriterion` (early-exit), `IterationCap` (constant-iteration) |
| `NotConverged` | `IterationCap`, `NoProgress` |

`Converged + IterationCap` is valid only in constant-iteration mode, where convergence was detected before the final iteration but the solver intentionally ran to the cap. `Converged + NoProgress` and `NotConverged + ConvergenceCriterion` are invalid by construction.

### 3.4 Iteration and solve reports

The core report is **scalar-agnostic**: it carries no `S`-typed objective, residual, or solution value — those travel in caller-owned workspace or in a separate typed output. It is a single concrete `Copy` type of uniform size across every solver, with no monomorphization cost.

Report fields are **private**. `IterationReport` has a public constructor (any `(u32, TerminationReason)` pair is individually well-formed), while `SolveReport` is constructed only through named constructors that admit exactly the valid combinations in §3.3, so illegal status/termination pairings cannot be expressed.

```rust
#[repr(C)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct IterationReport {
    iterations_executed: u32,
    termination: TerminationReason,
}

impl IterationReport {
    #[inline]
    pub const fn new(iterations_executed: u32, termination: TerminationReason) -> Self {
        Self { iterations_executed, termination }
    }

    #[inline]
    pub const fn iterations_executed(&self) -> u32 { self.iterations_executed }

    #[inline]
    pub const fn termination(&self) -> TerminationReason { self.termination }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct SolveReport {
    status: SolveStatus,
    iteration: IterationReport,
}

impl SolveReport {
    /// Converged and stopped as soon as the criterion was met (early-exit).
    #[inline]
    pub const fn converged_early(iterations_executed: u32) -> Self {
        Self {
            status: SolveStatus::Converged,
            iteration: IterationReport::new(iterations_executed, TerminationReason::ConvergenceCriterion),
        }
    }

    /// Converged, but ran to the configured cap (constant-iteration).
    #[inline]
    pub const fn converged_at_cap(iterations_executed: u32) -> Self {
        Self {
            status: SolveStatus::Converged,
            iteration: IterationReport::new(iterations_executed, TerminationReason::IterationCap),
        }
    }

    /// Did not converge; the iteration cap was reached.
    #[inline]
    pub const fn not_converged_cap(iterations_executed: u32) -> Self {
        Self {
            status: SolveStatus::NotConverged,
            iteration: IterationReport::new(iterations_executed, TerminationReason::IterationCap),
        }
    }

    /// Did not converge; stopped early on no progress.
    #[inline]
    pub const fn not_converged_stalled(iterations_executed: u32) -> Self {
        Self {
            status: SolveStatus::NotConverged,
            iteration: IterationReport::new(iterations_executed, TerminationReason::NoProgress),
        }
    }

    #[inline]
    pub const fn status(&self) -> SolveStatus { self.status }

    #[inline]
    pub const fn iteration(&self) -> IterationReport { self.iteration }

    #[inline]
    pub const fn iterations_executed(&self) -> u32 { self.iteration.iterations_executed }

    #[inline]
    pub const fn termination(&self) -> TerminationReason { self.iteration.termination }
}
```

`iterations_executed` counts completed calls to the solver step function. For successful terminal reports it is normally positive, and may be zero only for a solver configuration that legitimately completes without entering the step loop; boundary-validation rejection is not represented by a zero-iteration report, since it returns `Err(SolverError)`. In constant-iteration mode it equals the configured cap unless a fail-safe `Err` aborts the run.

### 3.5 Driver-loop mapping

The mapping from per-step `StepOutcome` to a terminal `SolveReport` is normative, so that two solvers cannot interpret the same step outcome differently and both claim to implement RFC 014. Each row corresponds 1-to-1 with a `SolveReport` constructor from §3.4.

| Driver observation | Terminal mapping | Constructor |
|---|---|---|
| Step returns `Converged`, early-exit mode | `Converged` + `ConvergenceCriterion` | `converged_early` |
| Step returns `Converged`, constant-iteration mode | record convergence, continue to cap, then return | `converged_at_cap` |
| Cap reached without recorded convergence | `NotConverged` + `IterationCap` | `not_converged_cap` |
| `NoProgress` step stops the run early (early-exit policy) | `NotConverged` + `NoProgress` | `not_converged_stalled` |
| Step returns `Err(e)` | propagate `Err(e)` | — |

In constant-iteration mode a `NoProgress` step does not terminate the loop early; the loop always runs to the cap, so the only constant-iteration terminations are `IterationCap` (with `Converged` or `NotConverged`). `NoProgress` as a termination reason is therefore an early-exit-mode outcome only.

`converged_at_cap` assumes the caller is the bounded driver and has verified that `iterations_executed` equals the configured cap; the constructor cannot check this, so RFC 006 must test this mapping against the active timing mode. This is the unavoidable boundary between scalar-free core reporting and solver-specific configuration.

### 3.6 Diagnostics are not embedded in the core report

Consistent with external design §2.9 ("a public solve entrypoint should not force every result to carry a large diagnostic structure"), `SolveReport` deliberately excludes `DiagnosticSnapshot`. Compact diagnostics are retrieved separately — from the device workspace (`DeviceWorkspace::diagnostic()`, RFC 005) or attached by the cluster boundary — and only when a `DiagnosticPolicy` enables them. This keeps the mandatory core report small (see §4.1).

### 3.7 Canonical solve entrypoint shape

The core convention for any solve entrypoint is:

```rust
fn solve(/* problem, workspace, config */) -> Result<SolveReport, SolverError>;
```

- `Ok(report)` where `report.status() == SolveStatus::Converged` (e.g. `SolveReport::converged_early(iterations)`) — the solver reached the convergence criterion within the cap.
- `Ok(report)` where `report.status() == SolveStatus::NotConverged` (e.g. `SolveReport::not_converged_cap(iterations)`) — the solver ran to a bounded, well-defined terminus without converging. This is a successful, expected outcome.
- `Err(SolverError)` — a boundary-validation rejection or fail-safe condition prevented a meaningful result.

### 3.8 Derivation rule (anti-fragmentation)

Device and cluster crates may define their own report types, but those types must be losslessly projectable onto the core `SolveReport`, enforced through a trait:

```rust
pub trait AsCoreReport {
    fn as_core_report(&self) -> SolveReport;

    #[inline]
    fn core_status(&self) -> SolveStatus {
        self.as_core_report().status()
    }
}
```

The baseline device report carries no diagnostic field; the diagnostic is read from the workspace only when policy-enables it. RFC 006 owns the final device report shape, but every shape must implement `AsCoreReport`. If RFC 006 chooses to offer a diagnostic-bearing report, it is a separate, explicitly policy-gated type.

```rust
// loeres-device (baseline shape; RFC 006 owns the final form)
use loeres::solver::{AsCoreReport, SolveReport};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct DeviceSolveReport {
    core: SolveReport,
}

impl DeviceSolveReport {
    #[inline]
    pub const fn from_core(core: SolveReport) -> Self { Self { core } }

    #[inline]
    pub const fn core(&self) -> SolveReport { self.core }
}

impl AsCoreReport for DeviceSolveReport {
    #[inline]
    fn as_core_report(&self) -> SolveReport { self.core }
}
```

`loeres-cluster` likewise carries a `SolveReport` inside its per-item outcome (RFC 008 `BatchItemOutcome::Solved`) and adds host-side fields (timing, telemetry handles) outside the core type. No execution crate may expose a terminal status category that cannot round-trip through `SolveStatus`.

### 3.9 Optional error-on-nonconvergence policy

Some callers prefer non-convergence to surface as an `Err`. That preference is a **wrapper policy**, not the core baseline. A convenience helper (cluster-side, or a device config flag) may map an `Ok(report)` whose `status()` is `NotConverged` into a caller-chosen error, but the core entrypoint must return the status form. Such wrappers must use a wrapper-specific error type or policy result, not a new `SolverError` variant: the core `SolverError` enum must remain free of non-convergence categories.

## 4. Rust Systems-Level Nuances & Memory Safety

### 4.1 Size budget and representation

The data-free enums carry `#[repr(u8)]` so their layout is fixed at one byte across every target profile rather than depending on the compiler's default discriminant choice. The reports carry `#[repr(C)]` for internal layout predictability and size-budget testing only; this RFC does not define a stable C ABI for solver reports (the FFI gateway boundary is RFC 009's concern). Compile-time assertions remain the normative size gate:

```rust
const _: () = {
    assert!(core::mem::size_of::<StepOutcome>() <= 2);
    assert!(core::mem::size_of::<SolveStatus>() <= 2);
    assert!(core::mem::size_of::<TerminationReason>() <= 2);
    assert!(core::mem::size_of::<IterationReport>() <= 12);
    assert!(core::mem::size_of::<SolveReport>() <= 16);
};
```

Because `SolveReport` excludes `DiagnosticSnapshot`, it fits inside the same 16-byte ceiling used for `SolverError`. Derived reports (`DeviceSolveReport`, cluster reports) may exceed the core ceiling when they bundle the optional diagnostic or host-side fields; that is acceptable because they are returned once per solve as the `T` of a `Result`, not as the per-step or per-element `E`.

### 4.2 Static dispatch only

`AsCoreReport` is a projection trait for static dispatch and tests only. It must not appear as `dyn AsCoreReport` in any `loeres`, `loeres-device`, or `loeres-backend-static` public signature; `xtask check-public-api` (RFC 010) must reject that form in edge-facing APIs. Cluster orchestration may box derived reports as allowed by RFC 008. Every other type here is `Copy` plain data with no references, allocation, or trait objects.

### 4.3 Semver extensibility

`StepOutcome`, `SolveStatus`, and `TerminationReason` are `#[non_exhaustive]`, matching the RFC 003 policy, so future solver families can add categories without a breaking change.

`IterationReport` and `SolveReport` are introduced with their semver strategy fixed from v0.x: both have private fields with public `const` constructors and accessors. External code cannot construct them by struct literal, so adding a private field in a later version is not a breaking change, subject to the size-budget assertions in §4.1. `SolveReport` admits only the valid status/termination combinations of §3.3 through its named constructors; no public path can build an invalid combination. Adding public fields to these structs is not a compatible change and is forbidden.

### 4.4 No `unsafe`

This RFC introduces no `unsafe`. The taxonomy is plain data.

## 5. Algorithmic & Numerical Fail-Safe Guardrails

### 5.1 Status/error classification

| Condition | Classification | Representation |
|---|---|---|
| Convergence criterion satisfied within cap | status | `Ok(SolveReport::converged_early \| converged_at_cap)` |
| Iteration cap reached without convergence | status | `Ok(SolveReport::not_converged_cap)` |
| No usable progress before cap | status | `Ok(SolveReport::not_converged_stalled)` |
| Non-finite public input | error | `Err(NonFiniteInput)` |
| Incompatible shapes / illegal dimension | error | `Err(DimensionMismatch)` / `Err(InvalidDimension)` |
| Input violates declared domain or configuration contract | error | `Err(InvalidInput)` |
| Valid problem unsupported by the selected solver | error | `Err(UnsupportedProblemStructure)` |
| Singular or ill-conditioned system detected | error | `Err(SingularMatrix)` / `Err(IllConditioned)` |
| Numerical-domain violation (e.g. checked-division failure) | error | `Err(NumericalDomain)` |
| Checked-arithmetic overflow | error | `Err(Overflow)` |
| Workspace cannot hold required scratch state | error | `Err(WorkspaceTooSmall)` |
| Cancellation observed | error | `Err(Cancelled)` |
| Optional backend unavailable | error | `Err(BackendUnavailable)` |
| Internal invariant violated | error | `Err(InternalInvariantViolation)` |

Cancellation is classified as a fail-safe interruption (no meaningful terminal result), so it remains `SolverError::Cancelled` at the solve layer and becomes `BatchItemOutcome::Cancelled` at the batch layer (RFC 008). This is why `Cancelled` and `BackendUnavailable` must be retained in the RFC 003 error set rather than dropped.

### 5.2 Mapping from the external-design status categories

External design §2.10 listed five status categories; under the split they resolve as: `Converged` → `SolveStatus::Converged`; `Continue` → `StepOutcome::Continue` (step-level, not terminal); `MaxIterationsReached` → `SolveStatus::NotConverged` + `TerminationReason::IterationCap`; `RejectedInput` → the specific boundary `SolverError` variant; `Failed` → the specific fail-safe `SolverError` variant.

### 5.3 Cross-RFC reconciliation required by this RFC

Adopting this taxonomy requires the following edits in sibling RFCs, applied in the same patch round.

**RFC 003.** `MaxIterationsReached` is removed (non-convergence is no longer an error). `PanicGateViolation` is removed from the runtime error set and owned instead by RFC 010 as a CI/release-gate outcome (there is no normal runtime solve condition that produces it). The canonical `SolverError` set becomes exactly:

- `DimensionMismatch { lhs: u32, rhs: u32 }`
- `InvalidDimension`
- `InvalidInput`
- `NonFiniteInput`
- `UnsupportedProblemStructure`
- `SingularMatrix`
- `IllConditioned`
- `NumericalDomain`
- `Overflow`
- `WorkspaceTooSmall`
- `Cancelled`
- `BackendUnavailable`
- `InternalInvariantViolation`

**RFC 006.** Replace the standalone `ConvergenceStatus` with `DeviceSolveReport` embedding `core: SolveReport` and implementing `AsCoreReport`; the `Result<StepOutcome, SolverError>` step signature it already references is satisfied by §3.2, and its driver follows the mapping in §3.5.

**RFC 008.** `BatchItemOutcome::Solved` carries a report implementing `AsCoreReport`; cancellation and backend-unavailable remain fail-safe outcomes consistent with §5.1.

**RFC 013.** The `status_match` comparison compares core `SolveStatus`; conditioning-stratified fixtures (ill-conditioned / merely-convex boxes) expect `Ok(NotConverged)`, never an error.

## 6. Verification, Validation, and CI Gates

### 6.1 Size, representation, and no-std gates

CI must compile the §4.1 assertions on every supported target profile and confirm `loeres::solver` builds under `#![no_std]` without `alloc`. A test must confirm each data-free enum is one byte under `#[repr(u8)]`.

### 6.2 Invalid-combination gate

A test must enumerate the four valid `(SolveStatus, TerminationReason)` pairs produced by the §3.4 constructors and confirm there is no public constructor for `Converged + NoProgress` or `NotConverged + ConvergenceCriterion`.

### 6.3 Status/error split gate

`xtask check-public-api` (RFC 010) must reject any `SolverError` variant denoting non-convergence at the iteration cap, must reject any device-facing report type exposing a terminal status category not derivable from `SolveStatus`, and must reject `dyn AsCoreReport` in edge-facing public signatures.

### 6.4 Derivation round-trip tests

For each device and cluster report type, tests must prove `AsCoreReport::as_core_report` is lossless for status and termination (construct each valid pair, wrap it, project it, assert equality).

### 6.5 Driver-mapping conformance

RFC 006's kernel must be tested in both timing modes against §3.5: early-exit convergence yields `converged_early`; constant-iteration convergence yields `converged_at_cap`; cap-without-convergence yields `not_converged_cap`; early stall yields `not_converged_stalled`. The test must additionally assert that `converged_at_cap(n)` is constructed only when `n == config.max_iterations`.

### 6.6 Reconciliation tests

A test must assert that `SolverError` (post-RFC-003 patch) contains no non-convergence variant and no `PanicGateViolation`, and that a max-iteration run returns `Ok(SolveReport::not_converged_cap(..))` rather than `Err`.

### 6.7 Implementation sprint plan

| Sprint | Work |
|---|---|
| S0 | Freeze status/error split, taxonomy names, valid-combination matrix, and size budgets. |
| S1 | Add `loeres::solver` module skeleton, enums with `#[repr(u8)]`, and `AsCoreReport`. |
| S2 | Add size/representation assertions, invalid-combination tests, and split-classification compile tests. |
| S3 | Implement the report constructors/accessors and the trait. |
| S4 | Run no-std, no-alloc, size-budget, and `check-public-api` checks. |
| S5 | Wire RFC 006 device-report derivation and the §3.5 driver mapping; document cross-RFC reconciliation. |
| S6 | Close RFC with the status/error taxonomy checklist. |

### 6.8 Acceptance criteria

RFC 014 may move to `done/` only when:

1. `loeres::solver` defines `StepOutcome`, `SolveStatus`, `TerminationReason`, `IterationReport`, `SolveReport`, and `AsCoreReport`;
2. all core outcome types satisfy the §4.1 size and representation budgets under `no_std`;
3. `SolveReport` exposes only the four valid status/termination combinations through named constructors, verified by §6.2;
4. `MaxIterationsReached` and `PanicGateViolation` have been removed from `SolverError`, and non-convergence is reported as a status;
5. RFC 006's `DeviceSolveReport` derives from `SolveReport` via `AsCoreReport` with a passing round-trip test and follows the §3.5 mapping;
6. no device-facing report exposes a status category outside the core taxonomy, and `dyn AsCoreReport` is absent from edge-facing public signatures;
7. RFCs 003, 006, 008, and 013 reference the canonical types and spellings introduced here.
