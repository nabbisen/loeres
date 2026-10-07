# RFC 043 - The Constrained Kernel Over a Bounded Scalar

**Status.** Proposed.

**Author tier.** `architect`.

**Governing scoping.** Architect review 096 (2026-10-08), pulling the trigger RFC 041 S3
named and `rfcs/handoffs/release-0.23.0/finalization-checklist.md` §6 carried forward.
Every figure below was measured by the architect on the `0.23.1` development tree.

## 1. Summary

RFC 041 shipped `Q32`, demonstrated the **box** kernel over it, and deferred one question:
whether a checked-arithmetic scalar tier is warranted. It named the **constrained** kernel
as the trigger, because that is where a bounded scalar meets arithmetic the box kernel never
performs.

Pulling that trigger found a defect that is not about `Q32`. **RFC 034 Amendment 1's
infeasibility-evidence predicate is unsound over any bounded scalar**, because it avoids
division by cross-multiplying with integers, and a saturating `mul` turns the comparison
`MAX >= MAX` — which reads **`true`**, the direction that asserts divergence. The predicate
therefore manufactures infeasibility evidence rather than suppressing it.

This RFC measures how far that reaches on real solves, fixes the predicate in a form that
cannot saturate, and answers the deferred tier question with the evidence in hand rather
than by reflex.

## 2. The defect

`has_infeasibility_evidence` (`crates/loeres-device/src/solve/constrained.rs:487-496`,
duplicated verbatim at `crates/loeres-cluster/src/solve/constrained.rs:478-487`) writes
RFC 034 Amendment 1's conditions 3 and 4 as `10·final ≥ 19·midpoint` and
`100·final ≥ 99·midpoint`, deliberately so that "nothing needs a division or a float".

Over `f64` that is sound. Over `Q32<FRAC_BITS>` — `i32`-backed
(`crates/loeres/src/scalar/fixed_point.rs:84`), with a clamping `mul`
(`fixed_point.rs:160-165`) — the `100`/`99` form costs **two decimal digits of the type's
representable range**. At `FRAC_BITS = 20` the representable magnitude is `< 2048`, so
`100·v` saturates once `v > 20.48`, and once both operands saturate the comparison is
unconditionally true.

Measured on 200 000 independently drawn snapshot quadruples per scale, differenced against
the identical comparison in `f64` (review 096 §0.1):

| max snapshot magnitude | `Q32` evidence where `f64` has none | `f64` evidence where `Q32` has none |
| --- | --- | --- |
| 10 | 0 | 0 |
| 25 | 786 | 0 |
| 120 | 17 708 | 0 |
| 1 000 | 131 630 | 0 |
| 2 000 | 151 493 | 0 |

**The false-negative column is empty at every scale.** Saturation cannot hide real
divergence through this predicate; it can only invent it. The first disagreement appears
exactly where `2048/100` predicts.

### 2.1 Why the existing narrowing is not a defence

The flag reaches a caller only through `infeasibility_evidence_of(feasible, last)` =
`!feasible && last.capped && last.infeasibility_evidence` (`constrained.rs:530-532`), so a
feasible iterate never carries it. But the multiplier update is additive
(`constrained.rs:612`), so a problem needing `max|λ| > 2048` saturates its dual ascent, the
violation stops shrinking, the iterate stays infeasible, and the projection hits its cap —
precisely `!feasible && capped`. The narrowing confines the false positive to the state a
bounded-scalar solve is **most likely** to end in.

What a caller is then told is not a wrong number but a wrong **diagnosis**, in the report's
own vocabulary: "there is evidence this problem is infeasible" about a problem that may be
feasible and merely out of range.

### 2.2 `FiniteScalar` cannot catch it

`Q32::is_finite()` is `true`, always (`fixed_point.rs:191-206`). The constrained kernel's
guards sit exactly where an `f64` solve would produce `inf` — the gradient at
`constrained.rs:810`, the candidate step at `:814` — and those are the places a bounded
scalar instead produces a silently clamped number that passes every guard. RFC 001 §100
anticipated this; the type's module doc already records it. What is new is that a *kernel*
now depends on it.

### 2.3 What is **not** established

The measurement above is of the predicate in isolation, on **independently drawn**
snapshots. Real `midpoint` and `final` snapshots are correlated along a trajectory, and the
correlation plausibly reduces the disagreement rate. **Nothing here shows that a real
constrained solve over `Q32` crosses the threshold.** S1 exists to find that out, and its
honest outcome may be that it does not.

## 3. Obligations

### 3.1 S1 — measure before fixing

A tracked harness in the shape of `xtask/src/checks/fixed_point.rs` — `#![cfg(test)]`, not
a gate, not a reported command, **no pinned threshold** — exercising the **constrained**
kernel over `Q32<20>` on a random corpus of inequality-constrained QPs, each solved again in
`f64` as the reference.

Reported per corpus, not per instance:

