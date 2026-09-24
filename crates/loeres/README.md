# loeres

Shared mathematical contracts for the Loeres family: stratified scalars,
storage-agnostic vector/matrix access, solver outcome/status and validation
vocabulary, dimensions, and allocation-free errors/diagnostics. Defines **no**
storage, runtime, or OS assumptions. The `problem` namespace defines a
storage-agnostic quadratic-program contract (RFC 027); LP is expressible as
`Q = 0` but not solved, and no SOCP contract exists.

- **Environment:** `#![no_std]`, no `alloc`
- **Depends on:** nothing (defines contracts only)
- **Status:** Active. Implemented core surfaces: `scalar` (RFC 001, six capability
  tiers), `access` / `dimension` (RFC 002, storage-agnostic vector/matrix
  contracts + views), `error` / `diagnostic` (RFC 003, allocation-free error
  topology), `solver` (RFC 014, outcome/status taxonomy), and `validation`
  (RFC 012, the validation-state vocabulary). `problem` (RFC 027) defines the
  quadratic-program contract.

The contract names *what* a quadratic program is, never how it is stored or
solved. `Q` symmetric positive semidefinite is a caller precondition and is not
verified. `curvature_bounds()` and `suggested_step_scale()` (RFC 032) bound
`λ_max(Q)` from above (Gershgorin) and below (the largest diagonal entry) so a
caller can pick a provably safe step; both bounds are meaningless without that
precondition. The kernels that consume it (`loeres-device`, `loeres-cluster`) report
`Converged` only for a feasible iterate; infeasibility is not detected (it is
reported as `NotConverged` with `NoProgress`); the projection is inexact by design
and no numeric convergence rate is claimed (RFC 027 §11.6, RFC 032).

See the workspace [README](../../README.md), the [architecture](../../docs/src/architecture.md)
chapter, and the [RFC index](../../rfcs/README.md).

Licensed under Apache-2.0.
