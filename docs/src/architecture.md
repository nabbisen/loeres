# Architecture

## Crates

Loeres is a Cargo workspace of five public crates.

| Crate | Environment | Responsibility |
|---|---|---|
| `loeres` | `#![no_std]`, no `alloc` | Implemented scalar, vector/matrix access, solver outcome/status, validation, dimension, error, and diagnostic contracts. The `problem` namespace is reserved; no generic problem-family contract ships. Defines no storage. |
| `loeres-backend-std` | `std` | Dynamic dense/sparse storage adapters and server math adapters. |
| `loeres-backend-static` | `#![no_std]`, no `alloc` | Fixed-size owned storage, borrowed static views, typed workspace blocks. |
| `loeres-cluster` | `std` | Server-side solving: dynamic models, batch execution, cancellation, parallelism, observability, safe gateway categories and a mock gateway; `ffi-gateway` is reserved for a future separately reviewed native adapter. |
| `loeres-device` | `#![no_std]`, no `alloc` | Deterministic edge solve entrypoints, bounded execution configuration, caller-owned typed workspace lifecycle. |

Through v0.20.0, `loeres-device` and `loeres-cluster` implement one shared
box/bound-constrained projected-first-order family. `loeres-cluster` also
provides per-item orchestration/cancellation, metadata-only observation, a safe
mock gateway seam, and a process-local validation evidence cache. It does not
ship broad LP/QP/SOCP modeling, a concrete native adapter, a distributed cache,
or broad throughput/multi-tenant stress evidence.

The PFO problem traits are execution-crate contracts, not implementations of a
generic `loeres::problem` family. PF-001 through PF-003 remain unimplemented,
and PF-004 is represented only by solver-specific oracle traits.

## Dependency direction

The dependency graph is acyclic and environment-separated:

```text
loeres-cluster ─▶ loeres-backend-std ─▶ loeres ◀─ loeres-backend-static ◀─ loeres-device
       └────────────────────────────────▶ loeres ◀───────────────────────────────┘
```

Forbidden, and enforced by workspace rules and CI: `loeres-device` or
`loeres-backend-static` depending on `loeres-backend-std` or `loeres-cluster`;
`loeres` depending on any backend or execution crate; any edge-facing
feature enabling `std`, `alloc`, async runtimes, logging frameworks, or FFI.

## Key contracts

- **Stratified scalars.** Capability tiers (base, finite, divisible, metric,
  advanced numerical, fixed-point) rather than one monolithic trait, so a solver
  states the minimum capability it needs and edge backends are never forced to
  implement operations they do not use.
- **Storage-agnostic access.** `loeres::access` defines shape, indexing,
  borrowing, and fallible access without committing to a memory layout or
  implying heavy linear-algebra kernels.
- **Solver outcome / status taxonomy.** `loeres::solver` owns the single
  shared taxonomy. Bounded progress — including non-convergence at the iteration
  cap — is a *status* (`SolveStatus::NotConverged`); boundary rejection and
  fail-safe conditions are *errors* (`SolverError`). Device and cluster reports
  derive losslessly from the core report.
- **Caller-owned typed workspaces.** Device solvers never allocate hidden
  scratch; the workspace is passed by unique mutable reference and its footprint
  is reviewable before execution.

## Verification

The repository's `xtask` automation enforces the boundary: no-`std`/no-`alloc`
builds for edge crates, dependency-graph checks, a public-API surface scanner
(forbidden types and `dyn` in edge APIs), panic-path audits, size budgets,
target-profile checks, and the cross-layer conformance corpus. See RFC 010 and
the roadmap's verification section for the full gate list.

`cargo xtask check` is developer evidence. The RFC 019
`cargo xtask release-gate` is a distinct package/readiness gate and remains
fail-closed until integrated documentation, tag/revision, clean-extraction, and
approval evidence exists. Advisory and documented-only evidence is not reported
as mandatory tested support.
