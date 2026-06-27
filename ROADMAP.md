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

## Current status (v0.6.4)

**Milestone 1 nearly complete — RFC 003, RFC 014, and RFC 001 implemented.**
`loeres` now ships the error/diagnostic topology (RFC 003), the solver
outcome/status taxonomy (RFC 014), and the six-tier scalar capability model
(RFC 001 — `BaseScalar` … `AdvancedNumericalScalar`, with `f32`/`f64` baseline
impls). The base-scalar ordering question is resolved: the architect chose
**Direction B** (base excludes ordering; ordering is `OrderedScalar`), recorded
as ADR-017, and Requirements §5.1.3 was amended to match. All gates pass; 37
tests.

### Next: Milestone 1 remainder — RFC 002 (access)

Sequence: ~~RFC 003 (errors)~~ ✓ → ~~RFC 014 (solver status)~~ ✓ →
~~RFC 001 (six-tier scalars)~~ ✓ → RFC 002 (access contracts). The v0.6.0
architect review **conditionally approved** RFC 001/003/014 (no rollback) and
directed design patches to RFC 002 before coding; those are now applied (v0.6.1):
`dimension` naming, `DimensionKind` without `Borrowed`, contiguous-only core
views (strided → RFC 004), an optional contiguous fast path, an explicit
access-error mapping, and no overlapping mutable views. RFC 002 is now ready to
implement as the final Milestone 1 core contract; access traits bound only
`BaseScalar` except where they compare / project / tolerance-check.

### Open design rounds (gate later-milestone *content*, not the skeleton)

1. RFC 006 — box/bound-constrained first device kernel scope (Milestone 2).
2. RFCs 007 / 008 / 012 — validation-state reconciliation (Milestone 3).
