# RFC 039 - What the Kernel Does With a Linear Objective

**Status.** Implemented (v0.22.3). Amendment 1 was made while Accepted, under RFC 000's in-place-amendment rule.

**Author tier.** `architect`.

**Governing scoping.** Architect review 091 §1, authorised by the owner on 2026-10-07.
Every figure below was measured by the architect on `36ed7b1`.

## 0.1 Amendment 1 (2026-10-07)

Made while Accepted, under RFC 000's in-place-amendment rule. The Status line carries only
the design-freeze date while this RFC is in `accepted/`, because `doc-currency` requires that
form there; **the amendment must be named in the Status line when this RFC moves to
`rfcs/done/`**, which `check-rfcs` enforces for `done/` only. Architect review 092 holds the
evidence.

**1. §2.1's premise was false: the existing exact reference cannot solve an LP at all.**
It said RFC 037 §5.6's dense enumeration "already handles `Q = 0`, since a zero Hessian is a
dense Hessian — so no new numerical code is needed". `exact_optimum` computes `Q⁻¹` to find
the unconstrained centre (`exact.rs:83-90`, `solve(q_rows.clone(), e)?`), and `Q = 0` is
singular, so the `?` propagates `None` for **every** LP, feasible or not. New numerical code
was needed and the implementer wrote it — `exact_lp_optimum`, enumerating `k = n` active sets
with `Gₛᵀλ = −c` and no `Qx` term, which is complete because a linear objective over a
bounded polytope attains its optimum at a vertex and a vertex needs `n` independent active
constraints.

**2. §2.2's "both caveats" are three, and the third has a remedy, not just a warning.**
A second non-convergence mechanism exists, found by the implementer's unopposed-direction
case: the plain outer-iteration cap, with **zero** projection cap hits, when a fixed step must
traverse a wide box. The architect then measured what §2.2 did not anticipate — that it is a
step-scale matter, not a limitation:

| `α` | iterations | outcome | `x₀` (optimum `1e6`) |
|---:|---:|---|---:|
| 0.3 | 50 000 | NotConverged | 15 000 |
| 10 | 50 000 | NotConverged | 500 000 |
| 100 | 10 001 | **Converged** | 1 000 000 |
| 1 000 | 1 001 | **Converged** | 1 000 000 |
| 100 000 | **11** | **Converged** | 1 000 000 |

Zero projection cap hits throughout, and the answer exact. The iteration count is
`≈ extent / (α·‖c‖)`.

So §2.2 requires a **third** element, and §2.3 is reframed: for a linear objective there is no
curvature to overshoot, so `α` has **no stability upper bound** — it governs distance
travelled per iteration and should be scaled to the problem's extent.
`suggested_step_scale`'s refusal at `U = 0` remains correct, because no *curvature-derived*
step exists, but the actionable advice is the opposite of caution.

## 1. Summary

The library tells an LP user to go away, and the measurement says it should not.

`crates/loeres/src/problem.rs:35-37` states:

> **LP is expressible (`Q = 0`) but not solved** by the projected kernels this contract
> feeds: projected gradient on a linear objective has no curvature to converge against
> (RFC 027 §11.6).

Measured instead, on 300 random LPs at `n = 4`, `m = 3`, `α = 0.3`, with optimality checked
by the first-order oracle validated in review 088 (`cᵀ(y − x*) ≥ 0` for every feasible `y`,
4 000 samples per instance):

| `projection_max_sweeps` | Converged | Converged but **not** optimal |
|---:|---:|---:|
| 500 (the value used throughout this project) | 297 / 300 | **0** |
| 5 000 | 299 / 300 | **0** |
| 50 000 | **300 / 300** | **0** |

**Every LP in the sample is solved optimally, given enough projection sweeps.** The worst
optimality violation across all converged runs was exactly `0.000e0`.

And the three failures at the default cap are not LP failures. Each reported
`NotConverged` / `NoProgress` with **nonzero `projection_cap_hits`** — 2, 4 and 3 — and a
terminal violation of `2.6e-5`, `1.9e-7` and `8.1e-10`. The binding constraint is the
**Dykstra projection's sweep cap**, not the absence of curvature.

This is explicable rather than lucky. For a convex problem `P(x − αc) = x` holds **iff** `x`
is optimal, so a reported `Converged` on an LP is sound by construction, for any `α > 0`.
What a linear objective costs is the *convergence* guarantee, not the soundness of the
answer.

So the claim is wrong in the **under**-promising direction. Under the owner's second
principle that is not the safe error: a visitor with an LP is turned away from something
that would serve them.

## 2. Design

### 2.1 S1 — characterise, against exact references

