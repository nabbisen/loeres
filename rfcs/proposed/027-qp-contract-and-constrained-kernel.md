# RFC 027 - QP Contract and Linearly Constrained Projected Kernel

**Status.** Proposed
**Tracks.** R4 first capability, approved by the project owner 2026-09-12 ("QP contract + constrained kernel"); requirements PF-001/PF-002; external design §2.7, §3.2; roadmap §3.5's deferred general linear-inequality projection.
**Touches.** `loeres::problem` (activates the reserved namespace), `loeres-device::{problem,solve}`, `loeres-cluster::{model,solve}`, `conformance/`, apex trio §PF rows.

---

### Extended Metadata

* **Rust Edition Compliance:** Rust 2024; MSRV 1.85.
* **Target Environment:** Core contracts; device and cluster kernels.
* **Proposed release:** first minor after `0.21.0`.
* **Depends on:** RFC 026 (supply-chain gate) landed first; `0.21.0` cut.

## 1. Summary

Loeres has one solver family: box-constrained projected first-order. `loeres::problem`
has been a reserved, empty module since v0.6; PF-001/002 are unimplemented; the
external design names `DynamicQp` as a required category that does not exist.
This RFC makes the word "optimization library" true beyond one family with the
smallest step that reuses every existing contract:

1. **A QP problem contract in core** — `f(x) = ½xᵀQx + cᵀx` over `lo ≤ x ≤ hi`
   and `Ax ≤ b`, expressed through the RFC 002 access traits, storage-agnostic.
2. **A constrained projected kernel** on device and cluster — the existing outer
   projected-gradient loop, with the closed-form box `clamp` replaced by a
   **bounded polyhedral projection** onto `{x : lo ≤ x ≤ hi, Ax ≤ b}`.

LP (PF-001) is expressed by the contract (`Q = 0`) but **not solved** by this
kernel — projected gradient on a linear objective has no curvature to converge
against. That limitation is stated, not hidden.

## 2. Affected crates

`loeres` (new public items in `problem`), `loeres-device`, `loeres-cluster`.
Backends unchanged.

## 3. Public API boundary impact

Additive. New traits in `loeres::problem`; new kernel entrypoints beside the
existing ones. No existing signature changes. `#[non_exhaustive]` where enums
appear (ED-010).

## 4. Dependency impact

**None intended.** If implementation finds a dependency necessary it stops and
reports; RFC 026's gate runs regardless.

## 5. `std` / `alloc` impact

None for `loeres`, `loeres-backend-static`, `loeres-device`. Device workspace
grows by the projection's dual vector (`m` scalars) and one scratch vector (`n`
scalars), fixed at compile time — see §11.4.

## 6. Device determinism impact

Bounded: outer cap × inner projection cap, both runtime config. No division in
the hot loop except `checked_div` by precomputed, boundary-validated `‖aᵢ‖²`.
No `sqrt`, no `AdvancedNumericalScalar`. Target-scoped as before (RFC 011).

## 7. Cluster scalability impact

Dynamic `m`, `n`; allocation at construction only; hot loop allocation-free as
RFC 016. Dense `A` first; CSR `A` via `MatrixAccess` is in scope if the
contiguous fast path is not required by the inner loop (it is not).

## 8. Error and diagnostic impact

New `SolverError` variants are **not** needed: infeasible or degenerate
constraints surface as `InvalidInput` (structural: `lo > hi`, zero row `aᵢ`),
`NumericalDomain` (in-loop non-finite), or `SolveStatus::NotConverged` with
`TerminationReason::IterationCap` when the projection cap binds. An
**infeasible polyhedron is not detectable in general by this method** and is
reported as non-convergence; §11.6 says so plainly.

## 9. Feature flag impact

Device kernel behind `owned-arrays` like RFC 006. No new features.

## 10. Semver impact

Minor. Additive public surface.

## 11. Design

### 11.1 The contract (`loeres::problem`)

```text
trait QuadraticObjective<S>   // Q via MatrixAccess (symmetric PSD asserted by caller,
                              // validated cheaply: square, finite), c via VectorAccess
trait BoxBounds<S>            // lo, hi via VectorAccess  (already the RFC 006/016 shape)
trait LinearInequalities<S>   // A via MatrixAccess (m×n), b via VectorAccess (m)
trait QuadraticProgram<S>: QuadraticObjective<S> + BoxBounds<S> + LinearInequalities<S>
```

Storage-agnostic, monomorphized, no `dyn`, no allocation, no layout commitment —
exactly the constraints ED-003/ED-009/§2.5 impose on core. The PFO oracle traits
in device/cluster remain; `QuadraticProgram` gains a blanket first-order oracle
(`∇f = Qx + c`) so the existing kernels can consume it unchanged.

### 11.2 Projection method — the design decision

Projection onto `{lo ≤ x ≤ hi} ∩ {Ax ≤ b}` has no closed form. Options weighed
against the device constraints (no alloc, bounded, no `sqrt`, checked division
only):

| Method | Fit | Verdict |
|---|---|---|
| **Dykstra / Hildreth** alternating projections onto halfspaces and the box | Each halfspace projection is `x − aᵢ·max(0,(aᵢᵀx − bᵢ)/‖aᵢ‖²)`: one checked division by a **precomputed, validated** constant; Hildreth's dual form needs only `m` multipliers; converges to the exact projection for polyhedra; trivially bounded | **Adopted** |
| ADMM | Needs a linear solve of `(Q + ρAᵀA)` — factorization, `sqrt` or heavy division, `n²` workspace | Rejected for device; possible later cluster path |
| Active-set | Combinatorial; iteration count not bounded by a simple cap | Rejected |
| Interior point | Out of scope by owner decision; barrier needs `log` | **Explicitly excluded** |

