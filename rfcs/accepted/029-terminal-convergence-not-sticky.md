# RFC 029 - Terminal Convergence, Not Sticky Convergence

**Status.** Accepted (design frozen 2026-09-24)
**Design approval.** Project owner accepted it and authorized the `accepted/` transition on 2026-09-24. Architect-authored from RFC 027 S4 C1-C3 review (architect review 058 §5.3), on an observation the implementer raised rather than buried.
**Tracks.** Correctness of reported status, the principle RFC 027 Amendment 5 (§0.5) established for feasibility, applied to stationarity.
**Touches.** `crates/loeres-device/src/solve.rs` (RFC 006 kernel), `crates/loeres-device/src/solve/constrained.rs` (RFC 027 kernel). Cluster is unaffected: it has no `ConstantIteration` mode.

---

## 1. Summary

Under `TimingMode::ConstantIteration`, both device kernels set a `converged`
flag the first time an outer step is within tolerance and **never clear it**.
The loop then runs to `max_iterations` regardless. At the end the flag - not the
final iterate - decides whether the report says `Converged`.

So `Converged` can be returned for an iterate that was momentarily stationary
hundreds of iterations earlier and is, at the point of return, moving fast and
far away.

## 2. Reproduced

RFC 027's device constrained kernel, `ConstantIteration`, `Q = I`, `c = 0`,
`step_scale = 2.1` (so `x_{k+1} = -1.1 x_k`, divergent), started at
`x = (1e-13, 0)` so iteration 1's change is `2.1e-13`, under a `tolerance` of
`1e-12`. Box and constraints stay slack throughout:

```text
status = Converged   termination = IterationCap   iterations = 400
projection_cap_hits = 0   max_constraint_violation = 0
final x = (3606.401402752558, 0)
```

The final iterate moved by roughly `340` on its last step. It is **feasible**,
so RFC 027 Amendment 5's gate correctly does not fire: what is false here is
*stationarity*, not feasibility. Amendment 5 does not cover this and was never
intended to.

The same flag shape is in the RFC 006 kernel (`crates/loeres-device/src/solve.rs`:
`let mut converged = false;` set inside the loop, read after it), so the defect is
shared and pre-existing; RFC 027 inherited it by faithfully mirroring RFC 006's
outer loop, exactly as its handoff required.

## 3. Why this matters

RFC 027 Amendment 5 established the principle that a reported status must be a
true statement about the returned iterate. This is the same principle applied to
the other half of the claim. `Converged` asserts two things - the iterate is
stationary, and (for a constrained solve) it is feasible. Amendment 5 made the
second true. The first is still evaluated at the wrong iterate.

`ConstantIteration` exists so that execution time does not depend on the data
(RFC 005/006). That makes it the mode most likely to be used where a wrong
answer is least acceptable, and the mode where the loop is *guaranteed* to keep
running after the flag is set.

## 4. Reachability

Requires `ConstantIteration`, plus an outer step sequence whose change is
non-monotone or divergent - which a badly chosen `step_scale` produces, and
which the kernels do not and cannot reject in general (the Lipschitz constant is
not known to them). It is not an everyday path. It is also not an exotic one:
the trigger is a step size that is too large, the most common way to misconfigure
a first-order method.

## 5. Proposal

**5.1** Under `ConstantIteration`, evaluate the convergence criterion on the
**final** outer iteration rather than reading a sticky flag. The report is
`converged_at_cap` only when the last step's change is within `tolerance` (and,
for the constrained kernel, the Amendment 5 feasibility gate also holds);
otherwise `not_converged_cap`.

**5.2** `EarlyExitAllowed` is unchanged. It returns at the moment the criterion
holds, so its claim is already about the returned iterate.

**5.3** The change is confined to what the two kernels report. No signature, no
workspace, no arithmetic, and no iteration count changes; `ConstantIteration`
still runs exactly `max_iterations` steps, so the timing property is preserved.

**5.4** RFC 006's `converged_at_cap` documentation is corrected to say the
criterion is evaluated at the final iteration.

## 6. Alternatives considered

**Leave it and document it.** Rejected: "`Converged` may refer to an earlier
iterate" is not a caveat a caller can act on, and it contradicts the principle
just established by Amendment 5.

**Clear the flag when a later step exceeds tolerance.** Equivalent in outcome to
5.1 but keeps a mutable flag whose meaning is a history rather than a fact.
5.1 is simpler and states the property directly.

**Fix only RFC 027's kernel.** Rejected: it would leave RFC 006 and RFC 027
disagreeing about what `Converged` means in the same timing mode, which is worse
than either behaviour alone.

## 7. Compatibility

A reported status changes only in cases where it was false. No public type,
signature, or feature changes. RFC 006 is `done/`; this RFC is the vehicle for
amending its behaviour, per RFC 000.

## 8. Exit criteria

1. Both device kernels evaluate the criterion at the final iteration under
   `ConstantIteration`.
2. A test in each kernel pins §2's configuration reporting `NotConverged`.
3. A test pins that a genuinely converged `ConstantIteration` run still reports
   `converged_at_cap` with `IterationCap`.
4. RFC 006's documentation corrected per §5.4.
5. `cargo xtask check` 16 gates and `cargo xtask conformance` unchanged.
