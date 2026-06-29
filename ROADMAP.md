# Roadmap

Loeres is developed design-first. Progress is gated by design acceptance and
automated verification, not by calendar dates. The authoritative, detailed plan
lives in [`docs/specs/loeres-roadmap-milestones-v1.md`](docs/specs/loeres-roadmap-milestones-v1.md);
this file is a short summary.

## Phases

- **Phase 0 — Repository & policy foundation.** Workspace skeleton, crate layout,
  CI/verification scaffolding, dependency-direction checks.
- **Phase 1 / Milestone 1 — `loeres`.** Stratified scalar capabilities,
  storage-agnostic access contracts, allocation-free error topology, and the
  core solver outcome/status taxonomy (RFCs 001–003, 014).
- **Phase 2 / Milestone 2 — Static backend & device.** Fixed-size storage,
  typed workspaces, and the first deterministic device solver (RFCs 004–006).
- **Phase 3 / Milestone 3 — Dynamic backend & cluster.** Heap/sparse adapters,
  async orchestration, observability, and the optional FFI gateway
  (RFCs 007–009).
- **Cross-layer.** Verification governance, target profiles, validation-state
  policy, and the conformance corpus (RFCs 010–013).

## Current status (v0.7.0)

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

### Complete: Milestone 2 — static backend + device kernel (RFC 004–006)

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

### Open design rounds (gate later-milestone *content*, not the skeleton)

1. RFC 006 — box/bound-constrained first device kernel scope (Milestone 2). **Resolved — implemented (v0.10.0); Milestone 2 complete.**
2. RFCs 007 / 008 / 012 — validation-state reconciliation (Milestone 3).
