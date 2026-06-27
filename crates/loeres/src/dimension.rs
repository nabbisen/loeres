//! Dimension descriptors for vector and matrix access.
//!
//! Two data-only, allocation-free types: [`Dim2`], a row/column pair, and
//! [`DimensionKind`], which records whether an extent is known at compile time
//! or at run time. Both are `Copy` and carry no storage or ownership semantics.
//!
//! Defined by RFC 002 (§3.2) and the external design §2.6.

/// The shape of a two-dimensional access object: a row count and a column count.
///
/// Dimensions are runtime values even when a backend knows them at compile time,
/// so a single trait surface spans static and dynamic storage. `Dim2` is plain
/// `Copy` data with no allocation.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Dim2 {
    /// Number of rows.
    pub rows: usize,
    /// Number of columns.
    pub cols: usize,
}

impl Dim2 {
    /// Construct a `Dim2` from a row and column count.
    #[inline]
    pub const fn new(rows: usize, cols: usize) -> Self {
        Self { rows, cols }
    }
}

/// Whether an access object's extents are fixed at compile time (`Static`) or
/// determined at run time (`Dynamic`).
///
/// This describes **only** compile-time-known versus run-time-known extents. It
/// deliberately carries no `Borrowed` variant: borrowed-versus-owned is a
/// storage/view-ownership property, not a dimension property — a borrowed view
/// may have static or dynamic dimensions. Ownership, if a future RFC needs to
/// expose it, belongs to a separate `StorageKind` / `AccessOrigin` concept, so
/// algorithms branch on shape or layout rather than on ownership.
///
/// The borrowed views in this crate report [`DimensionKind::Dynamic`], since a
/// slice carries a run-time length; the const-generic static backend (RFC 004)
/// is the source of [`DimensionKind::Static`].
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DimensionKind {
    /// Extents are known at compile time.
    Static,
    /// Extents are known only at run time.
    Dynamic,
}
