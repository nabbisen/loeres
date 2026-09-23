# loeres-device

Deterministic, bounded-iteration, panic-averse edge solver entrypoints with caller-owned typed workspaces.

- **Environment:** `#![no_std]`, no `alloc`
- **Depends on:** `loeres`, `loeres-backend-static`
- **Status:** Active. RFC 005 supplies configuration/workspace lifecycle
  contracts; RFC 006 implements the bounded device PFO kernel; RFC 027 (in the
  unreleased `0.21.1` tree) adds a constrained quadratic-program kernel.

## Public surface

- `problem` behind `owned-arrays`: fixed-size `ProjectedFirstOrderProblem` over
  box bounds.
- `solve`: `solve_projected_first_order`, `DeviceSolveReport`, and the
  caller-owned `ProjectedFirstOrderWorkspace`, all behind `owned-arrays`.
- `solve` (constrained, RFC 027): `solve_constrained_projected_first_order`, for
  a `loeres::QuadraticProgram` with `M >= 1` linear inequalities `Ax <= b` over a
  box, with `ConstrainedSolveConfig`, `ConstrainedSolveReport`, and the
  caller-owned `ConstrainedProjectedWorkspace<S, N, M>` (footprint
  `(3N + 2M)·size_of::<S>()` plus a 16-byte header), all behind `owned-arrays`.
  A device problem with **no** inequalities uses `solve_projected_first_order`
  instead; `M >= 1` is a compile-time assertion.
- `config`: validated `DeviceSolveConfig` and `TimingMode`; optional
  constant-iteration behavior is not a cryptographic constant-time claim.
- `workspace`: overwrite-on-entry reusable workspace lifecycle,
  `DeviceWorkspaceDiagnostic`, and compact core `DiagnosticSnapshot` access.
- `diagnostic`: reserved empty namespace for richer future device diagnostics;
  it currently exports no public items.

The shipped scope is one projected-first-order family — over a box, and over a
box with linear inequalities — not broad LP/SOCP parity. Determinism and
panic-averse evidence are target-profile-scoped and are not formal WCET or
panic-freedom proofs.

### Reading a constrained solve

`Converged` means **feasible within `projection_tolerance`** (RFC 027 Amendment 5)
as well as stationary, evaluated at the final iterate under `ConstantIteration`
(RFC 029). `NotConverged` with `TerminationReason::NoProgress` on a constrained
solve indicates an infeasible or too-tightly-capped polyhedron. The report also
carries `projection_cap_hits` and `max_constraint_violation`: read them, because
an inner cap hit is not an error and the returned point may be only
feasible-approximate.

### Limits of the constrained kernel (RFC 027 §11.6)

- LP is expressible (`Q = 0`) but not solved.
- Infeasibility is not detected; it is reported as `NotConverged` / `NoProgress`
  with a positive violation that does not shrink as the sweep cap is raised.
- The projection is inexact by design: it converges linearly at a rate set by
  the angles between constraint normals, and nearly parallel constraints can make
  the inner cap bind routinely. `projection_max_sweeps` has no default and must
  suit `projection_tolerance`.
- No convergence rate is claimed; `step_scale` must lie in `(0, 2 / λ_max(Q))`,
  the caller's responsibility.
- `Q` symmetric positive semidefinite is a caller precondition and is not
  verified.
- Device and cluster agree within tolerance, not bitwise (RFC 013).

See the workspace [README](../../README.md), the [architecture](../../docs/src/architecture.md)
chapter, and the [RFC index](../../rfcs/README.md).

Licensed under Apache-2.0.
