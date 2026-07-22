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
matrix access contracts, solver outcome/status and validation categories,
dimensions, and allocation-free error/diagnostic topology. Its `problem`
namespace is reserved: no generic public LP/QP/SOCP/problem-family contract
ships. Implemented PFO problem contracts belong to the device and cluster
execution crates. `loeres` is `#![no_std]` and does not depend on `alloc`.
Backends provide storage; execution crates provide solve paths. A cloud service
may use heap allocation, threads, and tracing without contaminating an embedded
controller that depends only on the edge crates, and the reverse can never
happen because edge-facing crates cannot depend on server-facing crates.

This book summarizes the architecture and threat model. The authoritative,
detailed design lives in the specifications under `docs/specs/` and in the RFC
set under `rfcs/`.

Before external predicate `P` succeeds, v0.20.0 remains the last externally
activated repository release and this exact `0.20.2` tree is a non-current
No-Go finalization candidate. After `P` succeeds, these same immutable bytes
are the activated `0.20.2` release with RFCs 001-021 implemented. Tracked bytes
alone do not establish whether `P` occurred. GitHub release creation, registry
publication, and certification remain separately authorized; none is implied
by `P`. Current solver breadth is one box/bound-constrained projected-first-
order family on device and cluster. The gateway is mock-only, validation
caching is process-local, and conformance is a bounded smoke corpus.
