# Changelog

All notable changes to Loeres are recorded here. The format is loosely based on
Keep a Changelog, and the project follows semantic versioning. Versions below
`1.0.0` are pre-stability; a `1.0.0` release requires explicit project-owner
sign-off (see RFC 000 and the requirements specification).

## [0.13.1] — 2026-06-30 — RFC 008 implementation-review corrections

A corrective patch over v0.13.0 addressing the RFC 008 implementation review
(B1–B6). No new public types; the orchestration contracts are unchanged.

### Changed

- `ClusterValidationPolicy::resolve` is now **pure policy/evidence resolution**: it
  runs no validation scans and never fabricates a `ValidationState::Validated`.
  `ValidateAllInputs` requires the validating job to supply recorded coverage (and
  rejects a trust assertion offered in lieu of validation); `RespectBackendValidationState`
  accepts a provided validated *or* trusted state. Docs no longer imply the cluster
  boundary performs scans no code can perform yet. (B1)
- Timeout deadlines are computed with `Instant::checked_add`; an extreme timeout that
  would overflow `Instant` now returns `ClusterError::InvalidConfig` instead of risking a
  panic. (B5)

### Documentation

- Root `README.md` quick-start no longer references a nonexistent `batch` feature; the
  cluster baseline is unconditional, only `parallel-rayon` is shown. (B2)
- `crates/loeres-cluster/README.md` updated from the Phase-0 skeleton note to the
  populated `batch` / `runtime` / `solve` surface, stating there is still no production
  std-side numerical kernel. (B3)
- DONE RFC 008 body reconciled with the shipped API: the `DispatchPolicy::AutoByBudget`
  sketch (removed by T3), the `DiagnosticSnapshot` open-question (resolved by D3), and the
  `sync` / `batch` feature-name posture (resolved by D6) are corrected, and an
  "Implementation decisions and departures" section records D1–D10 / T1–T3 / B1. (B4)
- `cancellation_poll_interval` and `ClusterExecutionContext::poll_interval` are documented
  as a hint for job-internal polling; the executor always checks cancellation and the
  timeout deadline at every item boundary. (B6)

### Verification

- 180 tests (2 net new cluster tests around the tightened `ValidateAllInputs` semantics).
  All gates pass — check, zero-bleed, no-std (`thumbv7em-none-eabihf`), check-rfcs,
  panic-audit.

## [0.13.0] — 2026-06-30 — RFC 008 cluster orchestration infrastructure

RFC 008 is implemented as an **orchestration-first** `loeres-cluster` slice: the
cluster orchestration machinery and contracts — **not** a production numerical
cluster solver. No std-side solver kernel exists yet (core exposes only the RFC
014 outcome vocabulary; the RFC 006 device kernel is edge-only and unreachable
from cluster), so `ClusterJob` is the stable dispatch seam where a future kernel
plugs in, and the machinery is validated against deterministic in-crate test jobs
that exercise orchestration behavior, **not** numerical correctness.

### Added — orchestration foundation (`loeres-cluster` `batch` / `runtime` / `solve`)

- `batch`: the per-item outcome contract — `BatchItemOutcome` (`Solved` /
  `Failed` / `Cancelled` / `Panicked`), `ClusterSolution` (`#[non_exhaustive]`,
  `DenseVector` variant), `BatchSolveReport`, and an explicit-count `BatchSummary`
  (`solved_converged` / `solved_not_converged` / `failed` / `cancelled` /
  `panicked`) so callers need not scan the outcome vector. The status/error split
  holds: a bounded-terminus non-convergence is `Solved` carrying
  `SolveStatus::NotConverged`, never `Failed`.
- `runtime`: `ClusterSolveConfig`, `BatchExecutionPolicy`, a reserved-but-inert
  `DispatchPolicy` (no `AutoByBudget` until RFC 010 supplies a metric),
  `ClusterValidationPolicy` consuming the RFC 012 vocabulary (`ValidateAllInputs`
  / `RespectBackendValidationState` / `TrustedByCaller`, with a `MissingCoverage`
  rejection that does not silently trust missing coverage), the cluster-owned
  `ClusterCancellationToken` (`Arc<AtomicBool>`; `cancel()` stores `Release`,
  `is_cancelled()` loads `Acquire`; cooperative, not preemptive), and a small
  `#[non_exhaustive]` `ClusterError` (`InvalidConfig` / `ExecutorInit` /
  `Shutdown`).
- `solve`: the `ClusterJob` hybrid-dispatch seam, `ClusterExecutionContext`,
  `solve_batch`, and (behind `async-tokio`) `solve_batch_async`. An empty batch is
  valid and returns an empty report. A contained worker panic is `Panicked` only
  under `panic = "unwind"`; under `panic = "abort"` the process aborts and no
  containment is promised. An inner `SolverError::Cancelled` is normalized to
  `BatchItemOutcome::Cancelled`.

### Features

- New optional, default-off feature gates `parallel-rayon` (a bounded Rayon worker
  pool) and `async-tokio` (Tokio blocking offload). The baseline synchronous batch
  path is unconditional and runtime-agnostic — no Tokio or Rayon type appears in
  the baseline public surface. The vestigial `sync` / `batch` skeleton features are
  removed.

### Verification

- 178 tests (30 new cluster tests covering summary tallying, cancellation /
  timeout / panic isolation, the inner-cancel normalization, the validation-policy
  arms, and sequential/parallel/async parity). All gates pass — check, zero-bleed,
  no-std (`thumbv7em-none-eabihf`), check-rfcs, panic-audit.

## [0.12.1] — 2026-06-30 — RFC 012 coherence hardening and doc cleanup

A corrective patch over v0.12.0 addressing the RFC 012 implementation review,
before RFC 008 consumes the validation vocabulary. No new types.

### Changed — `ValidationCoverage` coherent by construction (`loeres::validation`)

- `ValidationCoverage` fields are now private, read via `scope()` / `finite()`
  accessors, and `new(scope, finite)` normalizes the scope to always include
  `ValidationScope::FINITE`. Every `ValidationCoverage` addresses finite coverage
  (via `finite`), so the scope bit and the `finite` field can no longer
  contradict (e.g. `finite: Checked` with a scope lacking `FINITE`), and a struct
  literal can no longer bypass the invariant. (Source-incompatible only for code
  reading the fields directly; the type shipped in v0.12.0 and has no external
  consumers yet.)

### Documentation

- `crates/loeres/README.md` updated from the Phase-0 placeholder to the v0.12.0+
  implemented core surface (`scalar`, `access` / `dimension`, `error` /
  `diagnostic`, `solver`, `validation`).
- RFC 012 (done) gains a §7 implementation-decision closeout recording I1–I11,
  and §3.4 documents the `ValidationCoverage` coherence rule.

### Verification

- 148 tests (1 new: scope-normalization coherence). All gates pass — check,
  zero-bleed, no-std (`thumbv7em-none-eabihf`), check-rfcs, panic-audit.

## [0.12.0] — 2026-06-30 — RFC 012 core validation-state vocabulary

RFC 012 is implemented as a **core-first** addition: the `loeres::validation`
vocabulary describing *what input validation has been performed*. RFC 012 owns
only the representation — it runs no scans and changes no shipped solver
signature; backends remain the validators and record their outcome in these
types. Structural validity stays a construction precondition (RFC 004 / 007). All
core types are `no_std` / no-`alloc`. No new `SolverError` category; no
device / backend / cluster signature change.

### Added — `loeres::validation` (re-exported at the `loeres` root)

- `ValidationScope` — a `#[repr(transparent)]` `u8` coverage bitset with consts
  `EMPTY`, `FINITE`, `PROBLEM_CONFIG`, `PRELOOP`, and a release-local `ALL`
  (composed from the current bits, not a forever claim), plus `const fn`
  `empty` / `contains` / `union` / `intersect` and `BitOr` / `BitAnd`
  conveniences. Structural dimensions / bounds are construction-owned and are not
  bits here.
- `FiniteCoverage::{Checked, NotApplicable}` — finite-coverage for a `Validated`
  state. `NotApplicable` is permitted only for domains explicitly
  non-finite-incapable; a missing `FiniteScalar` capability is *unavailable*
  (rejected), not `NotApplicable`.
- `TrustKind` — a `#[non_exhaustive]` enum with `CallerAssertion` (RFC 008
  pipeline-trust categories added later).
- `TrustToken(u32)` — a compact numeric audit token (`new` / `value`).
- `ValidationCoverage { scope, finite }` — the recording descriptor for
  `Validated`; `const fn new` records coverage *after* the owning backend/solver
  ran the checks (not a proof-producing function).
- `TrustedByCaller { scope, kind, token, label }` — caller-assertion evidence;
  the asserted scope is visible in the value.
- `ValidationState::{Unvalidated, Validated(ValidationCoverage), Trusted(TrustedByCaller)}`
  — the state category enum.

### Notes

