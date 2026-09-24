# RFC 032 - Step-Size Guidance and a Stated Convergence Rate

**Status.** Implemented (v0.21.3). Amendment 1 was made while Accepted, under RFC 000's in-place-amendment rule.
**Design approval.** Amendment 1 (§0, 2026-09-24) by architect review 068. Architect-authored and scheduled in architect review 066 (Cycle 1); project owner authorized the cycle on 2026-09-24.
**Tracks.** Closes RFC 027 §11.6's "no convergence rate is claimed; `step_scale` is the caller's responsibility". Paired with RFC 031 in the same release.
**Touches.** `crates/loeres/src/problem.rs` (additive), `crates/loeres-device/src/solve/constrained.rs` and `crates/loeres-cluster/src/solve/constrained.rs` (validation only), user-facing docs.

---

## 0. Amendment 1 — 2026-09-24 (architect review 068)

**The slack figures in §3 were ensemble-specific and are not general.**

§3 stated `U <= 1.50 x λ_max` and `λ_max <= 3.04 x L` from the architect's
600-instance random PSD set, phrased as though they were properties of the
bounds. The implementer's own 600-instance re-verification gave **1.62** and
**4.21**: their generator draws `A` with as few as one row, so `Q` is often
rank-deficient and the diagonal bound is looser.

**Both bounds held everywhere in both sets.** What differs is the *slack*, which
is a property of the ensemble sampled and not of the bounds. A slack figure is
quotable only with the ensemble that produced it, and no general slack is claimed
by this RFC or by any surface it governs.

§3's sentence is corrected in place accordingly.

## 1. Summary

Give the caller a way to compute a **provably safe** `step_scale`, and let the
kernels reject a **provably divergent** one, using two cheap eigenvalue bounds
that need no new scalar tier.

## 2. The problem

Projected gradient on `½xᵀQx + cᵀx` converges only for
`α ∈ (0, 2/λ_max(Q))`. Today the library states that interval and computes
nothing: `step_scale` is supplied by the caller with no help and no check. It is
the most likely way these kernels are misused. RFC 029 made the resulting
divergence *report* correctly as `NotConverged`; it did not make it avoidable or
detectable up front.

## 3. Design — two bounds, and what each one licenses

For symmetric positive semidefinite `Q`:

```text
U = maxᵢ Σⱼ |Qᵢⱼ|      (Gershgorin)      U >= λ_max
L = maxᵢ Qᵢᵢ           (diagonal)         L <= λ_max
```

`U` holds because every eigenvalue lies in a Gershgorin disc and `Qᵢᵢ >= 0` for
PSD `Q`. `L` holds because `Qᵢᵢ = eᵢᵀQeᵢ <= λ_max`. Both were checked over 600
random PSD matrices by the architect and over a second, independently generated
600 by the implementer, **with no violation in either set**. Observed slack
differed between them — `1.50×`/`3.04×` against `1.62×`/`4.21×` — because slack
is a property of the ensemble sampled, not of the bounds (Amendment 1). **No
general slack figure is claimed.**

**What each licenses, and nothing more:**

| Condition | Conclusion | Why |
|---|---|---|
| `α < 2/U` | **provably convergent** | `U >= λ_max` so `2/U <= 2/λ_max` |
| `α >= 2/L` | **provably divergent** | `L <= λ_max` so `2/L >= 2/λ_max` |
| `2/U <= α < 2/L` | **indeterminate** — accepted, no claim | neither bound decides it |

The third row is the honest one and the reason this design is two-sided. An
upper bound alone can prove safety but can never reject, so a rule built on
Gershgorin only would either be silent about bad input or would reject good
input. Both are available cheaply, so both are used.

### 3.1 Public surface (additive)

On the `QuadraticProgram` contract, alongside `shape()`:

- `curvature_bounds() -> Result<CurvatureBounds<S>, SolverError>` returning `U`
  and `L`;
- `suggested_step_scale() -> Result<S, SolverError>` returning `1/U`, which is
  provably inside the safe interval — never optimal, always safe.

`CurvatureBounds<S>` is `#[non_exhaustive]`, output-only, matching
`ProgramShape`'s precedent.

