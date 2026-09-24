# RFC 034 - Conservative Infeasibility Detection

**Status.** Accepted (design frozen 2026-09-24)
**Design approval.** Amendment 1 (§0, 2026-09-24) by architect review 070. Architect-authored and scheduled as Cycle 2 in architect review 066.
**Tracks.** Closes RFC 027 §11.6's "infeasibility is not detected; reported as non-convergence". Depends on RFC 031's corpus and composes with RFC 027 Amendment 5, RFC 029 and RFC 033.
**Touches.** `crates/loeres/src/solver.rs` (one enum variant), both constrained kernels, `conformance/adversarial/`, user-facing docs.

---

## 0. Amendment 1 — 2026-09-24 (architect review 070)

**The §4 rule was unsound at realistic caps. It is replaced.**

§3's feasible ratios of `0.976–1.000` were measured at **4000 sweeps, after the
multipliers had plateaued**. A cap binds precisely *below* the convergence time,
and in that regime a feasible problem's multipliers are still rising roughly
linearly from zero toward their bounded limit. Measured on a thin feasible sliver
(three near-parallel rows plus one near-antiparallel):

```text
cap   ratio   viol_mid   viol_fin   old 3-condition rule
 300  1.890   3.11e-02   2.80e-02   FIRES  -> false `Infeasible`
1000  1.782   2.47e-02   1.77e-02   FIRES  -> false `Infeasible`
```

The original separation was measured in the one regime where the rule is never
invoked. That is the governing risk of §7 realised, and the error is the
architect's.

**0.1.1 The discriminant that does hold.** On an infeasible system the terminal
violation is **constant** between the two snapshots — it converges to the
positive Farkas distance — while on a slow feasible one it **shrinks**. Every
infeasible case measured holds `2.000 → 2.000`; every false positive shrinks.

**0.1.2 The replacement rule.** Report `SolveStatus::Infeasible` only when **all
five** hold:

1. the **final** projection hit `projection_max_sweeps` (RFC 033's tracking);
2. `projection_max_sweeps >= 64` — asymptotic divergence cannot be inferred from
   a handful of sweeps;
3. `max|λ|` at the final sweep `>= 1.9 ×` `max|λ|` at the midpoint sweep;
4. the terminal violation at the final sweep is **not shrinking** relative to the
   midpoint sweep (`viol_final >= 0.99 × viol_mid`);
5. `max_constraint_violation > projection_tolerance`.

**0.1.3 Why 1.9 rather than 1.5.** The measured feasible peak *in the regime that
matters* is `1.89`. `1.5` has no margin there. `1.9` was chosen against that
measurement, not fitted to the fixtures.

**0.1.4 Measured.** Over random feasible polytopes with 40% near-parallel and 10%
near-antiparallel rows, at caps `10 … 10000`: **0 false positives in 20,132
trials**, against 22 for the original rule. Detection is preserved — the three
strongly infeasible families fire at every cap `>= 100`, and a weakly infeasible
system (margin `1e-2`) fires only at caps `>= 3000`, which is one-sidedness
behaving as §6 describes.

**0.1.5 Where the rule applies.** At the **stationary** returns only — the early
exit and the device's `ConstantIteration` post-loop, where Amendment 5 and
RFC 033 already decide a status. **Not** on the outer iteration-cap path: there
the solve ran out of *outer* iterations and nothing about the projection's dual
warrants an infeasibility claim.

**0.1.6 Cost.** Two scalars of state per projection — `max|λ|` and the violation
at the midpoint sweep. Still no allocation, no `M`-length copy, no new scalar
tier, no `sqrt`.

**0.1.7 Exit criterion 5 stands unchanged** and is now achievable: over random
feasible polytopes `Infeasible` is never reported, and the detection rate on
infeasible instances is measured and stated rather than asserted.

## 1. Summary

When the polyhedron is infeasible, say so, instead of reporting a bounded
non-convergence that a caller must interpret. The signal is already computed:
Hildreth's multipliers **diverge linearly** on an infeasible system and stay
**bounded** on a feasible one, however slowly it converges.

## 2. Why it is now safe to attempt

Review 065 rated this the highest-risk theme for one reason: *a rule that fires
on a slow-but-feasible problem is worse than no detection, because it converts an
honest "not converged, here is the violation" into a false claim.*

Cycle 1 built the thing that can test that. The adversarial suite now contains
the exactly-cancelling infeasible family **and** the nearly-parallel family whose
projections cap while remaining feasible — the precise false-positive shape this
rule must survive. RFC 034 is scheduled now because that corpus exists, not
because the idea got better.

## 3. The signal, measured

Tracing `max|λ|` per sweep under the shipped `m + 1`-set formulation:

| Case | `max|λ|` at 4000 sweeps | `max|Δλ|` per sweep |
|---|---|---|
| infeasible, antipodal | `8.0e+03`, growing linearly | **`2.0000`**, constant |
| infeasible, 3-halfspace cycle | `8.0e+03`, growing | **`2.0001`**, constant |
| infeasible, 4 rows in 3-D | `8.0e+03`, growing | constant |
| **feasible**, near-parallel `ε = 1e-3` (the hardest shipped fixture) | `1.996`, **bounded** | `1.0e-06` |
| **feasible**, near-parallel `ε = 1e-5` | `2.000`, bounded | `1.0e-08` |
| **feasible**, barely feasible `w = 1e-9` | `4.000`, bounded | ~0 |
| **feasible**, ordinary vertex | `1.250`, bounded | `0` |

This is Farkas duality showing itself: an infeasible system has an unbounded
dual, and the constant `Δλ` is the certificate direction.

**Growth over the second half of a capped run**, which needs no division:

```text
286 random feasible polytopes   worst absolute growth = 3.587e-08
infeasible antipodal / 3-cycle  absolute growth       = 4.0e+02
```

Ten orders of magnitude apart.

**A false-positive shape found while measuring, and excluded by design.** A first
attempt used the *ratio* `λ(S)/λ(S/2)` and reported `inf` on the random feasible
set. That was `0/0`: **100 of 286 instances had `λ` identically zero** because no
row was ever active. A ratio is the wrong instrument; absolute growth with a
positive-violation precondition is not, and §4's rule uses the latter.

## 4. The rule — three conditions, all required

Report `SolveStatus::Infeasible` only when **all** hold:

1. the **final** projection hit `projection_max_sweeps` (already computed for
   RFC 033);
2. `max|λ|` at the final sweep is at least **1.5 times** its value at the
   midpoint sweep — linear divergence, not slow convergence;
3. `max_constraint_violation > projection_tolerance` at the returned iterate.

Condition 3 alone excludes every `λ ≈ 0` instance, which is what made the naive
ratio unusable. Conditions 1 and 2 together exclude a feasible problem whose
projection is merely capped: those have bounded `λ` (§3).

**Otherwise the existing behaviour is unchanged** — `NotConverged` with
`NoProgress` and the two honest fields, exactly as RFC 033 leaves it.

**Cost:** one scalar of extra state, `max|λ|` snapshotted at the midpoint sweep.
No new allocation, no new scalar tier, no `sqrt`.

**Why 1.5 and not 2.** Linear divergence from zero gives a ratio approaching
exactly `2`, so a threshold *at* `2` is on the boundary and misses real cases.
Measured: the 3-halfspace cycle reaches only `1.99889` at 200 sweeps, `1.999889`
at 2000 and `1.999998` at 100000 — **below `2` at every realistic budget** —
while feasible cases sit at `0.976` to `1.000`. `1.5` has roughly half an order
of margin on both sides and is not fitted to either.

## 5. Public surface

`SolveStatus` gains `Infeasible`. The enum is **`#[non_exhaustive]`**, so
downstream matches already carry a wildcard arm and this is **not a breaking
change**.

`Infeasible` means: *the kernel has evidence the polyhedron is empty.* It does
not mean the kernel has proved it — see §6.

## 6. What this RFC deliberately does not claim

**Detection is one-sided.** A `Infeasible` verdict is strong evidence; the
absence of one proves nothing. An infeasible polyhedron whose divergence is too
slow to register within the cap is still reported `NotConverged`, and that is
correct rather than a gap.

**No Farkas certificate is produced.** Extracting and validating a certificate
is a separate, larger piece of work; RFC 031's infeasible fixtures already carry
externally computed certificates for the corpus's own use.

**No LP.** Deciding feasibility in general is an LP; this detects a signal the
existing iteration already produces.

## 7. Risks

| Risk | Mitigation |
|---|---|
| **A false `Infeasible` on a slow-but-feasible problem** — the risk that governs the whole design | Three independent conditions; measured separation of ten orders; the adversarial suite's nearly-parallel family is exactly this shape and must keep reporting `NotConverged` |
| Thresholds tuned to the corpus that measured them | The factor in condition 2 derives from a *property of linear divergence* (ratio → 2), set at `1.5` for margin, not fitted. It must be justified against random instances the implementer generates, not only against the fixtures |
| A caller treats `Infeasible` as an error | It is an `Ok` outcome like every other status (RFC 006 DEVICE-006). Documented |

## 8. Exit criteria

1. `SolveStatus::Infeasible` ships; no downstream match breaks.
2. Both constrained kernels implement §4's three-condition rule, with the
   midpoint snapshot as the only new state.
3. **Every** nearly-parallel and barely-feasible adversarial fixture continues to
   report `NotConverged`, never `Infeasible`. This is the criterion that matters.
4. The exactly-cancelling infeasible family reports `Infeasible`, and its
   fixtures declare it.
5. A randomized differential test (RFC 030): over random **feasible** polytopes,
   `Infeasible` is never reported; over randomly generated infeasible ones, the
   rate at which it is reported is measured and stated, not asserted to be 1.
6. `TERMS_OF_USE.md` and both user guides state that detection is one-sided.
7. 17 gates, smoke 24/24, extended 6/6; the adversarial suite's failure count
   unchanged or reduced, never increased.
