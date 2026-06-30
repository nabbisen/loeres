# RFC 016 — Std-Side Projected First-Order Cluster Kernel

**Status.** Done — the first production std-side numerical kernel for `loeres-cluster`: a dynamic box/bound-constrained projected first-order solver over `loeres-backend-std` dynamic storage, the dynamic-storage analog of the RFC 006 device kernel, plugged into the RFC 008 `ClusterJob` seam via a typed model and solve entrypoint. Graduates cluster orchestration from deterministic test jobs to real solving. **Done (v0.14.0).** Design pass (D1–D8, F1–F8), reconciliation (C1–C5), and the implementation-decision pass (I1–I10) are complete; implemented in `loeres-cluster` (`model` + `solve::projected_first_order`) and shipped in v0.14.0. See §7 for the settled implementation decisions and the two evidence-integrity refinements surfaced during coding.
**Tracks.** Phase 3 / Milestone 3 — Dynamic Infrastructure and Cloud Cluster
**Touches.** `loeres-cluster/src/model.rs` (new — typed dynamic projected-first-order problem contract + reusable workspace), `loeres-cluster/src/solve.rs` (typed solve entrypoint + `ClusterJob` adapter; existing batch entrypoints unchanged), `loeres-cluster/src/lib.rs` (re-exports). `batch` and `runtime` are **consumed unchanged** — RFC 016 adds no new orchestration contract.

### Extended Metadata
* **Rust Edition Compliance:** Rust 2024 Baseline
* **Target Environment:** `loeres-cluster` (`std`); depends on `loeres` and `loeres-backend-std`; consumes the RFC 008 orchestration seam and the RFC 012 validation vocabulary. Cluster-only — must remain unreachable from any `no_std`/edge crate (zero-bleed).

## 1. Executive Summary & Problem Statement

RFC 008 shipped cluster orchestration as infrastructure — the per-item batch contract, cancellation/timeout layering, runtime-agnostic configuration, and the `ClusterJob` dispatch seam — but validated it only against deterministic in-crate **test jobs**. No production kernel exists: core (`loeres`) exposes only the RFC 014 outcome vocabulary, the only real solver is the edge-only RFC 006 device kernel (unreachable from cluster by the zero-bleed rule), and `loeres-backend-std` is storage-only.

This RFC delivers the first production kernel: a **dynamic box/bound-constrained projected first-order** solver over `DenseVector` storage. It deliberately mirrors the RFC 006 device kernel's solver family and convergence model so the first std-side numerics stay small and reviewable, while operating on runtime-sized dynamic storage and integrating with cluster orchestration. It exposes a **typed model and solve entrypoint** (mutating the iterate in place and returning a typed record), then a **thin `ClusterJob` adapter** for batch orchestration, and is the **first real `ClusterValidationPolicy::ValidateAllInputs` scan path**.

## 2. Architectural Context & Dependency Alignment

**Depends on (all shipped):** `loeres-backend-std` dynamic storage (RFC 007 — `DenseVector`); the RFC 014 outcome vocabulary (`SolveReport` / `SolveStatus`, `is_converged()` on `SolveStatus`); `SolverError` (RFC 003); the RFC 008 orchestration seam (`ClusterJob`, `ClusterExecutionContext`, `ClusterSolution`, `BatchItemOutcome`, `ClusterValidationPolicy`); the RFC 012 validation vocabulary (`ValidationCoverage`, `ValidationState`, `ValidationScope`).

**Independent of:** trusted-pipeline / validation caching / model identity / mutation epochs (RFC 015 — which will *cache* the coverage this kernel records); the size-budget gate (RFC 010); observability / FFI gateway (RFC 009).