Hildreth's method is Dykstra specialised to halfspaces: one multiplier per
constraint, cyclic sweeps, monotone convergence to the projection.

**The box is its own Dykstra set, not a bare clamp inside the sweep** (architect
review 043, R1). Cyclic projections that simply clamp between halfspace steps
converge to *a* feasible point, not to the Euclidean projection, and the
projected-gradient convergence argument needs the projection. Dykstra's scheme
therefore runs over two sets — the polyhedron `{Ax ≤ b}` via Hildreth multipliers
(`m` scalars) and the box via exact `clamp` with its own increment vector (`n`
scalars). With `m = 0` there is one set, one exact projection, and the increment
is identically zero, so the kernel performs precisely RFC 006/016's `clamp` —
**RFC 006/016 behaviour is preserved bit-for-bit for `m = 0`**, and the
conformance corpus asserts it. Treating the box as `2n` extra halfspaces was
rejected: `x − (x − hi)` is not bit-identical to `hi` in floating point, which
would break that guarantee.

### 11.3 The kernel

```text
outer:  x ← Π_C( x − α ∇f(x) )        bounded by max_iterations (existing)
Π_C:    Hildreth sweeps, bounded by projection_max_sweeps,
        stopping when max_i |Δx_i| ≤ projection_tolerance
```

Convergence of the outer loop is the existing step-norm criterion. Two caps,
two tolerances, all runtime config (ED-012). An inner cap hit is **not** an
error: the iterate is feasible-approximate and the outer loop continues.

**Inexact projection changes what convergence means** (review 043, R2). Exact-
projection theory does not apply once the inner cap can bind; the iterate
converges to a neighbourhood whose size is governed by `projection_tolerance`
and the cap. The solve record therefore carries two honest fields:
`projection_cap_hits` and the **terminal constraint violation**
`max(0, maxᵢ(aᵢᵀx − bᵢ))` (box violation is zero by construction). A caller
can see how feasible the answer is; the kernel never claims exact feasibility it
did not verify. Infeasible polyhedra show up here as violation that does not
shrink.

### 11.4 Workspace (device)

`ConstrainedProjectedWorkspace<S, N, M>`: gradient scratch `N`, projection
scratch `N`, box Dykstra increment `N`, multipliers `M`, precomputed `‖aᵢ‖²` `M`
— footprint `(3N + 2M)·size_of::<S>() + header`, reported via `WorkspaceFootprint`,
`reset_for_entry` overwrite-on-use (RFC 005 always-reusable).

### 11.5 Validation (RFC 012 discipline)

Structural, always: shapes; `lo ≤ hi` finite; every `‖aᵢ‖² > 0` and finite, computed with overflow mapped to
`Overflow` (a zero row is `InvalidInput`); `step_scale` finite `> 0`; caps `≥ 1`. Policy-governed
finite scans of `Q, c, A, b, x₀` skippable under `TrustedByCaller`; hot-loop
finiteness never skippable. `Q` symmetry/PSD is **not** verified — it is a
caller precondition, documented, because verifying it needs a factorization.

### 11.6 What this does not claim

- LP: expressible, not solvable by this kernel.
- Infeasibility: not detected; reported as non-convergence.
- Projection rate: Hildreth converges linearly at a rate set by the angles
  between constraint normals; nearly parallel constraints can make the inner cap
  bind routinely. Stated, not solved.
- Convergence rate: none claimed; `α ∈ (0, 2/λ_max(Q))` is the caller's
  responsibility as `step_scale` is today.
- Bitwise device/cluster identity: no — tolerance parity per RFC 013.

## 12. Rejected alternatives

Table in §11.2, plus: **generic `dyn` problem object in core** (ED §2.5 forbids);
**modeling DSL / `DynamicQp` builder in this RFC** (deferred — builder ergonomics
are a cluster-only follow-on once the contract exists; ED §3.2 category is
partially satisfied by the trait, fully by the follow-on).

## 13. Verification gates

- Conformance: `m = 0` cases reproduce RFC 013 fixtures exactly; new dimension-2/3
  fixtures with 1–3 halfspaces against closed-form optima; an infeasible case
  asserting `NotConverged`; a zero-row case asserting `InvalidInput`.
- Device: no `unwrap`/`expect`/indexing (panic-audit); footprint recorded;
  `thumbv7em-none-eabihf` build.
- Property test (host): projection output satisfies all constraints within
  `projection_tolerance` on random feasible polyhedra.
- Full suite incl. RFC 026 gate; `release-gate` at closeout.
- **Independent numerical review of §11.2–§11.3 before design freeze.** This is
  the first numerical design decision since RFC 006; the architect requests a
  reviewer with optimization background, tier recorded at request time.

## 14. Implementation sprint plan

S0 design freeze (after numerical review) → S1 core `problem` traits + tests →
S2 device projection + kernel → S3 cluster kernel + `ClusterJob` adapter →
S4 conformance fixtures → S5 docs/apex PF rows → S6 closeout.

## 15. Dependencies

RFC 026 landed; `0.21.0` cut. Nothing in RFCs 022–025 blocks design work.

## 16. Exit criteria

1. `loeres::problem` exports the four traits; PF-002 marked implemented, PF-001
   "contract only", PF-003 unchanged, in the requirements §5.1.3 disposition;
2. device and cluster kernels solve the conformance fixtures within tolerance;
3. `m = 0` reproduces RFC 006/016 results exactly;
4. all §11.5 validation rules tested, including trust-skip and never-skip;
5. no new dependency, or RFC 026 gate green on the one that was justified;
6. panic-audit, zero-bleed, no-std, MSRV, size-budget report green;
7. numerical review recorded with tier;
8. §11.6 limitations stated in every user-facing surface.
