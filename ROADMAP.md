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

### Next: Milestone 2 — static backend + device kernel (RFC 004–006)

Milestone 1 (`loeres` core contracts) is closed. The path continues with the
static storage engine (RFC 004), typed workspace mechanics (RFC 005), and the
first deterministic device solver kernel (RFC 006), all on `loeres-backend-static`
and `loeres-device`. RFC 002's optional contiguous fast path was scoped for the
RFC 006 kernel; the access traits bound only `BaseScalar` except where they
compare / project / tolerance-check.

### Open design rounds (gate later-milestone *content*, not the skeleton)

1. RFC 006 — box/bound-constrained first device kernel scope (Milestone 2).
2. RFCs 007 / 008 / 012 — validation-state reconciliation (Milestone 3).
