# Roadmap

Loeres is developed design-first. Progress is gated by design acceptance and
automated verification, not by calendar dates. The authoritative, detailed plan
lives in [`docs/specs/loeres-roadmap-milestones-v1.md`](docs/specs/loeres-roadmap-milestones-v1.md);
this file is a short summary.

> **Architecture recovery notice (2026-07-15, resolved 2026-07-30).** An
> independent architecture audit found four blockers in the v0.20.0 baseline:
> broken declared MSRV, release-tag/workflow mismatch, incomplete
> tag-revision gates, and materially stale normative specifications. v0.20.0
> was **No-Go** for release-readiness claims. The dependency-gated corrective
> schedule was the [Architecture Recovery Roadmap](docs/src/recovery-roadmap.md).
> RFC 019 and RFC 020 owned the blocking recovery and were Accepted (design
> frozen 2026-07-15) before Q2 staging. R0.5 lifecycle activation, the RFC
> 019 S1-S3 baseline, the RFC 020 S1 traceability matrix, reviewed S2 apex
> reconciliation, corrected S3 supporting documentation, corrected S4
> semantic currency checks, S5 integration, and RFC 019 S4/S5 gate
> implementation are owner-durable. Architecture review 024 accepted the
> non-publishing tagged evidence for canonical tag `0.20.1` at
> `ed282545...`, but RFC 021 later classified that immutable local tag as a
> permanently blocked, unpublished candidate (audit blocker B6). Reviews
> 031/032 accepted RFC 021 S1/S2, the project owner authorized one atomic Q2
> finalization revision, and that revision released successfully on
> 2026-07-30 as repository release `0.20.2`: the canonical tag peeled to the
> finalization revision, the release workflow reached a successful terminal
> conclusion, and its required evidence upload succeeded. Repository release
> `0.20.2` carries RFCs 001-021; this tree is `0.20.3` in development and is
> not itself a release. RFC 024 subsequently retired the one-release
> conditional apparatus from ordinary post-release development.

## Next (approved 2026-09-12)

Consolidation, then capability: land the authorized RFC 022–024 follow-ups and
RFC 023, add RFC 025 (codified amendment rule) and RFC 026 (enforced supply-chain
gate), cut **`0.21.0`** as the clean baseline, then open R4 with **RFC 027** — a
storage-agnostic QP contract in `loeres::problem` and a bounded polyhedral
projection extending the projected-first-order kernels to `Ax ≤ b`. Detailed
sequencing: detailed roadmap §7.

## Phases

- **Phase 0 — Repository & policy foundation.** Workspace skeleton, crate layout,
  CI/verification scaffolding, dependency-direction checks.
- **Phase 1 / Milestone 1 — `loeres`.** Stratified scalar capabilities,
  storage-agnostic access contracts, allocation-free error topology, and the
  core solver outcome/status taxonomy (RFCs 001–003, 014).
- **Phase 2 / Milestone 2 — Static backend & device.** Fixed-size storage,
  typed workspaces, and the first deterministic device solver (RFCs 004–006).
- **Phase 3 / Milestone 3 — Dynamic backend & cluster.** Heap/CSR adapters,
  orchestration, one dynamic PFO kernel, metadata observation, safe mock
  gateway, and process-local validation cache (RFCs 007–009, 015, 016).
- **Cross-layer.** Verification governance, target profiles, validation-state
  policy, bounded conformance, and cache/trust fixtures (RFCs 010–013, 017).

## Current status (0.20.3 development tree)

RFCs 001-021 are implemented. The current numerical breadth is one
box/bound-constrained projected-first-order family on device and cluster.
Conformance is a bounded smoke corpus; no broad LP/QP/SOCP, large-N,
throughput, or adversarial parity claim is made. Observability is metadata-
only, the gateway is mock-only, and validation caching is process-local.
RFCs 019/020/021 shipped in repository release `0.20.2`, released 2026-07-30
under RFC 021's conditional-finalization protocol; immutable local tag
`0.20.1` and its accepted evidence remain historical blocked-candidate
evidence only and are never reused. This `0.20.3` tree is in development and
is not itself a release; it carries RFC 024's ordinary post-release apex
form, which retires the one-release conditional apparatus and records this
tree's identity and the release it was last reconciled against (`0.20.2`),
not release status.

