# Developer Handoff — RFC 027 QP contract and constrained projected kernel

**Governing RFC.** `rfcs/accepted/027-qp-contract-and-constrained-kernel.md` (design frozen 2026-09-12, review 043 applied). **Assigned to.** Implementer tier.
**Ordering.** Do **not** begin until RFC 026's gate is enforced on `main` and `0.21.0` is cut. This RFC's release is the first minor after `0.21.0`.

## 1. Purpose
Make the library true beyond one solver family: a storage-agnostic QP contract in `loeres::problem`, and both projected-first-order kernels extended to `Ax ≤ b` via a bounded Dykstra projection. Everything else in the project stays as it is.

## 2. Read first
RFC 027 §11 in full, especially §11.2 (**the box is its own Dykstra set with an increment vector — not a clamp inside the sweep**; review 043 R1) and §11.3 (**terminal constraint violation is recorded**; R2). RFC 006 §7 and RFC 016 §7 for the kernels you are extending. RFC 012 for trust semantics.

## 3. Slices (each independently reviewable — submit one review request per slice)
**S1 — core contract.** `loeres::problem`: `QuadraticObjective<S>`, `BoxBounds<S>`, `LinearInequalities<S>`, `QuadraticProgram<S>` over RFC 002 access traits; blanket first-order oracle `∇f = Qx + c` so existing kernels accept a `QuadraticProgram`. `#![no_std]`, no alloc, no `dyn`, `#[non_exhaustive]` on any enum. Tests: trait bounds compile for a static and a dynamic backend without importing each other. **Stop and report** if a required method would force a layout or an allocation.

**S2 — device projection + kernel.** `ConstrainedProjectedWorkspace<S, N, M>` (`3N + 2M` scalars, `WorkspaceFootprint`, `reset_for_entry`). Dykstra over two sets: Hildreth multipliers for `m` halfspaces (checked division by precomputed `‖aᵢ‖²`, validated `> 0` and finite at the boundary, overflow → `Overflow`), exact `clamp` for the box with its increment vector. Outer loop unchanged from RFC 006. Record gains `projection_cap_hits` and `max_constraint_violation`. **`m = 0` must reproduce `solve_projected_first_order` bit-for-bit** — write that test first. Bounds `S: FiniteScalar + MetricScalar + DivisibleScalar`; no `AdvancedNumericalScalar`. Behind `owned-arrays`. panic-audit clean.

**S3 — cluster kernel.** Same algorithm over `DenseVector`/`DenseMatrix`, allocation at construction only, plugged into the `ClusterJob` seam beside RFC 016's adapter. CSR `A` via `MatrixAccess` if it needs no contiguous fast path in the inner loop; report if it does. Validation per RFC 012/016 (structural always; finite scans skippable under `TrustedByCaller`; hot loop never).

**S4 — conformance.** Fixtures: every existing `m = 0` fixture passes through the new kernel with identical results; dimension-2/3 with 1–3 halfspaces against closed-form optima; an infeasible polyhedron asserting `NotConverged` with non-shrinking violation; a zero-row `A` asserting `InvalidInput`; a trust-skip case and a hot-loop NaN case. Host property test: projection output satisfies all constraints within `projection_tolerance` on random feasible polyhedra.

**S5 — documentation.** Requirements §5.1.3 PF disposition (PF-002 implemented; PF-001 contract-only, not solved; PF-003 unchanged); external design §3.2 (trait satisfies the category; builder deferred); crate READMEs; user guides. State §11.6's limitations in every user-facing surface, verbatim in substance.

## 4. Non-change scope
No IPM, no ADMM, no LP solve, no `DynamicQp` builder, no new dependency (if one seems needed, stop — RFC 026 gate runs regardless), no `sqrt`, no change to existing kernel signatures, no `#[allow(dead_code)]`.

## 5. Required evidence per slice
fmt · clippy `-D warnings` · tests · MSRV · `cargo xtask check` (incl. supply-chain, panic-audit, zero-bleed, no-std) · `mdbook build docs`; S2 additionally the recorded footprint and the `thumbv7em-none-eabihf` build; S4 the conformance runner output.

## 6. Prohibited shortcuts
Clamping inside the sweep "because it converges" (it converges to the wrong point). Reporting the projection as exact when the inner cap bound. Verifying PSD `Q` by anything other than documenting it as a caller precondition. Scoping the `m = 0` identity test to one fixture. Submitting slices S2+S3 together.

## 7. Review request
Standard eleven items; **record your tier**; one request per slice, to the architect.
