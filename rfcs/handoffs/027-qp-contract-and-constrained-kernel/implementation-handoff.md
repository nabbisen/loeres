# Developer Handoff — RFC 027 QP contract and constrained projected kernel

**Governing RFC.** `rfcs/accepted/027-qp-contract-and-constrained-kernel.md` (design frozen 2026-09-12, review 043 applied). **Assigned to.** Implementer tier.
**Ordering.** Do **not** begin until RFC 026's gate is enforced on `main` and `0.21.0` is cut. This RFC's release is the first minor after `0.21.0`.

## 1. Purpose
Make the library true beyond one solver family: a storage-agnostic QP contract in `loeres::problem`, and both projected-first-order kernels extended to `Ax ≤ b` via a bounded Dykstra projection. Everything else in the project stays as it is.

## 2. Read first
RFC 027 §11 in full, especially §11.2 (**the box is its own Dykstra set with an increment vector — not a clamp inside the sweep**; review 043 R1) and §11.3 (**terminal constraint violation is recorded**; R2). RFC 006 §7 and RFC 016 §7 for the kernels you are extending. RFC 012 for trust semantics.

## 3. Slices (each independently reviewable — submit one review request per slice)
**S1 — core contract.** `loeres::problem`: `QuadraticObjective<S>`, `BoxBounds<S>`, `LinearInequalities<S>`, `QuadraticProgram<S>` over RFC 002 access traits; first-order oracle `∇f = Qx + c` as the provided `QuadraticObjective::gradient_into` (RFC 027 §0.2.1 — core cannot see kernel traits; the kernel adapter is S2/S3's). *Landed and accepted, review 052.* `#![no_std]`, no alloc, no `dyn`, `#[non_exhaustive]` on any enum. Tests: trait bounds compile for a static and a dynamic backend without importing each other. **Stop and report** if a required method would force a layout or an allocation.

**S2 — device projection + kernel.** `ConstrainedProjectedWorkspace<S, N, M>` with **`M ≥ 1`** enforced by RFC 004's const-assertion pattern (`3N + 2M` scalars, `WorkspaceFootprint`, `reset_for_entry`). Dykstra over two sets: Hildreth multipliers for `m` halfspaces (checked division by precomputed `‖aᵢ‖²`, validated `> 0` and finite at the boundary, overflow → `Overflow`), exact `clamp` for the box with its increment vector. Outer loop unchanged from RFC 006. Record gains `projection_cap_hits` and `max_constraint_violation`. **Device `m = 0` is the RFC 006 entrypoint, not an `M = 0` instantiation** (RFC 027 §0.2.4) — do not add a second device `m = 0` path. The adapter from a `QuadraticProgram` to the kernel takes `step_scale` separately. Write first: a test that the constrained kernel with `m ≥ 1` constraints all inactive at the optimum matches RFC 006's result within tolerance (report if you observe bitwise equality; do not assert it). Bounds `S: FiniteScalar + MetricScalar + DivisibleScalar`; no `AdvancedNumericalScalar`. Behind `owned-arrays`. panic-audit clean.

**S3 — cluster kernel.** Same algorithm over `DenseVector`/`DenseMatrix`, allocation at construction only, plugged into the `ClusterJob` seam beside RFC 016's adapter. `m = 0` is accepted at runtime (zero-row `MatrixAccess`, canonically core's `MatrixView` over an empty slice) and **short-circuits to the single exact box projection with no Dykstra sweep** (RFC 027 §0.2.3). CSR `A` via `MatrixAccess` if it needs no contiguous fast path in the inner loop; report if it does. Validation per RFC 012/016 (structural always; finite scans skippable under `TrustedByCaller`; hot loop never).

**S4 — conformance.** Fixtures: on cluster, every existing fixture run through the constrained kernel with `m = 0` and the **same problem oracle** is bit-identical to RFC 016 — do **not** re-express fixtures as a `QuadraticProgram` for this assertion: their `q·(x − t)` oracle and `Qx + c` differ in floating point, so that comparison is tolerance-only (RFC 027 §0.2.5); dimension-2/3 with 1–3 halfspaces against closed-form optima; an infeasible polyhedron asserting `NotConverged` with non-shrinking violation; an **all-zero constraint row** `aᵢ = 0` asserting `InvalidInput` (a zero-row matrix, `m = 0`, is valid — RFC 027 §0.2.6); a trust-skip case and a hot-loop NaN case. Host property test: projection output satisfies all constraints within `projection_tolerance` on random feasible polyhedra.

**S5 — documentation.** Requirements §5.1.3 PF disposition (PF-002 implemented; PF-001 contract-only, not solved; PF-003 unchanged); external design §3.2 (trait satisfies the category; builder deferred); crate READMEs; user guides. State §11.6's limitations in every user-facing surface, verbatim in substance.

## 4. Non-change scope
No IPM, no ADMM, no LP solve, no `DynamicQp` builder, no new dependency (if one seems needed, stop — RFC 026 gate runs regardless), no `sqrt`, no change to existing kernel signatures, no `#[allow(dead_code)]`.

## 5. Required evidence per slice
fmt · clippy `-D warnings` · tests · MSRV · `cargo xtask check` (incl. supply-chain, panic-audit, zero-bleed, no-std) · `mdbook build docs`; S2 additionally the recorded footprint and the `thumbv7em-none-eabihf` build; S4 the conformance runner output.

## 6. Prohibited shortcuts
Clamping inside the sweep "because it converges" (it converges to the wrong point). Reporting the projection as exact when the inner cap bound. Verifying PSD `Q` by anything other than documenting it as a caller precondition. Scoping the `m = 0` identity test to one fixture. Submitting slices S2+S3 together.

## 7. Review request
Standard eleven items; **record your tier**; one request per slice, to the architect.
