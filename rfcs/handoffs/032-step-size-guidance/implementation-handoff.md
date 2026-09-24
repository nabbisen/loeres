# Developer Handoff — RFC 032 step-size guidance

**Governing RFC.** `rfcs/accepted/032-step-size-guidance.md` (design frozen 2026-09-24).
**Assigned to.** Implementer tier.
**Ordering.** **After RFC 031**, both slices. RFC 031's adversarial corpus is what judges this rule, and a finding there may change this RFC's scope before it is written. Do not start until RFC 031 is accepted in review.

---

## 1. Purpose

Let a caller compute a **provably safe** `step_scale`, and let the kernels reject
a **provably divergent** one, using two cheap bounds that need no new scalar tier.

## 2. The mathematics, and exactly what each bound licenses

For symmetric positive semidefinite `Q`:

```text
U = maxᵢ Σⱼ |Qᵢⱼ|    (Gershgorin)   U >= λ_max
L = maxᵢ Qᵢᵢ         (diagonal)      L <= λ_max
```

| Condition | Conclusion | Kernel behaviour |
|---|---|---|
| `α < 2/U` | provably convergent | accept |
| `α >= 2/L` | provably divergent | **reject, `SolverError::InvalidInput`** |
| `2/U <= α < 2/L` | indeterminate | **accept, make no claim** |

The architect verified both bounds over 600 random PSD matrices: no violation,
`U <= 1.50 x λ_max`, `λ_max <= 3.04 x L`. **Re-verify them yourself** as part of
§4's differential test rather than taking these figures on trust.

**The indeterminate band must be accepted.** Rejecting it would reject usable
steps, and it is the reason the design carries two bounds instead of one. An
implementation that rejects anything outside `(0, 2/U)` is wrong.

## 3. Slices

**S1 — bounds and `suggested_step_scale`.** **S2 — kernel validation and
documentation.** One review request each.

## 4. S1 — `crates/loeres/src/problem.rs`

Add to the `QuadraticProgram` trait, beside `shape()` and following its pattern
(provided method, structure-only, cannot be overridden because the trait is
blanket-implemented):

- `curvature_bounds(&self) -> Result<CurvatureBounds<S>, SolverError>` — returns
  `U` and `L`.
- `suggested_step_scale(&self) -> Result<S, SolverError>` — returns `1/U` by
  checked division.

`CurvatureBounds<S>`: `#[derive(Copy, Clone, Debug, PartialEq)]`,
`#[non_exhaustive]`, public fields, output-only — the `ProgramShape` precedent.

**Scalar tiers.** `abs` (`MetricScalar`), `add`/`mul` (`BaseScalar`), `max`
(`OrderedScalar`), and `DivisibleScalar` only for `suggested_step_scale`.
**No `AdvancedNumericalScalar`, no `sqrt`.** If a bound seems to need one, stop
and report — that is a design error, not an implementation detail.

**Accumulation.** Row sums are checked like the existing row-norm scan in the
constrained kernels: a non-finite intermediate is `SolverError::Overflow`, a
non-finite input is `NonFiniteInput`.

**Required test (RFC 030 registry row).** A randomized differential test over
random PSD `Q` — generate `Q = AᵀA` from a random `A`, which is PSD by
construction — asserting `L <= λ_max <= U` against an **independently computed**
`λ_max`. Power iteration in the *test* is acceptable as the reference (the
prohibition on unbounded iteration is about the shipped kernel, not a test).
Cover non-diagonal `Q`: a diagonal-only test would make `U` and `L` coincide and
prove nothing about the band.

## 5. S2 — kernel validation and documentation

**Both constrained kernels**, in `validate_boundary` (device
`crates/loeres-device/src/solve/constrained.rs`) and its cluster counterpart:
reject `step_scale >= 2/L` as `InvalidInput`. `L` is an `O(n)` diagonal scan,
cheaper than the `O(n²)` work already done there.

Do **not** touch the RFC 006/016 box kernels: their problem traits carry their
own `step_scale` and no `Q`.

**Tests:** a step at exactly `2/L` is rejected; a step just below `2/U` is
accepted and converges; **a step in the indeterminate band is accepted** — that
last one is the regression guard against over-rejection.

**Documentation:** `TERMS_OF_USE.md` and both user guides state what each bound
licenses *and* that the middle band is accepted without a claim. Replace RFC 027
§11.6's "no convergence rate is claimed" wording with the rate's stated form and
its dependence on `λ_min`, which the library does not compute — do not state a
numeric rate.

## 5a. F1 — document the `m = 0` step-validation divergence (architect review 068)

*S1 and S2 are accepted. This is the one follow-up, non-blocking, before the
`0.21.3` cut.*

The new `α >= 2/L` rejection applies to the cluster kernel's `m = 0` path, where
RFC 027 Amendment 2 says the kernel is the RFC 016 step "operation for
operation" and RFC 016 — having no `Q` — has no such rule. Review 068 ruled the
rule stays **uniform**: the identity claim is about the computed result for
accepted inputs and is untouched (`m0_identity` passes), while the accepted
*domain* now differs, and on it the constrained kernel is strictly more
informative.

**State it** on the user-facing surfaces that already describe `m = 0` — both
cluster surfaces and the kernel rustdoc — in one sentence: `m = 0` performs the
RFC 016 step operation for operation, and additionally validates `step_scale`
against `2/L`, so a provably divergent step is rejected where RFC 016 would run
to its cap.

**Do not edit `rfcs/done/027-*.md`** — it is in `done/` and RFC 025 §11 forbids
amending it in place. The caveat lives in RFC 032 and the surfaces above.

## 6. Explicit non-change scope

- No line search, no adaptive or per-iteration step; `step_scale` stays a
  caller-supplied constant.
- **Nothing calls `suggested_step_scale` on the caller's behalf.** Silently
  substituting a step would change results for existing callers.
- No exact `λ_max` in shipped code; no `λ_min`; no numeric rate.
- No existing signature changed, no new dependency, no new scalar tier.
- No `#[allow(dead_code)]`.

## 7. Prohibited shortcuts

- **Rejecting the indeterminate band.** §2.
- Testing only diagonal `Q`, where `U` and `L` coincide.
- Deriving the reference `λ_max` from anything the shipped code computes.
- Claiming a numeric convergence rate.
- Assuming PSD holds — it is an unverified caller precondition (RFC 027 §11.5),
  and the documentation must say the bounds are meaningless without it.

## 8. Required evidence

Per slice: fmt, clippy `-D warnings`, `cargo test --workspace --all-features`,
MSRV 1.85, `cargo xtask check` (17 gates, `differential` PASS),
`cargo xtask conformance` (smoke 24/24), `mdbook build docs`, and the
`thumbv7em-none-eabihf` build with `owned-arrays`.

Plus: the re-verified bound figures over your own random PSD set; a mutation
showing the differential test detects a broken bound; and **RFC 031's adversarial
suite run before and after S2**, so the rule's effect on cap-hit rates is
measured rather than assumed (RFC 032 exit criterion 6).

## 9. Acceptance criteria

RFC 032 §6 items 1-6.

## 10. Review request

Standard eleven-item format; **record your author tier**; send to the architect.
Do not move RFC 032 to `done/` — it moves with the release that carries it.
