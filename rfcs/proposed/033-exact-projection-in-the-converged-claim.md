# RFC 033 - An Exact Projection as Part of the `Converged` Claim

**Status.** Proposed (2026-09-24)
**Design approval.** Architect-authored from RFC 031 S2's finding (architect review 067); project owner approved adding it to Cycle 1 on 2026-09-24.
**Tracks.** Completes the line RFC 027 Amendment 5 and RFC 029 began. Resolves review 056 L1 properly, replacing the argument review 067 falsified.
**Touches.** `crates/loeres-device/src/solve/constrained.rs`, `crates/loeres-cluster/src/solve/constrained.rs`, the RFC 031 adversarial fixtures' expected status, user-facing docs.

---

## 1. Summary

`Converged` must additionally require that the projection producing the returned
iterate was **not capped**. A kernel that ran its inner projection to
`projection_max_sweeps` without converging cannot claim the point is the
projection, and today it does.

## 2. The evidence

RFC 031's adversarial suite, on its first run, produced solves that are feasible,
stationary, reported `Converged`, and **wrong**. Reproduced independently by the
architect (review 067) on the cluster kernel, nearly parallel normals
`a₁=(1,0)`, `a₂=(1,ε)`, exact optimum the vertex `(1,1)`:

```text
eps=0.02   Converged  cap_hits=0  violation=0  err=4.996e-9
eps=0.01   Converged  cap_hits=2  violation=0  err=4.542e-7
eps=0.005  Converged  cap_hits=2  violation=0  err=4.104e-4
eps=0.002  Converged  cap_hits=2  violation=0  err=1.341e-3
eps=0.001  Converged  cap_hits=2  violation=0  err=9.048e-4
```

Amendment 5's gate correctly does not fire — the point is feasible. RFC 029's
rule correctly does not fire — the outer step is stationary. Both legs of the
current claim hold, and the answer is still `1.3e-3` out. The only signal is
`projection_cap_hits`, which the batch seam does not carry.

## 3. The third leg

| Requirement of `Converged` | Established by |
|---|---|
| the iterate is feasible | RFC 027 Amendment 5 (§0.5) |
| the iterate is stationary, measured at the final iteration | RFC 029 |
| **the projection producing it was exact** | **this RFC** |

**Rule.** Report `Converged` only when the **final** outer iteration's projection
returned without hitting `projection_max_sweeps`. Otherwise
`SolveReport::not_converged_stalled` — `NotConverged` with `NoProgress`, the same
shape Amendment 5 uses.

The *final* projection, not any projection: an early outer iteration may
legitimately cap while the iterate is far from the optimum and later iterations
converge cleanly. `projection_cap_hits` continues to count all of them and is
unchanged.

## 4. Measured effect — verified before proposing

Prototyped on the cluster kernel:

```text
eps=0.02 -> Converged (unchanged)      eps<=0.01 -> NotConverged
easy feasible control -> Converged (unchanged)

smoke      24/24 pass    (unchanged)
extended    6/6  pass    (unchanged)
cluster unit tests 113/113 pass (unchanged)
adversarial 21/26 pass   (unchanged count; status_match moves 26/0 -> 21/5)
```

**No feasible fixture anywhere in the corpus is downgraded** except the nearly
parallel family this RFC exists for.

## 5. What this RFC does *not* do

**It does not make the failing fixtures pass, and it does not improve any
answer.** Under the prototype, `solution_within_tolerance` stays at four
failures: the kernel still returns a point `1.3e-3` from the optimum on nearly
parallel normals. What changes is that it no longer *claims* that point is the
projection.

This is deliberate. Making the projection accurate on ill-conditioned polyhedra
is a numerical problem — acceleration, a smarter inner method, or simply more
sweeps — and belongs to a later theme with its own evidence. Reporting honestly
is separable, small, and should not wait for it.

**Also out of scope:** any change to `projection_cap_hits`,
`max_constraint_violation`, the inner stopping rule, the RFC 006/016 box kernels
(which have no inner projection), any signature, and any dependency.

## 6. Consequence for review 056 L1

L1 — the batch seam carries only the core `SolveReport` — was settled as option
(a) in review 057, on the argument that a truthful status warns a batch caller.
Review 067 falsified that argument for exactly this failure mode, where the
status was `Converged`.

With this RFC the argument becomes true as originally stated: a batch caller
reading `NotConverged` *is* warned, in the case that matters. **L1 remains
settled as option (a)**, now on evidence rather than on an untested
generalisation, and `BatchItemOutcome` still does not need widening.

## 7. Risks

| Risk | Mitigation |
|---|---|
| A good answer is downgraded — `ε = 0.01` is within `1e-6` yet becomes `NotConverged` | Accepted and intended. `Converged` is a claim; a kernel that capped its projection cannot make it. The caller still has the iterate and both honest fields |
| Callers who treat `NotConverged` as failure see new failures on hard geometry | Real, and the reason this ships in a release whose notes say so plainly. The alternative is that they keep silently trusting wrong answers |
| Over-firing on an early cap | The rule reads the **final** projection only (§3) |

## 8. Exit criteria

1. Both constrained kernels gate `Converged` on the final projection not having
   capped, in the early-exit and `ConstantIteration` paths alike.
2. A unit test in each crate: a capped-projection solve reports `NotConverged` /
   `NoProgress`; an uncapped feasible control still reports `Converged`.
3. RFC 031's nearly-parallel adversarial fixtures declare the status they now
   receive; `status_match` returns to `26 passed / 0 failed`, while
   `solution_within_tolerance` **continues to record four failures**, which this
   RFC does not fix.
4. Smoke `24/24`, extended `6/6`, unit suites and `cargo xtask check` (17 gates)
   unchanged.
5. `TERMS_OF_USE.md` and both user guides state the three requirements of
   `Converged` together.
6. RFC 027 §0.5.4's claim that the status alone suffices for a batch caller is
   corrected to cite this RFC.