- Deferred by design: cluster trusted-pipeline mechanics, validation caching,
  model identity, and mutation epochs are RFC 008-owned; the shared conformance
  corpus and `xtask conformance` fixtures are RFC 013-owned. RFC 006's shipped
  `solve_projected_first_order` signature is unchanged (F3c — its existing inline
  checks satisfy the contract internally).
- RFC 012 moved `rfcs/proposed/ → rfcs/done/`; the RFC index and cross-links were
  updated accordingly. Tests: 147 total (8 new validation-vocabulary tests). All
  gates pass — check, zero-bleed, no-std (`thumbv7em-none-eabihf`), check-rfcs,
  panic-audit.

## [0.11.1] — 2026-06-30 — RFC 007 closeout corrections (construction hardening)

A corrective patch over v0.11.0, addressing the RFC 007 implementation review.
The dynamic storage constructors now fail closed on degenerate dimensions the
construction contract always intended to reject; one additive field is added to
`SparseIngestOptions`. No other public-type change.

### Fixed — dense construction (`loeres-backend-std::dense`)

- `DenseVector::from_vec` / `from_vec_with_options` reject an empty vector with
  `SolverError::InvalidDimension` (zero length), checked **before** the memory
  limit so `max_elements: Some(0)` still rejects empty rather than admitting it.

### Fixed — sparse construction (`loeres-backend-std::sparse`)

- `SparseMatrix::from_triplets` guards the CSR `row_ptr` length with
  `rows.checked_add(1)` → `InvalidDimension`, rejecting an extreme `rows` before
  any allocation (previously `rows + 1` could overflow).
- The three CSR buffers (`row_ptr`, `col_idx`, `values`) are built with
  `Vec::try_reserve_exact` as defense-in-depth; an unexpected capacity failure
  maps to `SolverError::Overflow` instead of aborting.

### Added — sparse row-dimension limit (`SparseIngestOptions::max_rows`)

- `SparseIngestOptions` gains `max_rows: Option<usize>` (additive; `Default` is
  `None`). `max_entries` bounds the stored-entry buffers; `max_rows` bounds the
  logical `rows` (the `row_ptr` buffer that `max_entries` does not cover).
  `from_triplets` rejects `rows > max_rows` with `InvalidInput` before
  allocation. Check order, all pre-allocation: zero dimensions →
  `InvalidDimension`; `rows + 1` overflow → `InvalidDimension`; `max_rows` →
  `InvalidInput`; `max_entries` → `InvalidInput`.

### Documentation

- `DenseIngestOptions::max_elements` documented as a final element-count limit
  (`len`, or `rows * cols`).
- RFC 007 (done) §3.5 reconciled with the implemented ingestion API: the
  `DenseIngestPolicy` / `SparseIngestPolicy` sketches are marked non-public
  design notes, and the Touches list reflects the as-built inline constructors
  and `internal.rs`.
- `crates/loeres-backend-std/README.md` updated from the Phase-0 placeholder to
  the v0.11.0+ public surface.

### Verification

- 139 tests (3 new dynamic-backend regressions: empty-vector rejection,
  extreme-row rejection, `max_rows` overrun). All gates pass — check, zero-bleed,
  no-std (`thumbv7em-none-eabihf`), check-rfcs, panic-audit.

## [0.11.0] — 2026-06-30 — RFC 007 dynamic dense/sparse storage adapters (Milestone 3 opens)

RFC 007 is implemented, opening Milestone 3 with the server-side dynamic storage
foundation in `loeres-backend-std`. The RFC is **storage-first**: it defines no
canonical validation-state type — that ownership is deferred to RFC 012, which is
sequenced next, ahead of RFC 008/009. All server-side `std`; zero-bleed-clean
(no dynamic type leaks into `loeres` core or `loeres-device`).

### Added — dynamic dense adapters (`loeres-backend-std::dense`, feature `dense`, default)

- `DenseVector<S>` (row-major `Vec<S>`) implements `VectorAccess`,
  `VectorAccessMut`, `ContiguousVectorAccess`, and `ContiguousVectorAccessMut`;
  `DenseMatrix<S>` (`{ rows, cols, data: Vec<S> }`, row-major) implements
  `MatrixAccess`, `MatrixAccessMut`, and `ContiguousMatrixAccess`. Both report
  `DimensionKind::Dynamic`. RFC 002 provides no `ContiguousMatrixAccessMut`, and
  none is invented.
- Constructors `from_vec` / `from_row_major_vec`, plus `_with_options` variants
  taking `DenseIngestOptions { max_elements: Option<usize> }`.
- `validate_finite() -> Result<(), SolverError>` (bound `S: FiniteScalar`) — a
  plain finite scan returning `NonFiniteInput` on the first non-finite element;
  no validation-state wrapper (RFC 012-owned).

### Added — dynamic sparse adapter (`loeres-backend-std::sparse`, feature `sparse`)

- `SparseMatrix<S>` in compressed-sparse-row (CSR) layout. `MatrixAccess::get`
  returns implicit zero (`S::zero()`) for in-bounds unstored entries;
  out-of-bounds returns `DimensionMismatch`. The
  `try_get_stored(row, col) -> Result<Option<S>, SolverError>` extension
  distinguishes a stored zero from an implicit one; `nnz()` reports the stored
  count.
- `from_triplets` ingestion with `SparseIngestOptions { max_entries: Option<usize> }`:
  rejects zero dimensions (`InvalidDimension`), over-limit payloads
  (`InvalidInput`, checked before final-storage allocation), out-of-bounds
  coordinates (`DimensionMismatch`), and duplicate `(row, col)` coordinates
  (`InvalidInput`, post-sort detection; no combine policy in the baseline).
- `validate_finite()` scans stored values only; absent entries are implicit zero
  and need no scan.

### Added — construction error mapping and internal helper

- A precise construction-error mapping: zero or overflowing extents →
  `InvalidDimension`; row-major length and sparse coordinate disagreements →
  `DimensionMismatch { lhs, rhs }` under a checked `u32` payload-fallback rule
  (both values fit `u32` → payload; otherwise `InvalidDimension`; never a
  truncating cast); duplicate and memory-limit failures → `InvalidInput`.
- `pub(crate) internal::dimension_mismatch` implementing the fallback rule,
  shared by the dense and sparse adapters (compiled only when at least one of the
  two features is enabled).

### Notes

- Features: `dense` (default) and `sparse` gate their modules; `serde`,
  `adapter-ndarray`, `adapter-nalgebra`, and `native-linalg` remain off by
  default and inert pending later RFCs. `view` / `batch` / `adapter` remain
  placeholders.
- `SparseVector` is out of scope for the baseline (deferred). Mutable sparse
  editing and efficient sparse traversal are deferred to extension APIs / later
  RFCs.
- RFC 007 moved `rfcs/proposed/ → rfcs/done/`; the RFC index and cross-links were
  updated accordingly. Tests: 136 total (20 new dynamic-backend tests). All gates
  pass — check, zero-bleed, no-std (`thumbv7em-none-eabihf`), check-rfcs, and
  panic-audit.

## [0.10.2] — 2026-06-29 — Apex `docs/specs` mirror resync (Milestone 2 currency)

Documentation-only release. No code, API, or gate behavior changes.

The in-repo apex specification mirrors under `docs/specs/` are resynced from the
owner-approved canonical specifications, advancing their currency from v0.7.0 /
Milestone-1 to **v0.10.1 / Milestone-2-complete**:

- `docs/specs/loeres-requirements-v1.md`
- `docs/specs/loeres-external-design-v1.md`
- `docs/specs/loeres-roadmap-milestones-v1.md`

The refreshed specs record Milestone 2 as complete (RFC 004 static storage,
v0.8.0; RFC 005 typed workspace mechanics, v0.9.0; RFC 006 baseline device
kernel, v0.10.0, hardened v0.10.1), the always-reusable workspace lifecycle, the
`loeres-device` `owned-arrays` feature gate, and the implemented `panic-audit`
gate. The External Design carries the owner's review patches reconciling the
workspace-lifecycle and configuration-category wording to the v0.10.1 baseline
(including ED-011, "Device Workspace Failure Semantics Are Explicit"). Mirrored
verbatim from the approved canonical artifacts.

## [0.10.1] — 2026-06-29 — RFC 006 fail-safe hardening and closeout corrections

A corrective patch over v0.10.0, addressing the RFC 006 implementation review.
No API contract change: the kernel now fails closed on inputs the fail-safe
design always intended to reject, and the closeout artifacts/gates are brought
into line. All `no_std` / no-`alloc`, verified on `thumbv7em-none-eabihf`.

### Fixed — kernel fail-safe validation (`loeres-device::solve`)

- The step scale `step_scale()` is validated before the loop: non-finite →
  `NonFiniteInput`, `<= 0` → `InvalidInput`. A zero scale produced zero iterate
  change (false `Converged`); a negative scale inverted the descent direction.
