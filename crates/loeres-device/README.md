# loeres-device

Deterministic, bounded-iteration, panic-averse edge solver entrypoints with caller-owned typed workspaces.

- **Environment:** `#![no_std]`, no `alloc`
- **Depends on:** `loeres`, `loeres-backend-static`
- **Status:** Active through v0.20.0. RFC 005 supplies configuration/workspace
  lifecycle contracts; RFC 006 implements the bounded device PFO kernel.

## Public surface

- `problem` behind `owned-arrays`: fixed-size `ProjectedFirstOrderProblem` over
  box bounds.
- `solve`: `solve_projected_first_order`, `DeviceSolveReport`, and the
  caller-owned `ProjectedFirstOrderWorkspace`, all behind `owned-arrays`.
- `config`: validated `DeviceSolveConfig` and `TimingMode`; optional
  constant-iteration behavior is not a cryptographic constant-time claim.
- `workspace`: overwrite-on-entry reusable workspace lifecycle,
  `DeviceWorkspaceDiagnostic`, and compact core `DiagnosticSnapshot` access.
- `diagnostic`: reserved empty namespace for richer future device diagnostics;
  it currently exports no public items.

The shipped scope is one box/bound-constrained projected-first-order family,
not broad QP/SOCP parity. Determinism and panic-averse evidence are
target-profile-scoped and are not formal WCET or panic-freedom proofs.

See the workspace [README](../../README.md), the [architecture](../../docs/src/architecture.md)
chapter, and the [RFC index](../../rfcs/README.md).

Licensed under Apache-2.0.