An LP corpus, and the kernel measured on it. Not the oracle this scoping used — **exact
references**. RFC 037 §5.6's dense enumeration **cannot** be reused: it computes `Q⁻¹` to
locate the unconstrained centre, and `Q = 0` is singular, so it returns `None` for every LP
(Amendment 1). A linear-objective reference is therefore new numerical code, and gets
RFC 030's standard — cross-validated, not merely written. At a vertex of a bounded polytope
stationarity is `Gₛᵀλ = −c` with no `Qx` term, and the optimum of a linear objective over a
bounded polytope is attained at a vertex, so enumerating the `k = n` active sets is complete.

The corpus must cover what §1's sample did not:

- an **unbounded objective direction** with a slack box, since every instance in §1 was
  bounded by its box and unboundedness therefore never arose;
- a **degenerate vertex**, where more rows are active than `n`;
- **ties** — an optimal face rather than a vertex, where the answer is non-unique and the
  question is only whether the returned point is on the face;
- the **cap sensitivity** of §1's table, reproduced as a measurement rather than quoted
  from this RFC.

Report, per instance: status, termination, `projection_cap_hits`, terminal violation, and
deviation from the exact optimum. Reuse `cargo xtask bench`'s reporting shape; this is the
same kind of evidence.

### 2.2 S2 — correct the claim, with both caveats

Replace "not solved" everywhere it appears — `problem.rs:35-37`, RFC 027 §11.6's wording as
quoted in current documents, and any user-guide echo — with what S1 measured. The
replacement must carry **both** caveats, or it trades one misleading claim for another:

1. **a `Converged` LP result is optimal**, and why (the fixed-point argument above);
2. **convergence is not guaranteed**, the default `projection_max_sweeps` is where it
   binds, and a capped projection is visible to the caller as `NotConverged` /
   `NoProgress` with nonzero `projection_cap_hits`.

Saying "LP is supported" would be the opposite error and is explicitly not what this RFC
asks for.

### 2.3 S3 — guidance for the case where the step guidance refuses

`suggested_step_scale` returns `SolverError::NumericalDomain` at `U = 0`, documented as "an
all-zero `Q` has no curvature and so no step this bound can call safe". That refusal is
**correct and stays**: there is no curvature-derived step to offer.

What is missing is that the refusal tells the caller nothing about what *does* matter. On
the evidence, the step is not the binding parameter for an LP — the projection cap is. The
error's documentation, and the LP section S2 writes, must say so and point at
`projection_max_sweeps`.

Whether a non-curvature step rule is worth offering is a question for S4, not an assumption
here.

### 2.4 S4 — then, and only then, the algorithm question

With S1's evidence, decide whether a dedicated LP method is warranted. This RFC does not
pre-judge it, and deliberately does not commit to one: a simplex or interior-point
implementation for a `no_std`, allocation-free, panic-averse device path is a large
undertaking, and §1 suggests the first thing an LP user needs is for the documentation to
stop turning them away.

S4's output may legitimately be "no, and here is why".

## 3. Explicit non-scope

- **No kernel change.** Not one line under `crates/*/src` that alters a solve. S3 touches
  documentation and, at most, an error's doc comment.
- **No new algorithm.** S4 decides; it does not implement.
- No change to `suggested_step_scale`'s behaviour — the refusal at `U = 0` is correct.
- No claim that LP is "supported" (§2.2).
- No new dependency; no public API addition without a separate decision.

## 4. Risks

| Risk | Mitigation |
|---|---|
| The correction reads as "LP is supported" | §2.2 requires both caveats in the same breath as the correction, and forbids the word. |
| 300 instances of one family is thin | It is scoping evidence and this RFC says so; S1 replaces it with a corpus and exact references. |
| An LP user raises `projection_max_sweeps` and pays for it elsewhere | S1 measures the cost, so the guidance carries a number rather than an instruction. |
| A later algorithm change invalidates the documented position | The position is tied to measurements a gate can re-run, not to prose. |

## 5. Exit criteria

1. An LP corpus exists, including an unbounded direction, a degenerate vertex and a tie.
2. Deviation from the **exact** optimum is reported for every LP instance within `n ≤ 8`.
3. The cap sensitivity of §1's table is reproduced by the harness, not quoted.
4. No document says LP is "not solved"; every place that did states the measured position
   with both caveats.
5. `suggested_step_scale`'s refusal points the caller at what governs an LP solve.
6. S4 records a decision about a dedicated algorithm, with S1's evidence cited, whichever
   way it goes.
7. `cargo xtask check` passes; `cargo xtask release-gate` passes in every slice.
