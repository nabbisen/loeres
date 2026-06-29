# loeres-backend-std

Dynamic dense/sparse storage and optional third-party numerical adapters for the server. **Server-only.**

- **Environment:** `std`, heap-backed
- **Depends on:** `loeres`
- **Status:** Active (RFC 007, v0.11.0). Dynamic storage adapters implementing
  the `loeres` access contracts; canonical validation state is RFC 012-owned.

## Public surface (v0.11.0)

- `dense` (feature `dense`, default): `DenseVector<S>` (row-major `Vec<S>`) with
  `VectorAccess` / `VectorAccessMut` / `ContiguousVectorAccess` /
  `ContiguousVectorAccessMut`; `DenseMatrix<S>` (row-major) with `MatrixAccess` /
  `MatrixAccessMut` / `ContiguousMatrixAccess`. Constructors `from_vec` /
  `from_row_major_vec` (+ `_with_options` taking `DenseIngestOptions`), plus
  `validate_finite`.
- `sparse` (feature `sparse`): `SparseMatrix<S>`, a CSR matrix with implicit-zero
  `MatrixAccess::get`, a `try_get_stored` stored-vs-implicit extension, `nnz`,
  `from_triplets` ingestion (`SparseIngestOptions`, duplicate rejection), and
  `validate_finite`.
- Adapter features `serde`, `adapter-ndarray`, `adapter-nalgebra`,
  `native-linalg` are off by default and inert pending later RFCs; `view` /
  `batch` / `adapter` are placeholders.

See the workspace [README](../../README.md), the [architecture](../../docs/src/architecture.md)
chapter, and the [RFC index](../../rfcs/README.md).

Licensed under Apache-2.0.
