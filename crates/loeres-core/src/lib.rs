//! `loeres-core` — the shared mathematical vocabulary of the Loeres family.
//!
//! Environment: `#![no_std]`, no `alloc`. Defines contracts only; it owns no
//! storage, runtime, or OS assumptions. Both the server (`loeres-cluster` /
//! `loeres-backend-std`) and edge (`loeres-device` / `loeres-backend-static`)
//! halves implement these same contracts without importing each other.
//!
//! Public module topography (external design §1.5):
//! `scalar`, `access`, `problem`, `solver`, `error`, `diagnostic`, `dimension`.
//!
//! Phase 0 skeleton: modules are documented placeholders; public items land in
//! each owning RFC's Milestone 1 work.
#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_code)]

pub mod access;
pub mod diagnostic;
pub mod dimension;
pub mod error;
pub mod problem;
pub mod scalar;
pub mod solver;
