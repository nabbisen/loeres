# Developer Handoff — RFC 033 an exact projection as part of the `Converged` claim

**Governing RFC.** `rfcs/accepted/033-exact-projection-in-the-converged-claim.md` (design frozen 2026-09-24).
**Assigned to.** Implementer tier.
**Ordering.** Cycle 1, after RFC 031 (landed, accepted) and alongside RFC 032. It must land **before** the `0.21.3` cut, because it changes a reported status and the release notes describe that change.

---

## 1. Purpose

A kernel that ran its inner projection to `projection_max_sweeps` without
converging cannot claim the returned point is the projection. Today it does.

## 2. The rule

Report `Converged` only when the **final** outer iteration's projection returned
**without** hitting `projection_max_sweeps`. Otherwise
`SolveReport::not_converged_stalled(executed)` — `NotConverged` with
`NoProgress`, the shape RFC 027 Amendment 5 already uses.

**The final projection, not any projection.** An early outer iteration may cap
legitimately while the iterate is far from the optimum, with later iterations
converging cleanly. `projection_cap_hits` keeps counting all of them and is
**unchanged**.

## 3. Sites

`dykstra_project` already returns whether it capped; the kernels currently use
that only to increment the counter. Keep the last value.

**Device** — `crates/loeres-device/src/solve/constrained.rs`: the early-exit
return **and** the `ConstantIteration` post-loop return. Both currently gate on
the Amendment 5 feasibility check; add the new condition to both.

**Cluster** — `crates/loeres-cluster/src/solve/constrained.rs`: the early-exit
return. Its post-loop return is already `not_converged_cap` and needs no gate.

The architect's prototype, for shape only — re-derive it rather than pasting:

```rust
let mut last_capped = false;
// ...
last_capped = dykstra_project(problem, x, workspace, config, ctx, m)?;
if last_capped { projection_cap_hits += 1; }
// ...
let report = if violation.lte_tolerance(config.projection_tolerance) && !last_capped {
    SolveReport::converged_early(executed)
} else {
    SolveReport::not_converged_stalled(executed)
};
```

**Do not touch** the RFC 006/016 box kernels: they have no inner projection.

## 4. Measured effect — expect exactly this

The architect prototyped on the cluster kernel before the RFC was written:

```text
eps=0.02 -> Converged (unchanged)          eps<=0.01 -> NotConverged
easy feasible control -> Converged (unchanged)

smoke      24/24 pass   (unchanged)
extended    6/6  pass   (unchanged)
cluster unit tests 113/113 pass (unchanged)
adversarial 21/26 pass  (unchanged count); status_match moves 26/0 -> 21/5
```

If your numbers differ from these, **stop and report** rather than adjusting
anything — a difference means the rule was implemented differently than measured.

## 5. Fixture expectations (RFC 031 corpus)

`status_match` moving to `21 passed / 5 failed` is the rule working: the
nearly-parallel fixtures declare `converged` and now receive `NotConverged`.
Update those five fixtures' `[expected] status` and `termination` to what the
corrected kernels produce, so `status_match` returns to `26 passed / 0 failed`.

**`solution_within_tolerance` must still record four failures afterwards.** This
RFC does not improve any answer — the kernel returns the same point and merely
stops claiming it is the projection. **Do not touch those four expected
solutions, their tolerances, or their caps.** If they start passing, something
is wrong and it is a finding, not a success.

## 6. Documentation

`TERMS_OF_USE.md` and both user guides: state the **three** requirements of
`Converged` together — feasible (Amendment 5), stationary at the final iteration
(RFC 029), and produced by an uncapped projection (this RFC).

**Do not edit `rfcs/done/027-*.md`.** RFC 027 §0.5.4's over-strong claim about the
batch seam is superseded by RFC 033 §6, not corrected in place: RFC 027 is in
`done/` and RFC 025 §11 forbids amending a `done/` RFC. The correction reaches
readers through RFC 033, the index row, and the surfaces above.

`CHANGELOG.md`: an RFC 033 subsection under the unreleased `0.21.3` record,
stating plainly that a status some callers currently see as `Converged` will
become `NotConverged` on hard geometry, and that no answer changes.

## 7. Explicit non-change scope

- No change to `projection_cap_hits`, `max_constraint_violation`, or the inner
  stopping rule.
- No change to any answer, any signature, any feature, or any dependency.
- No change to the RFC 006/016 box kernels.
- No `#[allow(dead_code)]`.

## 8. Prohibited shortcuts

- **Gating on `projection_cap_hits > 0` instead of the final projection.** That
  would downgrade a solve whose early iterations capped and whose final one did
  not — over-firing, which §2 exists to prevent.
- Loosening any RFC 031 expected solution or tolerance to make a fixture pass.
- Editing `rfcs/done/027-*.md`.
- Reporting the architect's prototype figures as your own measurement.

## 9. Required evidence

fmt, clippy `-D warnings`, `cargo test --workspace --all-features`, MSRV 1.85,
`cargo xtask check` (17 gates), `cargo xtask conformance` (smoke 24/24),
`--suite extended` (6/6), `--suite adversarial` (status_match 26/0,
`solution_within_tolerance` still 4 failures), `mdbook build docs`, and the
`thumbv7em-none-eabihf` build with `owned-arrays`.

Plus a mutation: gate on `projection_cap_hits > 0` instead, and show it
downgrades a solve the correct rule leaves `Converged`.

## 10. Acceptance criteria

RFC 033 §8 items 1-6.

## 11. Review request

Standard eleven-item format; **record your author tier**; send to the architect.
Do not move RFC 033 to `done/` — it moves with the release that carries it.
