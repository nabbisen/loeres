# Loeres RFC Index

This directory is the authoritative RFC registry for the Loeres library family.

RFC numbers are stable forever. Moving an RFC between `proposed/`, `done/`, and `archive/` changes its lifecycle state, but never changes its number or slug.

## Proposed RFCs

| RFC | Title | Phase | Primary crates |
|---:|---|---|---|
| [002](proposed/002-storage-agnostic-contracts.md) | Storage-Agnostic Matrix and Vector Access Contracts | Phase 1 / Milestone 1 | `loeres` |
| [004](proposed/004-static-storage-engine.md) | Const-Generic and Fixed-Size Static Storage Engine | Phase 2 / Milestone 2 | `loeres-backend-static` |
| [005](proposed/005-typed-workspace-mechanics.md) | Caller-Owned Typed Workspace Mechanics and Poison-Free Reuse | Phase 2 / Milestone 2 | `loeres-device`, `loeres-backend-static` |
| [006](proposed/006-deterministic-solver-kernel.md) | Baseline Deterministic Device Solver Kernel | Phase 2 / Milestone 2 | `loeres-device` |
| [007](proposed/007-dynamic-sparse-adapters.md) | Dynamic Dense and Sparse Storage Adapters | Phase 3 / Milestone 3 | `loeres-backend-std` |
| [008](proposed/008-async-orchestration-budgets.md) | Async Orchestration and Monomorphization Budgets | Phase 3 / Milestone 3 | `loeres-cluster` |
| [009](proposed/009-observability-ffi-gateways.md) | Observability, Metrics, and FFI Gateway Interfacing | Phase 3 / Milestone 3 | `loeres-cluster`, `loeres-backend-std` |
| [010](proposed/010-xtask-verification-governance.md) | xtask Verification Governance | Cross-cutting / Verification | `xtask`, CI, all crates |
| [011](proposed/011-target-profiles-and-deterministic-math.md) | Target Profiles and Deterministic Math Policy | Cross-cutting / Target Profiles | `loeres-device`, `loeres-backend-static`, `xtask` |
| [012](proposed/012-validation-state-and-trusted-input-policy.md) | Validation State and Trusted Input Policy | Cross-cutting / Validation | `loeres`, all solve entrypoints |
| [013](proposed/013-conformance-corpus-and-numerical-parity.md) | Conformance Corpus and Numerical Parity Policy | Cross-cutting / Conformance | `conformance`, `xtask`, device/cluster examples |

## Done RFCs

| RFC | Title | Status | Notes |
|---:|---|---|---|
| [000](done/000-rfc-lifecycle-policy.md) | RFC Lifecycle Policy | Implemented | Governs RFC folders, state transitions, numbering, index integrity, and cross-reference hygiene. |
| [001](done/001-stratified-scalar.md) | Stratified Scalar Capability Model | Implemented (v0.6.0) | `loeres` `scalar` module; six tiers `BaseScalar` … `AdvancedNumericalScalar`, with `f32`/`f64` baseline impls. |
| [003](done/003-allocation-free-errors.md) | Allocation-Free Error Topology and Formatting Restrictions | Implemented (v0.4.0) | `loeres` `error` / `diagnostic` modules; `SolverError`, `DiagnosticSnapshot`, `error_code_to_str`. |
| [014](done/014-core-solver-outcome-state.md) | Core Solver Outcome and Status Taxonomy | Implemented (v0.5.0) | `loeres` `solver` module; `SolveStatus`, `TerminationReason`, `StepOutcome`, `SolveReport`, `AsCoreReport`. |

## Archived RFCs

None yet.

## Mechanical checks required before moving an RFC

1. `Status.` must match the folder state.
2. All relative RFC links must resolve after moving.
3. `xtask check-rfcs` must validate dependency boundaries and folder-status symmetry.
4. `xtask zero-bleed` must reject any transitive `std` or `alloc` edge into `loeres`, `loeres-backend-static`, or `loeres-device` baseline builds.
5. Numerical parity tests must compare equivalent problem instances across device and cluster paths within `epsilon = 1e-5`, not by bitwise identity.