The `0.21.0` consolidation is underway in this tree. RFC 026 added an enforced
supply-chain gate (advisories, licenses, bans, sources, plus a per-edge-crate
zero-external-dependency assertion); RFC 024 Amendment 2 removed the retired
RFC 021 conditional module rather than suppressing it; RFC 022 Amendment 3 made
the evidence index's cited column and row count enforced assertions; RFC 025
codified in-place amendment of Accepted RFCs into RFC 000 with a `check-rfcs`
assertion that a shipped RFC carries no amendment. RFC 023 closed the three
user-facing obligations that `0.20.2` shipped without: two workspace-excluded
example crates with a `cargo xtask examples` gate asserting each one's resolved
dependency graph, `TERMS_OF_USE.md` (resolving requirements OQ-012), and the
user-facing book section that closes the persona gap. §5.9's example criterion is
satisfied by the gate rather than waived. RFC 027 remains next, after `0.21.0`
is cut.

The detailed entries below are chronological release history. Statements about
what was “next,” absent, or green apply only to the named historical revision
unless the current-status paragraph above repeats them.

**Milestone 1 complete — RFC 003, RFC 014, RFC 001, and RFC 002 implemented.**
`loeres` now ships the error/diagnostic topology (RFC 003), the solver
outcome/status taxonomy (RFC 014), the six-tier scalar capability model
(RFC 001 — `BaseScalar` … `AdvancedNumericalScalar`, with `f32`/`f64` baseline
impls), and the storage-agnostic access contracts (RFC 002 — `VectorAccess` /
`MatrixAccess` with mutable and contiguous-fast-path variants, the borrowed
`VectorView` / `MatrixView` reference views, `Dim2`, and `DimensionKind`). The
base-scalar ordering question is resolved: the architect chose **Direction B**
(base excludes ordering; ordering is `OrderedScalar`), recorded as ADR-017, and
Requirements §5.1.3 was amended to match. All gates pass; 62 tests.

### Historical completion: Milestone 2 — static backend + device kernel (RFC 004–006)

Milestone 1 (`loeres` core contracts) is closed. Milestone 2 is underway. The
static storage engine (**RFC 004**) is now **implemented (v0.8.0)**:
`loeres-backend-static` provides owned `FixedVector` / `FixedMatrix` (feature
`owned-arrays`) and the baseline contiguous static views over caller-owned
memory, with compile-time dimension invariants (const-assertion pattern
MSRV-validated on 1.85.0) and the RFC 002 access + contiguous fast-path traits
reporting `DimensionKind::Static`. The implementation-decision pass (D1–D6) was
accepted; advanced `static-views` are deferred (RFC 004 §7.2). All gates pass;
82 tests (62 core + 20 static backend).

The milestone continues with the typed workspace mechanics now done and the
first deterministic device solver kernel remaining. **RFC 005** is
**implemented (v0.9.0)**: the two-crate workspace boundary —
`loeres-backend-static::workspace` (the `WorkspaceFootprint` byte-footprint
contract, impls behind `owned-arrays`) and `loeres-device::workspace` /
`config` (the `DeviceWorkspace` / `DeviceWorkspaceDiagnostic` / `WorkspaceFor`
lifecycle plus `DeviceSolveConfig` / `TimingMode` with structural validation).
Concrete solver workspaces, problem families, the device report type, and the
solve kernel were RFC 006-owned. **RFC 006** is now **implemented (v0.10.0)**:
the baseline box/bound-constrained projected first-order device kernel —
`loeres-device::problem` (`ProjectedFirstOrderProblem`, a first-order-oracle +
box-bounds contract) and `loeres-device::solve` (the bounded-iteration
`solve_projected_first_order` kernel, the `DeviceSolveReport` outcome wrapping
the RFC 014 `SolveReport` via `AsCoreReport`, and the caller-owned
`ProjectedFirstOrderWorkspace` scratch), behind `owned-arrays`. Non-convergence
at the cap is an `Ok` status, never an error; the implementation-decision pass
(I1–I10) and departures are recorded in RFC 006 §7. **With RFC 004, 005, and 006
implemented, Milestone 2 is complete.** All gates pass; 109 tests (62 core + 22
static backend + 25 device).

