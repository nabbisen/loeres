//! Compact, data-only diagnostics for `loeres-core` (RFC 003).
//!
//! Diagnostics carry optional numeric context alongside a result or solver
//! state. They contain no strings, no heap allocation, and no logging-framework
//! types, so they remain usable in `no_std` / no-`alloc` device builds. Richer,
//! human-facing reporting belongs to `loeres-cluster` or host-side tooling.

/// Coarse category describing why a [`DiagnosticSnapshot`] was produced.
///
/// Marked `#[non_exhaustive]`: downstream `match`es must include a wildcard arm.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Default)]
pub enum DiagnosticCode {
    /// No diagnostic information.
    #[default]
    None,
    /// Boundary validation rejected the input before the solve loop.
    BoundaryValidationFailed,
    /// A bounded loop reached its iteration limit.
    IterationLimit,
    /// Conditioning crossed a warning threshold without failing the solve.
    ConditioningWarning,
    /// A caller-owned workspace was reset / reinitialized before reuse.
    WorkspaceReinitialized,
    /// A cancellation request was observed.
    CancellationObserved,
}

/// Compact, copyable diagnostic context (RFC 003 §3.4).
///
/// `iteration` is the iteration count at capture. `primary_index` and
/// `secondary_index` are optional coordinates (for example a failing row/column
/// or a variable pair); their meaning is defined by the producing solver, and
/// they are `0` when unused.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Default)]
pub struct DiagnosticSnapshot {
    /// Why this snapshot was produced.
    pub code: DiagnosticCode,
    /// Iteration count at capture.
    pub iteration: u32,
    /// Primary coordinate (producer-defined), or `0` when unused.
    pub primary_index: u16,
    /// Secondary coordinate (producer-defined), or `0` when unused.
    pub secondary_index: u16,
}

// RFC 003 §3.4: diagnostics must stay device-friendly and must not silently
// grow past the budget shared with `SolverError`.
const _: () = assert!(core::mem::size_of::<DiagnosticSnapshot>() <= 16);

impl DiagnosticSnapshot {
    /// An empty snapshot: [`DiagnosticCode::None`] with all counters zero.
    ///
    /// A `const` so callers in `no_std` contexts can initialize without
    /// `Default::default()` (which is not `const`).
    pub const EMPTY: Self = Self {
        code: DiagnosticCode::None,
        iteration: 0,
        primary_index: 0,
        secondary_index: 0,
    };
}
