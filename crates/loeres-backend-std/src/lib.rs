#![cfg_attr(docsrs, feature(doc_cfg))]
//! `loeres-backend-std` — dynamic, heap-backed storage for the server.
//!
//! Environment: `std`, dynamic allocation. Provides dense/sparse storage
//! adapters, all implementing the
//! `loeres` access contracts. Server-only: it must never be depended on
//! by `loeres-device` or `loeres-backend-static`.
//!
//! Public module topography (external design §1.5): `dense`, `sparse`.
//!
//! RFC 007 (v0.11.0) populates `dense` (behind the default `dense` feature) and
//! `sparse` (behind `sparse`) with row-major `Vec`-backed dense adapters and a
//! CSR sparse matrix, all implementing the RFC 002 access contracts. Validation
//! state is RFC 012-owned; this crate provides only ordinary construction
//! checks and finite-scan helpers.

#[cfg(feature = "dense")]
#[cfg_attr(docsrs, doc(cfg(feature = "dense")))]
pub mod dense;
#[cfg(feature = "sparse")]
#[cfg_attr(docsrs, doc(cfg(feature = "sparse")))]
pub mod sparse;

#[cfg(any(feature = "dense", feature = "sparse"))]
pub(crate) mod internal;

#[cfg(feature = "dense")]
#[cfg_attr(docsrs, doc(cfg(feature = "dense")))]
pub use dense::{DenseIngestOptions, DenseMatrix, DenseVector};
#[cfg(feature = "sparse")]
#[cfg_attr(docsrs, doc(cfg(feature = "sparse")))]
pub use sparse::{SparseIngestOptions, SparseMatrix};
