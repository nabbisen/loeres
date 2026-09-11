# Developer Handoff — RFC 023 user-facing surface and obligation closure

**Governing RFC.** `rfcs/accepted/023-user-facing-surface-and-obligation-closure.md`.
**Status.** Inherited from RFC 023 (Accepted). Architect review recorded;
owner approved implementation 2026-09-12.
**Assigned to.** Implementer tier.
**Prerequisites.** RFC 024 (`f63970f`) and RFC 022 (`4c3c2f2`+) landed. No
dependency on RFC 022 Amendment 3 or RFC 024 Amendment 2 — those may proceed in
parallel.

---

## 1. Purpose

Three obligations stated in normative documents were never met, and `0.20.2`
shipped without them: `examples/` split by execution environment
(requirements §4.1, external design §1.1, roadmap §5.9 release-readiness
gate); `TERMS_OF_USE.md` or equivalent (requirements §4.1/§10.1, OQ-012 open);
and the `cluster-user-guide` / `device-user-guide` / `verification` book pages
(external design §1.1). Close all three, and close the persona gap: the book has
only a maintainer path.

## 2. Applicable requirements

RFC 023 §11 (design), §13 (gates), §16 (exit criteria); requirements §4.1,
§10.1, §14 OQ-012; external design §1.1, §1.4 (downstream feature sets);
`project-instructions-rust-cli.md` documentation personas.

## 3. Change scope

### 3.1 Examples — **excluded from the workspace** (§11.1, load-bearing)

Two crates under `examples/`, each with its own `Cargo.lock`, listed in the
root manifest's `[workspace] exclude`. Not members. If they were members they
would share a lockfile and feature unification, so a device example could
resolve cluster crates through a sibling and still compile; the isolation claim
would be a build-graph artifact. Excluded, each graph is its own evidence.

| Crate | Features (external design §1.4) | Demonstrates |
|---|---|---|
| `examples/cluster-batch-solve/` | `loeres-cluster` with `parallel-rayon`; `loeres-backend-std` with `dense` | Build a dynamic PFO problem, `solve_batch`, per-item outcomes **including a non-converged item handled as an `Ok` status** |
| `examples/device-box-pfo/` | `loeres-device` `default-features = false`, `owned-arrays`; `loeres-backend-static` likewise | Fixed-size box-constrained problem, caller-owned typed workspace, `solve_projected_first_order`, **workspace reuse across calls** |

Examples are host programs and may use `std` in their own `main`. That does not
alter the edge crates' `no_std` guarantee, and the prose must not claim it does.

**The device example is `device-box-pfo`, not `device-fixed-qp`.** No QP
contract ships; OQ-001 resolved the family to bounded box PFO. Requirements
§4.1's path is amended **atomically** in the same change (§11.3). Do not ship a
directory name that claims a family the project does not have.

### 3.2 Isolation assertion — `xtask` (§11.3.1)

Per example: build under its declared feature set, then run `cargo metadata`
against **its own lockfile** and scan the **resolved package set** for
forbidden names. Do not parse manifests — a manifest shows declared deps, the
resolved graph shows what is reachable, including transitively and via feature
activation. Device forbidden set: `loeres-cluster`, `loeres-backend-std`,
`tokio`, `rayon`, `tracing`. Compilation alone is not evidence. An absent
example directory **fails**, never passes vacuously. Fold into `cargo xtask
check`.

What this proves: **dependency reachability**. What it does not prove:
bare-metal buildability — that is the existing `no-std` gate's claim. Keep the
two apart in every sentence you write.

### 3.3 `TERMS_OF_USE.md` (§11.4)

**A reviewed draft exists, untracked, at repo root.** Use it. Its rule: every
limitation it states must already be stated elsewhere in the project — it
consolidates for integrators, it introduces nothing. It complements the
Apache-2.0 warranty disclaimer; it does not restate or modify it. Land it with
the requirements §14 **OQ-012** row flipped to Resolved in the same change.

### 3.4 Book pages and personas (§11.5)

Add `docs/src/cluster-user-guide.md`, `docs/src/device-user-guide.md`,
`docs/src/verification.md`. `SUMMARY.md` gains a user-facing section **before**
the maintainer section. The guides are the prose home for the two examples;
`verification.md` summarizes the existing gate set and its evidence classes
(enforced / advisory / documented-only) **without introducing new claims**.

## 4. Explicit non-change scope

No crate under `crates/`. No public API widening — if an example cannot be
written against the public surface, **that is a finding to report**, not a
reason to add a `pub`. No `.github/workflows/`. No `0.20.2` artifact, no
`0.20.1` tag. No renaming beyond §3.1. Do not make the examples workspace
members "for convenience."

## 5. Required tests

- each example builds under its declared feature set;
- an example that gains a forbidden dependency fails the gate;
- an absent example directory fails;
- the device example's resolved graph is demonstrably free of the forbidden set
  (assert on the real graph, not a fixture only);
- `check-rfcs` / `doc-currency` / `link-audit` accept the OQ-012 transition and
  `SUMMARY.md` change.

## 6. Required evidence

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-features --all-targets -- -D warnings
cargo test --workspace --all-features
cargo +1.85.0 check --workspace --all-features      # and each example crate
cargo xtask check                                   # with the example gate
mdbook build docs
```

Plus the resolved-graph output for the device example, observed. `release-gate`
not required (no release surface); `docs/book/` must be absent if you run it.

## 7. Prohibited shortcuts

- Do not scope the isolation check to names you already know are absent —
  scan the full resolved set.
- Do not claim bare-metal buildability from a host build.
- Do not widen the public API to make an example compile.
- Do not add a limitation to `TERMS_OF_USE.md` that is stated nowhere else.
- Do not leave `docs/book/` output, and do not `#[allow(dead_code)]` anything —
  remove it (owner rule, 2026-09-12).

## 8. Known risks

| Risk | Mitigation |
|---|---|
| Excluded crates need their own lockfiles kept current | Gate builds them; MSRV check covers them |
| Example needs a `pub` that isn't there | Report it; that is an API finding for the architect |
| Guides drift from examples | Guides reference the example paths; link-audit covers them |

## 9. Acceptance criteria

RFC 023 §16, all eight items.

## 10. Review request

Standard eleven-item format; **record your author tier**. Send to the
architect. Do not move RFC 023 to `done/` — it moves with the release.
