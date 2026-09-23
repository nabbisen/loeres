# RFC 030 - Randomized Differential Testing for Numerical Kernels

**Status.** Implemented (v0.21.2).
**Design approval.** Project owner accepted it and authorized the `accepted/` transition on 2026-09-24. Architect-authored from the `0.21.1` distribution closeout (architect review 063 §7), at the project owner's request.
**Tracks.** Verification policy. Complements RFC 013 (conformance corpus and numerical parity) and RFC 010 (xtask verification governance); does not amend either.
**Touches.** `xtask/src/checks/` (a new gate and its registry), `crates/loeres-device/src/solve/tests.rs`, `crates/loeres-cluster/src/solve/projected_first_order/tests.rs`.

---

## 1. Summary

Every numerical solve kernel must carry a **randomized differential test against
an independently constructed exact reference**, and a `cargo xtask check` gate
must assert that each kernel has one — or an explicit, reasoned exemption.

Two of the four kernels in the tree have such a test today, because the architect
required it slice by slice during RFC 027. Nothing obliges the next kernel to
have one, and the two oldest kernels do not.

## 2. Why — the evidence from `0.21.1`

This release cycle found two defects that every other form of verification
missed.

**RFC 027 S2** returned a wrong projection on **20.6% of random feasible
polytopes** while reporting `SolveStatus::Converged`, with errors up to `4.67`.
It passed 18 unit tests, `fmt`, `clippy -D warnings`, all 16 gates and the
conformance corpus. Every functional test used a single constraint (`M = 1`),
where the defective formulation is exactly correct by construction, so the defect
could not appear.

**RFC 027 S3/S4** reported `Converged` on infeasible polyhedra, and RFC 029 on
divergent runs. Both survived their own suites.

In each case the thing that found the defect was the same: comparing the kernel's
output against an **independently written exact reference** on **randomly
generated inputs**, at a shape the hand-written tests did not cover. In each case
the fix was then pinned by a test that provably fails when the fix is reverted.

The generalisable lesson is narrow and testable:

> A closed-form fixture corpus tests **answers**. A randomized differential test
> tests the **formulation**. A kernel can be wrong in a way that every
> hand-chosen fixture is blind to, because fixtures are chosen by the same person
> who chose the algorithm.

RFC 027 S4 §6 recorded this directly: two of seven closed-form fixtures detected
a per-sweep multiplier reset, while the randomized test detected it every time.

## 3. Current coverage — the gap this closes

| Kernel | RFC | Differential test |
|---|---|---|
| `loeres-device::solve::constrained::solve_constrained_projected_first_order` | 027 | yes (`n = 2, m = 2`) |
| `loeres-cluster::solve::constrained::solve_constrained_projected_first_order_dyn` | 027 | yes (`n = 2, m = 2` and `n = 3, m = 3`) |
| `loeres-device::solve::solve_projected_first_order` | 006 | **none** — 23 tests, none differential |
| `loeres-cluster::solve::projected_first_order::solve_projected_first_order_dyn` | 016 | **none** — 30 tests, none differential |

The two unguarded kernels are the *oldest* and most depended-upon in the project.
Their box projection has a trivially available exact reference, so the omission is
opportunity, not difficulty.

## 4. Design

### 4.1 What counts as a numerical kernel

A public function that **iterates a numerical method to produce an approximate
solution**. Orchestration (`solve_batch`, `solve_batch_observed`), caching
wrappers (`solve_projected_first_order_dyn_cached`), and pure validation
entrypoints are not kernels; they are listed as exempt with a reason so the set
is stated rather than inferred.

### 4.2 What the test must do

1. **Generate inputs randomly** from a seeded, dependency-free generator, so runs
   are reproducible and no `dev-dependency` is added.
2. **Compare against an exact reference constructed independently of the kernel's
   algorithm** — for the projected-first-order family, active-set enumeration
   over the constraint rows *and* the box faces, which is `O(2^k)` in a small `k`
   and therefore honest at conformance dimensions. A reference that reuses the
   kernel's own iteration is not a reference and does not satisfy this RFC.
3. **Cover more than the minimal shape.** At least one instantiation with more
   than one constraint, and at least one with `n > 2`. RFC 027's defect was
   invisible at `M = 1`.