RFC 002's optional contiguous fast path was used by the RFC 006 kernel (primal
and gradient via fixed-size slices; bounds via the contiguous slice with a
per-element fallback); the access traits bound only `BaseScalar` except where
they compare / project / tolerance-check.

### Historical rollout: Milestone 3 — dynamic backend & cluster (RFC 007 →)

Milestone 3 opens with the server-side dynamic storage foundation. **RFC 007** is
now **implemented (v0.11.0)**: `loeres-backend-std` gains dynamic dense and
sparse storage adapters — row-major `Vec`-backed `DenseVector` / `DenseMatrix`
implementing the full RFC 002 access surface (reads, mutable writes, and the
contiguous fast paths), and a CSR `SparseMatrix` with implicit-zero
`MatrixAccess::get`, a `try_get_stored` stored-vs-implicit extension, and `nnz`.
Triplet ingestion rejects duplicate coordinates and enforces optional
pre-allocation memory limits (`DenseIngestOptions` / `SparseIngestOptions`);
`validate_finite` helpers scan stored values. Construction errors map precisely
(zero/overflow extents → `InvalidDimension`; length/coordinate disagreements →
`DimensionMismatch` under a checked `u32` payload-fallback rule; duplicates /
limits → `InvalidInput`). The RFC is deliberately **storage-first**: it defines
no canonical validation-state type. That ownership stays with RFC 012, which is
sequenced next — **before** RFC 008/009 depend on validated/trusted-input
semantics. The implementation-decision pass (I1–I10, I3 CSR / I7 minimal
extension) and the storage-first split are recorded in RFC 007. v0.11.1 hardened
construction: empty `DenseVector` and extreme-`rows` `SparseMatrix` fail closed
(`InvalidDimension`), an additive `SparseIngestOptions::max_rows` cap bounds the
CSR `row_ptr` buffer, and the sparse buffers use `try_reserve_exact`
defense-in-depth. All gates pass; 139 tests (62 core + 22 static backend + 32
device + 23 dynamic backend).

**RFC 012** is now **implemented (v0.12.0)**: the core-first validation-state
vocabulary in `loeres::validation` — `ValidationScope` (a `repr(transparent)`
coverage bitset with a release-local `ALL`), `FiniteCoverage`
(`Checked` / `NotApplicable`), a `#[non_exhaustive]` `TrustKind`, `TrustToken`,
the `ValidationCoverage` recording descriptor, `TrustedByCaller` evidence, and
the `ValidationState` category enum. RFC 012 owns only the representation (I9):
it runs no scans and changes no shipped solver signature — backends remain the
validators and record their outcome here, while structural validity stays a
construction precondition (RFC 004 / 007). Finite-not-applicable is kept distinct
from a missing-capability *unavailable* (rejected, not validated). Cluster
trusted-pipeline / caching are deferred to RFC 008, the shared conformance corpus
to RFC 013. The implementation-decision pass (I1–I11) is recorded in RFC 012
(§7). v0.12.1 made `ValidationCoverage` coherent by construction (private fields,
scope normalized to always include `FINITE`, accessors) so the scope bit and the
`finite` field cannot contradict before RFC 008 consumes them. All gates pass;
148 tests (71 core + 22 static backend + 32 device + 23 dynamic backend).

