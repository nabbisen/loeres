# Loeres RFC Index

This directory is the authoritative RFC registry for the Loeres library family.

RFC numbers are stable forever. Moving an RFC between `proposed/`, `done/`, and `archive/` changes its lifecycle state, but never changes its number or slug.

## Proposed RFCs

None.

## Done RFCs

| RFC | Title | Status | Notes |
|---:|---|---|---|
| [000](done/000-rfc-lifecycle-policy.md) | RFC Lifecycle Policy | Implemented | Governs RFC folders, state transitions, numbering, index integrity, and cross-reference hygiene. |
| [001](done/001-stratified-scalar.md) | Stratified Scalar Capability Model | Implemented (v0.6.0) | `loeres` `scalar` module; six tiers `BaseScalar` … `AdvancedNumericalScalar`, with `f32`/`f64` baseline impls. |
| [002](done/002-storage-agnostic-contracts.md) | Storage-Agnostic Matrix and Vector Access Contracts | Implemented (v0.7.0) | `loeres` `access` / `dimension` modules; `VectorAccess` / `MatrixAccess` (+ mut + contiguous fast path), `VectorView` / `MatrixView`, `Dim2`, `DimensionKind`. Closes Milestone 1. |
| [003](done/003-allocation-free-errors.md) | Allocation-Free Error Topology and Formatting Restrictions | Implemented (v0.4.0) | `loeres` `error` / `diagnostic` modules; `SolverError`, `DiagnosticSnapshot`, `error_code_to_str`. |
| [004](done/004-static-storage-engine.md) | Const-Generic and Fixed-Size Static Storage Engine | Implemented (v0.8.0) | `loeres-backend-static` `dimension`/`array`/`view`; owned `FixedVector`/`FixedMatrix` (`owned-arrays`) + baseline contiguous static views, const-assert dimension invariants, RFC 002 traits reporting `Static`. Advanced `static-views` deferred (§7.2). |
| [005](done/005-typed-workspace-mechanics.md) | Caller-Owned Typed Workspace Mechanics and Poison-Free Reuse | Implemented (v0.9.0) | `loeres-backend-static::workspace` footprint contract + `loeres-device` workspace lifecycle (`DeviceWorkspace`/`DeviceWorkspaceDiagnostic`/`WorkspaceFor`) and `config` (`DeviceSolveConfig`/`TimingMode`). Concrete workspaces/kernel deferred to RFC 006. |
| [006](done/006-deterministic-solver-kernel.md) | Baseline Deterministic Device Solver Kernel | Implemented (v0.10.0) | `loeres-device` `problem`/`solve`; box/bound-constrained projected first-order kernel (`ProjectedFirstOrderProblem`, `solve_projected_first_order`, `DeviceSolveReport`, `ProjectedFirstOrderWorkspace`) behind `owned-arrays`, outcomes via RFC 014 `SolveReport` + `AsCoreReport`. Completes Milestone 2. |
| [007](done/007-dynamic-sparse-adapters.md) | Dynamic Dense and Sparse Storage Adapters | Implemented (v0.11.0) | `loeres-backend-std` `dense`/`sparse`; row-major `Vec`-backed `DenseVector`/`DenseMatrix` (full RFC 002 mutable + contiguous traits) and a CSR `SparseMatrix` (implicit-zero `get`, `try_get_stored`, `nnz`), triplet ingestion with duplicate rejection + memory-limit options, `validate_finite` helpers. Canonical validation state deferred to RFC 012. Opens Milestone 3. |
| [008](done/008-async-orchestration-budgets.md) | Async Orchestration and Monomorphization Budgets | Implemented (v0.13.0) | `loeres-cluster` `batch`/`runtime`/`solve`; orchestration-first slice — per-item batch contract (`BatchSolveReport`/`BatchItemOutcome`/`BatchSummary`/`ClusterSolution`), runtime-agnostic config/cancellation/executor layer (`ClusterSolveConfig`, `ClusterCancellationToken`, `parallel-rayon`/`async-tokio` gated), and the `ClusterJob` hybrid-dispatch seam; consumes the RFC 012 validation vocabulary (`ClusterValidationPolicy`). Orchestration machinery exercised by deterministic test jobs — not a production cluster solver; std-side kernel + trusted-pipeline/caching deferred to a follow-on RFC, the size-budget gate to RFC 010. |
| [009](done/009-observability-ffi-gateways.md) | Observability, Metrics, and FFI Gateway Interfacing | Implemented (v0.15.0) | `loeres-cluster` `observe`/`gateway`; metadata-only telemetry categories, bounded labels, total `SolverError`/`BatchItemOutcome` classifiers, explicit observer sinks, `observe_batch_report`, `solve_batch_observed`, safe gateway boundary categories, and a pure Rust `MockGatewayJob`. No concrete native solver adapter ships in this RFC. |
| [010](done/010-xtask-verification-governance.md) | xtask Verification Governance | Implemented (v0.16.1) | `xtask` command namespace and aggregate gate: `check` / `release-gate`, RFC lifecycle/link checks, zero-bleed, no-std, feature matrix, interim target profiles, public API scan including cluster runtime-leak checks, panic audit, advisory size-budget reporting, unsafe audit, RFC 013 conformance hook, and local link audit. |
| [011](done/011-target-profiles-and-deterministic-math.md) | Target Profiles and Deterministic Math Policy | Implemented (v0.17.0) | `xtask/target-profiles.toml` and manifest-driven `cargo xtask target-profiles`; mandatory `cluster-linux-host` and `device-thumbv7em-hardfloat`, advisory-installed device portability profiles, documented-only WASM/aarch64 profiles, manifest rustflags for device `panic=abort`, and RFC 013 conformance-group metadata. |
| [012](done/012-validation-state-and-trusted-input-policy.md) | Validation State and Trusted Input Policy | Implemented (v0.12.0) | `loeres` `validation` module; `ValidationScope` (coverage bitset), `FiniteCoverage`, `TrustKind`, `TrustToken`, `ValidationCoverage`, `TrustedByCaller`, `ValidationState`. Core-first vocabulary; cluster trusted-pipeline / caching deferred to RFC 008, shared conformance corpus to RFC 013. |
| [013](done/013-conformance-corpus-and-numerical-parity.md) | Conformance Corpus and Numerical Parity Policy | Implemented (v0.18.0) | `conformance/` smoke fixtures and `cargo xtask conformance`; enforced device/cluster parity for the dimension-2 projected-first-order diagonal box quadratic family, with extended/adversarial placeholders staged. |
| [014](done/014-core-solver-outcome-state.md) | Core Solver Outcome and Status Taxonomy | Implemented (v0.5.0) | `loeres` `solver` module; `SolveStatus`, `TerminationReason`, `StepOutcome`, `SolveReport`, `AsCoreReport`. |
| [015](done/015-trusted-pipeline-validation-cache.md) | Trusted Pipeline Validation Cache | Implemented (v0.19.0) | `loeres-cluster::validation_cache`; model identity, mutation epochs, `ValidationEvidenceCache`, `CacheableProjectedFirstOrderProblem`, and carrier-only cached `f64` projected-first-order solving, with fail-closed mutation epochs. |
| [016](done/016-std-side-projected-first-order-cluster-kernel.md) | Std-Side Projected First-Order Cluster Kernel | Implemented (v0.14.0) | `loeres-cluster` `model`/`solve`; first std-side numerical kernel — dynamic box/bound-constrained projected first-order over `DenseVector` (`ClusterProjectedFirstOrderProblem`, `ClusterProjectedFirstOrderWorkspace`, `ProjectedFirstOrderConfig`, `solve_projected_first_order_dyn`, `ClusterProjectedFirstOrderJob`), step-norm convergence aligned with RFC 006, two-field `ProjectedFirstOrderSolveRecord` (`checked`/`trust`), plugged into the RFC 008 `ClusterJob` seam. Trusted-pipeline/caching deferred to RFC 015. |

## Archived RFCs

None yet.

## Mechanical checks required before moving an RFC

1. `Status.` must match the folder state.
2. All relative RFC links must resolve after moving.
3. `xtask check-rfcs` must validate RFC lifecycle state, index coverage, and relative links.
4. `xtask zero-bleed` must reject any transitive `std` or `alloc` edge into `loeres`, `loeres-backend-static`, or `loeres-device` baseline builds.
5. Numerical parity tests must compare equivalent problem instances across device and cluster paths within `epsilon = 1e-5`, not by bitwise identity.
