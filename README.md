# Loeres

[![License](https://img.shields.io/github/license/nabbisen/loeres)](LICENSE)
[![loeres-core Docs](https://docs.rs/loeres-core/badge.svg?version=latest)](https://docs.rs/loeres-core)    
[![loeres-core crate](https://img.shields.io/crates/v/loeres-core?label=loeres-core)](https://crates.io/crates/loeres-core)
[![loeres-core Deps Status](https://deps.rs/crate/loeres-core/latest/status.svg)](https://deps.rs/crate/loeres-core)
[![loeres-backend-std crate](https://img.shields.io/crates/v/loeres-backend-std?label=loeres-backend-std)](https://crates.io/crates/loeres-backend-std)
[![loeres-backend-std Deps Status](https://deps.rs/crate/loeres-backend-std/latest/status.svg)](https://deps.rs/crate/loeres-backend-std)
[![loeres-backend-static crate](https://img.shields.io/crates/v/loeres-backend-static?label=loeres-backend-static)](https://crates.io/crates/loeres-backend-static)
[![loeres-backend-static Deps Status](https://deps.rs/crate/loeres-backend-static/latest/status.svg)](https://deps.rs/crate/loeres-backend-static)    
[![loeres-cluster crate](https://img.shields.io/crates/v/loeres-cluster?label=loeres-cluster)](https://crates.io/crates/loeres-cluster)
[![loeres-cluster Deps Status](https://deps.rs/crate/loeres-cluster/latest/status.svg)](https://deps.rs/crate/loeres-cluster)
[![loeres-device crate](https://img.shields.io/crates/v/loeres-device?label=loeres-device)](https://crates.io/crates/loeres-device)
[![loeres-device Deps Status](https://deps.rs/crate/loeres-device/latest/status.svg)](https://deps.rs/crate/loeres-device)

**One optimization contract, two worlds — high-throughput server solving and deterministic `no_std` edge solving, without letting either contaminate the other.**

## Overview

Loeres is a Rust workspace of mathematical-optimization crates that share a single set of mathematical contracts while keeping a hard, compile-time boundary between two execution environments:

- **Cluster / server** — dynamic problem sizes, heap allocation, parallelism, async, and observability.
- **Device / edge** — `#![no_std]`, no `alloc`, bounded iteration, caller-owned workspaces, and panic-averse solve paths suitable for real-time and WCET-oriented review.

The guiding rule is: **share mathematical contracts, not storage, allocation, runtime, or operating-system assumptions.**

## Why / When

Reach for Loeres when you need optimization on *both* sides of that boundary from one consistent contract:

- **Server**: SaaS solvers, batch and scheduling systems, energy/logistics, analytics pipelines — where throughput, dynamic sizes, and integration matter.
- **Edge**: robotics, model-predictive control, industrial controllers, medical IoT — where no heap, bounded time, and analyzable failure matter.

The point is that a cloud service can use allocation, threads, and tracing without contaminating an embedded controller that depends only on the edge crates — and vice versa. Server breadth never becomes a device obligation.

## Quick Start

> **v0.5.0 — Milestone 1 in progress.** `loeres-core` owns the full outcome taxonomy: the error/diagnostic topology (RFC 003 — `SolverError`, `DiagnosticSnapshot`) and the solver outcome/status taxonomy (RFC 014 — `SolveStatus`, `SolveReport`, `AsCoreReport`), where non-convergence is a status, not an error. All verified `no_std`/no-`alloc` on a bare-metal target. The remaining Milestone 1 contracts — scalars (RFC 001) and access (RFC 002) — follow.

Build and verify from source:

```sh
# toolchain, components, and the bare-metal target come from rust-toolchain.toml
cargo check --workspace --all-features
cargo xtask zero-bleed   # no server <-> edge dependency bleed
cargo xtask no-std       # edge crates build for thumbv7em-none-eabihf
```

The intended downstream import model (specified in the external design, §1.4) is environment-selected by crate choice:

```toml
# Cluster / server user
loeres-cluster        = { version = "0.x", features = ["batch", "parallel-rayon"] }
loeres-backend-std    = { version = "0.x", features = ["dense"] }

# Device / edge user
loeres-device         = { version = "0.x", default-features = false }
loeres-backend-static = { version = "0.x", default-features = false, features = ["owned-arrays"] }
```

To navigate this release: the workspace lives under `crates/` (five crates) and `xtask/`; the design lives in `docs/specs/` (requirements → external design → roadmap) and `rfcs/`. For local development see `docs/src/development.md`.

## Design Notes

- **Five crates, one contract.** `loeres-core` (`no_std`, no-`alloc`) defines scalar, vector/matrix access, problem, solver-outcome, error, and dimension contracts. Backends (`-backend-std`, `-backend-static`) own storage; execution crates (`-cluster`, `-device`) own the server and edge solve paths. The dependency graph is acyclic and environment-separated; edge crates can never depend on server crates.
- **Stratified scalar capabilities** — six tiers (`BaseScalar`, `OrderedScalar`, `FiniteScalar`, `DivisibleScalar`, `MetricScalar`, `AdvancedNumericalScalar`) rather than one monolithic `Scalar` trait, so edge solvers are not forced to implement operations they never use. Ordering is split out of the base tier so order-free numeric types stay valid and floating-point `min`/`max` behavior is pinned.
- **Status / error split.** Bounded solver progress (including non-convergence at the iteration cap) is a *status* returned in `Ok`; boundary rejection and fail-safe conditions are *errors* returned in `Err`.
- **Caller-owned typed workspaces** on device — no hidden allocation; memory footprint is reviewable before execution.
- **Target-scoped determinism.** Floating-point reproducibility claims are tied to documented target profiles, not asserted globally.

## More Detail

- Specifications: [`docs/specs/`](docs/specs/) — requirements, external design, roadmap & milestones.
- RFCs: [`rfcs/`](rfcs/) — lifecycle policy (`done/000`), Milestone 1–3 and cross-cutting contracts (`proposed/001`–`014`).
- Book: [`docs/src/`](docs/src/) — introduction, architecture, threat model, and a maintainer bridge to the specs/RFCs (mdbook).
- Contributing: [`CONTRIBUTING.md`](CONTRIBUTING.md) — the design-first workflow and the RFC process.
- Roadmap & status: [`ROADMAP.md`](ROADMAP.md).

## License

Licensed under the Apache License, Version 2.0. See [`LICENSE`](LICENSE) and [`NOTICE`](NOTICE).
