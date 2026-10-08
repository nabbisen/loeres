# RFC 043 implementation handoff — The Constrained Kernel Over a Bounded Scalar

**To:** implementer (mid-capability model)
**From:** architect
**Authorized by.** The owner accepted RFC 043 on 2026-10-08. Read
`rfcs/accepted/043-the-constrained-kernel-over-a-bounded-scalar.md` in full. Architect
review 096 holds the scoping and the measurements.
**Base revision:** `main` at or after the acceptance commit.
**Target release:** `0.24.0`. See §6 — **the version is not yet bumped, and which slice
bumps it depends on S1's result.**

**Five review requests, and the order is not negotiable.** **A** is S1 (measure). **B** is
S2 (the predicate), **blocked on A**. **C** is S3 (the tier decision, written, no code).
**D** is S4 (the coarser-precision measurement). **E** is S5 (inherent `checked_*`).
`cargo xtask release-gate --intended-tag <v>` is required in each; §6 says which `<v>`.

## 0. What the architect verified, and what it did not

### 0.1 The measurement in RFC 043 §2 is of the predicate, not of the kernel

The architect reproduced `has_infeasibility_evidence`'s arithmetic through the **public**
`Q32` API — the function is private — and differenced it against the same comparison in
`f64` on **independently drawn** snapshot quadruples. That establishes the predicate is
unsound past a magnitude threshold. It establishes **nothing** about how often a real solve
crosses it, because real `midpoint` and `final` snapshots are correlated along a trajectory.

**Do not treat §2's table as a prediction of what S1 will find.** If S1's corpus never
fires the false positive, say so plainly; that is a result, not a failure, and RFC 043
§3.1 says so. The architect's harness was a session scratch file and is **not retained**:
nothing to inherit, nothing to depend on. The method is in review 096 §0.1 — the predicate
reproduced through the public `Q32` API, differenced against the same comparison in `f64`,
`xorshift64*` seeded from the scale, 200 000 draws each. **S1 owns the tracked version**
and may differ; it belongs under `xtask/src/checks/`, with every other path in this handoff
relative to the project root.

### 0.2 The architect has not run the constrained kernel over `Q32` at all

The bound at all three call sites is `FiniteScalar + MetricScalar + DivisibleScalar`
(`crates/loeres-device/src/solve/constrained.rs:351`, `:569`, `:775`) and `Q32` implements
all three (`crates/loeres/src/scalar/fixed_point.rs:138`, `:191`, `:208`, `:222`), so it
**should** instantiate today. "Should compile" is not "does compile". **S1's first act is to
find out, and if it does not compile, that is the finding** — report it before writing a
corpus.

### 0.3 The helpers exist twice, character-for-character

`has_infeasibility_evidence`, `largest_multiplier`, `scalar_from` and `Snapshots` are
duplicated between `crates/loeres-device/src/solve/constrained.rs:449-496` and
`crates/loeres-cluster/src/solve/constrained.rs:440-487`, warning comment included. S2
changes **both**, in step. Do **not** deduplicate them into core: RFC 043 §4 raises that
question and leaves it open deliberately, because the two kernels differ in allocation
discipline.

### 0.4 Two premises you should check rather than trust

The architect has been wrong about this project's exact references three times, and about
`FiniteScalar`'s novelty once. The two load-bearing premises here are:

1. **Call frequency.** RFC 043 §3.2 argues the division cost is negligible because the
   predicate is evaluated once per capped projection at `constrained.rs:672`, not per
   sweep. **Read that call site yourself** before accepting the argument.
2. **`checked_div`'s behaviour on a too-large quotient.** `fixed_point.rs:208-220` returns
   `Overflow` rather than saturating. Exercise it; do not infer it from the doc comment,
   which is what the architect did.

## 1. S1 — measure before fixing (review request A)

New module, same shape as `xtask/src/checks/fixed_point.rs`: `#![cfg(test)]`, **not** a
gate, **not** a reported command, **no pinned threshold**. Suggested path
`xtask/src/checks/fixed_point_constrained.rs`, registered in `xtask/src/checks.rs`
alongside line 14's `pub mod fixed_point;` — pick a different name if it reads better and
say why.

