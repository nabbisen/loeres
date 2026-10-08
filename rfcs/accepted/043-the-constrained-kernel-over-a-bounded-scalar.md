# RFC 043 - The Constrained Kernel Over a Bounded Scalar

**Status.** Accepted (design frozen 2026-10-08)

**Author tier.** `architect`.

**Governing scoping.** Architect review 096 (2026-10-08), pulling the trigger RFC 041 S3
named and `rfcs/handoffs/release-0.23.0/finalization-checklist.md` §6 carried forward.
Every figure below was measured by the architect on the `0.23.1` development tree.

## 0.1 Amendment 1 (2026-10-08)

Made while Accepted, under RFC 000's in-place-amendment rule. The Status line carries only
the design-freeze date while this RFC is in `accepted/`, because `doc-currency` requires
that form there; **the amendment must be named in the Status line when this RFC moves to
`rfcs/done/`**, which `check-rfcs` enforces for `done/` only.

**S1 has an exact reference available, and §3.1 understated it to the `f64` solve.**
`xtask/src/checks/exact.rs`'s `DenseQp` is "minimise ½xᵀQx + cᵀx subject to
`lower ≤ x ≤ upper`, `Ax ≤ b`" (`exact.rs:37-49`) and `exact_optimum` enumerates active
sets over "the `m` constraint rows, then the `2n` box faces" (`exact.rs:59`). It is **not**
restricted to separable or box problems; RFC 041 S2 used it on box problems because box
problems were what S2 solved.

Its preconditions, which S1's corpus must respect:

- `n ≤ MAX_N = 8` (`exact.rs:34-35`), RFC 037 §5.6's enumeration scope;
- `Q` symmetric positive definite — a caller precondition, unchecked, exactly as the
  kernels require (`exact.rs:38-40`). `Q = 0` is excluded: the routine inverts `Q`, which
  is why it returns `None` for every linear objective.

So §3.1's obligation is **strengthened**: the corpus is measured against the **exact
optimum** as the primary reference, with the `f64` solve retained as the second reference,
exactly the two-reference shape RFC 041 S2 used. The converged-but-wrong figure then means
what it says, rather than "disagrees with another approximate solve".

The architect wrote §3.1 against a remembered premise — "`exact_optimum` is a separable box
reference" — without re-reading the file. That is the fourth error about this project's
exact references, and the reason RFC 043's handoff §0.4 exists.

## 0.2 Amendment 2 (2026-10-08)

