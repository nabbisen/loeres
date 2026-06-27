# Loeres Roadmap and Milestones Specification v1

Status: Accepted — Milestone 1 (`loeres`) complete (current as of v0.7.0)  
Scope: RFC roadmap, implementation sprint ordering, verification gates, and milestone exit criteria  
Calendar policy: No calendar dates or duration estimates. All progress is gated by design acceptance and automated validation.

> **Document currency.** Current as of repository release **v0.7.0**; the design is
> accepted. **Phase 0** (workspace skeleton — five crates plus `xtask`) is complete
> (v0.3.0). **Milestone 1 (`loeres`) is complete:** RFC 003 (v0.4.0),
> RFC 014 (v0.5.0), RFC 001 (v0.6.0), and **RFC 002** (v0.7.0) are implemented and
> in `rfcs/done/`; RFC 002 (storage-agnostic access) **closed Milestone 1**.
> Milestone 2 (static backend + device kernel, RFC 004–006) and Milestone 3
> (dynamic backend + cluster, RFC 007–009) follow. 62 core tests pass with
> `release-gate` green, including the bare-metal `no_std` build. No design content
> has changed since v0.6.1: v0.6.2 resynced the in-repo `docs/specs` mirrors, and
> v0.6.3 renamed the core crate from `loeres-core` to `loeres` (directory
> `crates/loeres/`; module layout unchanged).

---

## Current v0.7.0 Roadmap Snapshot

| Area | Status | Next action |
|---|---|---|
| Phase 0 — repository/governance bootstrap | Complete since v0.3.0 | Maintain gates as RFC 010 evolves. |
| RFC 003 — allocation-free errors/diagnostics | Implemented since v0.4.0 | Use as fixed error topology for RFC 002 and later solvers. |
| RFC 014 — solver outcome/status taxonomy | Implemented since v0.5.0 | Use `SolveStatus` / `TerminationReason` / `StepOutcome` in device and cluster reports. |
| RFC 001 — stratified scalar model | Implemented since v0.6.0 | Continue using `BaseScalar` without ordering and `OrderedScalar` for projection/comparison. |
| RFC 002 — storage-agnostic access contracts | Implemented since v0.7.0 | Closed Milestone 1. |
| RFC 004–006 — static backend/device path | Next (Milestone 2) | Active now that Milestone 1 is closed. |
| RFC 007–009 — dynamic backend/cluster path | Not started | May begin after Milestone 1; keep zero-bleed checks active. |
| RFC 010–013 — cross-cutting governance/targets/validation/conformance | Designed as cross-cutting work | Keep aligned with implementation gates and upcoming solver/backend work. |

## 0. Purpose and Roadmap Principle

This document defines the roadmap for the Loeres library family after the requirements and external design stages. It does not define private numerical kernels, concrete solver loops, or final trait signatures. Its purpose is to sequence the work so that structural safety is proven before feature expansion begins.

Loeres is intentionally split across two execution worlds:

1. **Core Layer:** mathematical abstraction and shared public contracts.
2. **Device Layer:** fixed-size, real-time, `no_std`, allocation-free execution.
3. **Cluster Layer:** dynamic, cloud-native, high-throughput execution.

The roadmap therefore follows the same topological order:

```text
Requirements v1
    |
    v
External Design v1
    |
    v
Phase 0: Repository and governance bootstrap
    |
    v
Phase 1 / Milestone 1: loeres
    |
    v
Phase 2 / Milestone 2: loeres-backend-static + loeres-device
    |
    v
Phase 3 / Milestone 3: loeres-backend-std + loeres-cluster
    |
    v
Cross-layer verification and release hardening
```

The ordering is deliberate. The dynamic cluster layer must not be allowed to shape the core in ways that later contaminate the device layer with `std`, `alloc`, logging, thread-pool assumptions, object safety assumptions, or hidden heap-backed diagnostics.

---

## Section 1: RFC Lifecycle and Governance Model

### 1.1 RFC State Machine

Every technical RFC must move through the following states. A state transition is invalid unless its entry and exit criteria are satisfied.

| State | Meaning | Allowed next states | Required evidence |
|---|---|---|---|
| `Draft` | Initial authoring stage. Open questions are allowed. | `Proposed`, `Withdrawn` | Problem statement, public boundary impact, affected crates, non-goals. |
| `Proposed` | Architecture review, acceptance, and implementation planning happen here. A proposed RFC may be review-active or accepted/frozen but not yet implemented. | `Implemented`, `Draft`, `Superseded`, `Withdrawn` | Review comments resolved or explicitly deferred; traceability to requirements/external design; zero-bleed impact analysis where applicable. |
| `Implemented` | Code, tests, examples, and verification gates exist. | `Superseded` | CI pass, documentation pass, automated safety checks pass, and closeout evidence attached. |
| `Withdrawn` | The proposal was intentionally abandoned before implementation. | none | Withdrawal rationale and any replacement pointer. |
| `Superseded` | Replaced by a newer RFC or rendered obsolete. | none | Superseding RFC ID and migration rationale. |

A proposed RFC may become accepted/frozen before code lands, but it remains in `proposed/` until implemented. A frozen RFC is a contract. Later RFCs may extend it only through additive specialized traits, wrapper layouts, configuration objects, or new crates. They must not silently loosen earlier safety, memory, or dependency constraints.

### 1.2 RFC File Layout

RFCs are stored under the canonical RFC 000 folder scheme — `proposed/`, `done/`, `archive/`, with an optional `draft/` — not the milestone-encoded layout used in earlier drafts:

```text
rfcs/
  README.md
  proposed/
    001-stratified-scalar.md
  done/
    ...
  archive/
    ...
  draft/            (optional)
    ...
```