The corpus: random **inequality-constrained** QPs, each solved over `Q32<20>` and measured
against **two** references — the exact optimum and the `f64` solve of the same problem.
Report per corpus, not per instance:

1. instances where `converged` disagrees;
2. instances where `infeasibility_evidence` disagrees, **split by direction** — a
   `Q32`-only `true` is RFC 043 §2's false positive reaching a caller;
3. the distribution of `max|λ|` reached, and how many saturated at `Q32<20>`'s bound;
4. the converged-but-wrong count, with deviation measured **relative to the `f64` solve's
   own deviation against the same exact optimum**, the way RFC 041 S2 did it — not against
   an absolute constant.

Also move the §2 differential into tracked code: the predicate's arithmetic over `Q32`
against `f64`, on a reproducible corpus. A dependency-free xorshift is fine; the
architect's used `xorshift64*` seeded from the scale, and the project takes no new
dependency for this.

**`exact_optimum` is usable here, and the architect first wrote that it was not.**
`xtask/src/checks/exact.rs:37` is "minimise ½xᵀQx + cᵀx subject to `lower ≤ x ≤ upper`,
`Ax ≤ b`" and `exact_optimum` enumerates active sets over "the `m` constraint rows, then
the `2n` box faces" (`exact.rs:59`). It is **not** a box-only reference — RFC 041 S2 used
it on box problems because box problems were what S2 solved. RFC 043 Amendment 1 records
the correction and strengthens §3.1 accordingly.

Its preconditions constrain your corpus:

- `n ≤ MAX_N = 8` (`exact.rs:34-35`);
- `Q` symmetric positive definite — unchecked caller precondition (`exact.rs:38-40`). The
  routine inverts `Q`, so `Q = 0` returns `None` for every instance. A constrained-QP
  corpus wants SPD `Q` anyway; just do not let a degenerate draw through.

This was the architect's fourth error about this project's exact references. It was caught
before you got it; §0.4's other two premises were not, so check them.

## 1.1 A1 — sweep `FRAC_BITS` (added by architect review 097; do this before B)

**A is accepted as a measurement. Its conclusion is not, and this is why.** The corpus
holds `FRAC_BITS = 20` fixed (`xtask/src/checks/fixed_point_constrained.rs:57`) and every
quantity in it is `O(1)`. Review 096 §0.1 had already measured **zero** disagreement below
magnitude `20.48` at that precision, so the null result is what the scoping predicted — it
confirms the safe regime is safe rather than testing reachability.

Re-run unchanged except for that one constant and it fires: `Q32`-only evidence at
`FRAC_BITS` 24, 26 and 28, and **seven `f64`-only** cases at 24 where the `Q32` solve fails
to report infeasibility the `f64` solve reports. Your own predicate-level assertion at
`:703` also fails at 26 and 28, refusing a claim of the architect's that was wrong.

So: re-run the existing corpus **and** the existing adversarial batch at
`FRAC_BITS ∈ {12, 16, 20, 24, 26, 28, 30}`, reporting per precision:

1. evidence disagreements, **both directions**;
2. the two effective factors — `scalar_from(19)/scalar_from(10)` and
   `scalar_from(99)/scalar_from(100)`. RFC 043 Amendment 2 tabulates what they become; your
   run should reproduce it independently rather than copy it;
3. the **terminal violation distribution**. `report.max_constraint_violation()` is public and
   this harness already calls it at `:353`; the cluster `Outcome` drops it at `:206-211`. Be
   precise that it is a scale **proxy** — the predicate's operands are the inner sweeps'
   snapshots, not the outer terminal violation — and say so where you report it;
4. `Err(SolverError::Overflow)` as an **outcome class**, not a panic. At `FRAC_BITS` 26 and
   28 `instance 27` returns `Overflow` and the harness aborts at `:384`. That is the type's
   honest failure channel working; a measurement harness records it.

**No new kernel API, no accessor, no deduplication.** This is the same harness with one
constant swept, which is why it is an obligation on A and not a new slice.

**If the sweep shows the `FRAC_BITS = 24` disagreements are trajectory artifacts** of the
step-scaled tolerance (`Q::from_raw(4)` moves with `FRAC_BITS`) rather than operand
saturation, **say so plainly**. The architect has not established the mechanism and you
should not inherit that framing. Amendment 2's ruling survives either way on the structural
ground alone.