- The initial iterate is checked finite up front; each gradient and bound
  coordinate is checked finite per step (`NonFiniteInput`). With finite inputs
  and a finite positive scale, the `clamp`-projected iterate is provably finite,
  so a buggy oracle that returns `Ok(())` after writing NaN is now rejected
  rather than silently mutating the iterate.
- Dimension-mismatch payloads use a checked `usize -> u32` conversion
  (`InvalidDimension` on overflow), matching the `loeres` / `loeres-backend-static`
  convention; the previous truncating `as u32` casts are gone.
- Defensive guard: a `ContiguousVectorAccess` bound whose returned slice length
  disagrees with `len()` now yields `InternalInvariantViolation` rather than a
  silently short `zip`.

### Added — verification

- `xtask panic-audit` is implemented and wired into `release-gate`: it scans the
  `no_std` production crates (`loeres`, `loeres-backend-static`, `loeres-device`,
  excluding `tests.rs`) for `unwrap` / `expect` / `panic!` / `todo!` /
  `unimplemented!` and logging macros on code lines (RFC 006 §6.2).
- Tests for zero / negative / NaN / infinite step scale, non-finite initial
  iterate, non-finite gradient output, and a kernel-side non-finite bound check.

### Changed — documentation

- RFC 006 normative text (§3.3, the error/recovery table, §3.8, §6.6 acceptance)
  now describes the concrete `ProjectedFirstOrderWorkspace<S, N>` binding by
  shared `S, N`, with `WorkspaceFor<P>` as the sizing contract — removing the
  contradiction with §7. A new §7.1 records measured size/footprint evidence.
- `loeres-device` crate docs updated: RFC 006 is implemented (v0.10.0), not a
  placeholder.

## [0.10.0] — 2026-06-29 — Baseline deterministic device solver kernel (RFC 006)

The final Milestone 2 contract, and the first solver kernel in the library: a
bounded-iteration box/bound-constrained projected first-order solver for
`loeres-device`, all `no_std` / no-`alloc` and verified on `thumbv7em-none-eabihf`.
Minor bump: an RFC is resolved. **With RFC 004, 005, and 006 implemented,
Milestone 2 is complete.**

The kernel reports outcomes through the RFC 014 core `SolveReport`: non-convergence
at the iteration cap is an `Ok(DeviceSolveReport)` with status `NotConverged` and
termination `IterationCap`, never a `SolverError`. Errors are reserved for invalid
configuration, invalid bounds, dimension mismatch, and oracle failures.

The kernel surface lands behind the `owned-arrays` feature, since the primal and
gradient work vectors are RFC 004 `FixedVector<S, N>`.

### Added — `loeres-device::problem` (problem contract)

- `ProjectedFirstOrderProblem<S, N>` — a box/bound-constrained first-order-oracle
  contract: a read-only `Bounds` associated type (distinct from the work vectors),
  `validate_boundary`, `lower_bound` / `upper_bound`, a problem-provided
  `step_scale` (no internal division, keeping the bound at
  `FiniteScalar + MetricScalar`), `gradient_at`, and a reporting-only
  `objective_at`.

### Added — `loeres-device::solve` (kernel, report, workspace)

- `solve_projected_first_order` — the bounded-iteration kernel. Validates the
  config and problem boundary before the loop, then iterates
  `x <- clamp(x - alpha * grad f(x), lo, hi)` until the largest coordinate change
  is within `tolerance`. `x` is an explicit in/out parameter; the workspace is pure
  gradient scratch. Panic-averse (no indexing or unwrap; RFC 002 contiguous fast
  path with per-element fallback for bounds).
- `DeviceSolveReport` — a thin wrapper over the core `SolveReport`, re-exposing
  `status` / `iterations_executed` and implementing `AsCoreReport`.
- `ProjectedFirstOrderWorkspace<S, N>` — caller-owned gradient scratch, type-pinned
  to the iterate and problem by the shared `N`; implements `DeviceWorkspace` and
  `DeviceWorkspaceDiagnostic`. `WorkspaceFor<P>` remains the RFC 005 sizing
  contract, implemented by the concrete problem family.

### Timing modes

- `EarlyExitAllowed` returns as soon as the criterion is met (`converged_early`) or
  at the cap (`not_converged_cap`). Under `constant-iteration`, `ConstantIteration`
  always runs the full `max_iterations`, reporting `converged_at_cap` /
  `not_converged_cap` with `iterations_executed == max_iterations`.

### Notes

- RFC 006 moves `proposed/ -> done/`; its `§7` records the implementation-decision
  pass (I1–I10) and the departures (concrete workspace binding, concrete work
  vectors, fast-path scope, reporting-only objective).

## [0.9.0] — 2026-06-29 — Caller-owned typed workspace mechanics (RFC 005)

The second Milestone 2 contract: the two-crate caller-owned workspace boundary,
all `no_std` / no-`alloc` and verified on `thumbv7em-none-eabihf`. Minor bump:
an RFC is resolved. Only RFC 006 (the deterministic device kernel) now remains
in Milestone 2.

This RFC defines the workspace *lifecycle and storage boundary* only. Concrete
solver workspaces, problem families, the device report type, and the solve
kernel are deliberately left to RFC 006.

### Added — `loeres-backend-static::workspace` (storage-block footprint)

- `WorkspaceFootprint` — a byte-footprint contract (`footprint_bytes()`), defined
  in the baseline; impls for the RFC 004 owned arrays (`FixedVector` /
  `FixedMatrix`) sit behind `owned-arrays`. No wrapper types are introduced — the
  RFC 004 fixed arrays are themselves the storage blocks — and no `BYTES` constant
  is added to those types (`size_of`-based, consistent with v0.8.0).

### Added — `loeres-device::workspace` (lifecycle) and `::config` (policy)

- `DeviceWorkspace` — the single essential lifecycle method `reset_for_entry`
  (overwrite-on-use; no full-buffer zeroing in the correctness path).
- `DeviceWorkspaceDiagnostic` — an always-available, ungated compact-diagnostic
  extension returning the core `DiagnosticSnapshot`; the `diagnostic-snapshot`
  feature governs only richer/optional diagnostics, never this accessor.
- `WorkspaceFor<P>` — associates a solver family with its `Workspace` type and a
  `required_workspace_bytes()` footprint (computed from `size_of`).
- `DeviceSolveConfig<S>` and `TimingMode` — runtime execution policy (not
  const-generic solver identity). `TimingMode` is `#[non_exhaustive]` with the
  `ConstantIteration` variant gated behind the `constant-iteration` feature, so
  unsupported constant-iteration use fails at compile time; downstream matches use
  a wildcard arm.
- `DeviceSolveConfig::validate()` — structural validation: rejects
  `max_iterations == 0`, non-finite tolerance (`NonFiniteInput`), and negative
  tolerance (`InvalidInput`). **It does not reject zero tolerance** — whether a
  concrete solver forbids zero is RFC 006's decision (recorded closeout
  confirmation). Returns a structured `SolverError`; never panics.
- `loeres-device` gains a forwarded `owned-arrays` feature
  (`= ["loeres-backend-static/owned-arrays"]`); baseline lifecycle/config compile
  without it.

### Notes

- Implementation-decision pass (M1–M8) accepted; correction P1 (RFC 014
  report/status wording) and the five pre-coding patches applied to the RFC.
- The cross-crate §3 boundary is enforced for free by the dependency graph and
  zero-bleed; the intra-crate workspace-vs-kernel gate is deferred to RFC 006.
- RFC 005 moved `proposed/` → `done/` (status *Implemented (v0.9.0)*); RFC index
  and cross-links updated.
- 95 tests pass (62 core + 22 static backend + 11 device); full release gate
  green (check / zero-bleed / no-std / check-rfcs).



The first Milestone 2 contract. `loeres-backend-static` graduates from a
documented skeleton to a working static storage engine implementing the RFC 002
access surface, all `no_std` / no-`alloc` and verified on the bare-metal
`thumbv7em-none-eabihf` target. Minor bump: an RFC is resolved.

The const-assertion pattern used for the compile-time dimension invariants was
validated against the pinned MSRV (1.85.0, edition 2024) before the public
constructor signatures were frozen — a valid construction compiles, a mismatched
one fails with `error[E0080]` — so the `const fn -> Self` constructors carry no
runtime-`Result` fallback (RFC 004 §8.1).

### Added — `loeres-backend-static` static storage (feature-gated)

- `dimension` (baseline): re-exports `Dim2` / `DimensionKind`, the `STATIC_KIND`
  marker and `static_dim2` descriptor, plus the shared bounds-checked access
  primitives that the owned arrays and static views delegate to, using the
  RFC 002 §5.1 / ADR-020 error mapping (per-axis 2-D bounds, no silent
  `usize`→`u32` truncation, overflow failed closed as
  `InternalInvariantViolation`).
