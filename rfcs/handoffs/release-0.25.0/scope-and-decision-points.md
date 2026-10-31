# Release 0.25.0 — scope and decision points

**Author tier.** `architect`. **Date:** 2026-10-31.
**Status.** Planning. No slice has landed; the workspace is at `0.24.3`, a placeholder.
**Authorized by.** The owner authorized this scope's direction on 2026-10-31 — the T5 path, in
preference to answering Phase II's audience question first. Each RFC's design freeze, and the
release itself, are authorized separately.

**Whose file this is: the owner's.** What is in scope, what it costs, and what it does **not**
claim. The cut, when it comes, is its own file.

**Why this one is `0.25.0` and the last two were not.** §2 derives it rather than predicting it:
this scope adds a function a caller can call, which is the convention's second row.

## 1. Scope

### 1.1 The release's reason — a step a linear objective can actually use

**The library documents a rule and makes the caller apply it by hand.**
`crates/loeres/src/problem.rs:296-309`, `suggested_step_scale`'s own doc comment, says:

> …with no curvature to overshoot, `α` has **no stability upper bound** at all when `Q = 0`. It
> only sets the distance travelled per outer iteration, so the right scale comes from the
> problem's **extent**, not from `curvature_bounds`: the iteration count to traverse an
> unopposed direction is about `extent / (α · |c|)` (RFC 039).

The function then returns `SolverError::NumericalDomain`. **The rule is stated and not computed**,
so a caller who has not read that paragraph gets the measured outcome below.

**Measured, by `cargo xtask lp` on `0bf94b8`** — the harness RFC 039 S1 built, against
`exact::exact_lp_optimum`, not a randomized oracle:

| `α` on the unopposed-direction case (box extent `1e6`) | iterations | result | deviation from the exact optimum |
| ---: | ---: | --- | ---: |
| `0.3` | 50 000 | **not converged** (`IterationCap`, `cap_hits=0`) | **985 000.000000013** |
| `10` | 50 000 | not converged | — |
| `100` | 10 001 | converged | — |
| `1 000` | 1 001 | converged | — |
| `100 000` | **11** | converged | — |

The same report shows the random corpus is fine — 298/300 converged at the default sweep cap,
**zero** converged-but-not-optimal, worst deviation `6.1e-10` — so this is not a soundness
problem. It is a caller who models a budget in currency units or a distance in millimetres,
takes the default `α`, and is told `NotConverged` while being a million units from the answer.
The status is honest; the experience is not.

**What the slice adds.** One additive, public, `no_std`-safe way to obtain the extent-derived
step: given the box, the linear term and **the caller's own iteration cap**, return the `α` that
lets the distance be covered inside that cap — the inverse of the measured relation
`iterations ≈ extent / (α · |c|)`. The step then follows from what the caller already configured
instead of from a paragraph they have to find.

**What it must not do, and these are hard constraints from a `done/` RFC:**

1. **`suggested_step_scale`'s refusal at `U = 0` stays.** RFC 039 §3 lists "No change to
   `suggested_step_scale`'s behaviour — the refusal at `U = 0` is correct" as explicit non-scope,
   and `problem.rs:292-294` adds that nothing calls it on the caller's behalf because substituting
   a step silently would change results for existing callers. **The new path is additive and
   opt-in.** A design that turns the refusal into a number supersedes a `done` RFC's non-scope and
   would need that stated and argued, not assumed.
2. **It does not make LP "supported".** RFC 039 §2.2 forbids the word and requires both caveats in
   the same breath as any correction: a `Converged` LP result is optimal, *and* convergence is not
   guaranteed. S4's recorded decision — **no dedicated LP algorithm** — stands
   (`rfcs/handoffs/039-…/b-ruling-handoff.md` §4). This closes the **configuration** gap S4
   identified, which is the whole of what the measurement supports.
3. **No claim without a gate** (RFC 047 anti-goal). The new rule arrives with an assertion, not a
   report: on the LP corpus, including the unopposed-direction case, the suggested step converges
   within the configured cap. RFC 030's standard applies — randomized differential against an
   exact reference, cross-validated, not merely written. `cargo xtask lp` stays a **report**, as
   its module doc insists; the new assertion belongs where the gates are.

**Open for the RFC, not for the owner:** whether this is a free function in `loeres::problem`, a
defaulted trait method, or a constructor-side helper; which scalar tiers it binds (`FiniteScalar`
plus what); and how `|c|` is obtained without allocating on the device path. I will price those in
the RFC with the zero-bleed and size-budget costs attached.

### 1.2 The small item, in this release because this release touches `crates/` anyway

