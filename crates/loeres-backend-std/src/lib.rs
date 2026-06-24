//! `loeres-backend-std` — dynamic, heap-backed storage for the server.
//!
//! Environment: `std`, dynamic allocation. Provides dense/sparse storage
//! adapters and optional third-party numerical backends, all implementing the
//! `loeres` access contracts. Server-only: it must never be depended on
//! by `loeres-device` or `loeres-backend-static`.
//!
//! Public module topography (external design §1.5):
//! `dense`, `sparse`, `view`, `batch`, `adapter`.
//!
//! Phase 0 skeleton: modules are documented placeholders.

pub mod adapter;
pub mod batch;
pub mod dense;
pub mod sparse;
pub mod view;
