# Roadmap

Loeres is developed design-first. Progress is gated by design acceptance and
automated verification, not by calendar dates. The authoritative, detailed plan
lives in [`docs/specs/loeres-roadmap-milestones-v1.md`](docs/specs/loeres-roadmap-milestones-v1.md);
this file is a short summary.

## Phases

- **Phase 0 — Repository & policy foundation.** Workspace skeleton, crate layout,
  CI/verification scaffolding, dependency-direction checks.
- **Phase 1 / Milestone 1 — `loeres-core`.** Stratified scalar capabilities,
  storage-agnostic access contracts, allocation-free error topology, and the
  core solver outcome/status taxonomy (RFCs 001–003, 014).
- **Phase 2 / Milestone 2 — Static backend & device.** Fixed-size storage,
  typed workspaces, and the first deterministic device solver (RFCs 004–006).
- **Phase 3 / Milestone 3 — Dynamic backend & cluster.** Heap/sparse adapters,
  async orchestration, observability, and the optional FFI gateway
  (RFCs 007–009).
- **Cross-layer.** Verification governance, target profiles, validation-state
  policy, and the conformance corpus (RFCs 010–013).

## Current status (v0.6.0)

**Milestone 1 nearly complete — RFC 003, RFC 014, and RFC 001 implemented.**
`loeres-core` now ships the error/diagnostic topology (RFC 003), the solver
outcome/status taxonomy (RFC 014), and the six-tier scalar capability model
(RFC 001 — `BaseScalar` … `AdvancedNumericalScalar`, with `f32`/`f64` baseline
impls). The base-scalar ordering question is resolved: the architect chose
**Direction B** (base excludes ordering; ordering is `OrderedScalar`), recorded
as ADR-017, and Requirements §5.1.3 was amended to match. All gates pass; 36
tests.

### Next: Milestone 1 remainder — RFC 002 (access)

Sequence: ~~RFC 003 (errors)~~ ✓ → ~~RFC 014 (solver status)~~ ✓ →
~~RFC 001 (six-tier scalars)~~ ✓ → RFC 002 (access contracts). RFC 002 is now
**unblocked**: it bounds storage/access on `BaseScalar` and names
`OrderedScalar` / `MetricScalar` only on APIs that compare, project, or evaluate
tolerance (per the architect's note).

### Open design rounds (gate later-milestone *content*, not the skeleton)

1. RFC 006 — box/bound-constrained first device kernel scope (Milestone 2).
2. RFCs 007 / 008 / 012 — validation-state reconciliation (Milestone 3).