### Two reporting corrections

- **A figure in request A is not reproducible from the committed code.** §2 reports cap hits
  as "`27/32` and `11/16` in the two sub-batches". There is no per-sub-batch counter —
  `q32_capped`/`f64_capped` are single accumulators incremented in both loops (`:495-496`,
  `:526`, `:529`, `:575`, `:578`) and printed once against `{tried}` (`:598`). The run prints
  `Q32 27/32, f64 27/32`. Drop `11/16` or add the counter that produces it.
- **"No pinned threshold" is not quite true.** `:474-477` asserts
  `converged_but_wrong.is_empty()` with the classifier `relative > 50.0 && q_deviation > 1e-4`
  at `:425`. That is a threshold and a pass criterion. **Keep it** — relative with an
  absolute guard is better than RFC 041 S2's absolute `1e-3`, which is why S4 exists — but
  name it as a threshold rather than claiming none.

## 1.2 A2 — the remaining measurement items (architect review 098); runs **alongside B**

A1 is accepted and **B is released to start**. These five are measurement hygiene, not
blockers: decision 1 is settled, so the correctness fix does not wait on them. Do them in
parallel, in either order.

1. **Sweep `1..=30`, not seven chosen points.** The set in §1.1 is the architect's and it
   **omits 25 and 27** — and 27 is the row Amendment 2 singles out, where the condition-3
   factor becomes **1.6**, inside the band RFC 034 proved unsound. Seven more macro arms
   costs about four seconds. Stop choosing points.
2. **Stop sweeping two parameters.** The `Q32` tolerance is `Q32::<F>::from_raw(4)`
   (`xtask/src/checks/fixed_point_constrained.rs:409-410`) — four *quantization steps*, so
   it moves with `FRAC_BITS` (`9.8e-4` at 12 to `3.7e-9` at 30) while the `f64` arm holds
   `1e-10`. §1.1 asked for one swept constant and the harness sweeps two. Report each
   precision both ways, or once with the tolerance fixed in absolute terms and `from_raw(4)`
   as its own row, and **say which figures move with which parameter**. The architect already
   ran this at a fixed `1e-3`: the `FRAC_BITS = 24` evidence disagreement persists
   index-for-index, and the converged-status mismatches flatten to a constant 8. Reproduce
   that rather than taking it.
3. **Express the converged-but-wrong threshold in steps** — S4's obligation, which
   Amendment 3 extends to this harness. The `300/300` count at `FRAC_BITS = 12` measures the
   threshold, not the solver: the absolute guard `1e-4` (`:425`) is **below one quantization
   step** (`2.44e-4`), and the deviations it flags are one to sixteen steps. Re-report
   `12`, `16` and `30` against a step-relative threshold; `30` will stay nonzero at about
   `1.5 × 10⁹` steps, which is the point.
4. **Report the figures A1's prose omitted**, all of them already in the committed output:
   converged-status mismatches on the feasible sliver (`8/16` at 12, `4/16` at 16, **`11/16`
   at 30** — the largest disagreement count in the sweep); converged-but-wrong on that batch
   (`9`, `5`, **`1/16` at 20**, `16/16` at 30); and that the `FRAC_BITS = 20` instance `[7]`
   deviates **274 steps** against RFC 041 S2's measured worst of ≈47 steps at the same
   precision. **Cap the per-instance list at the worst ten** — the `FRAC_BITS = 12` line
   currently prints 300 tuples on one line and buries exactly these figures.
5. **Record the tolerance window in the module doc.** No single absolute tolerance is
   representable across `1 <= FRAC_BITS <= 30`: nothing finer than `2.44e-4` at 12, nothing
   larger than `2` at 30. `from_f64(1e-5)` at `FRAC_BITS = 12` rounds to zero and the kernel
   correctly rejects it with `InvalidInput`.

No new kernel API, no accessor, no deduplication — unchanged from §1.1.

### Two things to carry forward about how A1 was reported

