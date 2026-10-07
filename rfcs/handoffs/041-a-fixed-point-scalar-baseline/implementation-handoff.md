# RFC 041 implementation handoff — A Fixed-Point Scalar Baseline

**To:** implementer (mid-capability model)
**From:** architect
**Authorized by.** The owner accepted RFC 041 on 2026-10-07. Read
`rfcs/accepted/041-a-fixed-point-scalar-baseline.md` **including Amendment 1**, which corrects
what this RFC claimed was new. Architect review 094 holds the scoping.
**Base revision:** `main` at or after the acceptance commit.
**Target release:** `0.23.0`, alongside RFC 042.

**Three review requests.** **A** covers S1 (the type and its tier disposition). **B** covers S2
(the box kernel over it, measured). **C** covers S3 (the checked-arithmetic-tier decision,
written, no code). `cargo xtask release-gate --intended-tag 0.23.0` is required in each.

## 0. What the architect verified, and one thing the architect got wrong

### 0.1 The vacuity is already documented — do not present it as a discovery

`FiniteScalar`'s own doc comment says it:

> For fixed-point / bounded integer-like scalars these may be trivial constants
> (`true`, `false`, `false`).

RFC 041 §1 called this "the hard problem", which overstates it; Amendment 1 corrects that. The
**consequence** is the undocumented part, and it is what your documentation must carry:
**43 `is_finite()` call sites across 12 files** — 13 in the cluster constrained kernel, 11 in
the device one — each a no-op for this family, leaving `BaseScalar`'s arithmetic (which returns
`Self` and has no failure channel) with no overflow detection at all.

So write "the kernels' finiteness guards do not protect this family, and here is what that
means", not "these predicates are trivial".

### 0.2 The tier contracts you must satisfy, read rather than assumed

| Tier | Members | Note for a fixed-point type |
|---|---|---|
| `BaseScalar` | `zero`, `one`, `add`, `sub`, `mul`, `neg`, `is_zero` | all return `Self`; **no failure channel** |
| `OrderedScalar` | `min`, `max` (no default body), `clamp` provided | RFC 001 §124: ordinary total-order extrema, no NaN contract |
| `FiniteScalar` | `is_finite`, `is_nan`, `is_infinite` | `true`, `false`, `false` — vacuous |
| `DivisibleScalar` | `checked_div`, `checked_recip` provided | see §0.3 |
| `MetricScalar` | `abs` (**must be panic-free**), `epsilon`, `lte_tolerance` provided | `abs` of the most-negative value is the classic two's-complement panic; saturate |
| `AdvancedNumericalScalar` | — | RFC 001 forbids it as a baseline bound; decide, do not assume |

`OrderedScalar::min`/`max` have **no default body** precisely so each implementation pins its
own behaviour. `clamp`'s contract says it "never panics" and returns `hi` if `lo > hi`, which is
"panic-avoidance, not a projection semantics callers may rely on" — keep that property.

### 0.3 `DivisibleScalar`'s contract is float-shaped too

`checked_div` is documented to return `SolverError::Overflow` for "finite operands whose
quotient is non-finite". **For a type with no non-finite values that condition cannot arise.**
So a fixed-point `checked_div` must decide what it does when the true quotient exceeds the
representable range, and `Overflow` is the honest answer even though the documented trigger does
not apply. State that decision; do not silently saturate a division and report `Ok`.

### 0.4 S2 has an exact reference already, unlike RFC 039

The architect checked this rather than assuming it, having been wrong on exactly this point for
RFC 039. `xtask/src/checks/conformance/reference.rs` solves `½ Σ qᵢ(xᵢ − tᵢ)²` over box plus
`Ax ≤ b` using `quadratic_diag` **directly — six sites, no matrix inversion**, so it has none of
the singular-`Q` problem that made `exact_optimum` return `None` for every LP. And `rows_of`
loops `for i in 0..m` before pushing box faces, so **`m = 0` is representable**: a box-only
problem yields box faces only.

It is `#[cfg(test)]`, which is the right home for S2's comparison anyway. **No new numerical
code is needed for S2.**

