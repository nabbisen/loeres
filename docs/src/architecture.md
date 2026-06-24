# Architecture

## Crates

Loeres is a Cargo workspace of five public crates.

| Crate | Environment | Responsibility |
|---|---|---|
| `loeres` | `#![no_std]`, no `alloc` | Mathematical contracts: scalar capabilities, vector/matrix access, problem families, solver outcome/status, dimensions, allocation-free errors. Defines no storage. |
| `loeres-backend-std` | `std` | Dynamic dense/sparse storage adapters and server math adapters. |
| `loeres-backend-static` | `#![no_std]`, no `alloc` | Fixed-size owned storage, borrowed static views, typed workspace blocks. |
| `loeres-cluster` | `std` | Server-side solving: dynamic models, batch execution, cancellation, parallelism, observability, optional FFI gateways. |
| `loeres-device` | `#![no_std]`, no `alloc` | Deterministic edge solve entrypoints, bounded execution configuration, caller-owned typed workspace lifecycle. |

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
