# loeres-device

Deterministic, bounded-iteration, panic-averse edge solver entrypoints with caller-owned typed workspaces.

- **Environment:** `#![no_std]`, no `alloc`
- **Depends on:** `loeres`, `loeres-backend-static`
- **Status:** Active. RFC 005 supplies configuration/workspace lifecycle
  contracts; RFC 006 implements the bounded device PFO kernel; RFC 027 (in the
  `0.21.1` release) adds a constrained quadratic-program kernel.

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

`Converged` requires three things together: the iterate is **feasible within
`projection_tolerance`** (RFC 027 Amendment 5), **stationary** at the *final*
iteration (RFC 029, which matters under `ConstantIteration`), and produced by an
**exact projection** — the final outer iteration's projection returned without
hitting `projection_max_sweeps` (RFC 033). `NotConverged` with
`TerminationReason::NoProgress` on a constrained solve indicates an infeasible
polyhedron or a projection that hit its cap. The report also carries
`projection_cap_hits` (every capped projection, early ones included) and
`max_constraint_violation`: read them, because an inner cap hit is not an error
and the returned point may be only feasible-approximate.

### Limits of the constrained kernel (RFC 027 §11.6)

- LP is expressible (`Q = 0`) but not solved.
- Infeasibility is not detected as a status; it is reported as `NotConverged` /
  `NoProgress` with a positive violation that does not shrink as the sweep cap is
  raised. `infeasibility_evidence()` on the report is only a heuristic hint, wrong in
  both directions (RFC 034): set on about 3 in 100,000 feasible near-parallel problems
  and 2 in 10,000 thin slivers (wedges whose projection needs far more sweeps than the
  cap), and missing most weakly infeasible systems.
- The projection is inexact by design: it converges linearly at a rate set by
  the angles between constraint normals, and nearly parallel constraints can make
  the inner cap bind routinely. `projection_max_sweeps` has no default and must
  suit `projection_tolerance`.
- The step is bounded, not chosen for you (RFC 032): for symmetric positive
  semidefinite `Q`, `curvature_bounds()` gives `U ≥ λ_max` and `L ≤ λ_max`; a step
  `< 2/U` is provably convergent, a step `≥ 2/L` is provably divergent and is
  rejected as `InvalidInput`, and the band between is accepted with no claim.
  `suggested_step_scale()` returns `1/U`: safe, never optimal, never applied for
  you. No numeric convergence rate is claimed.
- `Q` symmetric positive semidefinite is a caller precondition and is not
  verified.
- Device and cluster agree within tolerance, not bitwise (RFC 013).

See the workspace [README](../../README.md), the [architecture](../../docs/src/architecture.md)
chapter, and the [RFC index](../../rfcs/README.md).

Licensed under Apache-2.0.
