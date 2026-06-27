//! `loeres` — the shared mathematical vocabulary of the Loeres family.
//!
//! Environment: `#![no_std]`, no `alloc`. Defines contracts only; it owns no
//! storage, runtime, or OS assumptions. Both the server (`loeres-cluster` /
//! `loeres-backend-std`) and edge (`loeres-device` / `loeres-backend-static`)
//! halves implement these same contracts without importing each other.
//!
//! Public module topography (external design §1.5):
//! `scalar`, `access`, `problem`, `solver`, `error`, `diagnostic`, `dimension`.
//!
//! Milestone 1: the [`scalar`] capability tiers (RFC 001), the [`error`] and
//! [`diagnostic`] topologies (RFC 003), the [`solver`] outcome/status taxonomy
//! (RFC 014), and the [`access`] / [`dimension`] storage-agnostic contracts
//! (RFC 002) are implemented; [`problem`] remains a documented placeholder
//! pending its owning RFC.
#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_code)]

pub mod access;
pub mod diagnostic;
pub mod dimension;
pub mod error;
pub mod problem;
pub mod scalar;
pub mod solver;

pub use access::{
    ContiguousMatrixAccess, ContiguousVectorAccess, ContiguousVectorAccessMut, MatrixAccess,
    MatrixAccessMut, MatrixView, MatrixViewMut, VectorAccess, VectorAccessMut, VectorView,
    VectorViewMut,
};
pub use diagnostic::{DiagnosticCode, DiagnosticSnapshot};
pub use dimension::{Dim2, DimensionKind};
pub use error::{SolverError, error_code_to_str};
pub use scalar::{
    AdvancedNumericalScalar, BaseScalar, DivisibleScalar, FiniteScalar, MetricScalar, OrderedScalar,
};
pub use solver::{
    AsCoreReport, IterationReport, SolveReport, SolveStatus, StepOutcome, TerminationReason,
};

#[cfg(test)]
mod tests;