### 3.2 Scalar tiers — no new tier

`U` and `L` need `abs` (`MetricScalar`), `add`/`mul` (`BaseScalar`) and `max`
(`OrderedScalar`). `suggested_step_scale` additionally needs one checked
division (`DivisibleScalar`). **No `AdvancedNumericalScalar`, no `sqrt`** — the
device path is unaffected.

### 3.3 Kernel validation

Boundary validation in both constrained kernels rejects `α >= 2/L` with
`SolverError::InvalidInput`, under the RFC 012 policy that governs structural
checks. It **never** rejects the indeterminate band.

`L` is an `O(n)` scan of the diagonal, so this is cheaper than the `O(n²)` shape
work already done at the boundary.

### 3.4 The rate that may be stated

Under `α ∈ (0, 2/U)` and `Q` positive **definite** with smallest eigenvalue
`λ_min > 0`, projected gradient contracts linearly with factor
`max(|1 − αλ_min|, |1 − αλ_max|)`. The library does not compute `λ_min` and
therefore states the rate's *form* and its dependence, not a number. For `Q`
merely semidefinite, no rate is claimed — the same position as today.

## 4. Explicit non-scope

- **No line search, no adaptive or per-iteration step.** `step_scale` stays a
  caller-supplied constant.
- **No exact `λ_max`** by power iteration: unbounded iteration counts belong to
  neither device contract.
- **No `λ_min`, no strong-convexity estimate**, so no numeric rate.
- **No automatic step selection.** `suggested_step_scale` is offered; nothing
  calls it on the caller's behalf. Silently substituting a step would change
  results for existing callers.
- **No change to the box-only RFC 006/016 kernels' problem traits**, which carry
  their own `step_scale` and no `Q`.
- No new dependency; no existing signature changed.
- **This RFC does not address RFC 031 S2's nearly-parallel-normals finding**
  (architect review 067), and
  must not be described as if it did. Those fixtures fail *at* `step_scale = 1.0`,
  which is exactly this RFC's suggested step for `Q = I` (`U = 1`). The defect
  there is the inner projection's accuracy, not the outer step size; RFC 033
  covers it.

## 5. Risks

| Risk | Mitigation |
|---|---|
| `U` is conservative, so the suggested step is slower than optimal | Stated plainly: safe, not optimal. Measured slack was `<= 1.50×` on random PSD `Q`; a caller who knows `λ_max` may still pass their own |
| The rejection rule fires on a usable step | It cannot: `α >= 2/L` implies `α >= 2/λ_max`, which does not converge for any `Q` |
| `Q` is not actually PSD — an unverified caller precondition | Both bounds assume PSD, which RFC 027 §11.5 already makes the caller's responsibility. The RFC states that the bounds are meaningless for a non-PSD `Q`, exactly as the kernel's convergence already is |
| Overflow in the row sums | Checked accumulation, `SolverError::Overflow`, as the existing row-norm scan does |

## 6. Exit criteria

1. `curvature_bounds` and `suggested_step_scale` ship on `QuadraticProgram`,
   `#[non_exhaustive]` output type, no new scalar tier.
2. Both constrained kernels reject `α >= 2/L` as `InvalidInput` and accept the
   indeterminate band unchanged.
3. A randomized differential test (RFC 030 registry row) over random PSD `Q`
   asserts: `L <= λ_max <= U` against an independent reference; that
   `suggested_step_scale` converges; and that a step at `2/L` is rejected.
4. `TERMS_OF_USE.md` and both user guides state what the bounds license and the
   indeterminate band.
5. `cargo xtask check` 17 gates, `differential` PASS, smoke conformance 24/24.
6. RFC 031's adversarial suite reports, before and after, both its cap-hit rates
   **and the deviation from the exact optimum**, so the rule's effect is measured
   rather than assumed. Deviation is required because RFC 031 S2 showed it is
   **not monotone** in the angle between constraint normals (architect review 067
   measured `1.34e-3` at
   `ε = 0.002` against `4.10e-4` at `ε = 0.005`), so cap hits alone cannot judge
   a step rule.