## 1. S1 — the type

A `Q`-format signed fixed-point scalar in `loeres`, behind `fixed-point-hooks`, with the
fractional bit count a const parameter so the type is not wired to one precision.

- **Saturating**, per RFC 001 §100 and RFC 041 §2. Wrapping silently returns a wrong answer;
  panicking is forbidden device-facing; "range-proven by construction" cannot be claimed for
  general solver input. Document the discipline on the type, in those terms.
- **Panic-free on every tier method.** `MetricScalar::abs` is the trap: in two's complement
  `abs(MIN)` overflows. Saturate.
- **State each tier's disposition** — satisfied, satisfied vacuously, or not satisfied — in the
  type's own documentation, with §0.1's consequence under `FiniteScalar`.

### 1.1 Two gates will fail until you handle them, and that is them working

- **RFC 036's reserved-inert registry** asserts both directions: every declared feature with no
  `cfg(feature)` site is registered, and every registered one is inert. Making
  `fixed-point-hooks` live **must** fail `published-metadata` until you de-register it.
  De-registering is part of S1; do not pre-emptively remove the entry before the feature gates
  something, or you break the other direction instead.
- **`feature-matrix`** builds explicit feature combinations. A newly live feature may need a
  row. Check rather than wait for the gate.

## 2. S2 — the box kernel over it, measured

Demonstrate `solve_projected_first_order` (the box kernel, **not** the constrained one — RFC 041
§4 puts the projection out of scope) over the new type.

Compare against **two** references, because they answer different questions:

1. the **exact** separable reference of §0.4, for whether the fixed-point answer is right;
2. the **`f64`** solve of the same problem, for what precision costs.

**The failure mode to hunt is a wrong answer that converges.** Saturation can produce a
plausible iterate that satisfies the convergence test, and RFC 039's lesson is that a
`Converged` status is only as honest as the arithmetic under it. Report any instance where the
fixed-point solve reports `Converged` and the deviation from the exact optimum exceeds the
configured tolerance — and if there are none, say so explicitly rather than omitting the line.

A box kernel clamps to bounds every iteration, so saturation at the bounds **may** be benign.
That is the hypothesis; measure it, do not assert it.

## 3. S3 — the decision

Whether RFC 001 §100's "future specialized checked-arithmetic scalar tier" is needed, on S1's
and S2's evidence. **"No, and here is why" is a complete answer**, as it was for RFC 039 S4.

If the answer is yes, **do not implement it** — a new tier is public API and a contract
addition, and it gets its own RFC.

## 4. Non-scope

- **No change to any existing tier.** RFC 001 §100 forbids overloading `BaseScalar`; S3 decides
  whether to *add*.
- **Not the constrained kernel** over fixed-point (§0.3 is why: `checked_div`'s contract needs
  its own decision first).
- No general fixed-point library, no `Q`-format conversion suite beyond what S2 needs.
- No claim that the device path "supports fixed-point" until S2 has measured it.
- No `#[allow(…)]`. Do not tag; do not publish.

## 5. Required evidence

```text
cargo fmt --all -- --check          # run last, immediately before committing
cargo clippy --workspace --all-features --all-targets -- -D warnings
cargo test --workspace --all-features
cargo +1.85 check --workspace --all-features
cargo xtask check                   # 20 gates
cargo xtask published-metadata      # standalone; see §1.1
cargo xtask release-gate --intended-tag 0.23.0      # must PASS
```

State in each request:

1. **Examples affected** — expected `none`.
2. For S1: the tier disposition table as implemented, and the overflow discipline's wording.
3. For S1: that `abs(MIN)` does not panic — exercise it.
4. For S1: the `published-metadata` failure **before** de-registering `fixed-point-hooks`, and
   the pass after. That is the registry working and is worth showing.
5. For S2: the comparison against both references, and the explicit statement about
   converged-but-wrong instances, including "none" if that is the answer.
6. For S3: the decision and its evidence.
7. Anything in this handoff that does not match the tree. Three of the architect's premises
   about this project's exact references have been wrong; §0.4 was checked for that reason, but
   check it yourself.
