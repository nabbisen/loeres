# RFC 037 corrections C4

**To:** implementer (mid-capability model)
**From:** architect
**Authorized by.** Architect review 088, which accepted S6 and C1-C3.
**Base revision:** `6f98554`, or later on `main` once review 088 is committed.

**Nothing here blocks `0.22.2`.** The release is held only on the owner's headline
decision. S6's figures are correct — the architect verified them against exact rational
arithmetic (review 088 §2) and they agree to `5.6e-17`.

This slice closes the gap between what that verification showed and what the
**repository** can show, and narrows two sentences.

## C4.1 — evidence for the dense reference on a coupled `Q`

Your cross-validation compares the dense reference against the separable one on 43
fixtures. **A diagonal `Q` is a special case of a dense one**, so those 43 problems
exercise the dense routine only on diagonal inputs — while the deviation figures come
from a **tridiagonal** corpus. In the tree, the coupled path rests on one hand-written
unit test.

Two additions, both measured by the architect before being prescribed.

### C4.1.1 A pinned exact vector, independently derived

For the `n = 4, m = 2, off = 0.50` corpus point the exact optimum is

```text
x* = (7/17, 5/17, 5/17, 7/17)
```

derived by KKT enumeration in **exact rational arithmetic**, in an implementation
independent of `exact.rs`. Both rows of `A` are exactly active there:
`7/17 + 5/17 + 5/17 = 1`.

Add a unit test asserting `exact_optimum` returns that vector for that problem, to a
tolerance appropriate to `f64` (the architect's comparison against the rationals showed
agreement to `5.6e-17` per component). This is a coupled-`Q` test whose expected value
did not come from the routine under test — the discipline RFC 031 used for its fixtures.

Write the expected values as the exact fractions in a comment beside the decimal
literals, so a later reader can re-derive them.

### C4.1.2 A randomized optimality oracle

For a convex QP, `x*` is optimal **iff** `∇f(x*)ᵀ(y − x*) ≥ 0` for every feasible `y`.
That needs no second solver, so it validates `exact_optimum` without another
implementation to trust.

Add a randomized test over coupled `Q` instances within `n ≤ MAX_N`: for each, take
`exact_optimum`'s answer, sample feasible `y`, and assert the inner product is never
materially negative. Seed it deterministically, as RFC 030's differential tests do, so a
failure is reproducible.

Measured on the `n = 4` point before prescribing:

| `x` under test | most negative `∇f(x)ᵀ(y − x)` over sampled feasible `y` |
|---|---|
| the true optimum | `0.0` across 11 368 feasible samples |
| a deliberately wrong `(0.40, 0.30, 0.30, 0.40)` | **−2.47e-2** |

Nine orders of magnitude of separation, so the oracle is decisive rather than marginal.
Rejection-sample `y` inside the box and keep the feasible ones; at these sizes that is
cheap.

### C4.1.3 Narrow the chapter's claim

`docs/src/effectiveness.md:74-77` currently ends:

> … so the two agreeing is evidence the new one is correct, not merely an assertion that
> it is.

The agreement is evidence that the generalisation **did not break the separable case**.
Say that, and then say what covers the coupled case once C4.1.1 and C4.1.2 exist. A
reader currently finishes that sentence believing the method behind the table was
validated on the problems the table reports.

## C4.2 — the speedup's cause is inferred, not measured

`docs/src/effectiveness.md:107-109`, and the same wording in `throughput`'s output:

> The gap **is** scheduling and work granularity at this batch size, **not a defect** …

Keep the ideal-speedup denominator — it is what makes `13.1×` interpretable, and without
it a reader cannot judge the figure. Change the causal assertion to *consistent with*,
and add that the cause was not isolated by this measurement. The arithmetic you give
(64 items across 32 threads is two apiece) is a fair argument and should stay; it is the
word "is" that outruns it.

## Non-scope

- No change to S6's scope, to `MAX_N`, to the 43-fixture cross-validation, or to the
  deviation figures themselves — they are verified correct.
- Do **not** pin a deviation figure in `bench-baseline`. Unchanged from review 087: a
  float deviation is not an integer count.
- No change to the eleven pinned rows, to `bench`'s corpus, or to the three legs.
- No new dependency; no criterion; no wall-time gate.
- Do **not** change `README.md:5`. The owner's decision, applied with the four
  disclaimers in the same commit.
- No `#[allow(…)]`. Do not tag; do not run `cargo publish`.

## Required evidence

```text
cargo fmt --all -- --check          # run last, immediately before committing
cargo clippy --workspace --all-features --all-targets -- -D warnings
cargo test --workspace --all-features
cargo +1.85 check --workspace --all-features
cargo xtask check                                   # 20 gates
cargo xtask bench
cargo xtask bench-baseline
cargo xtask throughput
cargo xtask release-gate --intended-tag 0.22.2      # must PASS
```

State in the request:

1. **Examples affected** — expected `none`; C4 touches no example.
2. For C4.1.1, that the pinned vector test fails if the expected value is perturbed by
   one digit — break it once, record the message, restore.
3. For C4.1.2, the oracle's separation on a true and a wrong optimum, as the table above
   records it, measured by you.
4. Anything in this handoff that does not match the tree.