**Boundary commitments.**
* **Zero-bleed.** The kernel is cluster-only. No edge or `no_std` crate gains a path to it; it must not be reachable from `loeres-device`.
* **No generic core solver (D2).** RFC 016 follows the RFC 006 projected-first-order *contract* where applicable but introduces **no** generic solver abstraction in `loeres`. Core has no generic solver surface today; adding one would broaden this RFC into a core/kernel refactor. Shared semantics are by contract, not shared code. A future RFC may extract common iteration semantics only if the device and std paths demonstrate stable overlap.
* **Outcome split preserved.** Non-convergence at the iteration cap is a `SolveStatus::NotConverged` returned in `Ok` (a `Solved` batch outcome), never a `SolverError` in `Err`.

## 3. Concrete Technical Specification

### 3.1 Solver scope (D1)

A **dynamic box/bound-constrained projected first-order method** over runtime-sized primal vectors: a bounded loop of gradient-driven steps, each projected onto `[lo, hi]`. Aligned with RFC 006 §3.1, dynamic storage.

Explicitly **excluded** (each a later RFC): dense/sparse convex QP, KKT / active-set, interior-point; ADMM and operator-splitting; mixed-integer programming, branch-and-bound; nonlinear adapters; native / gateway-backed solvers (RFC 009); modeling DSLs; sparse dynamic assembly inside the kernel; unbounded adaptive loops.

### 3.2 Typed problem contract (`loeres_cluster::model`, D3)

A first-order oracle over box bounds, dynamic storage — the dynamic analog of RFC 006 §3.2. Design sketch:

```rust
pub trait ClusterProjectedFirstOrderProblem<S>
where
    S: FiniteScalar + MetricScalar,
{
    /// Number of primal variables (runtime, not const-generic).
    fn dimension(&self) -> usize;

    /// Lower/upper box bounds for projection (dynamic vectors).
    fn bounds(&self) -> (&DenseVector<S>, &DenseVector<S>);

    /// Gradient `∇f(x)` written into `grad` — the hot-loop oracle.
    fn gradient_at(&self, x: &DenseVector<S>, grad: &mut DenseVector<S>) -> Result<(), SolverError>;

    /// Problem-provided step scale `α` (validated finite, `> 0` before the loop).
    fn step_scale(&self) -> S;

    /// Optional problem-specific pre-loop validation hook, run *in addition to*
    /// the kernel-enforced universal checks of §3.5 — NOT the owner of
    /// dimension/bounds safety. Default: `Ok(())`.
    fn validate_boundary(&self) -> Result<(), SolverError> {
        Ok(())
    }
}
```

**Scalar bound (F8).** The minimum bound is **`S: FiniteScalar + MetricScalar`**, stated on the trait: `MetricScalar` extends `OrderedScalar` (supplying `clamp` for projection and the `|·|` / comparison used by the convergence test), and `FiniteScalar` supplies the per-element finiteness predicate. No `#[inline]` on bodiless declarations; implementors place `#[inline]` on method bodies (RFC 006 F5).

**No `objective_at` (F5).** v1 convergence is step-norm based (§3.4 D8) and neither `SolveReport` nor `BatchItemOutcome` carries an objective value, so an `objective_at` method would be an unused obligation that could also turn a valid solve into `Failed`. Objective reporting waits for an actual reporting field / diagnostic policy and a later optional extension trait.

**Reference problem (D4).** v1 ships **no public modeling helper** — only this trait, plus deterministic in-crate test/reference problems and documentation examples. A public quadratic-box problem or builder is a later ergonomic RFC.

### 3.3 Workspace model (D5)

The explicit reusable workspace is **`ClusterProjectedFirstOrderWorkspace<S>`**, holding a **single** heap gradient-scratch vector (a `DenseVector<S>` sized to `dimension`). The candidate iterate is computed with scalar temporaries and written into `x` in place, so no second `next` vector and no per-iteration allocation are needed (C4). Frozen API:

