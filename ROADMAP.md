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

## Current status (v0.2.0)

Design baseline. The Milestone-1 contracts are drafted; the RFC 014
reconciliation and cross-document cleanup are complete; and the RFC 001
`OrderedScalar` scalar-tier split is resolved (the scalar model is now six tiers,
with ordering / `min` / `max` / `clamp` on a dedicated `OrderedScalar`).

### Open design rounds (precede implementation)

1. RFC 006 — box/bound-constrained first device kernel scope.
2. RFCs 007 / 008 / 012 — validation-state reconciliation.

Once these are settled, Phase 0 (workspace skeleton) begins implementation under
the design-before-code workflow.