The §1.1 lifecycle states map onto these folders: `Draft` may live in optional `draft/` or in `proposed/` depending on repository policy; `Proposed` lives in `proposed/`; `Implemented` lives in `done/`; `Withdrawn` and `Superseded` live in `archive/`.

RFC files use flat, stable, sequential numbering per RFC 000 (`NNN-slug.md`), not a milestone-encoded prefix:

```text
RFC 001 -> 001-stratified-scalar.md
RFC 002 -> 002-storage-agnostic-contracts.md
RFC 003 -> 003-allocation-free-errors.md
RFC 014 -> 014-core-solver-outcome-state.md
```

### 1.3 Mandatory RFC Template

Every RFC must include the following sections:

1. **Summary**
2. **Affected crates**
3. **Public API boundary impact**
4. **Dependency impact**
5. **`std` / `alloc` impact statement**
6. **Device determinism impact statement**
7. **Cluster scalability impact statement**
8. **Error and diagnostic impact**
9. **Feature flag impact**
10. **Semver impact**
11. **Rejected alternatives**
12. **Verification gates**
13. **Implementation sprint plan**
14. **Exit criteria**

### 1.4 Zero-Bleed Review Gate

The Zero-Bleed Review Gate is mandatory for every RFC that touches any of the following crates:

- `loeres`
- `loeres-backend-static`
- `loeres-device`
- shared examples used by device-facing crates
- verification tooling that constrains device-facing crates

The gate must verify:

| Check | Requirement |
|---|---|
| Direct dependency check | `loeres` must not depend on `std`, `alloc`, `ndarray`, `nalgebra`, `rayon`, `tokio`, `log`, `tracing`, `serde`, or any heap-backed container crate by default. |
| Transitive dependency check | `loeres-backend-static` and `loeres-device` must not transitively pull `std` or `alloc`. |
| Feature leakage check | Cluster-only features must not be activatable through device-facing feature sets. |
| Public signature check | Device-facing public APIs must not expose `Vec`, `String`, `Box`, `Arc`, `HashMap`, async runtime handles, thread-pool handles, or logging/tracing types. |
| Target check | Device-facing crates must compile for at least one `none` target profile with `default-features = false`. |
| Diagnostic check | Error and diagnostic payloads must stay within the frozen byte-size budgets from RFC 003. |

The gate should be automated by `xtask`, but a human reviewer must also inspect the dependency and public signature impact for every frozen RFC.

### 1.5 Architectural Monotonicity

Later RFCs must be monotonic with respect to earlier accepted constraints.

Rules:

1. A later RFC must not add a required method to a frozen core trait unless the earlier RFC explicitly reserved that extension point.
2. A later RFC must not require `alloc` in `loeres`, `loeres-backend-static`, or `loeres-device`.
3. A later RFC must not replace caller-owned device workspace with hidden allocation.
4. A later RFC must not move cluster-specific orchestration concepts into core.
5. A later RFC must not require dynamic dispatch in device-facing execution paths.
6. A later RFC must not reinterpret “deterministic” as bitwise cross-platform identity unless a separate, target-specific profile explicitly claims it.

If a constraint must be changed, the proper action is not silent revision. The project must create a superseding RFC with an explicit compatibility and safety analysis.

### 1.6 Implementation Sprint Model

Each accepted RFC is implemented through bounded sprints. These are sequence gates, not calendar estimates.

| Sprint | Purpose | Exit criteria |
|---|---|---|
| `S0 Design Freeze` | Move RFC from review to accepted/frozen. | All open questions either resolved or deferred to named RFCs. |
| `S1 Skeleton` | Add crate/module/type skeletons without private algorithmic complexity. | Workspace builds; docs compile; no forbidden dependency appears. |
| `S2 Contract Tests` | Add compile-time and public-boundary tests. | Public API behavior and feature gates are test-covered. |
| `S3 Implementation` | Implement the accepted public contract. | Unit tests pass; no private implementation exceeds RFC scope. |
| `S4 Verification` | Run zero-bleed, panic-path, size, and target checks. | All mandated gates pass. |
| `S5 Documentation` | Add examples and user-facing documentation. | Docs explain correct cluster/device usage without cross-contamination. |
| `S6 Closeout` | Move RFC to implemented. | Acceptance checklist is attached to the RFC closeout note. |

### 1.7 Governance Approval Rules

A milestone may not advance merely because code exists. It advances only when its RFCs are implemented and its exit criteria pass.

Additional rules:

- The project owner must approve any `v1.0` or public-stability release.
- A milestone may be partially implemented without being considered complete.
- A failed verification gate blocks milestone advancement.
- Examples that bypass safety rules are treated as API bugs unless clearly marked as internal test fixtures.

---

## Section 2: Phase 1 — Foundational Core Architecture (Milestone 1)

### 2.1 Milestone Objective

Milestone 1 freezes the mathematical and diagnostic contracts of `loeres`. This phase must be completed before device or cluster implementation work depends on the shared traits.

The goal is not to design all future algorithms. The goal is to define the smallest stable abstraction surface that both device and cluster crates can implement without importing each other’s execution assumptions.

### 2.2 Milestone Entry Criteria

Milestone 1 may begin only when:

- Requirements v1 or later is accepted.
- External Design v1 or later is accepted for RFC generation.
- The repository has a minimal workspace skeleton.
- The RFC lifecycle policy in Section 1 is adopted.
- The initial `xtask` crate exists, even if only with placeholder commands.

### 2.3 RFC 001 — Stratified Scalar Capability Model

#### Scope

Define the public scalar capability hierarchy for `loeres` without forcing algorithms to depend on operations they do not need.

Required capability families:

| Capability | Purpose | Baseline status |
|---|---|---|
| `BaseScalar` | Basic arithmetic and identity values; no ordering. | Required for all problem representation. |
| `OrderedScalar` | Ordering and Loeres-defined `min` / `max` / `clamp` (NaN-propagating for floats). | Optional; required by projection / box-constrained solvers; implied by `MetricScalar`. |
| `FiniteScalar` | Boundary validation for NaN/Inf-like states where applicable. | Required by public solve entrypoints that accept floating-like values. |
| `DivisibleScalar` | Checked division and inversion-like operations. | Optional; required only by algorithms that divide. |
| `MetricScalar` | Magnitude, tolerance, residual, and convergence comparisons. | Optional; required by convergence-aware solvers. |
| `AdvancedNumericalScalar` | Square root, logarithm, exponential, powers, barrier-like functions. | Optional; forbidden as a baseline requirement. |

#### Design constraints

- No scalar trait may require `std`.
- No scalar trait may require allocation.
- Base scalar capability must not require ordering or division.
- Advanced transcendental operations must not be pulled into the base capability.
- Checked operations must return structured errors or option-like states rather than panic.
- Fixed-point and integer-like future scalar profiles must not be made impossible by premature floating-point assumptions.

#### Key risks to resolve

- Whether `PartialOrd` is sufficient for all base comparisons. *(Resolved by RFC 001 / v0.6.0: ordering moved out of `BaseScalar` into `OrderedScalar`; `BaseScalar` requires only `PartialEq`.)*
- How NaN-like semantics are represented for scalar families that do not have NaN. *(Resolved by RFC 001 / v0.6.0: `min` / `max` / `clamp` are NaN-propagating for floats and ordinary total-order extrema for NaN-free families.)*
- Whether core should define marker traits for scalar categories or method-bearing traits for every category.
- How to avoid pulling a third-party numeric trait crate into `loeres` by default.

#### Implementation sprint outline

| Sprint | Work |
|---|---|
| S0 | Freeze scalar family names and capability boundaries. |
| S1 | Add module skeleton under `loeres::scalar`. |
| S2 | Add compile tests for scalar capability layering. |
| S3 | Implement minimal trait definitions and primitive scalar adapters only if accepted by the RFC. |
| S4 | Run no-std and zero-dependency verification. |
| S5 | Document which solver families require which scalar capabilities. |
| S6 | Close RFC with trait compatibility checklist. |

#### Exit criteria

- Scalar categories are frozen.
- `loeres` still compiles as `#![no_std]` without `alloc`.
- No scalar category creates a dependency from core to device or cluster.
- Future solver RFCs can state their scalar requirements by capability family.

### 2.4 RFC 002 — Storage-Agnostic Matrix and Vector Access Contracts

#### Current status

Implemented in v0.7.0; this completed Milestone 1.

#### Scope

Define the public access contracts for vectors, matrices, dimensions, fallible indexing, mutable writes, borrowed contiguous views, and optional contiguous fast paths.

This RFC preserves the external design decision that the core surface is about access, dimensions, and storage-neutral contracts, not heavy linear algebra kernels. It also incorporates the v0.6.1 design discussion: use `loeres::access` for access traits, use `loeres::dimension` for dimension types, keep core views contiguous-only, defer strided/submatrix view types to RFC 004, and prohibit overlapping mutable views.

#### Finalized topics

- Module naming is `loeres::access`; `loeres::linalg` is not used for base contracts.
- Dimension naming and imports use `loeres::dimension`, not `dim`.
- Vector length and matrix shape queries are layout-neutral.
- Base access is fallible and scalar/storage agnostic.
- Mutable access must be checked and must not permit overlapping mutable aliases in safe APIs.
- Core borrowed views are contiguous baseline views only.
- Strided, row, column, and submatrix views belong to RFC 004 / `loeres-backend-static` advanced view work unless a later RFC explicitly promotes a narrow trait into core.
- Optional contiguous fast-path traits are allowed so kernels are not forced through per-element `Result` access in tight loops.
- Error mapping must use the existing `SolverError` topology: invalid dimensions map to `InvalidDimension`; shape disagreement maps to `DimensionMismatch`; out-of-bounds or invalid index-like access maps to an RFC 002-defined policy over existing variants without introducing a new allocation-heavy error payload.
- Backend-specific kernels may expose fast paths without forcing matrix multiplication, factorization, sparse traversal, or BLAS-like operations into base core traits.

#### Design constraints

- No `dyn Trait` in edge-facing access paths.
- No core methods that imply heap allocation.
- No core methods that require one memory layout.
- No mandatory matrix multiplication, factorization, decomposition, or BLAS-like operation in base access traits.
- Large fixed-size structures must not be passed by value in device-facing examples.
- Base core access traits must be implementable by both static and dynamic backends without dependency edges between them.

#### Implementation sprint outline

| Sprint | Work |
|---|---|
| S0 | Confirm the v0.6.1 design-finalized decisions and freeze access/dimension names. |
| S1 | Add `access` module skeleton and doc comments. |
| S2 | Add compile tests for owned storage, borrowed contiguous views, mutable access, and forbidden alias patterns where feasible. |
| S3 | Implement accepted access trait definitions, contiguous view types/traits, and optional contiguous fast-path traits. |
| S4 | Run no-std, no-alloc, no-dyn, and public API checks. |
| S5 | Add examples showing dynamic and static backends implementing the same core contracts without importing each other. |
| S6 | Close RFC 002 and Milestone 1 with layout-neutrality and zero-bleed evidence. |

#### Exit criteria

- Core access contracts are storage-neutral.
- `loeres::access` and `loeres::dimension` naming is consistent across docs, code, and examples.
- Borrowed contiguous view requirements are clear enough for RFC 004.
- Dynamic backend requirements are clear enough for RFC 007.
- Optional fast traversal is available without making layout-specific kernels mandatory.
- No public core API references concrete storage crates.

