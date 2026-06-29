//! Static workspace storage-block support (RFC 005).
//!
//! Baseline: the [`WorkspaceFootprint`] byte-footprint contract. Its impls for
//! the RFC 004 owned arrays are behind `owned-arrays`. No wrapper types are
//! introduced — the RFC 004 fixed arrays are themselves the storage blocks
//! (RFC 005 §4, decision M1).

/// Byte-footprint reporting for static, caller-owned workspace storage.
///
/// The footprint is the in-memory size of the storage block in bytes, used by
/// device workspaces to report `required_workspace_bytes()` (RFC 005 §5.2). The
/// canonical implementation returns `core::mem::size_of::<Self>()`; no `BYTES`
/// constant is added to the RFC 004 array types (decision M8, consistent with
/// the RFC 004 D2 decision).
pub trait WorkspaceFootprint {
    /// The storage footprint in bytes.
    fn footprint_bytes() -> usize;
}

#[cfg(feature = "owned-arrays")]
mod owned_array_impls {
    use super::WorkspaceFootprint;
    use crate::array::{FixedMatrix, FixedVector};

    impl<S, const N: usize> WorkspaceFootprint for FixedVector<S, N> {
        #[inline]
        fn footprint_bytes() -> usize {
            core::mem::size_of::<Self>()
        }
    }

    impl<S, const R: usize, const C: usize, const N: usize> WorkspaceFootprint
        for FixedMatrix<S, R, C, N>
    {
        #[inline]
        fn footprint_bytes() -> usize {
            core::mem::size_of::<Self>()
        }
    }
}

#[cfg(all(test, feature = "owned-arrays"))]
mod tests;