- **A premise about a tracked file was recalled, not re-read.** §2.1 of request A1 says the
  factor asymmetry is something Amendment 2's table *"(as I recall it) did not distinguish"*.
  It distinguishes it, and includes the 25 and 27 rows the sweep skipped. The measurement was
  independently derived, which is what mattered; the claim about the document was wrong.
  §0.4 of this handoff exists because the architect broke the same rule four times.
- **"Reporting what was found, not a summary with the figures held back"** has to include
  item 4's figures. Three nonzero, precision-dependent counts were in the printed output and
  not in the prose.

## 2. S2 — the predicate, by division (review request B — **accepted**, architect review 099)

Only after A is accepted, because **what S2 is depends on what A found**: a correctness fix
if the false positive is reachable, hardening if it is not. The owner is told which.

**Division is necessary and not sufficient** (RFC 043 Amendment 2). It fixes the *operands*
and does nothing for the *constants*: `scalar_from(99)` has already clamped before any
division happens, and within `Q32`'s documented range the factors degrade to `1.0`/`1.0` —
and at `FRAC_BITS = 27` to **1.6**, inside the band RFC 034 proved unsound. So B carries
three things, not one:

1. **Build each fractional constant by dividing `one()` down, never by multiplying up.**
   `1.9 = one + (one − one/10)`, `0.99 = one − (one/10)/10`. Every intermediate stays below
   `2`, and both land within one quantization step for every `FRAC_BITS` from 12 to 27.
2. **Detect the one upward build that remains** — `10` — by monotonicity: an accumulation of
   `one()` strictly increases in exact arithmetic, so a step that fails to increase the
   accumulator is a clamp. `if !(next > acc) { return None; }`, needing only `PartialOrd`,
   which `OrderedScalar` already carries. **Not a new tier and not a new trait method.**
3. **When the constants cannot be built, the predicate yields `false`** — no evidence — the
   same rule as a `checked_div` error below. For `Q32` that is `FRAC_BITS >= 28`, where `10`
   is not representable and no construction recovers the factors.

The architect verified (1)–(3) across nine precisions before writing them here: factors build
correctly at 12–27 and are correctly refused at 28, 29 and 30. **Reproduce that yourself**;
do not take the table on trust.

Then reformulate conditions 3 and 4 through `DivisibleScalar::checked_div`, in both kernels:

- widen the private helper's bound from `MetricScalar` to include `DivisibleScalar` — it is
  already present at every call site, so this is a one-line change, not an API change;
- build the ratio constants from the existing integer helper
  (`scalar_from(19).checked_div(scalar_from(10))`), so no float literal enters a `no_std`
  path;
- **an `Err` from `checked_div` yields `false`** — no evidence. The predicate's contract is
  to *assert* divergence, and an unrepresentable ratio is not an assertion. This covers
  `Overflow` and the `midpoint == 0` `NumericalDomain` case alike;
- carry the RFC 034 Amendment 1 warning comment across, and extend it: it currently warns
  about the factor's **value**; add that the **form** is now division precisely because the
  cross-multiplied form was unsound over a bounded scalar.

**The precondition is not optional.** `constrained.rs:480-486` says of the `1.9` factor:
*"do not 'tidy' it either way."* This changes its form, which can still move a boundary
case. Ship only behind a differential showing **no `f64` verdict changed on any pinned
conformance case**. If one moves, **stop and return to the architect** — a moved verdict is
a finding, not a merge conflict to resolve.

Report the device-profile effect through RFC 037's harness. If the division is not
negligible, the fallback is the multiplicand form plus documentation, and **the architect
decides, not you**.

## 2.1 Before the cut: correct the CHANGELOG entry (architect review 099 §1)

`[0.23.1]`'s S2 entry says missed evidence occurs *"at `FRAC_BITS >= 27` … on up to 15 of 16
instances, **never the reverse**"*. The committed harness's own output says otherwise:

| `FRAC_BITS` | fabricated | missed |
| ---: | ---: | ---: |
| 1–3 | 0 | 15 |
| **4, 5** | **1** | 0 |
| 6 | 0 | 2 |
| **7–20** | **0** | **0** |
| 21 | 0 | 4 |
| 22, 23 | 0 | 7 |
| **24–30** | 0 | **15** |