### 2.5 RFC 003 — Allocation-Free Error Topology and Formatting Restrictions

#### Scope

Define the public error and compact diagnostic categories for `loeres`. RFC 014 owns terminal solve status, step outcome, and solver report taxonomy.

This RFC is required before solver entrypoints are designed because device and cluster APIs must agree on failure/error semantics. Solver progress status is reconciled separately by RFC 014.

#### Required topics

- Public error category enum.
- Public diagnostic category enum or compact diagnostic code.
- Semver extensibility policy.
- Formatting policy.
- Byte-size budget for device-facing error and diagnostic values.
- Zero-allocation conversion from error codes to static message strings.

#### Mandatory constraints

- Core errors must implement `core::fmt::Debug`.
- Core errors must not implement `core::fmt::Display` by default.
- Core errors and diagnostic enums must be semver-extensible, for example through `#[non_exhaustive]` where appropriate.
- Diagnostic values must not contain `String`, `Vec`, `Box`, formatting payloads, or backend-specific object references.
- The RFC must define explicit byte-size budgets and compile-time size assertions for important public result/error shapes.
- Human-readable text must be exposed through zero-allocation static lookup, such as an `error_code_to_str`-style facility, without freezing the exact function name in this roadmap.

#### Key risks to resolve

- Large `Result<T, E>` shapes may bloat stack frames on 32-bit device targets.
- Rich diagnostics may accidentally become logging frameworks.
- Adding public enum variants later may break downstream exhaustive matches unless semver rules are explicit.
- Cluster users need enough failure information for observability without forcing device users to pay the payload cost.

#### Implementation sprint outline

| Sprint | Work |
|---|---|
| S0 | Freeze error category taxonomy and semver policy. |
| S1 | Add `error` and `diagnostic` module skeletons. |
| S2 | Add compile-time size tests and no-formatting tests. |
| S3 | Implement accepted error/diagnostic definitions and static lookup. |
| S4 | Run no-std, no-alloc, and size-budget checks. |
| S5 | Document mapping between failure classes and solver behavior. |
| S6 | Close RFC with diagnostic bloat checklist. |

#### Exit criteria

- `loeres` compiles as core-only `#![no_std]`.
- Error and diagnostic payload sizes are frozen for the current milestone.
- No `Display` implementation is present by default.
- Cluster and device RFCs can depend on the same public failure categories, while RFC 014 supplies the shared progress/status categories.

### 2.6 Milestone 1 Exit Criteria

Milestone 1 is complete only when all of the following are true:

- RFC 001, 002, 003, and 014 are `Implemented`.
- The public core trait and error signatures are reviewed and frozen.
- `loeres` builds with `default-features = false` under the selected `no_std` target profile.
- Dependency graph checks prove that `loeres` has no forbidden direct or transitive dependency.
- Compile-time tests prove that cluster and device backends can implement the core contracts without importing each other.
- The roadmap owner accepts that downstream changes must be additive or superseding, not silently mutating frozen core traits.

---

## Section 3: Phase 2 — Static Backend and Real-Time Kernel (Milestone 2)

### 3.1 Milestone Objective

Milestone 2 creates the device-side execution path:

```text
loeres-device
    -> loeres-backend-static
        -> loeres
```

The objective is to prove that a useful optimization problem can be represented, validated, solved, and diagnosed without `std`, without `alloc`, without hidden work buffers, and without runtime dependency on cluster infrastructure.

### 3.2 Milestone Entry Criteria

Milestone 2 may begin only when:

- Milestone 1 is complete.
- Core scalar/access/error and solver-outcome contracts are frozen.
- Device target profiles are selected for the milestone.
- The zero-bleed gate is operational for `loeres` and can be extended to device crates.
- A reference device example target exists, even if it initially does not solve a real problem.

### 3.3 RFC 004 — Const-Generic and Fixed-Size Static Storage Engine

#### Scope

Design `loeres-backend-static` public storage wrappers and views for stack-allocated or caller-owned memory.

#### Required topics

- Owned fixed vectors and matrices.
- Borrowed vector and matrix views.
- Advanced strided or sub-matrix views behind explicit features if accepted.
- Dimension representation.
- Compile-time shape metadata.
- Runtime validation for borrowed buffers.
- Legal const-expression profile for the selected Rust compiler version.
- Stable fallback layout strategies if complex const-generic arithmetic is not viable.

#### Mandatory constraints

- The baseline backend must be `#![no_std]` and allocation-free.
- The baseline backend must not require `heapless`, `arrayvec`, or any external crate unless separately accepted.
- Const generics are reserved for memory sizing and shape identity.
- Complex expressions such as generic `R * C` array lengths must not be assumed stable without compiler-profile validation.
- Fallback flattened slice layouts must be designed before implementation begins.
- Public APIs must avoid passing large arrays by value.

#### Key risks to resolve

- Stable Rust limitations around generic const expressions.
- Stack bloat from large fixed arrays.
- Accidental copies caused by owned-value APIs.
- Borrowed views that are too weak for embedded DMA or RTOS-owned memory.
- Over-featured static view support bloating the baseline.

#### Implementation sprint outline

| Sprint | Work |
|---|---|
| S0 | Freeze fixed storage layout families and fallback strategy. |
| S1 | Add `loeres-backend-static` skeleton with no default dependencies. |
| S2 | Add compile-fail tests for illegal by-value movement and shape misuse where feasible. |
| S3 | Implement accepted owned and borrowed storage contracts. |
| S4 | Run target builds, zero-bleed checks, and size checks. |
| S5 | Add examples for owned arrays and borrowed control-loop buffers. |
| S6 | Close RFC with const-generic legality report. |

#### Exit criteria

