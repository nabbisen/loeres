# Developer Handoff — RFC 031 adversarial and extended conformance suites

**Governing RFC.** `rfcs/accepted/031-adversarial-and-extended-conformance.md` (design frozen 2026-09-24).
**Assigned to.** Implementer tier.
**Ordering.** **First slice of Cycle 1**, before RFC 032. RFC 032's step rule is judged against the corpus this builds, and a finding here changes RFC 032's scope before it is written.

---

## 1. Purpose

Populate the two suites RFC 013 staged as placeholders, and report *how hard*
each fixture was, so RFC 027 §11.6's "nearly parallel constraints can make the
inner cap bind routinely — stated, not solved" becomes measured.

## 2. What already exists — do not rebuild it

- `conformance/extended/` and `conformance/adversarial/` exist with `README.md`.
- `--suite` parsing already works: `xtask/src/checks/conformance.rs` has
  `parse_suite`, and `Suite::Extended | Suite::Adversarial => run_placeholder_suite(suite)`.
  **The work is replacing that match arm with real execution**, not adding
  argument handling.
- The schema-3 fixture shape is fixed by the smoke corpus; copy its field set
  (`fixture_id`, `suite`, `problem_class`, `solver_family`, `dimension`,
  `constraints`, `scalar_family`, `validation_state`, `conformance_groups`,
  `variant`, `[config]`, `[problem]`, `[expected]`) and set `suite` to the new
  suite's name.

## 3. Slices

**S1 — reporting, then `extended/`.** **S2 — `adversarial/`.** One review request
each.

## 4. S1 — difficulty reporting and the extended suite

### 4.1 Reporting (do this first, on the smoke suite, so the shape is proven)

Per fixture and aggregated per suite, report the three numbers the kernels
already compute and the corpus currently discards: **`projection_cap_hits`**,
terminal **`max_constraint_violation`**, and **outer iterations executed** with
the configured cap beside them.

**This is reporting only.** A fixture passes or fails exactly as today. Do not
add a pass criterion, a threshold, or a budget on any of these — RFC 031 §4 is
explicit, and a threshold on an unmeasured quantity is how a flaky gate is built.

Prove the shape on smoke first: `cargo xtask conformance` must still print
`24 total / 24 passed / 0 failed` with the new figures beside each fixture.

### 4.2 `extended/`

Schema-3 fixtures above the smoke corpus's `n <= 3`, `m <= 3`. Cover at least
`n = 5` and `m = 5`, and on cluster at least one runtime dimension not used
anywhere in the smoke corpus.

**Expected values must come from an independent exact reference** — active-set
enumeration over the constraint rows *and* the box faces, the RFC 030 discipline.
**Never** from running a kernel and recording what it produced. A fixture whose
expected value was read off the kernel proves nothing and will be rejected.

## 5. S2 — the adversarial suite

One fixture family per property, each stating in its own comment which property
it stresses and the value that makes it adversarial:

1. **Nearly parallel constraint normals** — a family with the angle between
   normals decreasing across fixtures; record the smallest angle used.
2. **Degenerate boxes**, including at least one coordinate with `lo == hi`.
3. **Ill-conditioned `Q`** — a stated condition-number range.
4. **Barely feasible** — a feasible region of small but non-zero volume.
5. **Exactly-cancelling infeasible geometry** — RFC 027 §0.3.4 requires the
   corpus to keep this shape rather than avoid it.

## 5a. F1 — a box-interacting adversarial family (architect review 067)

*S1 and S2 are accepted. This is the one follow-up, and it closes a measured gap
rather than a suspected one.*

The architect injected the RFC 027 handoff's own prohibited shortcut — a plain
`clamp` in place of the box Dykstra increment — into the cluster kernel and ran
every subject in the tree:

| Subject | Result |
|---|---|
| smoke | **24/24 pass — misses it** |
| extended | **4 of 6 fail — catches it** |
| adversarial | 5 failed, unchanged — **misses it** |
| cluster unit tests, incl. RFC 030's randomized differential tests | **113/113 pass — misses it** |

Every one of those subjects except `extended` uses a box wide enough never to
interact with the constraint rows, so the box's Dykstra increment is never
exercised. That is a blind spot shared by RFC 030's mandatory differential tests.

**Add an adversarial family in which a box face and a constraint row are both
active at the optimum**, so the two sets genuinely interact. At least one fixture
where the unconstrained minimiser is cut off by a box face *and* a row, and at
least one where the box face binds only after the row has moved the iterate.
Expected values from the same independent reference, and record in each comment
which face and which row are active.

**Evidence:** the plain-`clamp` mutation applied to the cluster kernel, showing
the new family fails where the current adversarial suite passes.

## 6. Explicit non-change scope

- **Not one line under `crates/` outside tests.** No kernel change of any kind.
- **No new pass criterion and no threshold.**
- **Default `cargo xtask conformance` and `cargo xtask check` keep running smoke
  only** — 17 gates, 24/24, unchanged runtime and unchanged failure surface.
- No timing or throughput measurement; that is a separate unscheduled theme.
- No new dependency. No `#[allow(dead_code)]`.

## 7. Prohibited shortcuts

- **Deriving an expected value by running the kernel.** The single most important
  rule here.
- **Adjusting a fixture until it passes.** If an adversarial fixture fails, that
  is the RFC working: **stop and report it to the architect as a finding.** It
  may change RFC 032 before RFC 032 is written. Do not soften the fixture, widen
  its tolerance, or drop it.
- Enforcing the new suites in `cargo xtask check`.
- Choosing adversarial parameters so mild that nothing is stressed — record the
  extreme value used in each fixture's comment so the review can judge it.

## 8. Required evidence

fmt, clippy `-D warnings`, `cargo test --workspace --all-features`, MSRV 1.85,
`cargo xtask check` (17 gates, smoke 24/24), `mdbook build docs --dest-dir
target/xtask-book/local`, plus:

- `cargo xtask conformance --suite extended` and `--suite adversarial` output in
  full, including the per-fixture difficulty figures;
- for the adversarial suite, **the observed cap-hit rate**, which is the number
  RFC 032 will be judged against and the first measurement of §11.6's claim;
- a statement of how each expected value was derived.

## 9. Acceptance criteria

RFC 031 §6 items 1-6.

## 10. Review request

Standard eleven-item format; **record your author tier**; send to the architect.
Do not move RFC 031 to `done/` — it moves with the release that carries it.
