# Developer Handoff — RFC 027 QP contract and constrained projected kernel

**Governing RFC.** `rfcs/accepted/027-qp-contract-and-constrained-kernel.md` (design frozen 2026-09-12, review 043 applied). **Assigned to.** Implementer tier.
**Ordering.** Do **not** begin until RFC 026's gate is enforced on `main` and `0.21.0` is cut. This RFC's release is the first minor after `0.21.0`.

## 1. Purpose
Make the library true beyond one solver family: a storage-agnostic QP contract in `loeres::problem`, and both projected-first-order kernels extended to `Ax ≤ b` via a bounded Dykstra projection. Everything else in the project stays as it is.

## 2. Read first
RFC 027 §11 in full, especially §11.2 (**the box is its own Dykstra set with an increment vector — not a clamp inside the sweep**; review 043 R1) and §11.3 (**terminal constraint violation is recorded**; R2). RFC 006 §7 and RFC 016 §7 for the kernels you are extending. RFC 012 for trust semantics.

## 3. Slices (each independently reviewable — submit one review request per slice)
**S1 — core contract.** `loeres::problem`: `QuadraticObjective<S>`, `BoxBounds<S>`, `LinearInequalities<S>`, `QuadraticProgram<S>` over RFC 002 access traits; first-order oracle `∇f = Qx + c` as the provided `QuadraticObjective::gradient_into` (RFC 027 §0.2.1 — core cannot see kernel traits; the kernel adapter is S2/S3's). *Landed and accepted, review 052.* `#![no_std]`, no alloc, no `dyn`, `#[non_exhaustive]` on any enum. Tests: trait bounds compile for a static and a dynamic backend without importing each other. **Stop and report** if a required method would force a layout or an allocation.

**S2 — device projection + kernel.** *Landed and accepted at `06adfd3`, architect review 055. The first submission `e271241` was rejected by review 054; corrections C1-C7 (§3.1) are complete and verified.* `ConstrainedProjectedWorkspace<S, N, M>` with **`M ≥ 1`** enforced by RFC 004's const-assertion pattern (`3N + 2M` scalars, `WorkspaceFootprint`, `reset_for_entry`). Dykstra over **`m + 1` sets** (RFC 027 §0.3.1 — **not** two sets; see §3.1): Hildreth multipliers for `m` halfspaces (checked division by precomputed `‖aᵢ‖²`, validated `> 0` and finite at the boundary, overflow → `Overflow`), exact `clamp` for the box with its increment vector. Outer loop unchanged from RFC 006. Record gains `projection_cap_hits` and `max_constraint_violation`. **Device `m = 0` is the RFC 006 entrypoint, not an `M = 0` instantiation** (RFC 027 §0.2.4) — do not add a second device `m = 0` path. The adapter from a `QuadraticProgram` to the kernel takes `step_scale` separately. Write first: a test that the constrained kernel with `m ≥ 1` constraints all inactive at the optimum matches RFC 006's result within tolerance (report if you observe bitwise equality; do not assert it). Bounds `S: FiniteScalar + MetricScalar + DivisibleScalar`; no `AdvancedNumericalScalar`. Behind `owned-arrays`. panic-audit clean.

**S3 — cluster kernel.** *Landed and accepted at `2972819`, architect review 056.* Same algorithm as the **corrected** S2 — Dykstra over `m + 1` sets, multipliers persisting for the whole projection call, no polyhedron-level increment vector, and the three-part stopping rule (RFC 027 §0.3.1-§0.3.3). Read `crates/loeres-device/src/solve/constrained.rs` at `06adfd3` before starting; do **not** read `e271241`'s version, which is the retired design. **Carry an equivalent randomized differential test**: random feasible polytopes compared against an exact active-set reference computed in-test over the constraint rows *and* the box faces, asserting agreement, zero cap hits, and `max_constraint_violation` within tolerance. Review 055 established that the deterministic `m >= 2` cases do not detect a per-sweep multiplier reset and only the randomized test does; a cluster kernel without one is not reviewable. Every feasible-problem test asserts the returned point satisfies its own constraints. Over `DenseVector`/`DenseMatrix`, allocation at construction only, plugged into the `ClusterJob` seam beside RFC 016's adapter. `m = 0` is accepted at runtime (zero-row `MatrixAccess`, canonically core's `MatrixView` over an empty slice) and **short-circuits to the single exact box projection with no Dykstra sweep** (RFC 027 §0.2.3). CSR `A` via `MatrixAccess` if it needs no contiguous fast path in the inner loop; report if it does. Validation per RFC 012/016 (structural always; finite scans skippable under `TrustedByCaller`; hot loop never).

**S4 — conformance.** *Landed and accepted at `f47d232`, architect review 057; corrective items C1-C3 in §3.2 come next.* **Assert bit-identity as numeric equality plus a NaN check, never raw `to_bits()`** (RFC 027 Amendment 4, §0.4.1): identity holds *up to the sign of zero*, and a fixture with a zero coordinate otherwise fails for a reason that is not a defect. Architect review 056 reproduced this — `target [0.0, 3.0]`, `start [-0.0, 0.5]` gives `-0.0` against RFC 016's `+0.0`, equal in value, different in bits. Do **not** respond to such a failure by loosening the assertion. **Also exercise a real `loeres-backend-std::SparseMatrix` `A`** here, which review 056 deferred from S3: it needs `loeres-backend-std/sparse` enabled in `crates/loeres-cluster/Cargo.toml` `[dev-dependencies]`, so run `cargo xtask check` with attention to `feature-matrix` and `zero-bleed` and **report the feature-graph effect**; S3's in-test `Triplets` type already proved the kernel needs no contiguous fast path, so this is integration evidence, not a correctness question. Fixtures: on cluster, every existing fixture run through the constrained kernel with `m = 0` and the **same problem oracle** is identical to RFC 016 under §0.4.1 — do **not** re-express fixtures as a `QuadraticProgram` for this assertion: their `q·(x − t)` oracle and `Qx + c` differ in floating point, so that comparison is tolerance-only (RFC 027 §0.2.5); dimension-2/3 with 1–3 halfspaces against closed-form optima; an infeasible polyhedron asserting `NotConverged` with non-shrinking violation; an **all-zero constraint row** `aᵢ = 0` asserting `InvalidInput` (a zero-row matrix, `m = 0`, is valid — RFC 027 §0.2.6); a trust-skip case and a hot-loop NaN case. Host property test: projection output satisfies all constraints within `projection_tolerance` on random feasible polyhedra.

**S5 — documentation.** Runs after §3.2's C1-C3. **Review 056's L1 is settled as option (a) by Amendment 5 §0.5.4** - once the status is truthful a batch caller reading `NotConverged` is correctly warned, so `BatchItemOutcome` does **not** need widening and no new RFC is required: document the batch path as carrying status only, and point callers needing the magnitudes at the typed entrypoint `solve_constrained_projected_first_order_dyn`. Also state that `Converged` means feasible within `projection_tolerance`, and that `NotConverged` with `NoProgress` on a constrained solve indicates an infeasible or too-tightly-capped polyhedron (C4). *Superseded context, retained:* the original L1 framing was: `ClusterConstrainedJob` erases to `BatchItemOutcome`, which carries only the core `SolveReport`, so `projection_cap_hits` and `max_constraint_violation` — the two fields RFC 027 §11.3 exists to provide — are invisible through the batch seam. A batch caller sees `Converged` with no way to tell an exact answer from a feasible-approximate one. Two options: **(a)** document the batch path as not carrying a feasibility signal and direct such callers to the typed entrypoint `solve_constrained_projected_first_order_dyn`; or **(b)** widen `BatchItemOutcome`, which is shipped public API, so it needs its own RFC and a version decision. Do not write either into the docs until the owner has chosen. Then: requirements §5.1.3 PF disposition (PF-002 implemented; PF-001 contract-only, not solved; PF-003 unchanged); external design §3.2 (trait satisfies the category; builder deferred); crate READMEs; user guides. State §11.6's limitations in every user-facing surface, verbatim in substance.


### 3.1 S2 corrections (architect review 054) — CLOSED by `06adfd3`, accepted in review 055

*Retained as the record of what was required and why. C1-C7 are all complete; the
randomized differential test stays in S2 and is not moved to S4 (review 055).*

Governing text: **RFC 027 Amendment 3, §0.3** (`rfcs/accepted/027-qp-contract-and-constrained-kernel.md`).
Full analysis and reproduction: `.git-exclude/reviewed/054-rfc027-s2-constrained-device-kernel-architect-review-2026-09-23.md`.

All work is in `crates/loeres-device/src/solve/constrained.rs` and its `tests.rs`.

**C1 (blocking) — delete `polytope_increment`.** Remove the field from
`ConstrainedProjectedWorkspace`, its parameter from `new()` (six buffers → five),
its `zero_in_place` call, the `target = x + polytope_increment` loop at the top of
each sweep, and the `new_polytope_increment = target − x` loop after the Hildreth
pass. Each halfspace is its own Dykstra set; its increment *is* the scalar `λᵢ`.
The box keeps `box_increment` exactly as it is. Keep the multipliers persistent
for the whole `dykstra_project` call — that part of the current code is correct
and is what makes the scheme Dykstra (RFC 027 §0.3.2).

**C2 (blocking) — replace the inner stopping rule.** Converge only when all three
hold in the same sweep (RFC 027 §0.3.3):
1. `maxⱼ|Δxⱼ| ≤ projection_tolerance` — as now;
2. `maxᵢ|Δλᵢ| ≤ projection_tolerance` — `diff` is already computed in the row
   loop; accumulate its `abs().max(...)`, no new storage;
3. `max(0, maxᵢ(aᵢᵀx − bᵢ)) ≤ projection_tolerance` — evaluated only in a sweep
   where (1) and (2) already hold, so it costs one `O(M·N)` pass per *converged*
   projection, not per sweep.

**C3 (blocking) — footprint returns to `3N + 2M`.** C1 removes exactly the one
extra buffer. `size_of::<ConstrainedProjectedWorkspace<f64,2,1>>()` must become
`(3·2+2·1)·8 + 16 = 80` bytes and `<f64,3,3>()` `(3·3+2·3)·8 + 16 = 136`. Update
the footprint tests to these values. **U1 is resolved: RFC 027 §11.4 stands, no
amendment to it.** Do **not** weaken the outer convergence criterion — review
054 upholds the submission's own §4 reasoning that RFC 006's criterion must stay
unchanged and that `outer_previous` cannot be reused.

**C4 (blocking) — tests with `m ≥ 2`.** No existing functional test uses more
than one linear constraint, which is why this defect passed 18 tests: with
`M = 1` a single Hildreth pass is exact and the defect cannot appear. Add:

- **The review-054 regression case**, which fails loudly on the current kernel.
  `N = 2`, `M = 2`, `Q = I`, `c = −t`, `step_scale = 1.0`, box `[−10, 10]²`,
  `x₀ = (0, 0)`, `max_iterations = 5000`, `tolerance = 1e-12`,
  `projection_max_sweeps = 5000`, `projection_tolerance = 1e-14`:

  ```text
  A = [[ 1.6   , -1.3082],     b = [ 1.6013, -0.3988]
       [-0.0457,  0.9197]]     t = ( 5.3481,  4.7872)

  expected x ≈ ( 0.673643, -0.400146)   within 1e-6
  current kernel returns ( 5.345268, -0.168013), status Converged,
  projection_cap_hits 0, max_constraint_violation 7.170922
  ```

- **A vertex solution**: a feasible `m ≥ 2` program with **two constraints active
  at the optimum**, expected value derived by hand and KKT-checked in the doc
  comment.
- **An activity flip**: a feasible `m ≥ 2` program where a constraint is active
  early and inactive at the optimum.
- **A feasibility assertion in every feasible-problem test** —
  `max_constraint_violation() ≤ projection_tolerance`. Not one current test
  asserts the returned point satisfies its own constraints; this assertion alone
  would have caught the defect.

**C5 (blocking) — `projection_max_sweeps`.** Tight tolerances need far more than
the `200` used throughout the tests (~2700 sweeps at `1e-12` in the architect's
measurements). Do not raise the *default* silently; instead document the
relationship on `ConstrainedSolveConfig::projection_max_sweeps` and use a cap in
tests that is adequate for the tolerance each test sets.

**C6 (non-blocking) — the `multipliers` field doc contradicts the code.** It says
"reset to zero at the start of each sweep's polytope correction"; the code resets
once per call, which is correct. Fix the doc and state why persistence is
required (RFC 027 §0.3.2).

**C7 (non-blocking) — rewrite the `ConstrainedProjectedWorkspace` paragraph** on
why `gradient` cannot hold `outer_previous`, in two sentences, against the
corrected five-buffer set.

**Infeasible polyhedra:** do not special-case them, and do not reshape S4's
fixtures to avoid exactly-cancelling geometries (RFC 027 §0.3.4). Under C1+C2 an
infeasible polyhedron correctly runs to the cap and reports its true violation.
The existing `an_infeasible_polyhedron_reports_a_positive_constraint_violation`
test should additionally assert `projection_cap_hits() > 0`.

**Evidence required:** the standard per-slice set (§5), plus the recorded
footprint at the C3 values, the `thumbv7em-none-eabihf` build, and the
review-054 regression case passing.


### 3.2 Post-S4 corrective items (architect review 057) — before S5

Governing text: **RFC 027 Amendment 5, §0.5**. Analysis and measurements:
`.git-exclude/reviewed/057-rfc027-s4-conformance-architect-review-2026-09-24.md`.

S4 correctly refused to assert a status for an infeasible polyhedron and
escalated it. The ruling: `Converged` must mean feasible. The architect
implemented and measured this before prescribing it - zero regressions across
all 24 fixtures and the full workspace suite - so these are mechanical.

**C1 (blocking) - gate `Converged` on feasibility, both kernels.** Report
`Converged` only when the outer step is stationary **and**
`max_constraint_violation <= projection_tolerance`; otherwise
`SolveReport::not_converged_stalled(executed)`.

- `crates/loeres-device/src/solve/constrained.rs`: the early-exit return in
  `solve_constrained_projected_first_order`, **and** the `constant_iteration`
  path that currently returns `converged_at_cap` on the `converged` flag.
- `crates/loeres-cluster/src/solve/constrained.rs`: the early-exit return in
  `solve_constrained_projected_first_order_dyn`.

Measured effect on the section-2 configuration (`x0 <= -1`, `x0 >= 1`, one
loose row, `step_scale 0.5`, `t = (3,3)`):

```text
before : Converged    / ConvergenceCriterion / iters 42 / cap_hits 42 / violation 2
after  : NotConverged / NoProgress           / iters 42 / cap_hits 42 / violation 2
feasible control: Converged, cap_hits 0, violation 0 - unchanged
```

Add a unit test in **both** crates pinning exactly this: an infeasible
polyhedron reports `NotConverged` with `NoProgress` and a positive violation,
and a feasible control still reports `Converged`.

**C2 (blocking) - `conformance/smoke/qp-linear-2d-infeasible-001.toml`** asserts
`status = "not-converged"` and `termination = "no-progress"`, keeping its
existing `projection_cap_hits > 0`, violation `>= 0.5`, and
non-shrinking-at-tenfold-cap assertions.

**C3 (blocking) - remove the `not-asserted` status escape hatch.** With C2 it has
no remaining user: delete the schema value, its parsing in
`xtask/src/checks/conformance/`, and the test asserting it is rejected for a
feasible fixture. Per the project's standing rule, remove what is unused rather
than keep it - and a fixture schema permitting "assert nothing about the status"
is worse than ordinary dead code, because it is an escape hatch a future author
can reach for instead of diagnosing a failure. **Do not** add
`#[allow(dead_code)]` anywhere in its place.

**Evidence:** fmt, clippy `-D warnings`, `cargo test --workspace --all-features`,
MSRV 1.85, `cargo xtask check` (16 gates), `cargo xtask conformance` showing
`24 total / 24 passed`, and the before/after status on the infeasible case.

## 4. Non-change scope
No IPM, no ADMM, no LP solve, no `DynamicQp` builder, no new dependency (if one seems needed, stop — RFC 026 gate runs regardless), no `sqrt`, no change to existing kernel signatures, no `#[allow(dead_code)]`.

## 5. Required evidence per slice
fmt · clippy `-D warnings` · tests · MSRV · `cargo xtask check` (incl. supply-chain, panic-audit, zero-bleed, no-std) · `mdbook build docs`; S2 additionally the recorded footprint and the `thumbv7em-none-eabihf` build; S4 the conformance runner output.

## 6. Prohibited shortcuts
Clamping inside the sweep "because it converges" (it converges to the wrong point). Reporting the projection as exact when the inner cap bound. Verifying PSD `Q` by anything other than documenting it as a caller precondition. Scoping the `m = 0` identity test to one fixture. Submitting slices S2+S3 together.

## 7. Review request
Standard eleven items; **record your tier**; one request per slice, to the architect.