4. **Assert feasibility, not only proximity** — the returned point satisfies its
   own constraints within the stated tolerance. That single assertion would have
   caught the S2 defect on the day it landed.
5. **State a mutation it detects**, verified by the author, recorded in the
   registry row.

### 4.3 The gate

A new `cargo xtask check` gate, `differential`, with a registry in
`xtask/src/checks/differential.rs` following the `examples` gate's existing
const-table idiom:

```rust
Kernel {
    path: "crates/loeres-device/src/solve/constrained/tests.rs",
    entrypoint: "solve_constrained_projected_first_order",
    coverage: Coverage::Differential {
        test: "random_feasible_two_constraint_polytopes_match_the_exact_projection",
        detects: "per-sweep multiplier reset (RFC 027 §0.3.2)",
    },
},
```

The gate asserts, fail-closed:

- **Coverage symmetry** — every discovered kernel entrypoint has exactly one
  registry row, and every registry row names a real entrypoint. This is RFC 022's
  symmetry pattern, which the project already relies on and which caught a
  ticked-but-uncited row during this very release.
- **The named test exists** in the named file.
- **Exemptions carry a reason string**; an empty reason fails.

The gate checks that the discipline is *present*, not that it is *sound*; the
architect review checks soundness. That boundary is deliberate — a gate that
tried to judge a reference's independence would be a gate nobody could trust.

## 5. Explicit non-scope

- **Not a replacement for the conformance corpus.** RFC 013 governs cross-path
  parity on shared fixtures; this governs per-kernel formulation correctness.
  A kernel needs both, and RFC 013 is unchanged.
- **No property-testing dependency.** `proptest`/`quickcheck` would add a
  dev-dependency and a failure-shrinking model the project does not need at
  conformance dimensions; a seeded LCG in the test is sufficient and is what
  both existing tests already use.
- **No runtime budget change.** Both existing differential tests run in ~10 ms.
- **No change to any kernel's behaviour, signature, or public surface.**

## 6. Rejected alternatives

| Alternative | Reason rejected |
|---|---|
| Leave it to architect review, as in RFC 027 | It worked only because one reviewer required it four times running. It is a person, not a rule, and it does not survive that person's absence |
| Fold it into RFC 013's conformance corpus | Conformance answers cross-path agreement on *shared fixtures*; review 056 and 057 both found the corpus blind to formulation defects the randomized test caught. Different question, different home |
| Require it only for *new* kernels | Leaves RFC 006 and RFC 016 — the most depended-upon code in the project — permanently unguarded |
| Make the gate judge reference independence | Not mechanically decidable; a gate that pretends to decide it is worse than one that does not claim to |

## 7. Compatibility and cost

No public API, feature, dependency, or behaviour change. The gate count moves
from sixteen to seventeen, which ripples into the handful of documents stating
it (§9 item 5). Retrofitting the two box kernels is the bulk of the work and is
bounded: their exact reference is `clamp` composed with the unconstrained
minimiser, enumerable over box faces at conformance dimensions.

## 8. Risks

| Risk | Mitigation |
|---|---|
| A future author writes a reference derived from the kernel, satisfying the gate vacuously | The gate cannot detect this; the registry's `detects` field and architect review are the control. Stated rather than papered over |
| Randomized tests become flaky | Seeded generators only; no time-, thread-, or environment-dependent input. Both existing tests are deterministic |
| The gate becomes a box-ticking ritual | The `detects` field forces the author to have actually run a mutation; a row that cannot name one is not ready |

## 9. Exit criteria

1. `cargo xtask check` runs a `differential` gate that fails closed on a missing
   row, a missing named test, or an unreasoned exemption.
2. Every numerical kernel in the tree has a registry row; orchestration and
   caching entrypoints are listed as exempt with reasons.
3. `loeres-device::solve::solve_projected_first_order` gains a randomized
   differential test meeting §4.2, including the feasibility assertion.
4. `loeres-cluster::solve::projected_first_order::solve_projected_first_order_dyn`
   gains the same.
5. The gate count is corrected wherever it is stated.
6. Each new test's `detects` mutation is verified by the author and reported.