**RFC 008** is now **implemented (v0.13.0)**: the orchestration-first `loeres-cluster` slice populating `batch` / `runtime` / `solve`. It delivers the per-item batch contract (`BatchSolveReport`, `BatchItemOutcome`, an explicit-count `BatchSummary`, `ClusterSolution`), a runtime-agnostic configuration / cancellation / executor layer (`ClusterSolveConfig`, a cluster-owned `ClusterCancellationToken` over `Arc<AtomicBool>` with `Release` / `Acquire` semantics, and `parallel-rayon` / `async-tokio` backends behind feature gates that keep Tokio and Rayon out of the baseline public surface), and the `ClusterJob` hybrid-dispatch seam. It consumes the RFC 012 vocabulary through `ClusterValidationPolicy` (`ValidateAllInputs` / `RespectBackendValidationState` / `TrustedByCaller`), rejecting missing required coverage rather than silently trusting it. The status/error split holds at the batch layer: a bounded-terminus non-convergence is a *solved* `NotConverged` item, never `Failed`, and a contained worker panic is `Panicked` (only under `panic = "unwind"`; under `panic = "abort"` the process aborts and none is promised). Per the decision freeze (D1), this is orchestration **infrastructure, not a production cluster solver** — no std-side solver kernel exists yet (core exposes only the outcome vocabulary; the device kernel is edge-only and unreachable from cluster), so `ClusterJob` is the stable plug-in seam and the machinery is exercised by deterministic test jobs (orchestration behavior, not numerical correctness). The std-side kernel and trusted-pipeline / caching are deferred to a follow-on cluster RFC; the size-budget gate to RFC 010. The implementation-decision pass (D1–D10, T1–T3) is recorded in the RFC. v0.13.1 applied the implementation-review corrections (B1–B6): `ClusterValidationPolicy::resolve` is now pure policy/evidence resolution that runs no scans and never fabricates a `Validated` state, timeout deadlines use `checked_add` (overflow → `InvalidConfig`), the crate and root READMEs and the DONE RFC body were reconciled to the shipped API, and the cancellation-poll interval is documented as a job-internal hint. All gates pass; 180 tests (71 core + 22 static backend + 32 device + 23 dynamic backend + 32 cluster).

**RFC 016** is now **implemented (v0.14.0)**: the first production std-side numerical kernel for `loeres-cluster`, populating `model` and `solve::projected_first_order`. It is a dynamic box/bound-constrained projected first-order solver over `loeres-backend-std` `DenseVector` — the dynamic-storage analog of the RFC 006 device kernel — exposed as a typed entrypoint (`solve_projected_first_order_dyn` over a `ClusterProjectedFirstOrderProblem` oracle plus a reusable single-scratch `ClusterProjectedFirstOrderWorkspace`) and an erased `ClusterProjectedFirstOrderJob` plugged into the RFC 008 `ClusterJob` seam, graduating cluster orchestration from deterministic test jobs to real solving. Convergence is step-norm (`max_i |Δxᵢ| ≤ tolerance`), iteration counting mirrors RFC 006 (`converged_early` on the converging step, `not_converged_cap` at the cap), and the status/error split holds — non-convergence at the cap is a *solved* `NotConverged` item, never `Failed`. Validation follows RFC 012: structural checks (dimension/bound shape, finite `lo ≤ hi`, `step_scale` finite and `> 0`, config) always run, while policy-governed `FINITE` scans of bounds and the initial iterate may be skipped under `TrustedByCaller`; hot-loop finiteness checks are never skippable, so an in-loop non-finite gradient/bound/candidate maps to `NumericalDomain` even under trust and a trusted solve never yields `Solved` over NaN/Inf. The `ProjectedFirstOrderSolveRecord` carries a `checked_scope` plus a named `ProjectedFirstOrderFiniteEvidence` (`Scanned` / `Trusted(..)` / `DomainInapplicable`) that records exactly what the kernel verified versus what the caller asserted — honestly, without misencoding a trusted-away scan as `Checked` or `NotApplicable` (the v0.14.1 implementation-review correction; RFC §7). Trusted-pipeline / model-identity caching over this typed surface is deferred to RFC 015. All gates pass on both the working tree and a clean extraction; 198 tests (71 core + 22 static backend + 32 device + 23 dynamic backend + 50 cluster).

