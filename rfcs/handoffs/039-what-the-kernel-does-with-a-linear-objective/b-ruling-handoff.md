# RFC 039 request B — the ruling, and what S2 and S3 must say

**To:** implementer (mid-capability model)
**From:** architect
**Authorized by.** Architect review 092 §3, which rules on the question request A §3 held B
for. RFC 039 **Amendment 1** records it in the RFC itself.
**Base revision:** `main` at or after review 092's commit.

**Holding B was right.** The question you asked had a better answer than either of us had when
you asked it, and guessing at my intent would have produced a weaker S2.

## 0. The ruling

**Yes, S2 needs a third element — and it is a remedy with a formula, not a warning.**

Your unopposed-direction case is not a limitation of the method. The architect ran it at
increasing step scales:

| `α` | iterations | outcome | `x₀` (optimum `1e6`) | cap hits |
|---:|---:|---|---:|---:|
| 0.3 | 50 000 | NotConverged | 15 000 | 0 |
| 10 | 50 000 | NotConverged | 500 000 | 0 |
| 100 | 10 001 | **Converged** | 1 000 000 | 0 |
| 1 000 | 1 001 | **Converged** | 1 000 000 | 0 |
| 100 000 | **11** | **Converged** | 1 000 000 | 0 |

Eleven iterations, exact answer, zero projection cap hits. The iteration count is
`≈ extent / (α·‖c‖)` — your arithmetic, confirmed across five step scales.

So the case is a **step too small for the geometry**, not a method that cannot reach the
optimum. Reproduce the table yourself before writing the wording; it is the evidence the wording
rests on.

## 1. S2 — three elements, not two

Every one of the thirteen locations in the implementation handoff §2.1 must carry all three.
Keep it short at each site; the user guides can carry more than a crate README.

1. **A `Converged` LP result is optimal.** `P(x − αc) = x` holds iff `x` is optimal for a convex
   feasible set, so soundness does not depend on curvature. Measured: 0 of 298-300 converged
   instances were sub-optimal, worst deviation `6.08e-10`.
2. **Convergence is not guaranteed, and the projection's sweep cap is one place it binds.**
   Visible to the caller as `NotConverged` / `NoProgress` with **nonzero**
   `projection_cap_hits`. Raising `projection_max_sweeps` resolved every such instance —
   300/300 at 5 000.
3. **The outer iteration cap is the other place it binds, and the step scale is the lever.**
   A fixed step must traverse the distance to the optimum, so a wide box with a small `α`
   exhausts `max_iterations` with **zero** projection cap hits — distinguishable from (2) by
   exactly that field. The remedy is to scale `α` to the problem's extent, since the iteration
   count is about `extent / (α·‖c‖)`.

**Do not write that LP is "supported".** Unchanged from RFC 039 §2.2.

**Do not edit the six protected locations** in the handoff §2.2 — the two CHANGELOG lines, the
two `done/` RFCs, the two "not solved to tolerance" lines in `getting-started.md` and
`device-mpc-step`. And **no apex edit without a ruling** (handoff §2.3): "PF-001 (LP) —
contract-only, not solved" is a commitment, not a description. If you think an apex line reads
as a description, quote it and ask.

## 2. S3 — reframed, and bigger than the handoff said

The original instruction was to point the `NumericalDomain` refusal at
`projection_max_sweeps`. That was too small.

For a linear objective there is **no curvature to overshoot**, so `α` has **no stability upper
bound**. It governs distance travelled per iteration. `suggested_step_scale`'s refusal at
`U = 0` stays correct — no *curvature-derived* step exists, and that is what the function
computes — but the advice a caller needs is the opposite of caution: make `α` **large**, scaled
to the extent of the feasible region.

So the refusal's documentation, and the LP section S2 writes, must say:

- there is no curvature-derived step for `Q = 0`, which is why this returns an error rather
  than a number;
- `α` is nonetheless unconstrained from above by stability for a linear objective;
- it sets distance per iteration, so scale it to the problem's extent — with the
  `extent / (α·‖c‖)` relation named;
- and both caps are where non-convergence shows up, distinguished by `projection_cap_hits`.

**Still no behaviour change.** Do not alter what `suggested_step_scale` returns, add an error
variant, or add a public function. If you conclude the library *should* offer a distance-aware
default step for the curvature-free case, that is the open item in §3, not this slice.

## 3. Open item to record, not to build

Your request C closing paragraph proposed an adaptive or distance-aware step rule as the
cheaper future step than simplex. **Record it**, and record it accurately: §0 shows a *static*
rule scaled to extent already works, so the open question is not whether such a rule is
possible but **whether the library should offer one as a default**, and what it would key off
(the box extent is available from `BoxBounds`; `‖c‖` from the linear term).

Add it to `rfcs/handoffs/release-0.22.3/finalization-checklist.md` §6, in the style of the items
already there.

## 4. One correction to your request C

Your decision stands — no dedicated LP algorithm now — and your first two arguments hold. But
**withdraw the third**: it calls the unopposed case "a pathological corpus construction, not a
realistic LP", because "a caller who sets a bound a million units from the feasible region's
interior is not modelling a real constraint".

That is not safe to assert. A variable ranging over `1e6` is an ordinary model — a budget in
currency units, a distance in millimetres, a byte count. Dismissing it would leave such a caller
unserved and unwarned.

§0's measurement gives a stronger replacement: the case converges in eleven iterations at a step
scaled to its geometry, so it is a configuration matter rather than an algorithm matter. Same
conclusion, measured rather than argued, and it does not dismiss a legitimate model.

Restate the decision with that substitution in your request B, so the record carries the
corrected reasoning rather than only the superseded argument.

## 5. Non-scope

- No kernel change; `check-public-api` stays green.
- No new public API, no new error variant, no change to `suggested_step_scale`'s behaviour.
- No edit to the six protected locations; no apex edit without a ruling.
- No LP figure pinned anywhere.
- No `#[allow(…)]`. Do not tag; do not publish.

## 6. Required evidence

```text
cargo fmt --all -- --check          # run last, immediately before committing
cargo clippy --workspace --all-features --all-targets -- -D warnings
cargo test --workspace --all-features
cargo +1.85 check --workspace --all-features
cargo xtask check                   # 20 gates
cargo xtask release-gate --intended-tag 0.22.3      # must PASS
```

State in the request:

1. **Examples affected** — `cluster-qp-constrained` if its doc comment changes; else `none`.
2. §0's table as **you** measured it.
3. The thirteen locations with their new wording, and `git diff --stat` showing none of the six
   protected ones was touched.
4. The restated S4 decision (§4).
5. Anything in this handoff that does not match the tree.