```rust
impl<S: FiniteScalar + MetricScalar> ClusterProjectedFirstOrderWorkspace<S> {
    /// Allocate scratch sized to `dimension`; `dimension == 0` is rejected (InvalidDimension).
    pub fn new(dimension: usize) -> Result<Self, SolverError>;
    /// The dimension this workspace was sized for.
    pub fn dimension(&self) -> usize;
    /// Reset scratch for a fresh solve (overwrite-on-use; no allocation).
    pub fn reset_for_entry(&mut self);
}
```

Scratch is allocated once at construction and reused via `reset_for_entry()`; **the iteration loop allocates nothing.** This reusable workspace serves the **typed API** (§3.4), for callers controlling allocation across repeated solves. The **erased `ClusterJob` adapter** does *not* hold a shared mutable workspace (§3.6, F1): it allocates a fresh local workspace once per job run, before the loop. The hybrid is thus "reusable for the typed path, local-per-run for the erased path" — neither path allocates per iteration.

### 3.4 Typed solve entrypoint (`loeres_cluster::solve`)

The dynamic analog of RFC 006 §3.3 (no const `N`; runtime dimension match). The entrypoint mutates the iterate in place and returns a small typed record (F2) so the validation outcome is surfaced for RFC 015 without changing `SolveReport` or `BatchItemOutcome`:

```rust
pub struct ProjectedFirstOrderSolveRecord {
    pub report: SolveReport,
    /// What the kernel actually checked this run (always includes the structural
    /// PROBLEM_CONFIG scope; includes FINITE when scanned).
    pub checked: ValidationCoverage,
    /// Caller responsibility transfer, if any (FINITE scope under TrustedByCaller,
    /// or a provided Trusted state carried forward). RFC 012 vocabulary; no
    /// parallel trust model. RFC 015 decides later what is cacheable.
    pub trust: Option<TrustedByCaller>,
}

pub fn solve_projected_first_order_dyn<P, S>(
    problem: &P,
    x: &mut DenseVector<S>,                                  // explicit in/out iterate
    workspace: &mut ClusterProjectedFirstOrderWorkspace<S>,  // gradient scratch (reused)
    config: &ProjectedFirstOrderConfig<S>,                   // kernel numerics: max_iterations, tolerance
    ctx: &ClusterExecutionContext,                           // RFC 008: cancellation polling + validation policy
) -> Result<ProjectedFirstOrderSolveRecord, SolverError>
where
    P: ClusterProjectedFirstOrderProblem<S>,
    S: FiniteScalar + MetricScalar,
{
    // 1. if ctx.is_cancelled() { return Err(Cancelled) }                 (D7: before scans)
    // 2. config.validate()                                               (D6)
    // 3. workspace.reset_for_entry()
    // 4. kernel-enforced universal/structural checks (§3.5(a), F3):
    //      dimension > 0; x/workspace/lo/hi lengths == dimension;
    //      lo <= hi elementwise; step_scale finite & > 0
    // 5. policy-governed finite scans (§3.5(b)) -> (checked: ValidationCoverage, trust: Option<TrustedByCaller>)
    // 6. problem.validate_boundary()                                     (problem-specific hook)
    // 7. executed = 0
    //    while executed < config.max_iterations {
    //        if executed % ctx.poll_interval() == 0 && ctx.is_cancelled() { return Err(Cancelled) }
    //        problem.gradient_at(x, grad)?;                  // grad in the SINGLE scratch vector
    //        if grad has any non-finite (FiniteScalar) { return Err(NumericalDomain) }    (F4)
    //        step = 0
    //        for i in 0..n {                                  // scalar temporaries; NO per-iter vector alloc (C4)
    //            cand = clamp(x[i] - α·grad[i], lo[i], hi[i]);
    //            if cand non-finite { return Err(NumericalDomain) }                        (F4)
    //            step = max(step, |cand - x[i]|);             // D8 max-coordinate step
    //            x[i] = cand;                                  // in place
    //        }
    //        executed += 1;
    //        if step <= config.tolerance { return Ok({ converged_early(executed), checked, trust }) }  (C5)
    //    }
    // 8. return Ok({ not_converged_cap(config.max_iterations), checked, trust })                       (C5)
}
```