**RFC 009** is now **implemented (v0.15.0)**: `loeres-cluster::observe` exposes metadata-only telemetry categories (`SolverFamilyId`, `ProblemClassId`, `DimensionBucket`, `IterationsBucket`, `ElapsedBucket`, `OutcomeKind`), redacted static labels, `SolveTelemetryEvent`, explicit `SolveObserver` / `NoopObserver` sinks, pure classifiers over `SolverError` and `BatchItemOutcome`, `observe_batch_report`, and the thin `solve_batch_observed` wrapper. The classifier is total over the current `SolverError` variants and fails closed to `InternalError` for future unclassified variants; non-convergence stays a solved status and `ClusterError` remains batch-level. `loeres-cluster::gateway` exposes the safe gateway boundary categories and a pure Rust `MockGatewayJob` that tests solved/not-converged reports, structured gateway failure mapping, cancellation/timeout normalization, and adapter-side rejection of unwrapped `SingleThreadOnly` backends. No concrete native solver adapter, modeling DSL, trusted validation cache, tracing dependency, or metrics dependency ships in this RFC. RFC 008's `ClusterJob` seam and RFC 014's status/error split remain intact, and zero-bleed keeps observability/gateway concerns out of core/static/device crates. All working-tree gates pass; the clean-copy release gate also passes. 216 tests (71 core + 22 static backend + 32 device + 23 dynamic backend + 68 cluster).

**RFC 010** is now **implemented (v0.16.1)**: `xtask` has a stable verification command namespace and `cargo xtask check` is the canonical aggregate release gate (`release-gate` remains an alias). The aggregate runs host checking, zero-bleed, no-std, feature matrix, interim target profiles, RFC lifecycle/link validation, public API scanning (including cluster runtime-leak checks), panic audit, size-budget reporting, unsafe audit, conformance hook, and repository link audit. Gate output classifies checks as enforced, advisory/reporting, or owner-RFC hooks: size-budget is an advisory baseline until owner RFCs freeze thresholds, and conformance reports `not-enforced` until RFC 013 supplies fixtures. Runtime crate APIs are unchanged from v0.15.0.

**RFC 011** is now **implemented (v0.17.0)**: target-profile governance is manifest-driven through `xtask/target-profiles.toml`. `cargo xtask target-profiles` validates schema version, unique profile names, required metadata, profile classes, and buildable-vs-documented-only fields; records `rustc -vV` host metadata; enforces `cluster-linux-host` and `device-thumbv7em-hardfloat`; reports optional installed targets as advisory; lists `wasm32-no-threads` and `cluster-linux-aarch64` as documented-only; and emits RFC 013 conformance groups as metadata only. Device hard-float builds are invoked with manifest rustflags `-C panic=abort`, without claiming formal panic absence. Runtime crate APIs are unchanged.

**RFC 013** is now **implemented (v0.18.0)**: the repository has an enforced `conformance/` smoke corpus and `cargo xtask conformance` now runs real parity checks instead of reporting a pending hook. The smoke suite covers the shared projected-first-order family over a dimension-2 diagonal box quadratic problem with converged, iteration-cap, and invalid-bound fixtures. The runner materializes fixtures host-side through `xtask`, calls the real RFC 006 device solver and RFC 016 cluster solver, compares status/error categories and solution vectors with tolerances, and reports objective/residual categories explicitly as `not-applicable` for this slice. Runtime crates do not parse fixtures and runtime APIs are unchanged.

**RFC 015** is now **implemented (v0.19.0)**: `loeres-cluster` owns a cluster-only validation evidence cache. The release adds Loeres-generated `ModelIdentity`, `MutationEpoch`, exact evidence keys/lookups, `ValidationEvidenceCache`, `ProvidedValidationEvidence`, the `CacheableProjectedFirstOrderProblem<P>` carrier, and a carrier-only cached `f64` projected-first-order solve path. Reusable evidence is model-owned only: current iterates, workspace/config checks, current `step_scale`, cancellation, and hot-loop finite checks remain active. Wrong identity or stale epoch fails closed with `InvalidInput`; cache misses or insufficient model-owned scope scan and continue when data is valid. The final v0.19.0 release advances mutation epochs before mutable model access, so stale evidence fails closed even if mutation later returns an error or unwinds. Edge crates and runtime crate APIs outside `loeres-cluster` are unchanged.