Made while Accepted, under RFC 000's in-place-amendment rule, on architect review 097
(S1's result). Same Status-line rule as Amendment 1.

**§2's one-directionality claim is wrong outside `FRAC_BITS = 20`, and §2 framed the defect
as a property of the data when it is also a property of the precision.** S1 measured the
corpus at the committed `FRAC_BITS = 20` only, which review 096 §0.1 had already shown to be
inside the regime where no disagreement can occur. Re-run at finer precisions, S1's own
corpus and adversarial batch disagree in **both** directions on real solves — `Q32`-only
evidence (§2's false positive reaching a caller) at `FRAC_BITS` 24, 26 and 28, and seven
**`f64`-only** cases at 24, where the `Q32` solve fails to report infeasibility the `f64`
solve reports. §6's "never certifies a diverging problem as converged" is retracted at those
precisions. The mechanism per direction is **not** established and A1 below owes it.

### The structural finding

`scalar_from(n)` builds `n` by repeated addition of `one()` and saturates like everything
else, so **within `Q32`'s own documented range `1 <= FRAC_BITS <= 30`
(`crates/loeres/src/scalar/fixed_point.rs:7-12`) RFC 034 Amendment 1's factors are not
representable**:

| `FRAC_BITS` | max representable | effective condition-3 factor | effective condition-4 factor |
| ---: | ---: | ---: | ---: |
| ≤ 24 | ≥ 128 | 1.9 — as chosen | 0.99 — as chosen |
| 25, 26 | 64, 32 | 1.9 | **1.0** |
| 27 | 16 | **1.6** | **1.0** |
| 28, 29, 30 | 8, 4, 2 | **1.0** | **1.0** |

The `FRAC_BITS = 27` row matters most: `crates/loeres-device/src/solve/constrained.rs:480-486`
records that the factor *"was 1.5 in the original RFC 034 and that was unsound; do not
'tidy' it either way"*. At `FRAC_BITS = 27` the predicate applies **1.6**, inside the band
RFC 034 proved unsound, with that comment sitting directly above the code that does it. No
precondition is violated and no adversarial data is needed.

### What this does to §3.2's remedy

**Reformulating by `checked_div` is necessary and not sufficient.** Division fixes the
*operands*; it does nothing for the *constants*, because `scalar_from(99)` has already
clamped before any division happens. §3.2 is therefore amended with a construction
discipline, verified across nine precisions before being written here:

1. **Build a fractional constant by dividing `one()` down, never by multiplying `one()`
   up.** `1.9 = one + (one − one/10)` and `0.99 = one − (one/10)/10` keep every intermediate
   below `2`, and both land within **one quantization step** of their exact value for every
   `FRAC_BITS` from 12 to 27 — where the cross-multiplied form has already lost the factor
   entirely by 25.
2. **Detect the one upward build that remains.** An accumulation of `one()` is strictly
   increasing in exact arithmetic, so *a step that fails to increase the accumulator is a
   clamp*. That test needs only `PartialOrd`, which every `OrderedScalar` already carries:

   ```rust
   let next = acc.add(S::one());
   if !(next > acc) { return None; }   // `n` is not representable in `S`
   ```

   This is not a new tier and not a new trait method. §2's option (c) said no tier can ask
   whether a result was clamped; that is true in general, and **false for a construction
   known to be monotonic**, which is the only case the kernel needs.
3. **When the constants cannot be built, the predicate yields `false`** — no evidence — the
   same rule §3.2 already sets for a `checked_div` error. For `Q32` that is `FRAC_BITS >= 28`,
   where `10` itself is not representable and no construction can recover the factors.

Verified: with (1) and (2), factors build correctly at `FRAC_BITS` 12–27 and are correctly
**refused** at 28, 29 and 30.

### §3.1 gains obligation A1

Review 097 §5: re-run the existing corpus and adversarial batch at
`FRAC_BITS ∈ {12, 16, 20, 24, 26, 28, 30}`, reporting per precision the evidence
disagreements in both directions, the two effective factors, the terminal violation
distribution, and `Overflow` returns as an outcome class rather than a panic. No new kernel
API, no accessor, no deduplication — the same harness with one constant swept. **A is
accepted as a measurement; its "unreached" conclusion is not, and A1 is what replaces it.**

### Release position unchanged

A1 is `#[cfg(test)]`-only and §5's table is unaffected. S2 remains a patch; S5 still carries
the bump.

## 0.3 Amendment 3 (2026-10-08)

Made while Accepted, under RFC 000's in-place-amendment rule, on architect review 098
(A1's result). Same Status-line rule as Amendments 1 and 2.

**A1 is complete and Amendment 2's table is confirmed by independent measurement.** The
effective factors were re-derived from `scalar_from`'s own formula rather than copied, and
the `99/100` factor is measured degrading one precision before `19/10`, as Amendment 2's
table states.

**The mechanism for the `Q32`-only direction is now established.** The genuinely-infeasible
batch's terminal violation is `≈1.986`, so condition 4's left operand `100 · v ≈ 198.6`
fits inside `FRAC_BITS = 20`'s representable `2048` and **saturates** against `FRAC_BITS = 24`'s
`128` — exactly where the disagreement appears, and exactly at review 096 §0.1's `max/100`
threshold (`20.48` versus `1.28`). **The `f64`-only direction remains unexplained**: both
multiplier snapshots saturating yields `MAX >= MAX`, which is `true`, so condition 2 going
false at `FRAC_BITS = 24` is accounted for by nothing yet on the table. It needs the inner
snapshots, which no public accessor exposes, and **adding one stays out of scope**.

**Tolerance co-variation is eliminated.** The harness set the `Q32` tolerance to four
quantization steps, so A1's sweep varied precision *and* tolerance together. Held fixed in
absolute terms, the `FRAC_BITS = 24` disagreement persists index-for-index, while the
converged-status mismatches flatten to a constant — so the evidence disagreements are
precision-driven and the status mismatches were the second parameter.

### §3.4 (S4) gains scope

S4's step-relative threshold was written for RFC 041 S2's box demonstration. **It now also
governs the constrained harness**, because A1's `300/300` converged-but-wrong count at
`FRAC_BITS = 12` is the same absolute-threshold defect review 095 §7 found: the guard
`1e-4` is **below one quantization step** at that precision, and the deviations it flags are
one to sixteen steps — ordinary quantization, not wrongness. At `FRAC_BITS = 30` the
deviations are about `1.5 × 10⁹` steps and are genuinely wrong; the two ends of the range
are different phenomena and the threshold must be able to say so.

### A representability window, to be documented

No single absolute tolerance is representable across the documented `1 <= FRAC_BITS <= 30`:
nothing finer than `2.44e-4` exists at 12, nothing larger than `2` exists at 30. A tolerance
that rounds to zero is correctly rejected as `InvalidInput`, but nothing tells a caller the
window `[2.44e-4, 2)` exists. S4 records it.

### A2, alongside B

Review 098 §8 carries five measurement items — sweep `1..=30` (the architect's own
instruction omitted 25 and 27, where the condition-3 factor is **1.6**), stop sweeping two
parameters, express the threshold in steps, report the figures A1's prose omitted, record
the tolerance window. **A2 runs alongside B, not before it:** decision 1 is settled and the
correctness fix does not wait on harness hygiene.

## 0.4 Amendment 4 (2026-10-08)

Made while Accepted, under RFC 000's in-place-amendment rule, on architect review 099 (B's
and A2's result). Same Status-line rule as Amendments 1-3.

**§3.2's named risk — the `f64` last bit — did not materialise, and the reason is exact.**
The divide-down construction reproduces both literals **bit-for-bit** in `f64`:
`one + (one − one/10)` is `0x1.e666666666666p+0`, which *is* `1.9`, and
`one − (one/10)/10` is `0x1.fae147ae147aep-1`, which *is* `0.99`. So the `f64` path compares
the same values against the same constants, and the pinned conformance corpus being identical
line for line (`infeasibility_evidence` set on the same 2 of 17 paths) is explained rather
than merely observed.

**Amendment 2's illustrative snippet is wrong.** `if !(next > acc) { return None; }` trips
`clippy::neg_cmp_op_on_partial_ord` under `-D warnings`, because for a partially ordered type
`!(a > b)` and `a <= b` differ on incomparable operands. The correct form is
`if next <= acc { return None; }`. Read Amendment 2 item 2 with that substitution.

### What S2 traded, with the numbers

Measured on the genuinely-infeasible adversarial batch (16 instances), all 30 precisions:

| | fabricated evidence | missed evidence |
| --- | --- | --- |
| before S2 | 1/16 at `FRAC_BITS` 24, 26, 28, 30 | 7/16 at `FRAC_BITS` 24 only |
| after S2 | 1/16 at `FRAC_BITS` 4, 5 | 4 → **15 of 16** across `FRAC_BITS` **21–30** |

S2 removed the fabricated diagnosis from every precision a caller would plausibly choose, and
**widened the loss of true positives from one precision to ten, and from 7/16 to 15/16**.
**This RFC accepts that trade and records it as one:** a fabricated "there is evidence this
problem is infeasible" misleads a caller about their problem, while a missing hint
under-informs them about the solver, and `infeasibility_evidence` is a hint and not a status.
The onset is `FRAC_BITS = 21`, **not 27**, and the reverse direction is **not** absent — 4 and
5 fabricate one instance each.

### The usable band

`FRAC_BITS 7..=20` is the only part of the documented `1..=30` range where the constrained
kernel over `Q32` disagrees with an `f64` solve in **neither** direction on this corpus.
Outside it: `1..=6` fails in both directions and hits `InvalidInput` from constraint rows
quantizing to all-zero; `21..=30` misses evidence; `25..=30` loses the factors themselves;
`29..=30` adds converged-but-wrong and 99 of 300 overflows.

**A caller who picks `FRAC_BITS = 24`, well inside the documented range and a reasonable
choice for data in `[-1,1]`, silently loses infeasibility detection on 15 of 16 genuinely
infeasible problems, and nothing tells them.** Closing that is §3.6 below.

### The onset brackets `max|λ|`

Missed evidence begins exactly where the representable magnitude halves from `2048` to
`1024`, which locates the saturating operand in **`(1024, 2048]`** — the `max|λ|` figure S1
reported as unobtainable through the public API. The onset precision is an indirect
measurement of it, free and already in the committed output. **The mechanism is not
established**: a step from `0/16` to `4/16` between adjacent precisions is sharper than
simple two-operand saturation predicts, so something about the `max|λ|` distribution across
the four sweep caps is also in play. §3.6 item 2 is the cheap test.

## 3.6 S6 — document the usable band (added, this release)

1. State the measured band in `Q32`'s module doc, with the directions and counts above, and
   say plainly that it is one corpus at `n = 4` with `O(1)` data, not a theorem.
2. Group the existing figures by the four sweep caps at `FRAC_BITS 19..=22`. If the onset
   tracks the cap, `max|λ|` scales with sweeps and the step is the cap distribution rather
   than a single operand crossing the bound. One line of grouping, no new measurement.
3. No new kernel API, no accessor, no deduplication.

S6 is documentation and a regrouping of collected figures: it does not move the release's
position, and §5's table is unaffected.

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
kernel over `Q32<20>` on a random corpus of inequality-constrained QPs, measured against
**two** references: `exact_optimum` (the exact optimum; see Amendment 1 for its
preconditions) and the `f64` solve of the same problem.

Reported per corpus, not per instance:

1. instances where `converged` disagrees between `Q32` and `f64`;
2. instances where `infeasibility_evidence` disagrees, **split by direction**;
3. the distribution of `max|λ|` reached, and the count that saturated;
4. the converged-but-wrong count — a `Q32` solve reporting `converged` whose iterate the
   **exact** reference says is not optimal, with the deviation measured the way RFC 041 S2
   measured it (relative to the `f64` solve's own deviation against the same exact optimum,
   not to an absolute constant).

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
