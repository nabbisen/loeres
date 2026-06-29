//! `loeres-device` — deterministic edge-side solver entrypoints.
//!
//! Environment: `#![no_std]`, no `alloc`. Optimizes for bounded iteration,
//! fixed memory, small binaries, and analyzable, panic-averse solve paths.
//! Depends on `loeres` and `loeres-backend-static` only — never on
//! `loeres-cluster`, `loeres-backend-std`, async runtimes, threads, logging,
//! or FFI gateways.
//!
//! Public module topography (external design §1.5):
//! `problem`, `solve`, `config`, `workspace`, `diagnostic`.
//!
//! Milestone 2 is complete. RFC 005 provides `config` (runtime
//! `DeviceSolveConfig` / `TimingMode` policy and structural validation) and
//! `workspace` (the caller-owned `DeviceWorkspace` / `DeviceWorkspaceDiagnostic`
//! / `WorkspaceFor` lifecycle contracts). RFC 006 (v0.10.0) adds the baseline
//! box/bound-constrained projected first-order kernel: `problem`
//! (`ProjectedFirstOrderProblem`) and `solve` (`solve_projected_first_order`,
//! the `DeviceSolveReport` outcome over the RFC 014 `SolveReport`, and the
//! caller-owned `ProjectedFirstOrderWorkspace` scratch). The `problem`/`solve`
//! kernel surface is gated behind `owned-arrays`, since the primal/gradient
//! work vectors are RFC 004 `FixedVector<S, N>`. `diagnostic` is reserved for
//! future richer diagnostics.
#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_code)]

pub mod config;
pub mod diagnostic;
pub mod problem;
pub mod solve;
pub mod workspace;
