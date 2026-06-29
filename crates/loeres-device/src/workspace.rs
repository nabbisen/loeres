//! Caller-owned typed workspace lifecycle (RFC 005).
//!
//! [`DeviceWorkspace`] is the single essential lifecycle method (reset-on-entry);
//! [`DeviceWorkspaceDiagnostic`] is an always-available, ungated compact-diagnostic
//! extension; [`WorkspaceFor`] associates a solver family with its workspace type
//! and footprint. Concrete workspaces and solver kernels are RFC 006-owned.

use loeres::DiagnosticSnapshot;

/// Caller-owned solver workspace lifecycle.
///
/// A device workspace is owned by the caller, passed by unique `&mut`, and safe
/// to discard or immediately reuse after any solver outcome (poison-free reuse,
/// RFC 005 §7). The lifecycle core is a single logical initialization step.
pub trait DeviceWorkspace {
    /// Logically initialize the workspace for a fresh solve entry.
    ///
    /// Overwrite-on-use: this must not require zeroing the whole buffer unless a
    /// specific field must be initialized for correctness.
    fn reset_for_entry(&mut self);
}

/// Always-available compact diagnostics for a device workspace.
///
/// Kept separate from [`DeviceWorkspace`] so the lifecycle core stays minimal.
/// This accessor and [`DiagnosticSnapshot`] are never gated by the
/// `diagnostic-snapshot` feature (RFC 005 §10, decision M4); that feature governs
/// only richer/optional diagnostics.
pub trait DeviceWorkspaceDiagnostic {
    /// A compact diagnostic snapshot of the current workspace state.
    fn diagnostic(&self) -> DiagnosticSnapshot;
}

/// Associates a solver family `P` with its workspace type and footprint.
///
/// The concrete `P` problem families and `Workspace` shapes are RFC 006-owned;
/// this trait fixes only the lifecycle/sizing contract.
pub trait WorkspaceFor<P> {
    /// The workspace type for problem family `P`.
    type Workspace: DeviceWorkspace;

    /// The required workspace footprint in bytes.
    ///
    /// May be computed from `core::mem::size_of::<Self::Workspace>()` (decision M8).
    fn required_workspace_bytes() -> usize;
}

#[cfg(test)]
mod tests;
