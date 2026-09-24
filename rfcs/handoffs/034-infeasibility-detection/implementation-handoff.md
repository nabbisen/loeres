# Developer Handoff — RFC 034 conservative infeasibility detection

**Governing RFC.** `rfcs/accepted/034-infeasibility-detection.md` (design frozen 2026-09-24).
**Assigned to.** Implementer tier.
**Ordering.** Cycle 2, first and only RFC. Depends on RFC 031's corpus and RFC 033's final-projection tracking, both shipped in `0.21.3`.

---

## 1. Purpose

Say "infeasible" when the polyhedron is empty, instead of reporting a bounded
non-convergence the caller must interpret. **The governing risk is a false
positive**, so read §5 before §3.

## 2. Slices

**S1 — the status and the rule**, both kernels.
**S2 — conformance fixtures and documentation.** One review request each.

## 2a. C1 — the corrective slice (architect review 070). Do this first.

**S1 landed at `1d905cf` and is accepted as an implementation. The rule it
implements is the architect's and is unsound**; RFC 034 **Amendment 1 (§0)**
replaces it. §3 below is superseded by Amendment 1 and retained only as the
record of what was originally specified.

**What changed and why:** the RFC's feasible ratios of `0.976–1.000` were
measured at 4000 sweeps, after the multipliers plateaued. A cap binds *below*
the convergence time, where a feasible problem's multipliers are still rising —
ratios up to `1.89`, which the factor of `1.5` cannot separate.

**Implement Amendment 1 §0.1.2's five conditions**, in both kernels:

- add the **midpoint violation** snapshot (one more scalar beside `max|λ|`);
- raise the factor from `1.5` to **`1.9`** — and the "do not tidy" comment now
  guards `1.9`, with `1.5` recorded as the superseded value;
- add the precondition `projection_max_sweeps >= 64`;
- add condition 4: the terminal violation is **not shrinking** between the two
  snapshots (`viol_final >= 0.99 × viol_mid`);
- keep the rule at the **stationary** returns only, as S1 already does — review
  070 endorsed that reading (Amendment 1 §0.1.5).

**Required evidence, and it is the point of the slice:**

- the `#[ignore]`d strict test **un-ignored and passing**;
- your own false-positive count over random feasible polytopes including
  near-parallel *and* near-antiparallel rows (the sliver shapes), across caps
  from 10 to at least 3000. The architect measured **0 in 20,132 trials**;
- the detection rate **re-measured** by margin decade and **stated, not
  asserted** — it will fall relative to S1's 48.3%/62.0%, and that is expected;
- the adversarial suite's failure count unchanged at 5, smoke 24/24, extended 6/6.

**S2 follows C1**, not before.

## 3. The rule *(superseded by Amendment 1 §0.1.2 — retained as the original record)*

Report `SolveStatus::Infeasible` only when **all three** hold:

1. the **final** projection hit `projection_max_sweeps` — RFC 033 already tracks
   this as `last_capped`; reuse it, do not re-derive it;
2. `max|λ|` at the final sweep ≥ **1.5 ×** `max|λ|` at the midpoint sweep
   (`sweep == projection_max_sweeps / 2`);
3. `max_constraint_violation > projection_tolerance` at the returned iterate.

Otherwise behaviour is **unchanged**: `NotConverged` / `NoProgress` with the two
honest fields, as RFC 033 leaves it.

### 3.1 Why 1.5, and why not 2 — do not "tidy" this

Linear divergence from zero gives a ratio approaching exactly `2`, so a threshold
*at* `2` sits on the boundary and misses real cases. Measured on the
3-halfspace cycle: **`1.99889`** at 200 sweeps, `1.999889` at 2000, `1.999998`
at 100000 — below `2` at every realistic budget. Feasible cases sit at `0.976`
to `1.000`.

`1.5` has roughly half an order of margin either side. **Raising it to 2 is a
regression**, and the S1 review will check that the constant is still `1.5`.

### 3.2 Where the state lives