- Static storage builds without `std` or `alloc`.
- The selected fixed-size and borrowed-view APIs implement core access contracts.
- The stable compiler profile is documented.
- Fallback layout strategy is accepted, even if not immediately used.

### 3.4 RFC 005 — Caller-Owned Typed Workspace Mechanics and Poisoning Semantics

#### Scope

Design device workspace ownership, sizing, reset, and failure-state semantics.

#### Required topics

- Typed workspace lifecycle.
- Workspace initialization.
- Workspace reset or clear operation.
- Solver failure semantics.
- Safe-to-discard state.
- Reuse contract after success and failure.
- Compile-time and runtime sizing responsibilities.
- Diagnostic extraction without logging.

#### Mandatory constraints

- Solvers must not allocate hidden work buffers.
- Workspaces are passed by unique mutable reference.
- Workspace state after solver failure must be explicitly specified.
- Partial failures must not produce undefined Rust behavior or unbounded loops.
- The roadmap-level default is: after a failed solve, the workspace must be safe to discard immediately. Reuse must be legal only if the workspace type or API guarantees that it was reset or reinitialized.
- Manual heap-backed clearing cycles are forbidden in the device baseline.

#### Design note on poisoning

The external design used reset-required semantics by default. This roadmap narrows the RFC task: define the precise state machine so the implementation can make failure behavior mechanically checkable.

Acceptable RFC outcomes include either:

1. **Reset-required workspace:** failed solve marks the workspace as requiring reset before reuse.
2. **Always-reusable workspace:** solver implementation guarantees that all public failures leave the workspace in a normalized reusable state.
3. **Type-state workspace:** failure returns a changed workspace state that prevents accidental reuse until reset.

The RFC must choose exactly one baseline policy for v0.x.

#### Key risks to resolve

- Poisoned workspace reuse in embedded control loops.
- Workspace reset that costs too much time for real-time loops.
- Workspace diagnostics that bloat stack frames.
- Generic workspace formulas that require unstable const expressions.

#### Implementation sprint outline

| Sprint | Work |
|---|---|
| S0 | Freeze workspace lifecycle and failure-state policy. |
| S1 | Add workspace skeletons and type/state markers if accepted. |
| S2 | Add tests for success, failure, reset, and reuse contracts. |
| S3 | Implement sizing and workspace initialization APIs. |
| S4 | Run no-alloc, panic-path, and size-budget checks. |
| S5 | Document control-loop reuse patterns. |
| S6 | Close RFC with poisoned-state safety checklist. |

#### Exit criteria

- Workspace lifecycle is unambiguous.
- Solver RFCs can depend on workspace semantics without inventing local rules.
- Failure handling is safe by construction or mechanically checked by tests.
- No hidden allocation is introduced.

### 3.5 RFC 006 — Baseline Deterministic Solver Engine

#### Scope

Design and implement the first device solver engine. The preferred v0.x baseline is a box/bound-constrained projected first-order kernel or another closed-form-projection family with a bounded loop. General linear-inequality projection (`Ax <= b`) and broad dense QP/IPM scope are out of the first device kernel unless a later RFC explicitly accepts the additional inner-solver complexity.

#### Required topics

- Supported problem family.
- Required scalar capabilities.
- Required access contracts.
- Workspace requirements.
- Runtime configuration struct.
- Maximum-iteration semantics.
- Constant-iteration vs early-exit behavior.
- Panic-averse release gates.
- Floating-point target profile.
- Error and diagnostic output.

#### Mandatory constraints

- Execution configuration must be runtime data, not type-level solver identity.
- Const generics may express dimensions and workspace sizing, but not routine policy values such as `max_iter` or tolerance presets unless separately justified.
- Solver loops must be bounded.
- Public entrypoints must return structured failure states.
- No background threads, async runtime, logging framework, or heap allocation is allowed.
- Device solver behavior must be documented as target-scoped deterministic, not universally bit-identical across every CPU.

#### Key risks to resolve

- False determinism claims across floating-point targets.
- Panic paths from indexing, division, or unchecked numerical domains.
- Binary size growth from over-generic scalar/problem instantiations.
- Numerical convergence failure on adversarial inputs.
- Instruction-cache pressure on embedded profiles.

#### Implementation sprint outline

| Sprint | Work |
|---|---|
| S0 | Freeze baseline problem family and timing mode. |
| S1 | Add solver crate skeleton and public config/status types. |
| S2 | Add deterministic test corpus and failure-mode tests. |
| S3 | Implement accepted baseline kernel. |
| S4 | Run QEMU or bare-metal emulation, panic-path analysis, binary-size checks, and zero-bleed checks. |
| S5 | Add device examples for safe configuration, validation, and diagnostics. |
| S6 | Close RFC with target-profile and binary-size evidence. |

#### Exit criteria

- The baseline solver runs on the selected bare-metal or emulated target.
- The solver passes panic-averse analysis gates selected for v0.x.
- Binary size target is frozen and met.
- Workspace failure semantics are respected.
- Numerical behavior is tested against the shared parity corpus within the accepted epsilon.

### 3.6 Milestone 2 Exit Criteria

Milestone 2 is complete only when:

- RFC 004, 005, and 006 are `Implemented`.
- `loeres-backend-static` and `loeres-device` compile with no `std` or `alloc`.
- The zero-bleed gate proves no illegal dependency edge exists.
- The selected device target runs at least one end-to-end example.
- Panic-path analysis passes under the selected release profile.
- Binary size and diagnostic size budgets are measured and frozen.
- Workspace failure and reset semantics are documented and tested.

---

## Section 4: Phase 3 — Dynamic Infrastructure and Cloud Cluster (Milestone 3)

### 4.1 Milestone Objective

Milestone 3 creates the cluster-side execution path:

