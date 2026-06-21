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

## Current status (v0.5.0)

**Milestone 1 in progress — RFC 003 and RFC 014 implemented.** `loeres-core` now
owns the full outcome taxonomy: the `Err` side (RFC 003 — `SolverError`,
`DiagnosticSnapshot`) and the `Ok` side (RFC 014 — `SolveStatus`,
`TerminationReason`, `StepOutcome`, `SolveReport`, `AsCoreReport`), with
non-convergence reported as a status, not an error. All gates pass
(check / zero-bleed / no-std / check-rfcs); 21 tests.

### Next: Milestone 1 remainder — `loeres-core` contracts

Sequence: ~~RFC 003 (errors)~~ ✓ → ~~RFC 014 (solver outcome/status)~~ ✓ →
RFC 001 (six-tier scalars) → RFC 002 (access). RFC 014 was taken ahead of
001/002 because it depends only on RFC 003 and is scalar-agnostic.

**Blocker before RFC 001:** the requirements §5.1.2 base-scalar wording flag
(architect) should be resolved so the scalar code traces to a clean spec.
RFC 002 (access) bounds on `BaseScalar`, so it is gated behind RFC 001.

### Open design rounds (gate later-milestone *content*, not the skeleton)

1. RFC 006 — box/bound-constrained first device kernel scope (Milestone 2).
2. RFCs 007 / 008 / 012 — validation-state reconciliation (Milestone 3).
