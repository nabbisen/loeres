# RFC 031 - Adversarial and Extended Conformance Suites

**Status.** Implemented (v0.21.3).
**Design approval.** Architect-authored and scheduled in architect review 066 (Cycle 1); project owner authorized the cycle on 2026-09-24.
**Tracks.** R3 assurance expansion — the "wider numerical conformance" candidate, second bounded slice after RFC 026. Extends RFC 013; amends nothing.
**Touches.** `conformance/extended/`, `conformance/adversarial/`, `xtask/src/checks/conformance*`, `conformance/README.md`.

---

## 1. Summary

Populate the two suites RFC 013 staged as placeholders, and report **how hard**
each fixture was rather than only whether it passed. The adversarial suite is
built from the geometries RFC 027 §11.6 names as hard but does not measure.

## 2. Why now

RFC 027 §11.6 says the projection "converges linearly at a rate set by the angles
between constraint normals, and nearly parallel constraints can make the inner
cap bind routinely. **Stated, not solved.**" That sentence ships in
`TERMS_OF_USE.md`. Nothing in the tree measures it: every smoke fixture is
well-conditioned, and the corpus reports pass/fail only, so a fixture that
converges after one sweep and one that scrapes in at the cap are indistinguishable
in the output.

RFC 032 (Cycle 1) proposes a step-size rule. The corpus that would judge such a
rule is exactly the one this RFC builds, which is why the two share a release.

## 3. Design

### 3.1 `extended/` — same geometry, larger sizes

Schema-3 fixtures at dimensions and constraint counts above the smoke corpus's
`n ≤ 3`, `m ≤ 3`, with closed-form optima where one exists and a stated
tolerance where one does not. Purpose: catch defects that appear only at size —
runtime-length handling on cluster, and workspace shape on device.

### 3.2 `adversarial/` — the geometries §11.6 names

Each fixture carries, in its own comment, which property it stresses:

- **Nearly parallel constraint normals** (angles down to a stated minimum), the
  case §11.6 says makes the cap bind.
- **Degenerate boxes**, including zero-width coordinates (`loᵢ == hiᵢ`).
- **Ill-conditioned `Q`** across a stated condition-number range.
- **Barely-feasible constraint sets** — a feasible region of small but non-zero
  volume.
- **Exactly-cancelling infeasible geometry**, which RFC 027 §0.3.4 requires the
  corpus to keep rather than avoid.

### 3.3 Reported difficulty, not only pass/fail

The runner gains, per fixture and per suite, the numbers the kernels already
compute and currently discard at the corpus boundary: **`projection_cap_hits`**,
**terminal `max_constraint_violation`**, and **outer iterations executed**
against the cap.

This is reporting, not a new pass criterion: a fixture passes or fails exactly as
it does today. The figures make §11.6's claim observable, and give RFC 032 a
baseline to be judged against.

### 3.4 Suite selection

`cargo xtask conformance --suite extended` and `--suite adversarial` run the new
suites; the default and `cargo xtask check` continue to run **smoke only**, so
neither the gate's runtime nor its failure surface changes in this RFC.

## 4. Explicit non-scope

- **No kernel change.** Not one line under `crates/` outside tests.
- **No new pass criterion**, and no threshold on cap-hit rates. Measuring first
  is the point; setting budgets on an unmeasured quantity is how a flaky gate
  gets built.
- **No timing or throughput** — that is T4, unscheduled.
- **Not enforced in `cargo xtask check`.** If the extended and adversarial suites
  later earn enforcement, that is a separate decision on evidence.

## 5. Risks

| Risk | Mitigation |
|---|---|
| An adversarial fixture fails because the kernel is genuinely weak there | That is the RFC working. A failure is reported to the architect, not designed around — and it changes RFC 032's scope before RFC 032 is written (review 066 §3) |
| Fixtures encode current behaviour as correct | Expected values come from an independent exact reference, the RFC 030 discipline, never from running the kernel |
| Suite runtime grows without bound | Suites stay outside the default gate; dimensions stated per fixture |

## 6. Exit criteria

1. `extended/` and `adversarial/` each contain fixtures meeting §3.1/§3.2, with
   every expected value derived independently of the kernels.
2. The runner reports cap hits, terminal violation, and iterations per fixture
   and aggregated per suite.
3. `--suite extended` and `--suite adversarial` run; the default remains smoke.
4. `conformance/README.md` describes both suites and states that they are
   reported, not enforced.
5. `cargo xtask check` unchanged: 17 gates, smoke 24/24.
6. Any fixture that fails is reported to the architect as a finding, not adjusted
   to pass.