- `array` (feature `owned-arrays`): `FixedVector<S, N>` and
  `FixedMatrix<S, R, C, N>`, `#[repr(transparent)]` owned wrappers with
  compile-time dimension invariants (`N > 0`; `N == R*C` and `R,C > 0`),
  footprint constants (`ELEMENTS`, plus `ROWS` / `COLS` for matrices), and the
  RFC 002 access, mutable, and contiguous fast-path traits. Both report
  `DimensionKind::Static`.
- `view` (baseline): `StaticVectorView` / `StaticVectorViewMut` /
  `StaticMatrixView` / `StaticMatrixViewMut` — const-sized contiguous views over
  caller-owned `&[S; N]` / `&mut [S; N]` (peripheral buffers, DMA regions,
  RTOS-owned state). They implement the access traits **directly** — not as
  wrappers around the core `Dynamic`-reporting views — and report `Static`.

### Notes

- Implementation-decision pass (D1–D6) accepted: core-mirroring constructor
  names (`from_array` / `from_row_major_array`; `from_array_ref` / `_mut`;
  `from_row_major_ref` / `_mut`), `ELEMENTS` / `ROWS` / `COLS` footprint
  constants, module-level feature gating, no `trybuild` dev-dependency (the
  compile-fail property is documented and MSRV-validated), single `array.rs`
  (133 ELOC), and advanced `static-views` deferred (RFC 004 §7.2).
- RFC 004 moved `proposed/` → `done/` (status *Implemented (v0.8.0)*); RFC index
  and cross-links updated. A stale README enumeration of `rfcs/done/` (omitting
  `002` since v0.7.0) was corrected while adding `004`.
- 82 tests pass (62 core + 20 static backend); full release gate green
  (check / zero-bleed / no-std / check-rfcs).



A documentation-currency release that closes the apex-spec lag opened by v0.7.0.
The in-repo `docs/specs/` mirrors were last synced at v0.6.3 (RFC 002 shown as
design-finalized / not implemented); the canonical v0.7.0 specs have now been
reviewed, approved by the project owner, and replaced upstream, so this release
mirrors them into the repository. No design, contract, code, or public API
change; patch bump.

### Changed — `docs/specs/` resynced to the approved v0.7.0 design

The three in-repo specs are mirrored byte-identical from the approved canonical
v0.7.0 set. Net effect:

- RFC 002 moves from "design-finalized, not yet implemented" to **implemented in
  v0.7.0** across the requirements, external-design, and roadmap mirrors;
  **Milestone 1 is marked complete**; RFC 004–006 become the next (Milestone 2)
  work.
- **ADR-020** now appears in the requirements mirror, recording the exact-size
  `MatrixView::from_row_major` constructor contract (decision A1).
- Currency framing advanced from "as of v0.6.3" to "as of v0.7.0"; the v0.6.3
  crate-rename history (ADR-019) is preserved, not rewritten.
- The external-design mirror incorporates the two owner-review corrections: a
  leftover "Milestone 1 (`loeres`) is in progress" in the document-currency block
  fixed to "complete," and a §2.2 sentence reworded so trait method names are
  described as governed by the implemented Milestone 1 RFCs (changes via accepted
  RFC amendment or superseding RFC) rather than as future RFC topics.

As with the v0.6.4 reconciliation, the specs' "as of v0.7.0" framing reflects
design state, while this repository release carrying the mirror is v0.7.2.

### Security / threat model

No new data flows, external integrations, or auth logic. Existing controls
re-verified and remain valid.

### Verification

`cargo check`, `clippy -D warnings`, `fmt`, 62 tests, `xtask zero-bleed`,
`xtask no-std` (bare-metal `thumbv7em-none-eabihf`), and `xtask check-rfcs` all
pass; the mirrored specs are byte-identical to the approved canonical set.

## [0.7.1] — 2026-06-27 — Colocated unit-test layout (internal)

A test-organization refactor to match the project's testing guideline: a
module's unit tests live in a colocated `tests.rs` beside the module, not in a
centralized `src/tests/` tree. No production code, public API, or behavior
change; all 62 tests still run (now reported under their module path, e.g.
`access::tests::…`).

### Changed

- Moved each module's unit tests next to it and declared `#[cfg(test)] mod tests;`
  in the module:
  - `src/tests/access.rs` → `src/access/tests.rs`
  - `src/tests/scalar.rs` → `src/scalar/tests.rs`
  - `src/tests/error.rs`  → `src/error/tests.rs`  (new `src/error/` dir)
  - `src/tests/solver.rs` → `src/solver/tests.rs` (new `src/solver/` dir)
- Removed the central `src/tests.rs` and `src/tests/` and the `mod tests;`
  declaration in `lib.rs`.
- `CONTRIBUTING.md` "Tests" section reworded to state the colocated convention
  explicitly (keep `#[test]` out of the module file; pair `some_module.rs` with
  `some_module/tests.rs`; escalate to `some_module/tests/(group).rs`; do not
  centralize).

### Verification

`cargo check`, `clippy -D warnings`, `fmt`, 62 tests, `xtask zero-bleed`,
`xtask no-std` (bare-metal `thumbv7em-none-eabihf`), and `xtask check-rfcs` all
pass. `check-rfcs` scans the same core module files (unaffected by the test move).

## [0.7.0] — 2026-06-27 — RFC 002 storage-agnostic access contracts; Milestone 1 complete

The final Milestone 1 core contract. `loeres` gains the storage-agnostic vector
and matrix **access** contracts and the **dimension** descriptors, implemented
from the v0.6.1-patched RFC 002 (B1–B6) plus the implementation-decision review
(A1/B1/C1). This is a public-API addition; minor bump. Milestone 1 (the `loeres`
core contracts) is now closed.

### Added — `loeres::dimension`

- **`Dim2`** — a `Copy`, allocation-free row/column pair (`{ rows, cols }`) with
  a `const fn new`.
- **`DimensionKind`** — `Static` / `Dynamic` only; no `Borrowed` variant
  (ownership is not a dimension property, RFC 002 §3.2 / B6). The borrowed core
  views report `Dynamic`; `Static` is the const-generic backend's (RFC 004).

### Added — `loeres::access`

- **Access traits** (layout-agnostic, fallible): `VectorAccess` /
  `VectorAccessMut` and `MatrixAccess` / `MatrixAccessMut`. Element access
  returns `Result<_, SolverError>`; no panics, no layout commitment.
- **Optional contiguous fast path**: `ContiguousVectorAccess`,
  `ContiguousVectorAccessMut`, and `ContiguousMatrixAccess` — a kernel branches
  in once on `Some(slice)` for a tight loop and falls back to per-element access
  on `None` (scoped for the RFC 006 device kernel).
- **Borrowed reference views**: `VectorView` / `VectorViewMut`, and a simple
  contiguous **row-major** `MatrixView` / `MatrixViewMut`. Column-major,
  strided, and sub-matrix views are deferred to the backends (RFC 004 / 007).
- Files split by domain (`access/vector.rs`, `access/matrix.rs`) under the
  `access.rs` root (decision C1).

### Decisions recorded (implementation-decision review)

- **A1 — exact-size row-major views.** `MatrixView::from_row_major` requires
  `data.len() == rows * cols` exactly (overflow-checked). Both undersized and
  oversized slices are rejected; a prefix of a larger buffer must be sliced
  explicitly. Length mismatch → `DimensionMismatch { lhs: actual, rhs: required }`;
  `rows * cols` overflow → `InvalidDimension`. Chosen as the strict, safer
  baseline (relaxable later without breaking callers).
- **B1 — per-axis 2-D bounds.** Bounds checked row-then-column; a row violation
  reports `{ lhs: row, rhs: rows }`, a column violation `{ lhs: col, rhs: cols }`;
  both-invalid reports the row first. Coordinates are validated before any
  `row * cols + col` arithmetic.
- **B4 — checked `usize` → `u32`.** All index/dimension payloads are converted
  with `u32::try_from`; an oversized value maps to `InvalidDimension`, never a
  truncated payload.

### Changed

- RFC 002 moved `proposed/` → `done/` (Status "Implemented (v0.7.0)"); §3.6 and
  §5.1 wording firmed to the A1/B1 decisions; RFC index and cross-links updated.
- `ROADMAP.md` and the `README.md` state callout advanced to v0.7.0 / Milestone 1
  complete. Workspace version `0.6.4` → `0.7.0` (`[workspace.package]` only;
  internal path-dep requirements are now `version = "0"`).
- Tests: 37 → 62 (25 spec-driven access tests covering the RFC 002 §6.2 corpus,
  including too-large rejection, overflow, checked-conversion, per-axis bounds,
  the square-matrix axis-ambiguity limitation, and the fast-path `None` fallback).

### Security / threat model

The access contracts are pure in-process, safe-Rust slice access — no new data
flow, FFI, or auth surface. They uphold the existing edge controls:
`#![forbid(unsafe_code)]`, checked indexing (no `unwrap`/unchecked access in the
baseline), and no overlapping mutable views in core (B5). Existing controls
remain valid; no threat-model change required.

### Verification

