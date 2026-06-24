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
//! Phase 0 skeleton: modules are documented placeholders.
#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_code)]

pub mod config;
pub mod diagnostic;
pub mod problem;
pub mod solve;
pub mod workspace;
