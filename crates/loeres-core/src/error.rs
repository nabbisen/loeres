//! Allocation-free error topology for `loeres-core` (RFC 003).
//!
//! [`SolverError`] is the canonical, copyable, allocation-free error returned by
//! every fallible core API. It implements `Debug` but deliberately **not**
//! `Display` or `core::error::Error`: formatting support encourages string
//! paths and inflates device binaries, so human-facing presentation belongs to
//! `loeres-cluster` or host-side tooling, not the edge baseline.
//!
//! Non-convergence is **not** an error here. A bounded loop that reaches its
//! iteration cap returns `Ok` with `SolveStatus::NotConverged` (RFC 014).

/// Allocation-free, copyable error categories shared across every Loeres crate.
///
/// Marked `#[non_exhaustive]`: downstream `match`es must include a wildcard arm,
/// so future solvers can add categories without a breaking change.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum SolverError {
    /// Two shapes that had to agree did not. Payloads are the mismatched extents.
    DimensionMismatch {
        /// Left-hand extent.
        lhs: u32,
        /// Right-hand extent.
        rhs: u32,
    },
    /// A single dimension was itself invalid (e.g. zero where positive required).
    InvalidDimension,
    /// Input violated a declared domain or problem contract.
    InvalidInput,
    /// A floating-like input was NaN or infinite.
    NonFiniteInput,
    /// The problem is well-formed but unsupported by the selected solver/profile.
    UnsupportedProblemStructure,
    /// The system matrix was singular under the selected solver.
    SingularMatrix,
    /// Conditioning exceeded the solver's declared stability threshold.
    IllConditioned,
    /// An operation would leave the solver's valid numerical domain — division by
    /// zero, square root of a negative, or logarithm of a non-positive value.
    NumericalDomain,
    /// A checked scalar or storage operation overflowed.
    Overflow,
    /// The caller-provided workspace cannot hold the required scratch state.
    WorkspaceTooSmall,
    /// A cluster cancellation token was observed.
    Cancelled,
    /// An optional backend was unavailable.
    BackendUnavailable,
    /// A library invariant was violated — a bug, surfaced as an error rather than
    /// a panic so device callers can fail closed.
    InternalInvariantViolation,
}

// RFC 003 §3.3: a bloated error bloats every `Result<T, SolverError>` return
// path and raises device stack pressure. Keep it small, forever.
const _: () = assert!(core::mem::size_of::<SolverError>() <= 16);

impl SolverError {
    /// True for malformed *caller input* — bad dimensions, or non-finite /
    /// otherwise invalid values supplied to a public entry point.
    #[inline]
    #[must_use]
    pub const fn is_input_error(self) -> bool {
        matches!(
            self,
            Self::DimensionMismatch { .. }
                | Self::InvalidDimension
                | Self::InvalidInput
                | Self::NonFiniteInput
        )
    }

    /// True for numerical failures encountered while solving.
    #[inline]
    #[must_use]
    pub const fn is_numerical_error(self) -> bool {
        matches!(
            self,
            Self::SingularMatrix | Self::IllConditioned | Self::NumericalDomain | Self::Overflow
        )
    }

    /// True for resource / availability failures (workspace, cancellation, backend).
    #[inline]
    #[must_use]
    pub const fn is_resource_error(self) -> bool {
        matches!(
            self,
            Self::WorkspaceTooSmall | Self::Cancelled | Self::BackendUnavailable
        )
    }
}

/// Map an error to a stable, allocation-free `snake_case` identifier.
///
/// Intended for host-side logging and diagnostics. The mapping is part of the
/// public contract and is pinned by tests; renaming a code is a reviewed change.
#[inline]
#[must_use]
pub const fn error_code_to_str(err: SolverError) -> &'static str {
    match err {
        SolverError::DimensionMismatch { .. } => "dimension_mismatch",
        SolverError::InvalidDimension => "invalid_dimension",
        SolverError::InvalidInput => "invalid_input",
        SolverError::NonFiniteInput => "non_finite_input",
        SolverError::UnsupportedProblemStructure => "unsupported_problem_structure",
        SolverError::SingularMatrix => "singular_matrix",
        SolverError::IllConditioned => "ill_conditioned",
        SolverError::NumericalDomain => "numerical_domain",
        SolverError::Overflow => "overflow",
        SolverError::WorkspaceTooSmall => "workspace_too_small",
        SolverError::Cancelled => "cancelled",
        SolverError::BackendUnavailable => "backend_unavailable",
        SolverError::InternalInvariantViolation => "internal_invariant_violation",
    }
}
