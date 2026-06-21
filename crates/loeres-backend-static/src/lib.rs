//! `loeres-backend-static` — fixed-size, allocation-free storage for the edge.
//!
//! Environment: `#![no_std]`, no `alloc`. Provides owned fixed arrays and
//! borrowed views over caller-owned memory (peripheral buffers, DMA regions,
//! RTOS-owned control-loop state) that implement the `loeres-core` access
//! contracts. Depends on `loeres-core` only.
//!
//! Public module topography (external design §1.5):
//! `array`, `view`, `workspace`, `dimension`.
//!
//! Phase 0 skeleton: modules are documented placeholders.
#![cfg_attr(not(test), no_std)]

pub mod array;
pub mod dimension;
pub mod view;
pub mod workspace;