**RFC 017** is now **implemented (v0.20.0)**: the enforced `conformance/smoke/`
corpus covers RFC 015 trusted/cache semantics through `schema_version = 2`
fixtures. The runner now materializes cache hit/miss, insufficient-scope,
stale-epoch, wrong-identity, current-iterate non-finite, hot-loop
numerical-domain, trusted-evidence insertion rejection, and sentinel-identity
insertion rejection cases. Successful cache cases compare against the device and
cluster `ValidateAllInputs` baselines; fail-closed cases compare structured
`SolverError` categories. Runtime APIs are unchanged.

### Open design rounds (gate later-milestone *content*, not the skeleton)

1. RFC 006 — box/bound-constrained first device kernel scope (Milestone 2). **Resolved — implemented (v0.10.0); Milestone 2 complete.**
2. RFC 007 — dynamic dense/sparse storage adapters (Milestone 3). **Resolved — implemented (v0.11.0) storage-first; canonical validation-state ownership deferred to RFC 012.**
3. RFC 012 — validation-state and trusted-input policy (Milestone 3). **Resolved — implemented (v0.12.0) core-first; cluster trusted-pipeline / caching deferred to RFC 008, conformance corpus to RFC 013.**
4. RFC 008 — async orchestration and monomorphization budgets (Milestone 3). **Resolved — implemented (v0.13.0) orchestration-first, with v0.13.1 implementation-review corrections (B1–B6); the std-side solver kernel landed as RFC 016 (v0.14.0), trusted-pipeline / caching deferred to RFC 015, the size-budget gate to RFC 010.**
5. RFC 016 — std-side projected first-order cluster kernel (Milestone 3). **Resolved — implemented (v0.14.0); first std-side numerical kernel, plugged into the RFC 008 `ClusterJob` seam; trusted-pipeline / caching deferred to RFC 015.**
6. RFC 009 — observability, metrics, and FFI gateway interfacing (Milestone 3). **Resolved — implemented (v0.15.0); metadata-only observability and safe mock gateway boundary, with concrete native adapters deferred to adapter-specific follow-up design.**
7. RFC 010 — xtask verification governance. **Resolved — implemented (v0.16.1); `cargo xtask check` is the aggregate gate, with enforced/advisory/hook result classes for target/profile/API/size/unsafe/link/conformance checks.**
8. RFC 011 — target profiles and deterministic math policy. **Resolved — implemented (v0.17.0); manifest-driven target-profile checks with mandatory/advisory/documented-only evidence classes.**
9. RFC 013 — conformance corpus and numerical parity policy. **Resolved — implemented (v0.18.0); enforced smoke corpus for device/cluster projected-first-order parity, with extended/adversarial placeholders staged.**
10. RFC 015 — trusted pipeline validation cache. **Resolved — implemented (v0.19.0); cluster-only model identity, mutation epochs, validation evidence cache, and carrier-only cached `f64` projected-first-order solving, with fail-closed mutation epochs.**
11. RFC 017 — trusted/cache conformance fixtures. **Resolved — implemented (v0.20.0); enforced smoke fixtures now cover RFC 015 validation-cache hit/miss, stale trust, scan-retention, and cache-insert rejection behavior.**
12. RFC 019 — release integrity and MSRV recovery. **Resolved — implemented and shipped in `0.20.2` (released 2026-07-30); release gates and package evidence closed under RFC 021's protocol.**
13. RFC 020 — normative documentation authority and currency. **Resolved — implemented and shipped in `0.20.2`; the apex trio reconciliation is current, and RFC 024 keeps it current across ordinary post-release development.**
14. RFC 021 — conditional release finalization. **Resolved — implemented and shipped in `0.20.2`; the conditional-finalization protocol released successfully, and RFC 024 retires its apparatus from ordinary development going forward.**