```text
loeres-cluster
    -> loeres-backend-std
        -> loeres
```

The objective is high-throughput, dynamic problem construction, parallel execution, partial-failure isolation, observability, and optional FFI integration without weakening the core or device constraints.

### 4.2 Milestone Entry Criteria

Milestone 3 may begin when:

- Milestone 1 is complete.
- The external design’s cluster public boundary is accepted.
- The cluster feature matrix is frozen enough to separate default-safe features from opt-in heavy integrations.
- The zero-bleed gate exists so cluster additions cannot leak backward into core/device crates.

Milestone 3 does not strictly require Milestone 2 implementation to be complete, but any shared test corpus or parity requirement that depends on device behavior cannot be closed until Milestone 2 provides a compatible baseline solver.

### 4.3 RFC 007 — Heap-Allocated and Sparse Storage Adapters

#### Scope

Design `loeres-backend-std` dynamic storage adapters for dense and sparse problem representations.

#### Required topics

- Dynamic vector and matrix wrappers.
- Sparse matrix representation boundary.
- Ownership and borrowing models.
- Conversion from user input into validated model structures.
- Backend optionality for `ndarray`, `nalgebra`, sparse crates, or internal representations.
- Memory allocation failure behavior where applicable.
- Trusted-input validation state integration, consuming the canonical validation-state model from RFC 012 once accepted.

#### Mandatory constraints

- `loeres-backend-std` may depend on `std` and allocation, but these dependencies must not leak into `loeres` or device-facing crates.
- Dynamic storage adapters must implement the same core access contracts frozen in RFC 002.
- Large input validation scans must be controllable through explicit validation-state APIs, not implicit global flags.
- The public API must distinguish unvalidated, validated, and trusted input states through the RFC 012 canonical model rather than inventing backend-local state categories.

#### Key risks to resolve

- Excessive copying between user models and backend structures.
- Hidden validation cost on large sparse matrices.
- Sparse/dense API divergence that undermines common solver orchestration.
- Feature-flag combinations that accidentally enable heavy dependencies by default.

#### Implementation sprint outline

| Sprint | Work |
|---|---|
| S0 | Freeze dense/sparse adapter scope and feature policy. |
| S1 | Add `loeres-backend-std` skeleton and feature-gated dependency slots. |
| S2 | Add validation-state and storage-contract tests. |
| S3 | Implement accepted adapters. |
| S4 | Run dependency separation and performance baseline checks. |
| S5 | Add examples for dynamic model construction. |
| S6 | Close RFC with storage conversion and validation-cost report. |

#### Exit criteria

- Dynamic backend implements core access contracts.
- Default features do not enable FFI or heavyweight optional integrations.
- Validation-state behavior is explicit and tested.
- No cluster dependency appears in core or device graphs.

### 4.4 RFC 008 — High-Throughput Async Orchestration and Monomorphization Budgets

#### Scope

Design `loeres-cluster` orchestration APIs for parallel and asynchronous solving.

#### Required topics

- Batch solve APIs.
- Per-item result semantics.
- Cancellation paths.
- Timeout configuration.
- Parallelism depth controls.
- Runtime integration boundaries.
- Monomorphization budget.
- Dynamic dispatch policy for high-level orchestration.
- Generic fast paths for inner mathematical kernels.

#### Mandatory constraints

- Batch solves must return per-item outcomes by default. One ill-conditioned model must not fail the entire batch unless the user explicitly chooses fail-fast behavior.
- High-level orchestration may use dynamic dispatch if needed to control binary size and compile times.
- Device-facing APIs must not inherit cluster dynamic dispatch choices.
- Inner math kernels may remain generic where performance requires monomorphization.
- The RFC must define measurable monomorphization budgets, such as code-size thresholds, number of allowed scalar/problem instantiations, or compile-time budgets.

#### Key risks to resolve

- Binary bloat from generic orchestration over many scalar/problem types.
- Ambiguous cancellation semantics when worker threads are executing numerical kernels.
- Tenant data leakage through shared batch diagnostics.
- Validation cost repeated across batch lanes.
- Inconsistent behavior between fail-fast and collect-all batch modes.

#### Implementation sprint outline

| Sprint | Work |
|---|---|
| S0 | Freeze orchestration scope, partial-failure model, and monomorphization budget. |
| S1 | Add cluster orchestration skeletons and configuration types. |
| S2 | Add tests for cancellation, timeout, per-item failures, and fail-fast mode. |
| S3 | Implement accepted orchestration path. |
| S4 | Run throughput, binary-size, compile-time, and dependency isolation checks. |
| S5 | Add examples for service-style batch solving. |
| S6 | Close RFC with monomorphization and partial-failure report. |

#### Exit criteria

- Batch APIs isolate per-item failures.
- Cancellation and timeout semantics are documented and tested.
- Monomorphization budget is measured and accepted.
- Cluster orchestration does not alter core or device constraints.

### 4.5 RFC 009 — Observability, Metrics, and FFI Gateway Interfacing

#### Scope

Design optional cluster observability and FFI boundaries.

#### Required topics

- Tracing integration.
- Metrics hooks.
- Structured event categories.
- Multi-tenant diagnostic isolation.
- Optional FFI gateway feature.
- Third-party solver boundary policy.
- Safety wrapper obligations.
- Data confidentiality rules.

#### Mandatory constraints

- Observability is cluster-only by default.
- FFI is cluster-only and default-off.
- No FFI type may appear in `loeres`, `loeres-backend-static`, or `loeres-device` public APIs.
- FFI gateways must expose failure as structured `loeres` errors or cluster-layer wrapper errors.
- Tenant identifiers, model contents, and numerical diagnostics must not be emitted by default in logs or traces.
- Metrics must support aggregation without leaking sensitive problem data.

