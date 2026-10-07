# RFC 041 - A Fixed-Point Scalar Baseline

**Status.** Accepted (design frozen 2026-10-07)

**Author tier.** `architect`.

**Governing scoping.** Architect review 094, authorised by the owner on 2026-10-07. Every
figure below was measured by the architect on the `0.22.4` development tree.

## 0.1 Amendment 1 (2026-10-07)

Made while Accepted, under RFC 000's in-place-amendment rule. The Status line carries only the
design-freeze date while this RFC is in `accepted/`, because `doc-currency` requires that form
there; **the amendment must be named in the Status line when this RFC moves to `rfcs/done/`**,
which `check-rfcs` enforces for `done/` only.

**The vacuity of `FiniteScalar` is not a new finding, and this RFC overstated it.**
`FiniteScalar`'s own doc comment already says so:

> Tier 3 — boundary validation for non-finite values. Implemented for any scalar used by public
> solve entrypoints that reject non-finite inputs. **For fixed-point / bounded integer-like
> scalars these may be trivial constants (`true`, `false`, `false`).**

RFC 001 wrote that deliberately. §1 below presented it as "the hard problem", which
misdescribes what is new.

**What is genuinely undocumented is the consequence.** The trait says the predicates are
trivial; nothing says what that costs at the call sites. Measured: **43 `is_finite()` call sites
across 12 files** — 13 in the cluster constrained kernel, 11 in the device one — each of which
is a no-op for this family, leaving `BaseScalar`'s failure-channel-free arithmetic with no
overflow detection at all.

So the theme's shape is unchanged and the obligation in §3.1 stands, but its justification is
the *consequence*, not the vacuity. State it that way in the type's documentation: not "these
predicates are trivial", which RFC 001 already says, but "the kernels' finiteness guards do not
protect this family, and here is what that means".

## 1. Summary

RFC 001 reserved hooks for a fixed-point scalar family and no baseline ships. `loeres`
declares a `fixed-point-hooks` feature that gates nothing — one of the fifteen
reserved-but-inert features RFC 036 registered. Review 065 called this "the most
embedded-aligned item on the list and the one most likely to matter to the device audience
the project is built for."

The theme's hard problem is not writing the type. It is this:

```rust
pub trait FiniteScalar: BaseScalar {
    fn is_finite(self) -> bool;
    fn is_nan(self) -> bool;
    fn is_infinite(self) -> bool;
}
```

`FiniteScalar` is IEEE-754 vocabulary. A fixed-point type has no NaN and no infinity, so a
correct implementation answers `false`, `false`, `true` **unconditionally** and the trait is
**vacuous** for it.

And `BaseScalar`'s arithmetic has no failure channel — `add`, `sub`, `mul` and `neg` all
return `Self`. For floats the protection chain is `overflow → ±inf → is_finite() == false →
NumericalDomain`, and the kernels lean on it in **43 `is_finite()` call sites across 12
files** (13 in the cluster constrained kernel, 11 in the device one). **For a fixed-point
scalar every one of those is a no-op.**

So this RFC ships a baseline *and* states honestly what the baseline does not get.

## 2. What RFC 001 already decided

RFC 001 §100 anticipated this and constrains the answer:

> Implementations for bounded or fixed-point scalar families **must document whether
> arithmetic is wrapping, saturating, range-proven by construction, or unavailable**. A scalar
> type whose ordinary addition, subtraction, multiplication, or negation **may panic** under
> valid solver input **must not implement `BaseScalar`** for device-facing use. Algorithms
> that require checked arithmetic beyond this contract **must use a future specialized
> checked-arithmetic scalar tier** rather than overloading `BaseScalar`.

Of the four disciplines only **saturating** is defensible device-facing: wrapping silently
returns a wrong answer, panicking is forbidden outright, and "range-proven by construction"
cannot be claimed for general solver input. This RFC therefore ships a saturating type and
says so at every surface.

RFC 001 §124 settles ordering: for families "that have a total order and no NaN (fixed-point,
bounded integer-like), `min` / `max` are the ordinary total-order extrema." No NaN gymnastics
are needed.

And §100's closing sentence names the route for anything more — a **new tier**, explicitly not
a change to `BaseScalar`. That is what keeps this theme from becoming a contract rewrite.

## 3. Design

### 3.1 S1 — one type, with its tier disposition stated

A fixed-point scalar in `loeres`, behind `fixed-point-hooks`, which stops being inert. A
`Q`-format signed fixed-point over an integer representation, with the fractional bit count a
const parameter so the type is not hard-wired to one precision.