`cargo check`, `clippy -D warnings`, `fmt`, 62 core tests, `xtask zero-bleed`,
`xtask no-std` (bare-metal `thumbv7em-none-eabihf` — `loeres::access` compiles
`#![no_std]` without `alloc`, RFC 002 §6.5), and `xtask check-rfcs` all pass.

> **Note — apex spec currency.** The canonical design specs (`docs/specs/` and
> the upstream project files) still describe RFC 002 as design-finalized /
> not-implemented and are dated "as of v0.6.3." They are owner-maintained apex
> artifacts and are intentionally not edited here; their RFC-002 status and
> currency will be reconciled in the next canonical revision.

## [0.6.4] — 2026-06-27 — In-repo spec mirror caught up to v0.6.3 (docs only)

A documentation-currency release that closes the spec divergence opened by the
v0.6.3 rename. In v0.6.3 the workspace was renamed `loeres-core` → `loeres`
everywhere except the canonical design specs under `docs/specs/`, which are the
project owner's apex artifacts and were left for the next canonical revision.
That revision is now accepted and replaced upstream, so this release mirrors it
into the repository. No design, contract, code, or public API change; hence a
patch bump.

### Changed — `docs/specs/` resynced to the accepted v0.6.3 design

The three in-repo specs are mirrored from the canonical design specifications
(byte-identical). Net effect:

- The `loeres-core` → `loeres` rename (and `loeres_core::` → `loeres::` module
  paths) now reflected throughout the specs.
- Currency bumped from v0.6.1 to v0.6.3 framing: Status lines, Document-currency
  blocks, the requirements §15 status snapshot, and the roadmap snapshot heading.
- **ADR-019** recorded in the requirements spec: the core contracts crate is
  named `loeres` (namespace reservation; foundation-crate-as-library-name
  convention; structural rename only).

No design content changed across v0.6.1 → v0.6.3; the specs' "current as of
v0.6.3" framing reflects design state, while this repository release carrying
the mirror is v0.6.4.

### Fixed — lagged narrative version labels

The v0.6.3 rename pass updated crate names but not release labels, leaving two
markers at v0.6.2. Both corrected to v0.6.4:

- `README.md` — Milestone 1 state callout.
- `ROADMAP.md` — "Current status" heading.

### Security / threat model

No new data flows, external integrations, or auth logic. Existing controls
re-verified and remain valid.

### Verification

`cargo check`, `clippy -D warnings`, `fmt --check`, 37 core tests,
`xtask zero-bleed`, `xtask no-std` (bare-metal `thumbv7em-none-eabihf`), and
`xtask check-rfcs` all pass; whole-tree currency sweep clean.

## [0.6.3] — 2026-06-22 — `loeres-core` renamed to `loeres`

Structural rename only. The core crate — the shared mathematical contracts
that all five crates depend on — is renamed from `loeres-core` to `loeres`,
and its directory moves from `crates/loeres-core/` to `crates/loeres/`. No
public API, contract, trait, type, or test changes; hence a patch bump under
Direction A of the rename review.

### Rationale

Naming the foundation crate `loeres` matches the Rust ecosystem convention
(`serde` / `serde_json`, `tokio` / `tokio-util`, etc.). 
Every downstream user of either the server or edge environment must
depend on this crate; `loeres = { … }` is the natural name for that import.

### Changed

- `crates/loeres-core/` directory renamed to `crates/loeres/`.
- `[package] name` in the crate manifest changed from `loeres-core` to
  `loeres`.
- Root `[workspace.dependencies]` key updated from `loeres-core` to `loeres`;
  path updated to `crates/loeres`.
- All four dependent crate manifests updated: `loeres-core = { workspace =
  true }` → `loeres = { workspace = true }`.
- `xtask` path strings updated throughout (`check_rfcs`, `no_std`, `zero_bleed`).
- Doc comments, crate-level README headings, `docs/src/` narrative, root
  `README.md`, `ROADMAP.md`, `CONTRIBUTING.md`, `SECURITY.md`, CI workflow
  comment, and sibling crate READMEs updated from `loeres-core` / `loeres_core`
  to `loeres`.
- **`done/` RFC Status fields updated** (R1 policy): rename noted in
  RFC 000, 001, 003, and 014.
- Workspace version bumped `0.6.2` → `0.6.3`.

### Not changed

- Public module paths inside the crate (`loeres::scalar`, `loeres::access`,
  etc.) are unchanged at the Rust level — the old `loeres_core::` prefix is
  replaced by `loeres::` automatically by the rename; module structure is
  identical.
- CHANGELOG historical entries for v0.1.0–v0.6.2 retain `loeres-core` as
  accurate historical record.
- `docs/specs/` canonical design specifications retain `loeres-core`; they are
  your apex artifacts and will be updated in the next canonical revision.

### Security / threat model

No executable logic change, no data flows, no integrations, no auth. Existing
controls remain valid.

### Verification

`cargo check`, `clippy -D warnings`, `fmt --check`, 37 core tests,
`xtask zero-bleed`, `xtask no-std` (bare-metal `thumbv7em-none-eabihf`), and
`xtask check-rfcs` all pass; whole-tree sweep clean (excluding CHANGELOG
history and canonical specs).

## [0.6.2] — 2026-06-22 — In-repo spec currency sync (docs only)

A documentation-currency release. The in-repo design specs under `docs/specs/`
had drifted behind the accepted design; most notably the **requirements** spec
was never reconciled with RFC 014, so it still framed non-convergence as a
panic/error case and lacked the status/error split — contradicting code shipped
in v0.5.0. This release resyncs them. No design, contract, code, or public API
change; hence a patch bump.

### Changed — `docs/specs/` resynced to the accepted v0.6.1 design

The three in-repo specs are now mirrored from the canonical design specifications
(byte-identical). Net effect of the reconciliation:

- **Requirements** — advanced from the `Draft for architecture review` / `v0.2`
  framing to `Accepted — Milestone 1 in progress`; added the Document-currency
  block; reconciled with **RFC 014** throughout (NG-007 and §3.5 no longer treat
  non-convergence as a panic/error case; **ADR-018** — non-convergence is a
  status, not an error — recorded). The §5.1.3 base-scalar amendment (ADR-017)
  and the six-tier scalar model were already present.
- **External design** and **Roadmap** — advanced to the `Accepted` /
  current-as-of-v0.6.1 framing with their Document-currency blocks.

### Fixed — stale narrative

- `docs/src/threat-model.md` — corrected the "contains no executable code yet"
  framing; `loeres-core` has shipped contracts since v0.4.0.
- `ROADMAP.md` — corrected the core test count (36 → 37, matching v0.6.1) and
  bumped the status label to v0.6.2.

### Security / threat model

No new data flows, external integrations, or auth logic. Per the release policy,
existing controls were re-verified and remain valid; the threat-model change is
limited to the stale framing sentence above.

### Verification

`cargo check`, `clippy -D warnings`, `fmt --check`, 37 core tests,
`xtask zero-bleed`, `xtask no-std` (bare-metal `thumbv7em-none-eabihf`), and
`xtask check-rfcs` all pass; whole-tree currency sweep clean.

## [0.6.1] — 2026-06-21 — v0.6.0 architect-review response; RFC 002 design patched

The v0.6.0 architecture review **conditionally approved** the implemented core
(RFC 001/003/014 — accepted, no rollback) and directed design patches to RFC 002
before coding, plus a few hygiene items. This patch release applies them; no
shipped public API changes (hence a patch bump).

### Changed — RFC 002 design (patched before implementation, per review B1–B6)

`rfcs/proposed/002-storage-agnostic-contracts.md` revised:

- **B1** — module is `dimension` (matching the crate and external design), not
  `dim`; the `Touches` line and re-exports updated.
- **B6** — `DimensionKind` drops `Borrowed` (an ownership property, not a
  dimension property); it now carries only `Static` / `Dynamic`.
- **B2** — core owns only a **simple contiguous row-major** matrix view;
  column-major / strided / sub-matrix views are deferred to
  `loeres-backend-static` (RFC 004). `StrideKind` removed from core.
- **B3** — added an **optional** contiguous fast-path surface
  (`ContiguousVectorAccess` / `ContiguousVectorAccessMut` / `ContiguousMatrixAccess`,
  returning `Option<&[S]>`), narrowly scoped to RFC 006 kernel needs; the
  fallible per-element traits remain the baseline. No sparse traversal (RFC 007).
- **B4** — added an explicit **access-error mapping** over the RFC 003
  `SolverError` set, with a rule that `usize` → `u32` diagnostic conversions are
  checked and **never silently truncated**.
- **B5** — core admits **no overlapping mutable views**; any future custom-strided
  mutable view (RFC 004) must be injective or read-only.
- Aligned the static-dispatch audit to `xtask check-public-api` (RFC 010),
  consistent with RFC 014.

### Changed — hygiene