**Step rule (mirrors RFC 006 F7).** `x_{k+1} = clamp(x_k − α·∇f(x_k), lo, hi)` using `BaseScalar::{sub, mul}` and `OrderedScalar::clamp` (via `MetricScalar`). The kernel performs **no division** (`α` is problem-provided, validated finite and strictly positive), so the bound stays **`S: FiniteScalar + MetricScalar`**; `DivisibleScalar` is not required.

**Convergence (D8).** Maximum coordinate step size: `max_i |x_next[i] − x[i]| <= tolerance`, using `MetricScalar` for `|·|` and the comparison. Projected-gradient-norm and objective-decrease criteria are **not** used in v1.

**Numeric config (D6).** `ProjectedFirstOrderConfig<S> { max_iterations: u32, tolerance: S }`, validated by `validate()`:
* `max_iterations == 0` → `SolverError::InvalidInput`;
* `tolerance` non-finite → `NonFiniteInput`; `tolerance <= 0` → `InvalidInput`.
RFC 008's orchestration `ClusterSolveConfig` is **not** reused for numeric fields; the two configs are orthogonal (orchestration flows through `solve_batch`, numeric config is per-kernel).

**Cancellation (D7).** Checked once **before** the validation scans (cheap exit), then every `ctx.poll_interval()` iterations; on observed cancellation the entrypoint returns `Err(SolverError::Cancelled)`, which rides the RFC 008 **F9 normalization** (`Failed { Cancelled }` → `BatchItemOutcome::Cancelled`) — no new cancellation contract.

**Iteration counting & report mapping (C5).** Mirrors RFC 006's early-exit path exactly. `executed` counts completed projected steps, incremented *after* each step (including the converging one). Convergence (`step <= tolerance`) returns `SolveReport::converged_early(executed)` immediately — first-step convergence is `converged_early(1)`, and `max_iterations == 1` with convergence on that step is also `converged_early(1)` (**not** `converged_at_cap`). Reaching the cap without convergence returns `not_converged_cap(max_iterations)`. v1 has no constant-iteration mode (so `converged_at_cap` is unused) and introduces no stall detector (so `not_converged_stalled` is unused); both remain available to later modes.

### 3.5 Validation scan path (first real `ValidateAllInputs`)

RFC 016 is the first place `ClusterValidationPolicy` drives **real scans** (RFC 008 made `resolve()` *pure* in B1 — it reasons over recorded evidence and never fabricates). Validation has two layers.

**(a) Kernel-enforced universal/structural checks — always run, never skippable (F3, F6).** Independent of the policy, the kernel/job-prep layer checks at the solver boundary:
* `problem.dimension() > 0`;
* `x.len()`, `workspace.dimension()`, `lo.len()`, `hi.len()` all equal `problem.dimension()`;
* `lo <= hi` elementwise;
* `step_scale` finite and strictly positive;
* numeric config valid (§3.4 D6).

These are **structural** (`PROBLEM_CONFIG` scope) and run even under `TrustedByCaller` — trust never licenses a dimension/config mismatch, and (until RFC 015 supplies model identity + epochs) a detached or cached `ValidationState` is **never** treated as proof that the current mutable `x` / workspace / config still agree.

