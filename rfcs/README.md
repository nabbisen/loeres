# Loeres RFC Index

This directory is the authoritative RFC registry for the Loeres library family.

RFC numbers are stable forever. Moving an RFC between `proposed/`, `accepted/`,
`done/`, and `archive/` changes its lifecycle state, but never changes its
number or slug. Loeres uses RFC 000's five-folder variant: implementation may
start only from `accepted/`.

Normative documents cite architecture reviews as evidence that a decision was
considered. The [review evidence index](review-evidence-index.md) resolves
each citation to a hash-pinned, tiered identity record; it binds evidence
identity only and confers no authority (RFC 022).

## Proposed RFCs

| RFC | Title | Status | Notes |
|---:|---|---|---|
None currently.

## Accepted RFCs

Design-frozen implementation contracts. Implementation is authorized; shipped
behavior is not yet claimed.

| RFC | Title | Status | Notes |
|---:|---|---|---|
| [027](accepted/027-qp-contract-and-constrained-kernel.md) | QP Contract and Linearly Constrained Projected Kernel | Accepted (design frozen 2026-09-12) | Activates `loeres::problem` with storage-agnostic QP traits (PF-002; PF-001 contract-only) and extends the PFO kernels with a bounded Hildreth/Dykstra polyhedral projection for `Ax ≤ b`; `m = 0` reproduces RFC 006/016 exactly. IPM excluded. Review 043 (R1 Dykstra box set, R2 terminal violation) applied. R4 first capability. |

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
| [010](done/010-xtask-verification-governance.md) | xtask Verification Governance | Implemented (v0.16.1) | `xtask` developer command namespace and aggregate `check`: RFC/link, zero-bleed, no-std, feature, target, public-API, panic, size, unsafe, conformance, and link evidence. RFC 019 supersedes the original `release-gate` alias: package/readiness is now distinct and fail-closed during recovery. |
| [011](done/011-target-profiles-and-deterministic-math.md) | Target Profiles and Deterministic Math Policy | Implemented (v0.17.0) | `xtask/target-profiles.toml` and manifest-driven `cargo xtask target-profiles`; mandatory `cluster-linux-host` and `device-thumbv7em-hardfloat`, advisory-installed device portability profiles, documented-only WASM/aarch64 profiles, manifest rustflags for device `panic=abort`, and RFC 013 conformance-group metadata. |
| [012](done/012-validation-state-and-trusted-input-policy.md) | Validation State and Trusted Input Policy | Implemented (v0.12.0) | `loeres` `validation` module; `ValidationScope` (coverage bitset), `FiniteCoverage`, `TrustKind`, `TrustToken`, `ValidationCoverage`, `TrustedByCaller`, `ValidationState`. Core-first vocabulary; process-local cluster caching followed in RFC 015 and shared conformance in RFC 013/RFC 017. |
| [013](done/013-conformance-corpus-and-numerical-parity.md) | Conformance Corpus and Numerical Parity Policy | Implemented (v0.18.0) | `conformance/` smoke fixtures and `cargo xtask conformance`; enforced device/cluster parity for the dimension-2 projected-first-order diagonal box quadratic family, with extended/adversarial placeholders staged. |
| [014](done/014-core-solver-outcome-state.md) | Core Solver Outcome and Status Taxonomy | Implemented (v0.5.0) | `loeres` `solver` module; `SolveStatus`, `TerminationReason`, `StepOutcome`, `SolveReport`, `AsCoreReport`. |
| [015](done/015-trusted-pipeline-validation-cache.md) | Trusted Pipeline Validation Cache | Implemented (v0.19.0) | `loeres-cluster::validation_cache`; model identity, mutation epochs, `ValidationEvidenceCache`, `CacheableProjectedFirstOrderProblem`, and carrier-only cached `f64` projected-first-order solving, with fail-closed mutation epochs. |
| [016](done/016-std-side-projected-first-order-cluster-kernel.md) | Std-Side Projected First-Order Cluster Kernel | Implemented (v0.14.0) | `loeres-cluster` `model`/`solve`; first std-side numerical kernel — dynamic box/bound-constrained projected first-order over `DenseVector` (`ClusterProjectedFirstOrderProblem`, `ClusterProjectedFirstOrderWorkspace`, `ProjectedFirstOrderConfig`, `solve_projected_first_order_dyn`, `ClusterProjectedFirstOrderJob`), step-norm convergence aligned with RFC 006, `ProjectedFirstOrderSolveRecord { report, checked_scope, finite }` with explicit `Scanned` / `Trusted(..)` / `DomainInapplicable` finite evidence, plugged into the RFC 008 `ClusterJob` seam. Process-local caching followed in RFC 015. |
| [017](done/017-trusted-cache-conformance-fixtures.md) | Trusted/Cache Conformance Fixtures | Implemented (v0.20.0) | Extends RFC 013 conformance with enforced RFC 015 trusted/cache fixture states: cache hit/miss, insufficient scope, stale epoch, wrong identity, current-iterate scan retention, hot-loop fail-safe retention, and reusable-cache rejection cases. |
| [018](done/018-cluster-solve-test-helper-cleanup.md) | Cluster Solve Test Helper Cleanup | Implemented (v0.20.0) | Small test-only cleanup for `loeres-cluster` solve tests: removed the opaque `kinds()` helper while preserving feature-gated sequential/parallel and sync/async comparisons. |
| [019](done/019-release-integrity-and-msrv-recovery.md) | Release Integrity and MSRV Recovery | Implemented (v0.20.2) | MSRV/release-gate recovery; shipped in repository release `0.20.2` (released 2026-07-30). [Handoff](handoffs/019-release-integrity-and-msrv-recovery/implementation-handoff.md). |
| [020](done/020-normative-documentation-authority-and-currency.md) | Normative Documentation Authority and Currency | Implemented (v0.20.2) | Apex trio reconciliation and documentation-authority hierarchy; shipped in `0.20.2`. RFC 024 keeps the apex current across ordinary post-release development. [Handoff](handoffs/020-normative-documentation-authority-and-currency/implementation-handoff.md). |
| [021](done/021-conditional-release-finalization.md) | Conditional Release Finalization | Implemented (v0.20.2) | Defined and executed the conditional-finalization protocol that shipped `0.20.2`; tag `0.20.2` peeled to the finalization revision and the release workflow succeeded 2026-07-30. [Handoff](handoffs/021-conditional-release-finalization/implementation-handoff.md). |
| [022](done/022-architecture-review-evidence-index.md) | Architecture Review Evidence Index | Implemented (v0.21.0) | Tracked, hash-pinned identity register for the architecture reviews cited by normative documents; fail-closed citation resolution and cited-column agreement from tracked bytes, with hash verification, coverage symmetry, and row count reported unavailable when the maintainer-held corpus is absent. Evidence identity only — never a lifecycle. Amendments 1-3 were made while Accepted. See the [evidence index](review-evidence-index.md). [Handoff](handoffs/022-architecture-review-evidence-index/implementation-handoff.md). |
| [023](done/023-user-facing-surface-and-obligation-closure.md) | User-Facing Surface and Documented-Obligation Closure | Implemented (v0.21.0) | Closed three unmet documented obligations: workspace-excluded `examples/` with a resolved-graph isolation assertion (`cargo xtask examples`), `TERMS_OF_USE.md` engineering-use limits (resolves OQ-012), and the user-guide/verification book pages that close the persona gap. Amendment 2 made the examples self-rooted after the clean-extraction suite found `[workspace] exclude` insufficient. [Handoff](handoffs/023-user-facing-surface-and-obligation-closure/implementation-handoff.md). |
| [024](done/024-post-release-documentation-steady-state.md) | Post-Release Documentation Steady State | Implemented (v0.21.0) | `doc-currency` admitted only a `0.20.2`-pinned conditional apex form and an expired pre-release draft form, so no ordinary commit could pass the gate after a release. Adds a this-tree/last-reconciled-release apex form (identity and lineage, never release status, strict inequality), binds implemented scope to the exact `rfcs/done/` set, and retires the one-release conditional apparatus — Amendment 2 removing its `xtask` module outright rather than suppressing it. [Handoff](handoffs/024-post-release-documentation-steady-state/implementation-handoff.md). |
| [025](done/025-in-place-amendment-of-accepted-rfcs.md) | In-Place Amendment of Accepted RFCs | Implemented (v0.21.0) | Codified the exception exercised by RFC 024 §0.4 and RFC 022 §0.3 as one RFC 000 section: Accepted-not-done RFCs may carry numbered amendments; `done/` RFCs are superseded, never amended. `check-rfcs` enforces the `done/` half as **accountability, not absence** — a shipped RFC's amendment headings must each be named by its Status line (Amendment 1, after the first attempted transition proved absence barred every legitimately amended RFC from shipping). Consolidation C.1. [Handoff](handoffs/025-in-place-amendment-of-accepted-rfcs/implementation-handoff.md). |
| [026](done/026-supply-chain-gate.md) | Supply-Chain Gate | Implemented (v0.21.0) | `cargo deny` advisories/licenses/bans/sources as an **enforced** gate in `check` and `release-gate`, plus a per-edge-crate zero-external-dependency assertion as a second zero-bleed witness. Found and remediated RUSTSEC-2026-0204 on arrival with no policy relaxation. Consolidation C.1. [Handoff](handoffs/026-supply-chain-gate/implementation-handoff.md). |

## Archived RFCs

None yet.

## Mechanical checks required before moving an RFC

1. `Status.` must match the folder state.
2. All relative RFC links must resolve after moving.
3. `xtask check-rfcs` must validate RFC lifecycle state, index coverage, and relative links.
4. `xtask zero-bleed` must reject any transitive `std` or `alloc` edge into `loeres`, `loeres-backend-static`, or `loeres-device` baseline builds.
5. Numerical parity tests must compare equivalent problem instances across device and cluster paths within `epsilon = 1e-5`, not by bitwise identity.