`Cargo.toml:31` says *"Member crates expose implemented public APIs through RFCs 001-034."* The
tree is at `RFCs 001-046, 048-053`. It is the same stale-range defect RFC 053 deleted from
`ROADMAP.md`, in a manifest comment — outside `APEX_DOCS`, so no gate sees it.

I found it during the `0.24.2` post-release bump and deliberately left it: that commit's whole
verification property is *every changed line is a version string*, which is the check that once
caught a thirteen-package relock, and a prose edit would have broken it. **Delete and point**, as
RFC 048 and RFC 053 S2 did; do not rewrite the range.

It also raises a question §4 records rather than answers: RFC 053 §7 said I was not claiming
`ROADMAP.md` was the only place prose sits beside a correct machine-held field, and this is the
first instance found since — in a file no apex sweep covers.

### 1.3 Not in this release

- **Phase III, the front door.** Still waiting on Phase II's audience question, which is the
  owner's. This scope was chosen over it precisely because it needs nothing from the owner.
- **A dedicated LP algorithm.** Settled "no" by RFC 039 S4, on evidence, and nothing since has
  changed that evidence.
- **T6's three items** — observability, the `ffi-gateway` seam, the process-local cache. Closed
  until a real consumer exists.
- **T4, a benchmark harness.** And note why it would not have answered the owner's question:
  on its likely shape — an `xtask` command plus fixtures — it adds no callable surface, so it
  would have been another patch.

## 2. The position is `0.25.0`, derived

`docs/src/development.md`'s table: *adds functionality a caller can call* → **a minor**, bumped by
the first such slice. §1.1 adds exactly that, so the release is `0.25.0` and the directory name is
a derivation this time, not a prediction. The bump touches the four gated places that section
enumerates, and the slice that performs it is the one that lands the new function.

**The three preceding releases, for the record, since the owner asked why the position had not
moved:** `0.24.0` (2026-10-08) was the last release to change `crates/`. Between its tag and
`0bf94b8` there are **63 commits on `main` and zero touching `crates/`**. The position was correct
each time; what was missing was any slice that would move it (§4).

## 3. Decision points

**D1 — authorize the scope.** Nothing in §1 needs a further decision from the owner, and nothing
in it is blocked.

**D2 — Phase II's audience question stays open and stays the owner's.** It does not block §1.
*Cost of leaving it:* Phase III remains unschedulable, so the next capability release after this
one has the same shortage of candidates this one had.

**D3 — the release and publication.** Separate, and publication the more so (RFC 021 §7). Unlike
`0.24.1` and `0.24.2`, this release **will** change library code, so the publication calculation
is a real one rather than a formality about byte-identical source, and I will re-derive it in the
cut plan rather than carry forward the last two answers.

## 4. What I got wrong about scheduling, since the owner asked

The owner asked why the project proceeded on `0.24` continuously. The honest answer is not the
version convention — it is that **I let the schedule be set by my own review findings instead of
by the destination.** Measured from the RFCs' own governing-scoping lines:

| RFC | came from |
| --- | --- |
| 048 | architect review 114 §4 |
| 049 | architect reviews 105 §4, 115 F2 |
| 050 | architect review 117 F1 |
| 051 | architect reviews 116 §3, 117 §7 |
| 052 | architect review 115 §5 F5 |
| **053** | **the owner's question of 2026-10-31** |

Five of six came out of the previous slice's review. Each was a real defect and each is better
closed than open, but taken together they were a stack I kept popping, and the only item in the
sequence that came from outside it was the one the owner raised. Scheduling is my role; capability
did not appear on the agenda until it was asked for.

**What changes.** Every scope file from here states, in one line, which phase of
`rfcs/destination-sequence.md` the release advances — and if the answer is "none", that is the
sentence the owner should see before authorizing, not after. For this release the answer is: it
advances no phase. It closes a measured capability gap, which is RFC 047 property **A**'s
territory rather than a phase's, and that is worth saying plainly rather than dressing as progress
toward the destination.

## 5. Open after this release

- **Phase II's audience question**, and Phase III behind it.
- **The apex-adjacent prose sweep** (§1.2): `Cargo.toml` was the first stale-range claim found
  outside `APEX_DOCS` since RFC 053. Whether a sweep is worth a slice depends on how many more
  there are, and nobody has counted.
- **The plural-form residual** in RFC 053 Amendment 1.
- **The themes `T4`-`T7`** still have no definitional home (RFC 053 §2.3).
- **`Carries RFCs`** bounded and ordered, not verified (RFC 052 §0.1).
- Carried unchanged from `rfcs/handoffs/release-0.24.0/scope-and-decision-points.md` §6: the
  `f64`-only disagreement direction at `FRAC_BITS = 24`; whether `check-public-api` should carry an
  API-shape baseline.