It implements the tiers it can, and the implementation states, **per tier**, whether that tier
is *satisfied*, *satisfied vacuously*, or *not satisfied*:

| Tier | Expected disposition |
|---|---|
| `BaseScalar` | satisfied, **saturating**, documented per §2 |
| `OrderedScalar` | satisfied — total order, §124's ordinary extrema |
| `FiniteScalar` | **satisfied vacuously** — no NaN, no infinity, so all three predicates are constant |
| `DivisibleScalar` | satisfied, with its own overflow disposition stated (`checked_div`'s contract is float-shaped too; see §4) |
| `MetricScalar` | satisfied |
| `AdvancedNumericalScalar` | decided by S1's measurement, not assumed here |

The vacuity of `FiniteScalar` and its consequence — the 43 silenced guards — must be stated in
the type's own documentation, not only in this RFC. A reader of the kernels sees 43 guards and
reasonably infers they protect every scalar family. They protect floats. That is the owner's
second principle at contract level, and it is the same class of defect RFC 039 corrected for
LP: a statement true in its own terms that leads a reader to a false conclusion.

### 3.2 S2 — the box kernel over it, with saturation measured

The **box** kernel demonstrated over the new type, per review 065's scope.

A box kernel clamps to bounds every iteration, so saturation at the bounds **may** be benign.
That is a hypothesis to test, not a claim to make: S2 measures the solve against an exact
reference, in the manner RFC 037 and RFC 039 established, and reports where the fixed-point
answer diverges from the float answer and from the exact optimum.

The failure mode to hunt is a **wrong answer that converges** — saturation producing a
plausible iterate that satisfies the convergence test. RFC 039's lesson applies: a `Converged`
status is only as honest as the arithmetic under it.

### 3.3 S3 — whether a checked-arithmetic tier is needed

Decided on S1's and S2's evidence, not here. RFC 001 §100 sanctions "a future specialized
checked-arithmetic scalar tier"; this RFC does not commit to one, and **"no, and here is why"
is a complete answer**, as it was for RFC 039 S4.

If the answer is yes, it is a separate RFC: a new tier is public API and a contract addition.

## 4. Explicit non-scope

- **The constrained kernel over fixed-point.** The projection divides, and
  `DivisibleScalar::checked_div`'s contract is float-shaped too — it returns `Overflow` for
  "finite operands whose quotient is non-finite", a condition that cannot arise for a type
  with no non-finite values. One thing at a time; the box kernel is the baseline.
- **Any change to `BaseScalar`, `FiniteScalar`, or any existing tier.** RFC 001 §100 forbids
  overloading `BaseScalar`, and S3 decides whether to *add* a tier.
- No general fixed-point arithmetic library, no `Q`-format conversion suite beyond what the
  kernel demonstration needs.
- No claim that the device path "supports fixed-point" until S2 has measured it.

## 5. Risks

| Risk | Mitigation |
|---|---|
| A vacuous `FiniteScalar` ships without its consequence stated | §3.1 makes that a documentation obligation at the type, not a footnote here. |
| Saturation yields a plausible wrong answer that converges | §3.2 measures against an exact reference and names this as the failure mode to hunt. |
| The theme becomes a contract rewrite | RFC 001 §100 names a new tier as the only sanctioned route; S1 and S2 touch no existing trait. |
| `fixed-point-hooks` becoming live changes RFC 036's registry | Expected: the registry asserts both directions, so it will fail until the feature is de-registered. That is the gate working, and the de-registration is part of S1. |
| Scope creep into the constrained kernel | Explicitly out (§4). |

## 6. Exit criteria

1. One fixed-point scalar type exists behind `fixed-point-hooks`, which no longer gates
   nothing, and is removed from RFC 036's reserved-inert registry.
2. Its arithmetic discipline is **saturating** and documented as such, per RFC 001 §100.
3. Each tier's disposition is stated — satisfied, satisfied vacuously, or not satisfied — in
   the type's own documentation.
4. The vacuity of `FiniteScalar`, and the fact that the kernels' finiteness guards do not
   protect this family, is stated where a reader of the type meets it.
5. The box kernel is demonstrated over the type, with its answer compared against an exact
   reference and against the `f64` answer.
6. S3 records a decision about a checked-arithmetic tier, with evidence cited, whichever way
   it goes.
7. `cargo xtask check` passes; `cargo xtask release-gate` passes in every slice.
