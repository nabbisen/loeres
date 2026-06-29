//! Heap-backed sparse storage adapters (RFC 007).
//!
//! `SparseMatrix<S>` is a compressed-sparse-row (CSR) matrix implementing
//! `MatrixAccess` with implicit-zero semantics: an in-bounds unstored entry
//! reads as `S::zero()`. Stored-vs-implicit is distinguished by the
//! `try_get_stored` extension. Server-only (`loeres-backend-std`).

use loeres::{BaseScalar, Dim2, DimensionKind, FiniteScalar, MatrixAccess, SolverError};

use crate::internal::dimension_mismatch;

/// Pre-allocation memory limit for sparse ingestion (RFC 007 §3.5).
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct SparseIngestOptions {
    /// Maximum stored-entry count accepted, or `None` for no limit.
    pub max_entries: Option<usize>,
}

/// A dynamically sized, heap-backed CSR sparse matrix.
#[derive(Clone, Debug)]
pub struct SparseMatrix<S> {
    rows: usize,
    cols: usize,
    // CSR: `row_ptr` has `rows + 1` entries; `col_idx`/`values` have `nnz`.
    row_ptr: Vec<usize>,
    col_idx: Vec<usize>,
    values: Vec<S>,
}

impl<S: BaseScalar> SparseMatrix<S> {
    /// Build a `rows`×`cols` CSR matrix from `(row, col, value)` triplets.
    ///
    /// Rejects: zero dimensions (`InvalidDimension`); more than `max_entries`
    /// (`InvalidInput`, checked before building final storage); out-of-bounds
    /// coordinates (`DimensionMismatch`); and duplicate `(row, col)` coordinates
    /// (`InvalidInput`). No combine policy in the baseline (RFC 007 §3.3).
    pub fn from_triplets(
        rows: usize,
        cols: usize,
        triplets: &[(usize, usize, S)],
        options: SparseIngestOptions,
    ) -> Result<Self, SolverError> {
        if rows == 0 || cols == 0 {
            return Err(SolverError::InvalidDimension);
        }
        // Memory limit first: avoid even temporary preparation on a known failure.
        if let Some(max) = options.max_entries {
            if triplets.len() > max {
                return Err(SolverError::InvalidInput);
            }
        }
        for &(row, col, _) in triplets {
            if row >= rows {
                return Err(dimension_mismatch(row, rows));
            }
            if col >= cols {
                return Err(dimension_mismatch(col, cols));
            }
        }

        // Temporary preparation: sort by (row, col), then reject duplicates.
        let mut entries = triplets.to_vec();
        entries.sort_by_key(|entry| (entry.0, entry.1));
        for pair in entries.windows(2) {
            if pair[0].0 == pair[1].0 && pair[0].1 == pair[1].1 {
                return Err(SolverError::InvalidInput);
            }
        }

        // Final storage: one buffer each for row_ptr, col_idx, values.
        let mut row_ptr = vec![0usize; rows + 1];
        for &(row, _, _) in &entries {
            row_ptr[row + 1] += 1;
        }
        let mut acc = 0usize;
        for slot in row_ptr.iter_mut() {
            acc += *slot;
            *slot = acc;
        }
        let mut col_idx = Vec::with_capacity(entries.len());
        let mut values = Vec::with_capacity(entries.len());
        for (_, col, value) in entries {
            col_idx.push(col);
            values.push(value);
        }

        Ok(Self {
            rows,
            cols,
            row_ptr,
            col_idx,
            values,
        })
    }

    /// The number of explicitly stored entries.
    pub fn nnz(&self) -> usize {
        self.values.len()
    }

    /// The matrix dimensions.
    pub fn dims(&self) -> Dim2 {
        Dim2::new(self.rows, self.cols)
    }

    /// `Some(value)` if `(row, col)` is explicitly stored, `None` if absent
    /// (implicit zero). Out-of-bounds returns `DimensionMismatch`.
    pub fn try_get_stored(&self, row: usize, col: usize) -> Result<Option<S>, SolverError> {
        if row >= self.rows {
            return Err(dimension_mismatch(row, self.rows));
        }
        if col >= self.cols {
            return Err(dimension_mismatch(col, self.cols));
        }
        Ok(self.lookup(row, col))
    }

    /// CSR row-local lookup. `row` must already be in bounds.
    fn lookup(&self, row: usize, col: usize) -> Option<S> {
        let start = self.row_ptr[row];
        let end = self.row_ptr[row + 1];
        match self.col_idx[start..end].binary_search(&col) {
            Ok(pos) => Some(self.values[start + pos]),
            Err(_) => None,
        }
    }
}

impl<S: FiniteScalar> SparseMatrix<S> {
    /// Scan stored values for non-finite entries, returning `NonFiniteInput` on
    /// the first. Implicit (absent) zeros need no scan.
    pub fn validate_finite(&self) -> Result<(), SolverError> {
        for value in &self.values {
            if !value.is_finite() {
                return Err(SolverError::NonFiniteInput);
            }
        }
        Ok(())
    }
}

impl<S: BaseScalar> MatrixAccess for SparseMatrix<S> {
    type Scalar = S;

    fn dims(&self) -> Dim2 {
        Dim2::new(self.rows, self.cols)
    }

    fn dimension_kind(&self) -> DimensionKind {
        DimensionKind::Dynamic
    }

    fn get(&self, row: usize, col: usize) -> Result<S, SolverError> {
        if row >= self.rows {
            return Err(dimension_mismatch(row, self.rows));
        }
        if col >= self.cols {
            return Err(dimension_mismatch(col, self.cols));
        }
        Ok(self.lookup(row, col).unwrap_or_else(S::zero))
    }
}

#[cfg(test)]
mod tests;
