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

## 3. The rule

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