`dykstra_project` currently returns `bool` (capped). It needs to report the
divergence test as well. Return a small private struct — it is not public API —
rather than threading a second out-parameter. One scalar snapshot at the midpoint
sweep; **no new allocation, no `M`-length copy, no new scalar tier, no `sqrt`.**

Both kernels: `crates/loeres-device/src/solve/constrained.rs` and
`crates/loeres-cluster/src/solve/constrained.rs`.

### 3.3 Public surface

`SolveStatus` in `crates/loeres/src/solver.rs` gains `Infeasible`. The enum is
already `#[non_exhaustive]`, so downstream matches carry a wildcard arm and this
is **not** a breaking change. Add a `SolveReport` constructor beside
`not_converged_stalled`, following its shape.

## 4. Measured reference figures

Verified by the architect before the RFC was written. Expect the same separation;
if yours differs by orders of magnitude, **stop and report**.

```text
max|lambda| after 4000 sweeps        per-sweep increment
  infeasible antipodal   8.0e+03            2.0000  (constant)
  infeasible 3-cycle     8.0e+03            2.0001  (constant)
  feasible eps=1e-3      1.996              1.0e-06
  feasible eps=1e-5      2.000              1.0e-08
  feasible barely w=1e-9 4.000              ~0

growth over the second half of a capped run
  286 random feasible polytopes   worst  3.587e-08
  infeasible                             4.0e+02
```

**A false-positive shape already found and excluded — do not reintroduce it.**
A ratio `λ(S)/λ(S/2)` alone reports `inf` on feasible instances where `λ` is
identically zero because no row is ever active — **100 of 286** in the
architect's random set. Condition 3 excludes every such instance. A rule that
drops condition 3, or that divides without guarding a zero denominator, is wrong
however well it scores on the fixtures.

## 5. The criterion that matters

**Every nearly-parallel and barely-feasible adversarial fixture must continue to
report `NotConverged`, never `Infeasible`.** Those are feasible problems whose
projections cap — the exact false-positive shape. If any of them reports
`Infeasible`, the rule is wrong, and that is a finding to report, **not** a
fixture to adjust.

The adversarial suite's failure count must be **unchanged or lower**, never
higher.

## 6. Required tests

1. Both kernels: an infeasible polyhedron reports `Infeasible`; a nearly-parallel
   feasible one whose projection caps reports `NotConverged`.
2. **The false-positive guard**: a feasible problem where no row is ever active
   (`λ ≡ 0`) is never `Infeasible`.
3. A randomized differential test (RFC 030 discipline) over random **feasible**
   polytopes asserting `Infeasible` is **never** reported; and over randomly
   generated infeasible ones, **measuring and reporting** the detection rate —
   RFC 034 §8.5 requires it stated, **not** asserted to be 1.
4. A mutation showing the rule fires when condition 3 is dropped.

## 7. Explicit non-change scope

- No Farkas certificate; no LP; no change to `projection_cap_hits`,
  `max_constraint_violation`, the inner stopping rule, or any answer.
- No change to the RFC 006/016 box kernels — they have no rows.
- No new dependency, no new scalar tier, no `#[allow(dead_code)]`.

## 8. Prohibited shortcuts

- Raising the `1.5` factor, or dropping any of the three conditions.
- Dividing without guarding a zero denominator (§4).
- Adjusting an adversarial fixture that reports `Infeasible` (§5).
- Asserting a detection rate of 1 on infeasible instances — detection is
  **one-sided** and the RFC says so.

## 9. Required evidence

fmt, clippy `-D warnings`, `cargo test --workspace --all-features`, MSRV 1.85,
`cargo xtask check` (17 gates), `cargo xtask conformance` (smoke 24/24),
`--suite extended` (6/6), `--suite adversarial` (failure count unchanged or
lower), `mdbook build docs`, the `thumbv7em-none-eabihf` build, and your own
measured separation figures per §4.

## 10. Acceptance criteria

RFC 034 §8 items 1-7.

## 11. Review request

Standard eleven-item format; **record your author tier**; send to the architect.
Do not move RFC 034 to `done/` — it moves with the release that carries it.