- **M2** — Requirements title `v0.2 (Actually v1)` → `v1`.
- **M1** — `xtask` now documents that its command semantics (`release-gate` as the
  aggregate; `check-rfcs` as a core-module source scan) are **temporary
  scaffolding** to be reconciled with RFC 010 before that RFC is accepted (where
  `check` is the aggregate, source scans move to `check-public-api`/a source-lint,
  and `check-rfcs` validates the RFC index).

### Added — tests

- Pinned `DivisibleScalar::checked_div` behavior on **NaN / infinity operands**
  (architect §5.1): non-finite operands do not yield `Ok(NaN/inf)`; `2.0 / inf`
  legitimately yields `Ok(0.0)`. (`loeres-core`: 37 tests now.)

### Not changed (deferred by the review)

- The full `xtask` ↔ RFC 010 command reconciliation lands before RFC 010 is
  accepted. `MetricScalar::epsilon()` remains provisionally named (RFC 006/013).
  `AdvancedNumericalScalar` remains baseline-unimplemented for primitives.

### Release audit

- **Security.** No code behavior change beyond an added test; the RFC 002 and
  documentation edits introduce no data flows, integrations, or auth. No
  threat-model change; existing controls remain valid. The RFC 002 access-error
  mapping and no-overlapping-mutable-view rule *strengthen* the eventual access
  layer's fail-closed and aliasing posture.
- **Docs.** Requirements (title), ROADMAP, README state, RFC 002, and CHANGELOG
  reconciled; whole-tree link/version sweep verified.

## [0.6.0] — 2026-06-21 — RFC 001: stratified scalar capability model

Third Milestone 1 contract, and resolution of the base-scalar ordering blocker.
The architect chose **Direction B** (base scalar excludes ordering; ordering is
the separate `OrderedScalar` tier — the RFC 001 model). RFC 001 moves to
`rfcs/done/`.

### Decided / reconciled

- **Architect decision (Direction B), recorded as ADR-017:** the base scalar tier
  excludes ordering; ordering is `OrderedScalar`, and metric comparison is
  `MetricScalar: OrderedScalar`. This keeps storage/access traits free of
  comparison semantics and lets solvers state their numerical needs explicitly.
- **Requirements §5.1.3 amended** to match: the "Base scalar" row no longer
  claims ordering, and a new "Ordered scalar" row was added. (The blocker was
  tracked informally as "§5.1.2"; the wording is actually in §5.1.3.) RFC 001,
  the External Design, and the Roadmap already reflected this model.

### Added

- **`loeres_core::scalar`** — the six-tier capability model (RFC 001):
  - `BaseScalar: Copy + Clone + PartialEq + Sized` — method-based arithmetic
    (`zero`/`one`/`add`/`sub`/`mul`/`neg`/`is_zero`); requires neither
    `PartialOrd` nor `Debug`.
  - `OrderedScalar: BaseScalar + PartialOrd` — `min`/`max`/`clamp`, with a
    **NaN-propagating** float contract (deliberately unlike `f32::min`) and a
    panic-free `clamp` (returns `hi` if `lo > hi`).
  - `FiniteScalar: BaseScalar` — `is_finite`/`is_nan`/`is_infinite`.
  - `DivisibleScalar: BaseScalar` — `checked_div`/`checked_recip` returning
    `Result<_, SolverError>` (zero denominator → `NumericalDomain`; non-finite
    quotient → `Overflow`; never `Ok(inf/NaN)`).
  - `MetricScalar: OrderedScalar` — `abs`/`epsilon`/`lte_tolerance`.
  - `AdvancedNumericalScalar: DivisibleScalar + MetricScalar` — `checked_sqrt`/
    `checked_ln`/`checked_exp`; **not** implemented for primitive floats in
    baseline core (requires `libm` or a later adapter).
  - Baseline `f32`/`f64` implementations of the five non-advanced tiers
    (`crates/loeres-core/src/scalar/primitive.rs`), via a DRY macro.
- Crate-root re-exports of all six scalar traits.
- 15 spec-driven tests (`loeres-core/src/tests/scalar.rs`): base algebra,
  ordering/NaN propagation, clamp bounds + NaN + inverted-bounds, guarded
  division (including overflow-to-non-finite), finite mutual-exclusivity, and the
  scalar laws. Tests use UFCS where the trait method shadows an inherent float
  method.

### Changed

- RFC 001 moved `proposed/` → `done/` (Implemented (v0.6.0)); RFC index and all
  cross-references updated.
- `cargo xtask check-rfcs` now also audits `scalar.rs` and `scalar/primitive.rs`.
- ADR-017 added to Requirements §15 and to `docs/src/adr.md`.
- Workspace version `0.5.0` → `0.6.0`.

### Verified

- 36 tests pass (12 error + 9 solver + 15 scalar); `release-gate` green
  (check / zero-bleed / **no-std** / check-rfcs) — the scalar tiers add only
  `core`, so the edge crates still build `no_std`/no-`alloc` for
  `thumbv7em-none-eabihf`; fmt + clippy `-D warnings` clean.

### Design notes

- On a concrete `f64`, `x.min(y)` resolves to the **inherent** (NaN-ignoring)
  method; Loeres's NaN-propagating semantics apply through the trait in generic
  solver code (`S: OrderedScalar`), which is the intended use. The primitive impl
  documents this.
- `MetricScalar::epsilon()` returns the type's machine epsilon as a provisional
  default tolerance unit; the name is provisional (RFC 001 §3.6), to be confirmed
  or renamed by RFC 006/013 before first public release.
- `AdvancedNumericalScalar` is defined but intentionally has no baseline
  primitive impl; a `libm`-gated impl is a later increment.

### Release audit

- **Security.** RFC 001 adds pure `no_std` math traits and `f32`/`f64` impls — no
  `unsafe`, data flows, integrations, or auth. No threat-model change; existing
  controls remain valid. The contract *strengthens* the device safety posture:
  division is guarded (`Result`, never a panic or silent inf/NaN), `min`/`max`/
  `clamp` are panic-free, and NaN propagation surfaces contract violations to the
  next finite check rather than masking them.
- **Docs.** Requirements (§5.1.3, ADR-017), ROADMAP, README, CHANGELOG, RFC index
  updated; whole-tree cross-reference sweep verified.

## [0.5.0] — 2026-06-21 — RFC 014: core solver outcome/status taxonomy

Second Milestone 1 contract. `loeres-core` gains the `Ok`-side of the outcome
split, completing the core taxonomy alongside RFC 003's `Err` side. RFC 014
moves to `rfcs/done/`. Taken out of strict sequence ahead of RFC 001/002
because RFC 014 depends only on the (done) RFC 003, is fully scalar-agnostic,
and the RFC itself specifies implementation "directly after RFC 003"; RFC 001
(scalars) remains gated on the requirements §5.1.2 flag, and RFC 002 (access)
bounds on RFC 001.

### Added

- **`loeres_core::solver`** implementing the RFC 014 status/error split, where
  **non-convergence is a status, not an error**:
  - `StepOutcome` (`Continue` / `Converged` / `NoProgress`), `SolveStatus`
    (`Converged` / `NotConverged`, with `const is_converged`), and
    `TerminationReason` (`ConvergenceCriterion` / `IterationCap` / `NoProgress`)
    — all `#[repr(u8)]`, `#[non_exhaustive]`, **1 byte** each.
  - `IterationReport` and `SolveReport` — `#[repr(C)]`, private fields, public
    `const` constructors/accessors. `SolveReport` exposes only the four valid
    `(status, termination)` combinations via named constructors
    (`converged_early`, `converged_at_cap`, `not_converged_cap`,
    `not_converged_stalled`); the two invalid combinations are unconstructable.
    `SolveReport` is **12 bytes** (`IterationReport` 8), scalar-agnostic, and
    deliberately excludes `DiagnosticSnapshot` to stay within the 16-byte core
    ceiling.
  - `AsCoreReport` — the static-dispatch projection trait by which device
    (RFC 006) and cluster (RFC 008) reports map losslessly onto `SolveReport`.
  - Compile-time size/representation assertions per RFC 014 §4.1.
- Crate-root re-exports of all six solver types.
- 9 spec-driven tests in `loeres-core/src/tests/solver.rs` (sizes, one-byte
  enums, the four valid combinations, accessor round-trips, the `AsCoreReport`
  round-trip via a reference report, and the RFC 003 reconciliation).

### Changed

- RFC 014 moved `proposed/` → `done/` (Implemented (v0.5.0)); RFC index and all
  cross-references updated.
- Tests reorganized into `loeres-core/src/tests/` (`error.rs`, `solver.rs`) per
  the file-separation guidance, with `tests.rs` as the module index.
- `cargo xtask check-rfcs` now also audits `solver.rs` (same no-format/no-alloc
  and `#[non_exhaustive]` rules).
- Workspace version `0.4.0` → `0.5.0`.

### Verified

- 21 tests pass (12 error + 9 solver); `release-gate` green
  (check / zero-bleed / no-std / check-rfcs); fmt + clippy `-D warnings` clean.
