# Loeres

[License: Apache-2.0](LICENSE)

> **Publication and recovery status.** Repository state does not establish that
> every workspace crate or its hosted documentation is published or current, so
> external registry/documentation badges are intentionally omitted. The last
> externally activated repository release is v0.20.0. RFCs 019/020/021 are
> conditionally staged for `0.20.2`; their metadata, apex, and `done/` paths do
> not prove external activation. Release/package readiness remains No-Go and
> fail-closed pending the RFC 021 evidence and authorization sequence.

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

> **v0.20.0 — Trusted/cache conformance hardening.** RFC 017 extends the enforced `conformance/smoke/` corpus with validation-cache fixtures for cache hit/miss, insufficient scope, stale/wrong evidence, current-iterate scan retention, hot-loop numerical-domain retention, and reusable-cache insertion rejection. Runtime crate APIs are unchanged.

Build and verify from source:

```sh
# toolchain, components, and the bare-metal target come from rust-toolchain.toml
cargo check --workspace --all-features
cargo xtask check        # developer aggregate; not release approval
```

The intended downstream import model (specified in the external design, §1.4) is environment-selected by crate choice:

```toml
# Cluster / server user
loeres-cluster        = { version = "0.x", features = ["parallel-rayon"] }
loeres-backend-std    = { version = "0.x", features = ["dense"] }

# Device / edge user
loeres-device         = { version = "0.x", default-features = false }
loeres-backend-static = { version = "0.x", default-features = false, features = ["owned-arrays"] }
```

To navigate this release: the workspace lives under `crates/` (five crates) and `xtask/`; the design lives in `docs/specs/` (requirements → external design → roadmap) and `rfcs/`. For local development see `docs/src/development.md`.

## Design Notes

- **Five crates, one contract.** `loeres` (`no_std`, no-`alloc`) defines scalar,
  vector/matrix access, solver-outcome, validation, error, diagnostic, and
  dimension contracts. Its `problem` namespace is reserved; no generic public
  LP/QP/SOCP/problem-family contract ships. Implemented PFO problem contracts
  belong to `loeres-device` and `loeres-cluster`. Backends (`-backend-std`,
  `-backend-static`) own storage; execution crates (`-cluster`, `-device`) own
  solve paths. The dependency graph is acyclic and environment-separated.
- **Stratified scalar capabilities** — six tiers (`BaseScalar`, `OrderedScalar`, `FiniteScalar`, `DivisibleScalar`, `MetricScalar`, `AdvancedNumericalScalar`) rather than one monolithic `Scalar` trait, so edge solvers are not forced to implement operations they never use. Ordering is split out of the base tier so order-free numeric types stay valid and floating-point `min`/`max` behavior is pinned.
- **Status / error split.** Bounded solver progress (including non-convergence at the iteration cap) is a *status* returned in `Ok`; boundary rejection and fail-safe conditions are *errors* returned in `Err`.
- **Caller-owned typed workspaces** on device — no hidden allocation; memory footprint is reviewable before execution.
- **Target-scoped determinism.** Floating-point reproducibility claims are tied to documented target profiles, not asserted globally.
- **Narrow current solver scope.** Device and cluster paths share one
  box/bound-constrained projected-first-order family. The conformance suite is a
  bounded smoke corpus; broad LP/QP/SOCP, large-N, and throughput parity are not
  claimed.
- **Bounded server integrations.** Observability is metadata-only, the gateway
  is mock-only, and validation caching is process-local. No concrete native
  adapter, persistent/distributed cache, or broad multi-tenant isolation
  evidence ships.

## More Detail

- Specifications: [`docs/specs/`](docs/specs/) — requirements, external design, roadmap & milestones.
- RFCs: [`rfcs/`](rfcs/) — shipped contracts `000`–`018` and conditionally
  staged RFCs `019`–`021` live in [`rfcs/done/`](rfcs/done/); the latter use
  RFC 021's exact conditional Status and do not prove activation. Accepted work
  lives in [`rfcs/accepted/`](rfcs/accepted/) and review-active work in
  [`rfcs/proposed/`](rfcs/proposed/) when present. See the [RFC index](rfcs/README.md).
- Book: [`docs/src/`](docs/src/) — introduction, architecture, threat model, and a maintainer bridge to the specs/RFCs (mdbook).
- Contributing: [`CONTRIBUTING.md`](CONTRIBUTING.md) — the design-first workflow and the RFC process.
- Roadmap & status: [`ROADMAP.md`](ROADMAP.md).

## License

Licensed under the Apache License, Version 2.0. See [`LICENSE`](LICENSE) and [`NOTICE`](NOTICE).