Two corrections: the onset is **21**, not 27, and `15/16` is reached at **24**; and
**"never the reverse" is false** — 4 and 5 fabricate one instance each. Replace the sentence
with the band (`7..=20` clean) and the two onsets. **The CHANGELOG is never edited once
released**, which is exactly why this is fixed now, while `[0.23.1]` is still unreleased.

State the shape as the trade it is, per Amendment 4: S2 removed the fabricated diagnosis from
every precision a caller would plausibly choose **and** widened the missed-evidence loss from
one precision to ten, 7/16 to 15/16. That trade is accepted; it is not a footnote.

## 2.2 S6 — document the usable band (new, this release)

RFC 043 §3.6. Two items, both documentation or regrouping:

1. **State the measured band in `Q32`'s module doc** — the type documents
   `1 <= FRAC_BITS <= 30`, and the constrained kernel over it agrees with an `f64` solve in
   neither direction outside `7..=20` on this corpus. Give the directions and counts from the
   table above. **Say plainly that it is one corpus** at `n = 4`, `m ∈ {1,2,3}`, `O(1)` data —
   not a theorem. A caller picking `FRAC_BITS = 24` inside the documented range currently
   loses infeasibility detection on 15 of 16 genuinely infeasible problems with no warning;
   that sentence is the deliverable.
2. **Group the figures you already collect by the four sweep caps, at `FRAC_BITS 19..=22`.**
   The onset is a sharp step (`0, 4, 7, 7, 15`), which is sharper than two-operand saturation
   alone predicts. If the onset tracks the cap, `max|λ|` scales with sweeps and the step is
   the cap distribution rather than one operand crossing the bound. **No new measurement and
   no accessor** — a grouping of what the harness already prints.

Note for the record: the onset at `FRAC_BITS = 21` is where the representable magnitude halves
from `2048` to `1024`, which brackets `max|λ|` to `(1024, 2048]` — the figure S1 reported as
unobtainable. The sweep measured it indirectly, for free.

## 3. S3 — the tier question, written (review request C)

No code. Write the answer into `crates/loeres/src/scalar.rs`'s tier documentation:

- **no tier from `BaseScalar` to `AdvancedNumericalScalar` can report whether a result was
  clamped** in general, and `BaseScalar`'s arithmetic has no failure channel by construction
  (RFC 001);
- **but that is false for a construction known to be monotonic**, which is the only case the
  kernel needs: ordering detects the clamp (§2 item 2), using a bound already present. So
  "detect saturation and withhold the flag" is not necessarily a tier in disguise, and S3
  should say so with the mechanism rather than only the argument;
- S2 removes the need **at this site** and does **not** settle the question;
- what evidence would settle it: a kernel whose correctness needs an overflow signal that
  no reformulation can avoid.

The point is that the next bounded-scalar kernel inherits a decision instead of
rediscovering it. Three or four paragraphs, not an essay.

## 4. S4 — the coarser-precision measurement (review request D)

Repeat RFC 041 S2's box-kernel demonstration at a second, coarser `FRAC_BITS`, with the
converged-but-wrong threshold **expressed in quantization steps**. **RFC 043 Amendment 3
extends this to the constrained harness too** (§1.2 item 3): A1 rediscovered the same
absolute-threshold defect there, so S4 fixes it in both places or it will be found a third
time. The arithmetic that
makes this necessary: `4.5e-5` at `FRAC_BITS = 20` is ≈ 47 steps; at `FRAC_BITS = 12` the
step is `≈2.4e-4`, so the same 47 steps is `≈1.1e-2` — eleven times past S2's absolute
`1e-3`, reading as a wrong answer when it is merely a coarser one. **State the threshold in
steps in the code, not in a comment next to an absolute constant.**

## 5. S5 — inherent `checked_*` on `Q32` (review request E)

Inherent `checked_add`, `checked_sub`, `checked_mul` on `Q32`, returning `Option<Self>`,
**inherent methods only** — no trait, no tier, nothing device-facing changes, no
`BaseScalar` implementor committed to anything. Document them as the capability a caller
can reach for when saturation is not acceptable, and that no kernel calls them.