- RFC 003 reconciliation holds: `SolverError` carries no non-convergence
  variant and no `PanicGateViolation` (confirmed by test).

### Deferred (RFC 000 granularity)

- The concrete `AsCoreReport` derivations — `DeviceSolveReport` (RFC 006,
  Milestone 2) and the cluster per-item report (RFC 008, Milestone 3) — land
  with those crates; the core trait and a reference round-trip are complete now.
- `xtask check-public-api` enforcement of the §6.3 rules (`dyn AsCoreReport`
  absent from edge APIs, no non-convergence error variant) is owned by RFC 010;
  the core satisfies the rules by construction.

### Release audit

- **Security.** RFC 014 adds only `Copy` plain-data types — no `unsafe`, data
  flows, integrations, or auth. No threat-model change; existing controls remain
  valid. The status/error split *strengthens* the device posture: non-convergence
  is a well-defined bounded outcome rather than a panic or error path.
- **Docs.** RFC index, CHANGELOG, ROADMAP, README updated; whole-tree
  cross-reference sweep verified.

## [0.4.0] — 2026-06-21 — RFC 003: allocation-free error topology

First Milestone 1 contract. `loeres-core` now ships real public API: the
allocation-free error and diagnostic topology specified by RFC 003. RFC 003
moves to `rfcs/done/` (Implemented). This is the first unit of the core
sequence (RFC 003 → 001 → 002 → 014).

### Added

- **`loeres_core::error`** — `SolverError`, a 13-variant `#[non_exhaustive]`,
  `Copy` error enum (the canonical set: `DimensionMismatch { lhs, rhs }`,
  `InvalidDimension`, `InvalidInput`, `NonFiniteInput`,
  `UnsupportedProblemStructure`, `SingularMatrix`, `IllConditioned`,
  `NumericalDomain`, `Overflow`, `WorkspaceTooSmall`, `Cancelled`,
  `BackendUnavailable`, `InternalInvariantViolation`). Implements `Debug` but
  **not** `Display`/`core::error::Error`. Adds `const fn error_code_to_str`
  (stable `snake_case` codes) and `const` classifiers `is_input_error` /
  `is_numerical_error` / `is_resource_error`.
- **`loeres_core::diagnostic`** — `DiagnosticCode` (`#[non_exhaustive]`) and the
  data-only `DiagnosticSnapshot { code, iteration, primary_index,
  secondary_index }` with a `const EMPTY` and `Default`.
- Crate-root re-exports: `SolverError`, `error_code_to_str`, `DiagnosticCode`,
  `DiagnosticSnapshot`.
- **Compile-time size budgets** (RFC 003 §3.3/§3.4): `const` assertions pin
  `size_of::<SolverError>() <= 16` and `size_of::<DiagnosticSnapshot>() <= 16`
  (both measure **12 bytes**).
- **`cargo xtask check-rfcs`** promoted from scaffold to a real gate enforcing
  RFC 003 §6.2 (no `Display`/`error::Error`/`format!`/`String`/`Vec`/`Box`/
  `alloc` in core error code) and §6.4 (`#[non_exhaustive]` on public
  error/diagnostic enums); added to `release-gate`.
- 12 spec-driven tests in `loeres-core/src/tests.rs` validating the variant set,
  size budgets, code stability/uniqueness, classification exclusivity, `Debug`,
  and that non-convergence is **not** an error variant (RFC 014).

### Changed

- RFC 003 moved `proposed/` → `done/` (Status: Implemented (v0.4.0)); RFC index
  updated; all inbound/outbound RFC cross-references rewritten to the new paths.
- Workspace version `0.3.0` → `0.4.0` (a resolved RFC is a minor bump).

### Design notes / deferred

- `error_code_to_str` matches exhaustively inside the crate, so adding a variant
  is a compile error until the mapping is updated — totality by construction.
- The three classifier helpers use a documented grouping (input = malformed
  caller data; numerical; resource); `UnsupportedProblemStructure` and
  `InternalInvariantViolation` are intentionally in no group. Flagged for
  architect confirmation of the exact partition.
- `loeres-cluster` will later wrap `SolverError` in a `Display`/`std::error::Error`
  type at the server boundary (RFC 003 §4.4); not part of core.

### Release audit

- **Security.** RFC 003 adds only plain, `Copy`, allocation-free data types —
  no `unsafe`, no data flows, no external integrations, no auth. No threat-model
  change; existing controls remain valid. The structured fail-closed error set
  in fact *supports* the threat model (no panics, no string leakage on device
  paths), and the new `check-rfcs` gate mechanically enforces the no-format /
  no-alloc core constraint.
- **Docs.** RFC index, CHANGELOG, ROADMAP, and README reflect the new state;
  whole-tree cross-reference sweep verified (no stale `proposed/003` links).

### Still open (architect)

- Requirements §5.1.2 base-scalar wording flag — gates **RFC 001** (next in the
  sequence), not RFC 003. Recommend clearing it before scalar implementation.

## [0.3.0] — 2026-06-21 — Phase 0: Cargo workspace skeleton

First implementation phase (roadmap §12.1; external design §1). This release
instantiates the workspace structure and the verification gates the structure
can already satisfy. It contains **no** solver, scalar, access, or validation
logic — those land in Milestone 1+. Design-before-code is preserved: the
skeleton realizes already-accepted structure and does not pre-empt the open
design rounds.

### Added

- **Cargo workspace** (`resolver = "3"`, edition 2024, MSRV 1.85) with the five
  crates and shared metadata via `[workspace.package]`:
  - `loeres-core` — `#![no_std]`, no `alloc`, `#![forbid(unsafe_code)]`, no deps.
  - `loeres-backend-static` — `#![no_std]`, no `alloc`; depends on `loeres-core`.
  - `loeres-device` — `#![no_std]`, no `alloc`, `#![forbid(unsafe_code)]`;
    depends on `loeres-core` + `loeres-backend-static`.
  - `loeres-backend-std` — `std`; depends on `loeres-core`.
  - `loeres-cluster` — `std`; depends on `loeres-core` + `loeres-backend-std`.
  Each crate carries its public module topography (external design §1.5) as
  documented placeholder modules, each tracing to its owning RFC. The feature
  surface from external design §1.6 is declared (no-op until its RFC wires it).
- **`xtask` automation crate** with the gates the skeleton supports implemented
  for real — `zero-bleed` (forbidden server↔edge dependency edges, roadmap §5.5),
  `no-std` (edge crates build for `thumbv7em-none-eabihf`), `check`, and an
  aggregate `release-gate` — plus the remaining RFC 010 / §5.4 gates registered
  as honest scaffolds. `cargo xtask <cmd>` alias added.
- **`rust-toolchain.toml`** pinning stable + rustfmt/clippy + the bare-metal
  target; **CI workflows** (`ci`, `no-std`, `msrv`, `release`) wired to `xtask`;
  `.github/SECURITY.md` and issue templates.
- **Docs:** per-crate `README.md`; maintainer docs `docs/src/development.md`
  (local dev / xtask) and `docs/src/adr.md` (ADR index), wired into the mdbook
  `SUMMARY.md`.

### Verified

- `cargo check --workspace --all-features` — clean.
- `cargo clippy --workspace --all-features -- -D warnings` — clean.
- `cargo xtask zero-bleed` — **PASS** (no forbidden dependency edge).
- `cargo xtask no-std` — **PASS** (`loeres-core`, `loeres-backend-static`,
  `loeres-device` build `no_std`/no-`alloc` for `thumbv7em-none-eabihf`).
- `cargo fmt --all` applied.

This meets the Phase 0 acceptance criteria (roadmap §12.1): the workspace
compiles with placeholder crates, edge-facing crates have no forbidden
dependency path, and the docs explain the server/edge split.

### Release audit

- **Security.** No executable application logic, data flows, external
  integrations, or auth were introduced — the crates expose no public API and
  the only runtime code (`xtask`) is a local dev tool that shells out to cargo.
  No new attack surface; the design-level threat model and its controls
  (compile-time server/edge isolation, FFI cluster-only/default-off,
  panic-aversion, boundary validation) remain valid. The structural isolation
  control is now **machine-enforced** by `zero-bleed` + `no-std` rather than
  asserted only in prose. `SECURITY.md` added.
- **Docs consistency.** README, ROADMAP, and CHANGELOG reflect the Phase 0
  state; the workspace layout matches external design §1.1/§1.5.

### Deferred (unchanged from v0.2.0)

- `examples/` (cluster/device) arrive with their solver milestones (M2/M3).
- Requirements §5.1.2 base-scalar wording flag remains open for the architect;
  it gates Milestone 1 scalar code, not this skeleton.

## [0.2.0] — 2026-06-21 — RFC 001 `OrderedScalar` split resolved

Design / governance baseline increment. This release resolves the first open
design round (RFC 001 — `OrderedScalar` scalar-tier split) and reconciles the
design-layer documents with it. No implementation code is included yet; coding
still follows the design-before-code workflow once Phase 0 (workspace skeleton)
lands.

