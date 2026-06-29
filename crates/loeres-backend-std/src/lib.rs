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
//! RFC 007 (v0.11.0) populates `dense` (behind the default `dense` feature) and
//! `sparse` (behind `sparse`) with row-major `Vec`-backed dense adapters and a
//! CSR sparse matrix, all implementing the RFC 002 access contracts. Validation
//! state is RFC 012-owned; this crate provides only ordinary construction
//! checks and finite-scan helpers. `view`, `batch`, and `adapter` remain
//! placeholders.

pub mod adapter;
pub mod batch;
#[cfg(feature = "dense")]
pub mod dense;
#[cfg(feature = "sparse")]
pub mod sparse;
pub mod view;

#[cfg(any(feature = "dense", feature = "sparse"))]
pub(crate) mod internal;

#[cfg(feature = "dense")]
pub use dense::{DenseIngestOptions, DenseMatrix, DenseVector};
#[cfg(feature = "sparse")]
pub use sparse::{SparseIngestOptions, SparseMatrix};