`checked_mul` must be exact about what it checks: the intermediate product is `i64`
(`fixed_point.rs:160-165`), so the failure is the **shifted** result leaving `i32`, not the
product overflowing. Say which in the doc comment.

`check-public-api` and `published-metadata` will both have opinions about new public items
behind a feature. Expect them to fire and report what they said.

## 6. The version bump — read before running `release-gate`

**The workspace is at `0.23.1`, a placeholder, not a prediction** (`docs/src/development.md`,
"A slice that moves the release's position bumps the version before the cut").

| Slice | Adds callable API? | `--intended-tag` |
| --- | --- | --- |
| S1 (A) | no — `#[cfg(test)]` | `0.23.1` |
| S2 (B) | no — private helper | `0.23.1` |
| S3 (C) | no — documentation | `0.23.1` |
| S4 (D) | no — `#[cfg(test)]` | `0.23.1` |
| **S5 (E)** | **yes — inherent `checked_*`** | **`0.24.0`** |

**S5's slice performs the bump**, as the first slice to move the release's position, and it
touches four gated places:

1. the workspace version **and** the five internal `[workspace.dependencies]` pins, or
   `published-metadata` fails closed;
2. all eight lockfiles —
   `for d in examples/*/ device-size-reference/; do cargo update --manifest-path "$d/Cargo.toml" --offline; done`;
3. the apex `This tree` field, or `doc-currency` reports three `APEX RELEASE` findings;
4. the `## [0.23.1] — unreleased` `CHANGELOG.md` heading, renamed to `0.24.0`.
   `release-gate` requires **exactly one** `## [<workspace version>]` heading and checks it
   **first**, ahead of the tag and cleanliness checks, so a half-done bump reports
   `CHANGELOG: expected exactly one ...`.

`Last reconciled repository release` does **not** move. It stays at `0.23.0` until the
post-release commit; RFC 024's inequality is strict.

If you reach S5 before A–D are accepted, **do not bump early**. Run `--intended-tag 0.23.1`
for the earlier slices and let S5's own slice carry the bump.

## 7. Non-scope

- A general fixed-point arithmetic library, or `AdvancedNumericalScalar` for `Q32`
  (RFC 041 §4 stands).
- Adding a scalar tier. S3 answers *whether* one is warranted; it does not add one.
- Deduplicating the device and cluster constrained kernels (§0.3).
- Changing `Q32`'s saturating discipline.
- T6's three remaining items and RFC 039's distance-aware default step. Both are listed in
  `rfcs/handoffs/release-0.23.0/finalization-checklist.md` §6 and are **not** in this
  release.

## 8. Required evidence

```text
cargo fmt --all -- --check          # run last, immediately before committing
cargo clippy --workspace --all-features --all-targets -- -D warnings
cargo test --workspace --all-features
cargo +1.85 check --workspace --all-features
cargo xtask check                   # 20 gates
cargo xtask release-gate --intended-tag <v>      # must PASS; §6 gives <v>
```

**Check `cargo xtask check`'s own exit status.** Do not pipe it through `head` or `tail`: a
pipeline's status is the last command's, and the architect pushed a commit on a failing gate
exactly that way.

`release-gate` creates its evidence directory with `fs::create_dir`, so a candidate run is
**once per revision** and does not clean up after itself.

State in each request:

1. **Examples affected** — expected `none` for A–D; for E, say so explicitly either way.
2. **The `CHANGELOG.md` entry you added**, quoted. This is required of every request. The
   architect's handoffs have omitted it three times (RFC 035, RFC 036, RFC 041) and the
   omission reached a cut each time.
3. For A: the four corpus figures in §1, **including a plain statement if the false
   positive never fired**; whether the kernel compiled over `Q32` without change (§0.2);
   and what reference you used, with its weakness named (§1).
4. For B: the conformance differential proving no `f64` verdict moved, and the
   device-profile figure for the division (§2).
5. For C: the decision text as written.
6. For D: the threshold in steps, and the deviation at both precisions.
7. For E: `checked_mul`'s documented failure condition, and what `check-public-api` and
   `published-metadata` said.
8. Anything in this handoff that does not match the tree. §0.4 lists the two premises the
   architect most expects to be wrong about — check them yourself rather than reporting
   back what this file claims.
