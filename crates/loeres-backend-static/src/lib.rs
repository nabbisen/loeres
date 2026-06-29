//! `loeres-backend-static` — fixed-size, allocation-free storage for the edge.
//!
//! Environment: `#![no_std]`, no `alloc`. Provides owned fixed arrays and
//! borrowed views over caller-owned memory (peripheral buffers, DMA regions,
//! RTOS-owned control-loop state) that implement the `loeres` access
//! contracts and report [`dimension::DimensionKind::Static`]. Depends on
//! `loeres` only.
//!
//! Public module topography (external design §1.5):
//! - [`dimension`] — static dimension descriptors and shared access support (baseline).
//! - [`view`] — borrowed const-sized contiguous static views (baseline).
//! - [`array`] — owned `FixedVector` / `FixedMatrix` (feature `owned-arrays`).
//! - [`workspace`] — scratch/workspace storage (placeholder; RFC 005).
//!
//! Feature posture (external design §1.6.2): the featureless baseline is the
//! borrowed contiguous static adapters plus dimension descriptors. `owned-arrays`
//! adds the owned wrappers; `static-views` adds advanced views (deferred,
//! RFC 004 §7.2). Advanced views are not promoted to default in v0.x.
#![cfg_attr(not(test), no_std)]

#[cfg(feature = "owned-arrays")]
pub mod array;
pub mod dimension;
pub mod view;
pub mod workspace;
