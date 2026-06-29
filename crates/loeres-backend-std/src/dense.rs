//! Heap-backed dense vectors and matrices (RFC 007).
//!
//! `DenseVector<S>` wraps a `Vec<S>`; `DenseMatrix<S>` is a row-major `Vec<S>`
//! with `rows`/`cols`. Both implement the RFC 002 access contracts and report
//! `DimensionKind::Dynamic`. Server-only (`loeres-backend-std`).

use loeres::{
    ContiguousMatrixAccess, ContiguousVectorAccess, ContiguousVectorAccessMut, Dim2, DimensionKind,
    FiniteScalar, MatrixAccess, MatrixAccessMut, SolverError, VectorAccess, VectorAccessMut,
};

use crate::internal::dimension_mismatch;

/// Pre-allocation memory limit for dense ingestion (RFC 007 §3.5).
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct DenseIngestOptions {
    /// Maximum element count accepted, or `None` for no limit.
    pub max_elements: Option<usize>,
}

/// A dynamically sized, heap-backed dense vector.
#[derive(Clone, Debug)]
pub struct DenseVector<S> {
    data: Vec<S>,
}

impl<S: loeres::BaseScalar> DenseVector<S> {
    /// Build from an owned `Vec<S>` (no memory limit).
    pub fn from_vec(data: Vec<S>) -> Result<Self, SolverError> {
        Self::from_vec_with_options(data, DenseIngestOptions::default())
    }

    /// Build from an owned `Vec<S>`, rejecting payloads over `max_elements`
    /// with `SolverError::InvalidInput`.
    pub fn from_vec_with_options(
        data: Vec<S>,
        options: DenseIngestOptions,
    ) -> Result<Self, SolverError> {
        if let Some(max) = options.max_elements {
            if data.len() > max {
                return Err(SolverError::InvalidInput);
            }
        }
        Ok(Self { data })
    }

    /// The element count.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Whether the vector is empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

impl<S: FiniteScalar> DenseVector<S> {
    /// Scan for non-finite elements, returning `NonFiniteInput` on the first
    /// one. Validation *state* is RFC 012-owned; this is a plain check helper.
    pub fn validate_finite(&self) -> Result<(), SolverError> {
        for value in &self.data {
            if !value.is_finite() {
                return Err(SolverError::NonFiniteInput);
            }
        }
        Ok(())
    }
}

impl<S: loeres::BaseScalar> VectorAccess for DenseVector<S> {
    type Scalar = S;

    fn len(&self) -> usize {
        self.data.len()
    }

    fn dimension_kind(&self) -> DimensionKind {
        DimensionKind::Dynamic
    }

    fn get(&self, index: usize) -> Result<S, SolverError> {
        match self.data.get(index) {
            Some(&value) => Ok(value),
            None => Err(dimension_mismatch(index, self.data.len())),
        }
    }
}

impl<S: loeres::BaseScalar> VectorAccessMut for DenseVector<S> {
    fn set(&mut self, index: usize, value: S) -> Result<(), SolverError> {
        let len = self.data.len();
        match self.data.get_mut(index) {
            Some(slot) => {
                *slot = value;
                Ok(())
            }
            None => Err(dimension_mismatch(index, len)),
        }
    }
}

impl<S: loeres::BaseScalar> ContiguousVectorAccess for DenseVector<S> {
    fn as_contiguous(&self) -> Option<&[S]> {
        Some(&self.data)
    }
}

impl<S: loeres::BaseScalar> ContiguousVectorAccessMut for DenseVector<S> {
    fn as_contiguous_mut(&mut self) -> Option<&mut [S]> {
        Some(&mut self.data)
    }
}

/// A dynamically sized, heap-backed row-major dense matrix.
#[derive(Clone, Debug)]
pub struct DenseMatrix<S> {
    rows: usize,
    cols: usize,
    data: Vec<S>,
}

impl<S: loeres::BaseScalar> DenseMatrix<S> {
    /// Build a `rows`×`cols` matrix from row-major data (no memory limit).
    pub fn from_row_major_vec(rows: usize, cols: usize, data: Vec<S>) -> Result<Self, SolverError> {
        Self::from_row_major_vec_with_options(rows, cols, data, DenseIngestOptions::default())
    }

    /// Build a `rows`×`cols` matrix from row-major data, rejecting an element
    /// count over `max_elements` with `SolverError::InvalidInput`.
    pub fn from_row_major_vec_with_options(
        rows: usize,
        cols: usize,
        data: Vec<S>,
        options: DenseIngestOptions,
    ) -> Result<Self, SolverError> {
        if rows == 0 || cols == 0 {
            return Err(SolverError::InvalidDimension);
        }
        let required = rows
            .checked_mul(cols)
            .ok_or(SolverError::InvalidDimension)?;
        if let Some(max) = options.max_elements {
            if required > max {
                return Err(SolverError::InvalidInput);
            }
        }
        if data.len() != required {
            return Err(dimension_mismatch(data.len(), required));
        }
        Ok(Self { rows, cols, data })
    }

    /// The matrix dimensions.
    pub fn dims(&self) -> Dim2 {
        Dim2::new(self.rows, self.cols)
    }
}

impl<S: FiniteScalar> DenseMatrix<S> {
    /// Scan for non-finite elements, returning `NonFiniteInput` on the first.
    pub fn validate_finite(&self) -> Result<(), SolverError> {
        for value in &self.data {
            if !value.is_finite() {
                return Err(SolverError::NonFiniteInput);
            }
        }
        Ok(())
    }
}

impl<S: loeres::BaseScalar> MatrixAccess for DenseMatrix<S> {
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
        // row < rows and col < cols, so row * cols + col is in bounds.
        let offset = row * self.cols + col;
        match self.data.get(offset) {
            Some(&value) => Ok(value),
            None => Err(SolverError::InternalInvariantViolation),
        }
    }
}

impl<S: loeres::BaseScalar> MatrixAccessMut for DenseMatrix<S> {
    fn set(&mut self, row: usize, col: usize, value: S) -> Result<(), SolverError> {
        if row >= self.rows {
            return Err(dimension_mismatch(row, self.rows));
        }
        if col >= self.cols {
            return Err(dimension_mismatch(col, self.cols));
        }
        let offset = row * self.cols + col;
        match self.data.get_mut(offset) {
            Some(slot) => {
                *slot = value;
                Ok(())
            }
            None => Err(SolverError::InternalInvariantViolation),
        }
    }
}

impl<S: loeres::BaseScalar> ContiguousMatrixAccess for DenseMatrix<S> {
    fn as_row_major(&self) -> Option<&[S]> {
        Some(&self.data)
    }
}

#[cfg(test)]
mod tests;