#### Key risks to resolve

- Unsafe FFI boundary unsafely widening the project’s trusted computing base.
- Observability payloads leaking sensitive optimization models.
- FFI feature accidentally becoming a default transitive dependency.
- Runtime tracing costs distorting cluster performance results.

#### Implementation sprint outline

| Sprint | Work |
|---|---|
| S0 | Freeze observability and FFI feature policy. |
| S1 | Add feature-gated module skeletons. |
| S2 | Add tests proving feature defaults and dependency isolation. |
| S3 | Implement observability hooks and FFI boundary stubs or wrappers as accepted. |
| S4 | Run security, dependency, and payload-redaction checks. |
| S5 | Add examples for safe tracing and metrics configuration. |
| S6 | Close RFC with TCB and default-off evidence. |

#### Exit criteria

- Observability is ergonomic for cluster users but absent from device baseline.
- FFI remains opt-in and cluster-only.
- Multi-tenant leakage risks are addressed by default behavior.
- Cluster integration does not alter device or core contracts.

### 4.6 Milestone 3 Exit Criteria

Milestone 3 is complete only when:

- RFC 007, 008, and 009 are `Implemented`.
- Cluster default features are safe and do not enable FFI by default.
- Batch partial-failure behavior is tested.
- Monomorphization budget is measured.
- Observability payloads are redacted by default.
- Dependency graph checks prove cluster dependencies do not leak backward into core or device crates.

---

## Section 5: Integration Testing and Verification Milestones

### 5.1 Verification Objective

Integration verification proves that Loeres is a coherent library family rather than a set of unrelated crates.

The goal is not bitwise identity between cluster and device. The goal is convergent numerical behavior within accepted tolerances while preserving strict environmental separation.

### 5.2 Cross-Layer Test Corpus

The repository must maintain a shared problem corpus:

```text
tests/corpus/
  lp/
  qp/
  socp/
  invalid/
  ill_conditioned/
  edge_reference/
```

Each corpus case must define:

- problem family;
- dimensions;
- scalar profile;
- expected success or failure class;
- expected convergence tolerance;
- expected validation behavior;
- whether the case is allowed for device, cluster, or both.

### 5.3 Numerical Convergence Parity Testing

The parity suite must run identical problem instances across compatible cluster and device solvers.

Rules:

- Success is numerical convergence within the accepted epsilon, initially `1e-5` for compatible floating-point profiles.
- Bitwise identity is not required.
- Failure classes must match at the public category level, even if internal diagnostics differ.
- Cluster BLAS/LAPACK-backed paths may diverge slightly from device scalar loops, but must remain within the accepted tolerance.
- Cases requiring unsupported scalar or algorithm capabilities must be marked explicitly rather than silently skipped.

### 5.4 `xtask` Automation Blueprint

The repository must include an internal automation crate:

```text
xtask/
  src/main.rs
  checks/
    dependency_graph.rs
    feature_matrix.rs
    no_std.rs
    panic_paths.rs
    size_budget.rs
    public_api.rs
    corpus.rs
```

Required commands:

| Command | Purpose |
|---|---|
| `cargo xtask zero-bleed` | Reject illegal dependency edges into core/device-facing crates. |
| `cargo xtask feature-matrix` | Validate public feature combinations and mutually exclusive configurations. |
| `cargo xtask no-std` | Build core/device-facing crates for selected no-std targets. |
| `cargo xtask target-profiles` | Build and check the accepted cluster/device target profiles from RFC 011. |
| `cargo xtask panic-audit` | Run selected panic-averse analysis for device entrypoints. |
| `cargo xtask unsafe-audit` | Report or reject unreviewed `unsafe` in governed crates. |
| `cargo xtask size-budget` | Measure binary, stack-relevant type, error, diagnostic, and code-size budgets. |
| `cargo xtask check-public-api` | Detect forbidden public types and forbidden `dyn` use in device-facing APIs. |
| `cargo xtask conformance` | Run shared numerical and failure-mode corpus tests. |
| `cargo xtask check` | Canonical aggregate release gate for milestone advancement. |
| `cargo xtask release-gate` | Optional alias for `cargo xtask check` for CI clarity. |

### 5.5 Dependency-Graph Freezing

The dependency graph must be machine-checked. At minimum, the graph policy must encode these illegal edges:

```text
loeres -> std
loeres -> alloc
loeres -> loeres-backend-std
loeres -> loeres-backend-static
loeres -> loeres-cluster
loeres -> loeres-device

loeres-backend-static -> std
loeres-backend-static -> alloc
loeres-backend-static -> loeres-backend-std
loeres-backend-static -> loeres-cluster

loeres-device -> std
loeres-device -> alloc
loeres-device -> loeres-backend-std
loeres-device -> loeres-cluster

loeres-backend-std -> loeres-device
loeres-cluster -> loeres-device
```

The exact graph representation is an implementation detail, but the policy must be hardcoded enough that a forbidden edge fails CI.

### 5.6 Public API Verification

The public API check must reject forbidden edge-facing public signatures.

Forbidden in `loeres`, `loeres-backend-static`, and `loeres-device` baseline public APIs:

- `Vec`
- `String`
- `Box`
- `Rc`
- `Arc`
- `HashMap`
- `BTreeMap`
- async runtime handles
- OS thread handles
- logging framework types
- tracing framework types
- FFI handle types
- `dyn Trait` in device execution entrypoints

Exceptions may exist only behind explicitly accepted RFCs and must not affect baseline device builds.

### 5.7 Panic-Path Verification

Device-facing crates must use panic-averse engineering rather than absolute unverifiable claims.

Required checks:

- no `unwrap` or `expect` in device-facing production code;
- no unchecked indexing in solver kernels unless statically justified;
- checked division or explicit numerical-domain validation;
- panic strategy documented for device targets;
- selected panic-analysis tooling run under the release profile used for size and target checks;
- manual review of any `unsafe` code if it is ever introduced.

### 5.8 Size and Monomorphization Verification

Size verification must include:

| Budget | Applies to | First defined in |
|---|---|---|
| Error payload size | `loeres` | RFC 003 |
| Diagnostic payload size | `loeres` / device | RFC 003 / RFC 005 |
| Static storage wrapper overhead | `loeres-backend-static` | RFC 004 |
| Workspace size | `loeres-device` | RFC 005 / RFC 006 |
| Device binary size | `loeres-device` examples | RFC 006 |
| Cluster monomorphization budget | `loeres-cluster` | RFC 008 |
| FFI feature footprint | `loeres-cluster` optional feature | RFC 009 |

The exact numeric budgets are not defined by this roadmap except where already mandated by external instructions. Each owning RFC must freeze the relevant numbers before implementation.

### 5.9 Release Readiness Gates

A release candidate may be cut only when:

- all implemented RFCs are in the `done/` directory;
- no accepted RFC is partially implemented without being marked as incomplete;
- `cargo xtask check` passes; `cargo xtask release-gate` may be used as a CI alias if present;
- device and cluster examples compile under their intended feature sets;
- dependency graph checks pass;
- public API checks pass;
- documentation describes the split between cluster and device without suggesting runtime mode switching;
- any public `v1.0` or stability release has explicit project owner approval.

### 5.10 Roadmap Completion Matrix

| Phase | Milestone | Required RFCs | Completion signal | Status |
|---|---|---|---|---|
| Phase 0 | Governance bootstrap | none | RFC lifecycle, repository skeleton, and initial `xtask` exist. | ✅ complete (v0.3.0) |
| Phase 1 | Core | RFC 001, 002, 003, 014 | Core contracts frozen and no-std verified. | ✅ complete — RFC 001/002/003/014 implemented; core contracts frozen and `no_std`-verified (v0.7.0) |
| Phase 2 | Device | RFC 004, 005, 006 | Device solver runs on selected no-std target with zero-bleed and size gates passing. | ⬜ not started |
| Phase 3 | Cluster | RFC 007, 008, 009 | Dynamic backend and cluster orchestration pass partial-failure, observability, and dependency isolation checks. | ⬜ not started |
| Integration | Cross-layer verification | corpus and `xtask` gates | Compatible cluster/device solvers converge within accepted epsilon and preserve separation. | ⬜ not started |

---

## Appendix A: RFC Dependency Graph

```text
RFC 001 Stratified Scalar Capability Model
    |
    +--> RFC 002 Storage-Agnostic Matrix and Vector Access Contracts
    |       |
    |       +--> RFC 004 Const-Generic and Fixed-Size Static Storage Engine
    |       |       |
    |       |       +--> RFC 005 Caller-Owned Typed Workspace Mechanics
    |       |               |
    |       |               +--> RFC 006 Baseline Deterministic Solver Engine
    |       |
    |       +--> RFC 007 Heap-Allocated and Sparse Storage Adapters
    |               |
    |               +--> RFC 008 High-Throughput Async Orchestration
    |                       |
    |                       +--> RFC 009 Observability, Metrics, and FFI Gateway
    |
    +--> RFC 003 Allocation-Free Error Topology
            |
            +--> RFC 014 Core Solver Outcome and Status Taxonomy
            |       |
            |       +--> RFC 006 Device Report/Status Derivation
            |       +--> RFC 008 Batch Partial-Failure Report
            |
            +--> RFC 005 Workspace Failure Semantics
            |
            +--> RFC 006 Device Failure Semantics
            |
            +--> RFC 008 Batch Partial Failure Semantics

Cross-cutting RFCs (apply across milestones):
    RFC 010 xtask Verification Governance
        -> governs release gates, check-public-api, panic/unsafe audits, and aggregate check
    RFC 011 Target Profiles and Deterministic Math
        -> informs RFC 006 target-scoped device claims and no-std target checks
    RFC 012 Validation State and Trusted Input Policy
        -> consumed by RFC 007 dynamic adapters and RFC 008 cluster validation policy
    RFC 013 Conformance Corpus and Numerical Parity
        -> consumed by RFC 006 device kernel tests and RFC 008/cluster parity checks
```

## Appendix B: Milestone Acceptance Checklist

Before marking any milestone complete, reviewers must answer yes to all applicable questions:

1. Are all required RFCs implemented?
2. Are all public APIs documented?
3. Are all feature flags documented?
4. Does zero-bleed automation pass?
5. Does no-std target compilation pass where applicable?
6. Are error and diagnostic budgets measured?
7. Are workspace semantics tested where applicable?
8. Are batch partial-failure semantics tested where applicable?
9. Are accepted unsafe or FFI boundaries documented where applicable?
10. Are rejected alternatives recorded?
11. Are release notes and migration notes prepared?
12. Has the project owner approved any stability or v1 release claim?

## Appendix C: Terms

| Term | Meaning |
|---|---|
| Zero-bleed | No forbidden dependency or public API assumption crosses from cluster into core/device-facing crates. |
| Panic-averse | Engineering practice that reduces and checks panic paths without overclaiming formal proof. |
| Constant-iteration | A timing mode in which iteration count is fixed by configuration, distinct from cryptographic constant-time behavior. |
| Typed workspace | Caller-owned memory object whose shape and lifecycle are part of the public device API. |
| Monomorphization budget | A measurable bound on generic instantiation cost, usually represented as binary size, compile time, or code-size growth. |
| Trusted input state | An explicit API state indicating that expensive validation scans may be skipped under caller responsibility. |
