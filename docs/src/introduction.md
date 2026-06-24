# Introduction

Loeres is a Rust workspace of mathematical-optimization crates. Its defining
property is not the solvers it provides, but the boundary it preserves: a hard,
compile-time separation between two fundamentally different execution worlds.

- **Cluster / server optimization** — high-throughput, dynamically allocated,
  parallel, cloud-native computation.
- **Device / edge optimization** — deterministic, allocation-free, panic-averse,
  real-time-safe computation for embedded, control-loop, and safety-relevant
  environments.

Loeres is deliberately *not* a single runtime that switches between these worlds
at run time. The separation is encoded in crate boundaries, dependency
direction, feature policy, CI gates, public APIs, and release procedures.

The core design principle is:

> **Share mathematical contracts, not execution assumptions.**

`loeres` defines the shared vocabulary — scalar capabilities, vector and
matrix access contracts, problem families, solver outcome/status categories,
dimensions, and an allocation-free error topology. It is `#![no_std]` and does
not depend on `alloc`. Backends provide storage; execution crates provide the
server and edge solve paths. A cloud service may use heap allocation, threads,
and tracing without contaminating an embedded controller that depends only on
the edge crates, and the reverse can never happen because edge-facing crates
cannot depend on server-facing crates.

This book summarizes the architecture and threat model. The authoritative,
detailed design lives in the specifications under `docs/specs/` and in the RFC
set under `rfcs/`.