### Changed

- **RFC 001 — Stratified Scalar Capability Model: five tiers → six tiers.**
  Adds `OrderedScalar` as Tier 2 (between `BaseScalar` and `FiniteScalar`):
  - `BaseScalar` now requires only `Copy + Clone + PartialEq + Sized` — it no
    longer requires `PartialOrd` or `core::fmt::Debug`. Ordering, `min`, `max`,
    and `clamp` move to `OrderedScalar`.
  - `OrderedScalar: BaseScalar + PartialOrd` defines Loeres-owned `min` / `max` /
    `clamp` with a **NaN-propagating** contract for floating-point (deliberately
    unlike `f64::min` / `f64::max`); `clamp` is panic-free with a documented
    `lo <= hi` precondition validated at the solve boundary.
  - Supertrait graph: `FiniteScalar: BaseScalar`, `DivisibleScalar: BaseScalar`,
    `MetricScalar: OrderedScalar`, `AdvancedNumericalScalar: DivisibleScalar +
    MetricScalar`. A `MetricScalar` bound therefore implies `OrderedScalar`.
  - `DivisibleScalar::checked_div` must not return `Ok` containing NaN/∞: finite
    operands whose quotient is non-finite return `Err` (`Overflow` / numerical
    domain), keeping near-zero conditioning a solver-level `MetricScalar` concern.
  - `AdvancedNumericalScalar` for primitive floats is **not** baseline core work
    (requires `libm` or a later adapter decision); transcendentals stay out of
    baseline core.
  - `epsilon()` accepted only as a provisional name (candidate
    `algorithmic_epsilon()`); to be re-decided by RFC 006 / RFC 013 before first
    public release.
  - New verification: ordering/NaN tests (§6.4) and scalar-law tests (§6.5).
- **External design reconciled to six tiers.** §2.2 scalar-family table adds the
  `OrderedScalar` row and corrects `BaseScalar` (equality only, no ordering);
  §2.3 adds an `OrderedScalar` opt-in row; §9 open question #2 (whether
  `BaseScalar` requires `PartialOrd`) is marked **resolved**.
- **Roadmap reconciled to six tiers.** §2.3 (RFC 001) capability table adds the
  `OrderedScalar` row; the "must not require division" constraint becomes "must
  not require ordering or division"; the `PartialOrd`-sufficiency and NaN-
  semantics risks are annotated as resolved.
- **ROADMAP.md / README.md** updated: open design round #1 (RFC 001
  `OrderedScalar`) is resolved; the README Design Notes describe the six tiers.

### Known reconciliation flag (deferred to the architect)

- **Requirements §5.1.2** still describes the base scalar as having
  "equality/ordering behavior", which now contradicts the six-tier `BaseScalar`
  (equality only). This apex requirements wording was **left unchanged** pending
  architect confirmation; the suggested amendment is to move "ordering" to the
  `OrderedScalar` capability. (Sibling RFCs 002/004/005/006/007 remain valid:
  `BaseScalar` is still the correct storage bound and `MetricScalar` now implies
  `OrderedScalar`, so RFC 006's box-projection step gains `clamp` for free.)

### Release audit

- **Security.** Documentation/RFC-only change — no executable code, data flows,
  external integrations, or auth logic — so no new attack surface is introduced.
  The design-level threat model (requirements §8; external design §5;
  `docs/src/threat-model.md`) and its controls (compile-time server/edge
  isolation, FFI restricted to the cluster crate and default-off, boundary
  validation, panic-aversion) remain valid and unchanged.
- **Documentation consistency.** The scalar model is now uniform across RFC 001,
  the external design, and the roadmap (no residual "five-tier" wording and no
  `BaseScalar`-with-ordering statements outside the flagged requirements line).

## [0.1.0] — 2026-06-21 — Design baseline

First release. This is a **design / governance baseline**: the public boundary,
crate topology, and contracts are frozen as accepted and proposed RFCs. No
implementation code is included yet; coding follows the design-before-code
workflow once the remaining design rounds land.

### Added

- Governing specifications (`docs/specs/`): requirements, external design, and
  roadmap & milestones (all v1).
- RFC set under `rfcs/`:
  - `done/000` — RFC lifecycle policy.
  - `proposed/001`–`009` — Milestone 1–3 contracts (scalar capabilities;
    storage-agnostic access; allocation-free errors; static storage; typed
    workspace; deterministic device kernel; dynamic/sparse backend; async
    orchestration; observability/FFI).
  - `proposed/010`–`013` — cross-cutting contracts (xtask verification
    governance; target profiles & deterministic math; validation-state policy;
    conformance corpus & numerical parity).
  - `proposed/014` — core solver outcome & status taxonomy.
- Standard project scaffolding: `README.md`, `CONTRIBUTING.md`, `LICENSE`
  (Apache-2.0), `NOTICE`, and `ROADMAP.md`; plus an mdbook skeleton (`docs/src/`)
  with a Maintainers & Contributors section that bridges the rendered book to
  the raw specifications and RFCs.

### Changed (design reconciliation incorporated into this baseline)

- Introduced RFC 014 as the single owner of the `loeres_core::solver` taxonomy;
  non-convergence at the iteration cap is now a **status**
  (`SolveStatus::NotConverged` + `TerminationReason::IterationCap`), not a
  `SolverError`.
- Reconciled RFCs 003, 005, 006, 008, 010, 011, and 013 with RFC 014: the
  canonical 13-variant `SolverError` set (`u32` dimension payloads; no
  `MaxIterationsReached`; no runtime `PanicGateViolation`); device report
  derivation via `AsCoreReport`; per-item batch outcomes carrying a core report;
  `check-public-api` governance; `SolveStatus`-based conformance.
- Cross-document cleanup: unified the RFC folder scheme to `proposed/done/archive`
  (RFC 000), flat-renumbered the roadmap and external-design RFC references,
  inserted RFC 014 into the dependency graphs, and corrected the RFC 011 target
  profiles (`thumbv7em-none-eabi` soft-float; `riscv32imac-unknown-none-elf`
  advisory; f32-first hard-float reference).

### Release audit

- **Security.** This release contains documentation and RFCs only — no
  executable code, no data flows, no external integrations, and no
  authentication logic — so no new attack surface is introduced. The
  design-level threat model (requirements §8; external design §5; consolidated
  in `docs/src/threat-model.md`) remains valid, and its controls (compile-time
  server/edge isolation, FFI restricted to the cluster crate and default-off,
  boundary validation, panic-aversion) are preserved by the current RFC set.
- **Documentation consistency.** The governing docs were verified against the
  reconciled RFC set: no stale `MaxIterationsReached` / `ConvergenceStatus`
  terminology, no milestone-style RFC numbering, and no folder-scheme drift
  outside RFC 014's explanatory prose.

[0.13.1]: https://github.com/nabbisen/loeres/releases/tag/v0.13.1
[0.13.0]: https://github.com/nabbisen/loeres/releases/tag/v0.13.0
[0.12.1]: https://github.com/nabbisen/loeres/releases/tag/v0.12.1
[0.12.0]: https://github.com/nabbisen/loeres/releases/tag/v0.12.0
[0.11.1]: https://github.com/nabbisen/loeres/releases/tag/v0.11.1
[0.11.0]: https://github.com/nabbisen/loeres/releases/tag/v0.11.0
[0.10.2]: https://github.com/nabbisen/loeres/releases/tag/v0.10.2
[0.10.1]: https://github.com/nabbisen/loeres/releases/tag/v0.10.1
[0.10.0]: https://github.com/nabbisen/loeres/releases/tag/v0.10.0
[0.9.0]: https://github.com/nabbisen/loeres/releases/tag/v0.9.0
[0.8.0]: https://github.com/nabbisen/loeres/releases/tag/v0.8.0
[0.7.2]: https://github.com/nabbisen/loeres/releases/tag/v0.7.2
[0.7.1]: https://github.com/nabbisen/loeres/releases/tag/v0.7.1
[0.7.0]: https://github.com/nabbisen/loeres/releases/tag/v0.7.0
[0.6.4]: https://github.com/nabbisen/loeres/releases/tag/v0.6.4
[0.6.3]: https://github.com/nabbisen/loeres/releases/tag/v0.6.3
[0.6.2]: https://github.com/nabbisen/loeres/releases/tag/v0.6.2
[0.6.1]: https://github.com/nabbisen/loeres/releases/tag/v0.6.1
[0.6.0]: https://github.com/nabbisen/loeres/releases/tag/v0.6.0
[0.5.0]: https://github.com/nabbisen/loeres/releases/tag/v0.5.0
[0.4.0]: https://github.com/nabbisen/loeres/releases/tag/v0.4.0
[0.3.0]: https://github.com/nabbisen/loeres/releases/tag/v0.3.0
[0.2.0]: https://github.com/nabbisen/loeres/releases/tag/v0.2.0
[0.1.0]: https://github.com/nabbisen/loeres/releases/tag/v0.1.0