1. instances where `converged` disagrees between `Q32` and `f64`;
2. instances where `infeasibility_evidence` disagrees, **split by direction**;
3. the distribution of `max|λ|` reached, and the count that saturated;
4. the converged-but-wrong count — a `Q32` solve reporting `converged` whose iterate the
   `f64` reference says is not optimal, with the deviation measured the way RFC 041 S2
   measured it (relative to the `f64` solve's own deviation, not to an absolute constant).

Also tracked: the randomized differential of §2, which currently exists only as a scratch
harness.

**The deliverable is the measurement.** A corpus on which §2's false positive never fires
is a valid and valuable result, and S2 must be told which world it is shipping into.

### 3.2 S2 — the predicate, reformulated by division

Replace the cross-multiplied comparisons with ratio comparisons through
`DivisibleScalar::checked_div`. Division shrinks where multiplication grows, so the
intermediate cannot leave the range.

- `DivisibleScalar` is **already** in the kernel's bound at all three call sites
  (`constrained.rs:351`, `:569`, `:775`); only the private helper's own bound is narrower,
  which is a one-line widening.
- `Q32::checked_div` returns `Overflow` rather than saturating (`fixed_point.rs:208-220`),
  so "cannot tell" is representable instead of being fabricated as `true`. **An `Err` from
  `checked_div` in this predicate yields `false`** — no evidence — because the predicate's
  contract is to assert divergence, and an unrepresentable ratio is not an assertion.
- `midpoint == 0` yields `NumericalDomain`, which falls under the same rule: no multipliers
  means no divergence evidence.
- The ratio constants are built generically from the existing integer helper
  (`scalar_from(19).checked_div(scalar_from(10))`), so no float literal enters a `no_std`
  path and the `f64` instantiation gets the correctly-rounded `1.9` and `0.99`.
- Both copies change in step. The RFC 034 Amendment 1 warning comment travels with them.

**Precondition, not optional.** `constrained.rs:480-486` carries an explicit warning that
the factor was `1.5` once and that `1.5` was unsound: *"do not 'tidy' it either way."* That
warning is about the factor's **value**; this obligation changes its **form**, which can
still move a boundary case. S2 ships only behind a differential test over the existing
conformance corpus showing **no `f64` verdict changed on any pinned case**. If any moves,
S2 stops and returns to the architect — a moved verdict is a finding, not a merge conflict.

The cost claim — one division per *capped projection*, not per sweep, evaluated once at the
projection boundary (`constrained.rs:672`) — is read from the source and **not measured**.
S2 reports the device-profile effect through RFC 037's harness. If it is not negligible,
the fallback is the multiplicand form (`final >= midpoint.mul(ratio)`, ten to a hundred
times the headroom but the same failure past it) plus documentation, and the architect
decides, not the implementer.

### 3.3 S3 — the tier question, answered in writing

RFC 041 S3 deferred "whether a future checked-arithmetic scalar tier is warranted" to this
trigger. Review 096 §2 tabulates the options; two of them need the tier and the recommended
one does not, because **no tier from `BaseScalar` to `AdvancedNumericalScalar` can report
whether a result was clamped**, and `BaseScalar`'s arithmetic has no failure channel by
construction (RFC 001).

S3 writes the answer down once, with reasons, in `crates/loeres/src/scalar.rs`'s tier
documentation: that §3.2 removes the need at this site, that it does **not** settle the
question, and what evidence would settle it. The point is that the next bounded-scalar
kernel inherits a decision instead of rediscovering it.

### 3.4 S4 — the coarser-precision measurement

Carried from `rfcs/handoffs/release-0.23.0/finalization-checklist.md` §6: repeat RFC 041
S2's box-kernel demonstration at a coarser `FRAC_BITS`, **with a step-relative threshold**.
An absolute `1e-3` would misread a coarser answer as wrong: `4.5e-5` at `FRAC_BITS = 20` is
≈ 47 steps, which at `FRAC_BITS = 12` is `≈ 1.1e-2`. The threshold is stated in steps.

### 3.5 S5 — inherent `checked_*` on `Q32`

Also from §6, and independent of the above. Inherent `checked_add`/`checked_sub`/
`checked_mul` on `Q32`, returning `Option<Self>`, as **inherent methods only** — no trait,
no tier, nothing device-facing changes. This is the capability §2's saturation would have
needed, offered to a caller who wants it without asserting that any kernel uses it.

## 4. Non-goals

- A general fixed-point arithmetic library, or `AdvancedNumericalScalar` for `Q32`
  (RFC 041 §4 stands).
- A new scalar tier. S3 answers whether one is warranted; it does not add one.
- Deduplicating the device and cluster constrained kernels. Review 096 §3 records that
  four private helpers exist character-for-character in both, that this RFC edits both, and
  that whether they belong in core is open. The two kernels differ in allocation
  discipline and the duplication may be deliberate. **Raised, not decided, and not
  assumed.**
- Changing `Q32`'s saturating discipline. RFC 041 §100 found saturating the only defensible
  device-facing choice and nothing here disturbs that.

## 5. Release position

Under `docs/src/development.md`'s "A slice that moves the release's position bumps the
version before the cut", the position follows from the slices:

| Slice | Adds callable API? | Position |
| --- | --- | --- |
| S1, S4 | no — `#[cfg(test)]` measurement | the `0.23.1` placeholder already on `main` |
| S2 | no — private helper, behaviour fix | patch |
| S3 | no — documentation | patch |
| S5 | **yes** — inherent `checked_*` | `0.24.0`, and S5's slice does the bump |

So the theme ships as `0.23.1` with S5 held back, or `0.24.0` with S5 included. **The
architect recommends letting S1's measurement decide**: a `0.23.1` that fixes a reachable
wrong diagnosis is a better release than a `0.24.0` that bundles it with convenience
methods. The owner authorizes either.

## 6. Risks

- **Functional — Medium.** A wrong diagnosis is reachable in principle and quantified;
  reachability on real solves is unmeasured, which is S1's whole purpose. The error is
  one-directional and never certifies a diverging problem as converged by this path.
- **Business — Low.** `Q32` is behind `fixed-point-hooks`, off by default, one release old,
  with no known consumer. The window to fix this before anyone depends on it is open now.
- **Operational — Low-Medium.** S2 edits two kernels in step and touches a comparison the
  codebase explicitly warns against tidying. §3.2's differential precondition is what holds
  this at Low-Medium; without it, Medium-High.
- **Security — Low.** No untrusted input, no allocation, no `unsafe`. Saturating arithmetic
  is the memory-safe failure by construction.