**(b) Finite-value scans — policy-governed (`FINITE` scope).** The pre-loop finiteness scans of bounds and the initial iterate are the part a caller may legitimately skip:
* **`ValidateAllInputs`** — run `lo` / `hi` / initial `x` finiteness scans; record `checked = ValidationCoverage::new(PROBLEM_CONFIG ∪ FINITE, Checked)`, `trust = None`.
* **`RespectBackendValidationState`** — consume the provided `ValidationState`; the **required coverage** for this kernel is `FINITE` (bounds + initial-iterate finiteness only — `step_scale` finiteness is validated structurally under `PROBLEM_CONFIG` and is never skippable, C2) plus `PROBLEM_CONFIG` (the structural checks of (a), which always run regardless); perform any required-but-missing `FINITE` scan here (never fabricate); reject only if a required scan is impossible. `checked` records the structural coverage plus any verified or freshly-scanned `FINITE` coverage; a provided `Trusted(..)` assertion is carried forward into `trust`.
* **`TrustedByCaller`** — skip the `FINITE` scans for the asserted scope, recording `checked = ValidationCoverage::new(PROBLEM_CONFIG, Checked)` and `trust = Some(..)`; the structural checks of (a) still run, and **runtime numerical-domain failures still surface** in the loop (§3.4 F4) as `NumericalDomain` — trust must never yield a `Solved` report over NaN/Inf state.

`PRELOOP` is **not** used as a separate scope here (it would be ambiguous against `PROBLEM_CONFIG`). The kernel-checked coverage and any caller trust transfer are returned in `ProjectedFirstOrderSolveRecord.{checked, trust}` (§3.4, C1) — the concrete surfacing channel RFC 015 will cache against model identity. A single `ValidationState` is deliberately not used, because one run can mix kernel-checked `PROBLEM_CONFIG` with caller-trusted `FINITE`.

### 3.6 Outcome mapping & the `ClusterJob` adapter

The typed entrypoint returns `Result<ProjectedFirstOrderSolveRecord, SolverError>` and mutates `x` in place: **converged and not-converged are both `Ok`** (RFC 014 ctors `converged_early` / `converged_at_cap` / `not_converged_cap` / `not_converged_stalled`); fail-safe failures are `Err`.

**Adapter ownership (F1).** RFC 008's seam is `run_boxed(&self, &ClusterExecutionContext) -> BatchItemOutcome<S>` with `ClusterJob<S>: Send + Sync` — `&self` cannot drive a mutable iterate or a reusable workspace without interior mutability. RFC 016 does **not** change that seam. The adapter `ClusterProjectedFirstOrderJob<P, S>` is therefore a **template** holding only immutable inputs (`problem: P`, `config: ProjectedFirstOrderConfig<S>`, and a starting iterate `initial: DenseVector<S>`); each `run_boxed` call allocates a **local** `x = initial.clone()` and a **local** `ClusterProjectedFirstOrderWorkspace::new(problem.dimension())` once, before the loop — no per-iteration allocation, and no shared mutable `&self` state. The reusable workspace of §3.3 remains available through the typed entrypoint for callers needing allocation control across repeated solves; it is deliberately not used by the erased path.

`run_boxed` maps:
* `Ok(record)` → `BatchItemOutcome::Solved { solution: ClusterSolution::DenseVector(x), report: record.report }` (`record.checked` / `record.trust` are dropped for the RFC 008 batch outcome; the typed surface retains them for RFC 015);
* `Err(SolverError::Cancelled)` → `Failed { Cancelled }` → (RFC 008 F9) `Cancelled`;
* other `Err(e)` → `Failed { error: e }`.

Panics remain contained by the RFC 008 executor (`Panicked`, under `panic = "unwind"` only). The typed surface is primary; the adapter is the only place model invariants meet the erased boundary, giving RFC 015 a concrete typed model to attach identity/epochs to rather than an opaque job.

### 3.7 Error mapping (F7)

Frozen, using existing `SolverError` variants only:

| Condition | `SolverError` |
|---|---|
| zero dimension / zero workspace / zero vector length | `InvalidDimension` |
| dimension disagreement (`x` / workspace / bounds vs. `problem.dimension()`) | `DimensionMismatch { lhs, rhs }`, with checked `usize → u32`; on cast overflow fall back to `InvalidDimension` |
| invalid box bounds (`lo > hi`, finite) | `InvalidInput` |
| non-finite pre-loop input (bounds / initial `x` / `step_scale`) under a scanning policy | `NonFiniteInput` |
| `max_iterations == 0` | `InvalidInput` |
| non-positive `tolerance` or non-positive `step_scale` | `InvalidInput` |
| non-finite gradient or non-finite candidate produced **during** iteration | `NumericalDomain` |
| cancellation observed | `Cancelled` |
| unsupported problem feature | `UnsupportedProblemStructure` |

**Non-finite bounds (C3).** Under scanning policies, non-finite `lo`/`hi` are caught by the `FINITE` scan (→ `NonFiniteInput`) before the structural order check classifies anything. Under `TrustedByCaller` (FINITE skipped), the `lo <= hi` check classifies only *finite*-bound ordering violations as `InvalidInput`; any non-finite bound propagates through projection and is caught by the hot-loop candidate check (→ `NumericalDomain`, §3.4 F4), so a non-finite bound never yields `Solved`.

### 3.8 Determinism

Deterministic by design: **sequential per-job kernel math**. Cluster-level Rayon (`parallel-rayon`) parallelizes *independent jobs*, never the per-job numerical order. No promise of bitwise identity across CPU architectures, compiler flags, or libm implementations unless a later target-profile RFC (RFC 011) makes that commitment — this is a server baseline, not device-grade target determinism.

## 4. Rust Systems-Level Nuances & Memory Safety

