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

## Current status (v0.4.0)

**Milestone 1 in progress — RFC 003 implemented.** `loeres-core` now ships its
first contracts: the allocation-free error/diagnostic topology (`SolverError`,
`DiagnosticSnapshot`, `error_code_to_str`), with compile-time size budgets and
the `check-rfcs` hygiene gate. Phase 0 (workspace skeleton) is complete: all
crates compile, the edge crates build `no_std`/no-`alloc` for
`thumbv7em-none-eabihf`, and `zero-bleed` + `no-std` + `check-rfcs` pass.

### Next: Milestone 1 remainder — `loeres-core` contracts

Implement, in order: ~~RFC 003 (errors)~~ ✓ → RFC 001 (six-tier scalars) →
RFC 002 (access) → RFC 014 (solver outcome/status). **Blocker before RFC 001:**
the requirements §5.1.2 base-scalar wording flag (architect) should be resolved
so the scalar code traces to a clean spec.

### Open design rounds (gate later-milestone *content*, not the skeleton)

1. RFC 006 — box/bound-constrained first device kernel scope (Milestone 2).
2. RFCs 007 / 008 / 012 — validation-state reconciliation (Milestone 3).
