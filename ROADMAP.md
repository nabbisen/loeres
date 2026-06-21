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

## Current status (v0.3.0)

**Phase 0 (workspace skeleton) complete.** The Cargo workspace exists with the
five crates plus `xtask`; everything compiles; the edge crates
(`loeres-core`, `loeres-backend-static`, `loeres-device`) build `no_std` /
no-`alloc` for `thumbv7em-none-eabihf`; and the `zero-bleed` dependency-direction
gate passes. The crates carry the public module topography (external design §1.5)
as documented placeholders — no public API yet. The RFC 001 `OrderedScalar`
split (v0.2.0) and the RFC 014 reconciliation are folded in.

### Next: Phase 1 / Milestone 1 — `loeres-core` contracts

Implement, in order, RFC 003 (errors) → RFC 001 (six-tier scalars) →
RFC 002 (access) → RFC 014 (solver outcome/status). **Blocker to clear first:**
the requirements §5.1.2 base-scalar wording flag (architect) should be resolved
so the scalar code traces to a clean spec.

### Open design rounds (gate later-milestone *content*, not the skeleton)

1. RFC 006 — box/bound-constrained first device kernel scope (Milestone 2).
2. RFCs 007 / 008 / 012 — validation-state reconciliation (Milestone 3).