### 4.1 Monomorphization
The typed kernel is generic over `P, S`; the `ClusterJob<S>` erasure (RFC 008's dispatch barrier) bounds bloat at the batch boundary. The per-problem-type kernel instantiation is exactly the monomorphization pressure the RFC 010 size-budget gate will later measure; RFC 016 names it but adds no gate.

### 4.2 Send / Sync and scalar bounds (F8)
The typed entrypoint requires `P: ClusterProjectedFirstOrderProblem<S>`, `S: FiniteScalar + MetricScalar`. The erased adapter stores the problem behind `ClusterJob<S>: Send + Sync`, so it additionally requires `P: Send + Sync + 'static` and `S: FiniteScalar + MetricScalar + Send + Sync + 'static` (the `parallel-rayon` path also needs `S: Send`, consistent with RFC 008's `solve_batch<S: Send>`). Cloning the `initial` iterate per run requires `S: Clone` (`DenseVector<S>: Clone`). These bounds are written into the RFC so they do not emerge accidentally from compiler errors during implementation.

### 4.3 Allocation discipline
Two allocation profiles, neither with hot-loop churn: the **typed path** allocates the reusable workspace once and reuses it across solves; the **erased adapter path** allocates a local iterate clone and a local workspace once per `run_boxed`. In both, the iteration loop allocates nothing (a single gradient-scratch vector; the candidate is an in-place scalar-temporary update, C4); setup, final-storage (returned iterate), and scratch allocations are documented.

### 4.4 No `unsafe`
The crate's `#![forbid(unsafe_code)]` posture holds; only safe `DenseVector` access is used. `loeres-cluster` is `std`, so the `panic-audit` gate (no_std crates only) does not scan it; nonetheless the solve path is fail-safe via `Result` and contains no `unwrap`/`expect`/indexing-panic by convention.

## 5. Algorithmic & Numerical Fail-Safe Guardrails

* **Pre-loop (structural, always):** `dimension > 0`; `x` / workspace / `lo` / `hi` lengths equal `dimension`; `lo <= hi` elementwise; `step_scale` finite and `> 0`; numeric config valid.
* **Pre-loop (finite scans, policy-governed):** `lo` / `hi` / initial `x` finiteness — run under `ValidateAllInputs` and required-but-missing `RespectBackendValidationState`, skippable under `TrustedByCaller` (§3.5). Pre-loop non-finite user data maps to `NonFiniteInput`.
* **Hot-loop (always, F4):** after `gradient_at`, the gradient is checked for non-finite values before use; each updated candidate is checked for finiteness before convergence reporting. Non-finite oracle output or arithmetic-domain failure discovered during iteration maps to `NumericalDomain`. Trust (`TrustedByCaller`) skips *pre-loop scans only* — it never suppresses these hot-loop checks, so a `Solved` report is never returned over NaN/Inf state.
* **Projection:** every step projects onto `[lo, hi]` via NaN-propagating `clamp` (`OrderedScalar` through `MetricScalar`).
* **Convergence (D8):** `max_i |x_next[i] − x[i]| <= tolerance`. **`NotConverged` at the cap is a status** (`SolveStatus::NotConverged`) in `Ok`, never `Err`.
* No division in the baseline step ⇒ no division-by-zero path.

## 6. Verification, Validation, and CI Gates

### 6.1 Numerical correctness
Unlike RFC 008 (orchestration-only), RFC 016 has real math: convergence to known optima on simple convex box-constrained problems (e.g. a separable quadratic with a box-clamped optimum); projection correctness at and across bounds; step-rule behavior; gradient-oracle wiring; the max-coordinate-step convergence test.

### 6.2 Status / error split
Not-converged-at-cap returns `Solved { NotConverged }` (never `Failed`); fail-safe errors return `Failed`; observed cancellation returns `Cancelled`; a problem whose oracle yields a non-finite gradient returns `Failed { NumericalDomain }` **even under `TrustedByCaller`** (hot-loop checks are not skippable); `lo > hi` returns `Failed { InvalidInput }`; iteration counting follows §3.4 (C5) — first-step or `max_iterations == 1` convergence is `converged_early(1)`, cap exhaustion is `not_converged_cap(max_iterations)`.

### 6.3 Validation-policy scans
`ValidateAllInputs` returns `ValidationState::Validated(..)` only after real scans; `RespectBackendValidationState` performs missing `FINITE` scans without fabricating; `TrustedByCaller` returns `Trusted(..)`, the structural checks of §3.5(a) still run, and hot-loop non-finite detection still surfaces `NumericalDomain`. The returned `ProjectedFirstOrderSolveRecord.{checked, trust}` carries the outcome; `resolve()` stays pure.

### 6.4 Orchestration integration
The `ClusterJob` adapter runs through `solve_batch` (sequential and `parallel-rayon` parity): mixed converged / not-converged / failed / cancelled batches yield the correct `BatchSummary`; mid-solve cancellation surfaces as `Cancelled`; the adapter allocates per run with no shared `&self` mutation.

### 6.5 Determinism
Repeated solves of the same problem yield identical iterates/reports on one target; parallel batch ordering does not perturb per-job results.

### 6.6 Dependency / zero-bleed
The kernel is cluster-only; no edge crate gains a path to it; `release-gate` zero-bleed and no-std (`thumbv7em-none-eabihf`) stay green; files ≤ 300 ELOC.

### 6.7 Acceptance criteria
1. A typed dynamic projected-first-order model (`loeres_cluster::model`), a typed solve entrypoint returning `ProjectedFirstOrderSolveRecord`, and a template `ClusterJob` adapter exist; the typed surface is primary and the adapter thin and `&self`-safe (local per-run allocation).
2. `solve_batch` runs real kernel jobs end-to-end with the status/error split preserved.
3. `ValidateAllInputs` is a real scan path recording `checked` / `trust` coverage in the record; the structural checks of §3.5(a) always run; `resolve()` remains pure.
4. No generic core solver is added to `loeres`.
5. The frozen error mapping (§3.7) and hot-loop finite checks (§3.4/§5) hold; `TrustedByCaller` never yields `Solved` over NaN/Inf state.
6. All five gates green on both working tree and clean extraction; `cargo fmt` clean; files ≤ 300 ELOC.

## 7 Implementation Decisions

The implementation-decision memo (I1–I10) settled the narrow items the reconciliation review deferred; coding then surfaced two refinements worth recording.

### 7.1 Settled choices (I1–I10)

* **Module layout (I6).** `model.rs` carries the trait, workspace, `ProjectedFirstOrderConfig`, and `ProjectedFirstOrderSolveRecord`; the entrypoint, adapter, and helpers live in `solve/projected_first_order.rs`. Each file is ≤ 300 ELOC; the kernel tests split into `solve/projected_first_order/tests.rs` (entrypoint) and `.../tests/orchestration.rs` (`solve_batch` integration) to keep the test file within budget.
* **Scalar spelling (I4, prior §4.2 item 2).** `sub` / `mul` / `zero` (`BaseScalar`), `clamp` / `max` (`OrderedScalar`), `abs` / `lte_tolerance` (`MetricScalar`), `is_finite` (`FiniteScalar`).
* **Hot-loop access (I5).** Element access uses the RFC 002 `ContiguousVectorAccess::as_contiguous` / `as_contiguous_mut` slice views after the structural length checks, with `VectorAccess::get` for one-off reads. No raw indexing; no panic path in the solver.
* **Single scratch (I4/C4).** One gradient-scratch vector; the candidate is a scalar-temporary in-place `x` update. No `next` vector, no per-iteration allocation.
* **Iteration counting (I3/C5).** Mirrors RFC 006: `executed` incremented after each step; `converged_early(executed)` on `step ≤ tolerance` (first-step / `max_iterations == 1` → `converged_early(1)`); `not_converged_cap(max_iterations)` at the cap. `converged_at_cap` / `not_converged_stalled` unused in v1.
* **Checked cast (I7).** A local `dim_u32(usize) -> Result<u32, SolverError>` helper in the kernel module; overflow → `InvalidDimension`.
* **Reference problems (I8/D4).** An in-crate separable quadratic with a closed-form box-clamped optimum anchors the numerical-correctness suite, with NaN-gradient, `lo > hi`, dimension-mismatch, and cancellation fixtures. No public reference problem ships.
* **Validation placement (I9).** A single kernel-prep path runs the always-on structural checks plus the policy-governed `FINITE` scans and produces `(checked, trust)`; `ClusterValidationPolicy::resolve` is untouched and stays pure.

### 7.2 Refinements surfaced during coding

* **`S: Clone` is redundant (F8).** `BaseScalar: Copy`, so `DenseVector<S>: Clone` holds via `Copy` and the adapter's per-run `initial.clone()` is a `Copy`. The erased-adapter bound set is `FiniteScalar + MetricScalar + Send + Sync + 'static` — no separate `Clone`.
* **`checked.finite()` under `TrustedByCaller` is `NotApplicable`, not `Checked`.** The reconciliation text (§3.5) wrote `ValidationCoverage::new(PROBLEM_CONFIG, Checked)` for the trusted arm. But `ValidationCoverage::new` normalizes the *scope* to always include the `FINITE` bit, while the separate `finite: FiniteCoverage` field records *how* finiteness was addressed. Recording `Checked` when the kernel did **not** scan finiteness (it was trusted) would fabricate validation evidence — a violation of the evidence-integrity rule (never synthesize coverage that was not produced). The kernel therefore records `finite = NotApplicable` under trust, with the responsibility transfer carried in `trust = Some(..)`; `Checked` is recorded only when the kernel actually ran the scans. A unit test pins this.
* **Structural ordering form.** The finite `lo > hi` check is written `l.is_finite() && h.is_finite() && l > h` (equivalent to `!(l <= h)` for finite operands, and clippy-clean under `neg_cmp_op_on_partial_ord`); non-finite bounds fall through to the `FINITE` scan (`NonFiniteInput`) or, under trust, to the hot-loop candidate check (`NumericalDomain`), per C3.
