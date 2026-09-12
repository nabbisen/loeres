# RFC 023 - User-Facing Surface and Documented-Obligation Closure

**Status.** Accepted (design frozen 2026-07-31)
**Design approval.** Amendment 1 (§0, 2026-09-12) by architect review 045.  Author-performed adversarial review pass (see RFC 022 §17
on the role-separation compromise); project owner authorized the `accepted/`
transition on 2026-07-31.
**Tracks.** Unmet documented obligations identified in the 0.20.2 onboarding
review (register items I-7, I-8, I-9); requirements OQ-012.
**Touches.** `examples/` (new), `TERMS_OF_USE.md` (new), `docs/src/` user
guides and `SUMMARY.md`, `docs/specs/loeres-requirements-v1.md` §4.1 and §14,
and an `xtask` example-isolation check.

---

### Extended Metadata

* **Rust Edition Compliance:** Rust 2024; declared MSRV 1.85.0.
* **Target Environment:** Downstream-facing artifacts and host-side verification.
* **Proposed release:** Corrective/additive release after `0.20.2`; exact
  version and tag require project-owner approval.
* **Relationship to RFC 022:** Independent scope; may share a release vehicle.

## 0. Amendment 1 — 2026-09-12 (architect review 045)

**Isolation mechanism.** §11.3.1 named `cargo metadata`. The implementation uses
`cargo tree --locked --manifest-path <example> --edges normal,build,dev --prefix
none` and is accepted: it reads the same resolve, from the example's own lockfile,
over the whole set including build and dev edges, and it avoids adding a JSON
dependency to a repository whose supply-chain policy exists to keep the graph
small. `cargo metadata` remains acceptable if a JSON parser is ever justified;
the requirement is the resolved graph, not the command.

**Reconciled prose.** External design §1.1 forbade `std`/`alloc` to a device
example, conflicting with §11.2; it listed `cluster-dynamic-qp/` and
`device-static-workspace/`, which this RFC neither ships nor mentioned. §1.1 is
reconciled in the same change: the forbidden set is the five crates; a cluster
QP example moves to RFC 027 (Amendment 1); the static-workspace example is
removed as redundant with `device-box-pfo/`.

## 1. Summary

Three obligations stated in normative documents were never satisfied, and
`0.20.2` shipped without them:

| Obligation | Source | State |
|---|---|---|
| `examples/`, split by execution environment | requirements §4.1; external design §1.1; roadmap §5.9 release-readiness | Absent; no gate covers it |
| `TERMS_OF_USE.md` or equivalent safety-critical disclaimer | requirements §4.1, §10.1 | Absent; tracked as OQ-012 **Open** |
| `cluster-user-guide.md`, `device-user-guide.md`, `verification.md` | external design §1.1 | Absent |

None is a defect in shipped code. Each is a documented commitment the project
made and did not keep, and the release-readiness criterion in roadmap §5.9 was
passed over rather than met. RFC 023 closes all three.

A fourth gap is closed incidentally. `project-instructions-rust-cli.md` requires
documentation organized by persona — new users, intermediate users,
maintainers/contributors. The book currently has only the maintainer path.

## 2. Affected crates

No crate under `crates/` changes. New example crates are added outside the
workspace (§11.1). `xtask` gains one check.

## 3. Public API boundary impact

No runtime public API changes. Examples consume the existing public surface
only; if an example cannot be written against the public surface, that is a
finding to report, not a licence to widen the surface.

## 4. Dependency impact

No new workspace dependency. Each example declares only the crates its
execution environment permits.

## 5. `std` / `alloc` impact statement

Unchanged for library crates. Example binaries are host programs and may use
`std` in their own `main`; this does not alter the `no_std`, no-`alloc`
guarantee of `loeres`, `loeres-backend-static`, or `loeres-device`. §11.2 is
explicit about which claim an example does and does not establish.

## 6. Device determinism impact statement

None. No kernel, workspace, config, timing mode, iteration bound, or target
profile changes.

## 7. Cluster scalability impact statement

None. No orchestration, cancellation, budget, observability, gateway, or cache
behavior changes.

## 8. Error and diagnostic impact

None. Examples surface existing error and status categories; they must
demonstrate the status/error split correctly — non-convergence handled as an
`Ok` status, never as failure — because a wrong example teaches the wrong
contract.

## 9. Feature flag impact

No new feature. Examples pin the intended downstream feature sets from external
design §1.4: cluster with `parallel-rayon` plus `dense`; device with
`default-features = false` and `owned-arrays`.

## 10. Semver impact

None. Examples are excluded from the workspace and are not published.

## 11. Design

### 11.1 Examples are excluded from the workspace, deliberately

Each example is its own crate under `examples/` with its own lockfile, listed in
the root manifest's `[workspace] exclude`.

This is the load-bearing decision. If examples were workspace members they would
share one lockfile and participate in feature unification, so a device example
could resolve cluster-side crates or features through a sibling and still
compile cleanly. The isolation claim would then be an artifact of the build
graph rather than a property of the example. Excluding them makes each example's
dependency graph its own evidence.

The `xtask` check therefore does two things per example: build it under its
declared feature set, and assert its resolved dependency graph contains no
forbidden crate. For the device example the forbidden set is `loeres-cluster`,
`loeres-backend-std`, `tokio`, `rayon`, and `tracing` — matching external design
§1.1. Compilation alone is not accepted as evidence.

### 11.2 What an example proves, and what it does not

A host-built device example establishes **dependency reachability** — that a
device integrator can solve a bounded problem without any server crate entering
the graph. It does **not** establish bare-metal buildability; that claim belongs
to the existing `no-std` gate against `thumbv7em-none-eabihf`, and the two must
not be conflated in prose.

A genuinely bare-metal example would require `#![no_main]`, a panic handler, and
a linker configuration — board-support concerns that would pull target-specific
machinery into the repository without strengthening the isolation claim. The
existing gate already proves the crates build for the target.

### 11.3 Scope of the examples

Two, matching the layout in requirements §4.1:

- `examples/cluster-batch-solve/` — construct a dynamic projected-first-order
  problem, run `solve_batch`, show per-item outcomes including a non-converged
  item handled as a status.
- `examples/device-box-pfo/` — construct a fixed-size box-constrained problem
  and a caller-owned typed workspace, call `solve_projected_first_order`, show
  workspace reuse across calls.

**The device example is renamed, and requirements §4.1 is amended to match.** The
requirement names the path `device-fixed-qp/`, but no QP contract ships;
OQ-001 was later resolved to a bounded box projected-first-order family. The
requirement's name predates that resolution and is stale in exactly the way the
v0.6.3 apex prose was.

Preserving it would put a claim the project does not support into a
user-facing directory name — in a project whose stated discipline is not
overclaiming, and where a reader browsing `examples/` sees the directory before
any prose that corrects it. RFC 020's conflict rule applies: reconcile stale
prose to the later resolved scope rather than propagate it. The rename and the
§4.1 amendment land atomically.

### 11.3.1 How the isolation assertion is implemented

The check runs `cargo metadata` for each example against its own lockfile and
scans the resolved package set for forbidden names. It does not parse manifests,
because a manifest shows declared dependencies while the resolved graph shows
what is actually reachable — including anything arriving transitively or through
feature activation. A forbidden crate anywhere in the resolved set fails the
gate.

### 11.4 Engineering-use terms

`TERMS_OF_USE.md` records what the project does and does not establish: no
safety certification, panic-averse rather than panic-free, target-scoped rather
than universal determinism, one narrow solver family, bounded smoke conformance,
metadata-only observability without multi-tenant isolation evidence, mock-only
gateway, process-local cache, pre-1.0 instability, and integrator
responsibility. It complements the Apache-2.0 warranty disclaimer rather than
restating or modifying it.

Every limitation it states must already be stated elsewhere in the project. The
document is a consolidation for integrators, not a new set of claims, and it
must not become the only place a limitation is recorded.

This resolves **OQ-012**, which requires the corresponding requirements §14 row
to move from Open to Resolved in the same change.

### 11.5 Book pages and personas

Three pages are added — `cluster-user-guide.md`, `device-user-guide.md`,
`verification.md` — and `SUMMARY.md` gains a user-facing section ahead of the
existing maintainer section. The two guides are the prose home for the two
examples; `verification.md` summarizes the existing gate set and its evidence
classes without introducing new claims.

## 12. Rejected alternatives

| Alternative | Reason rejected |
|---|---|
| Examples as workspace members | Shared lockfile and feature unification would let a device example resolve server crates through a sibling; the isolation claim would be unfalsifiable |
| Compile-only example gate | Compilation does not prove absence of a forbidden dependency edge |
| A true bare-metal device example | Requires a panic handler, `#![no_main]`, and linker configuration; adds board-support surface without strengthening the claim the `no-std` gate already covers |
| Amend external design §1.1 to drop the missing book pages | Weakens a specification to fit an omission; three short pages cost less than the precedent |
| Fold this into RFC 022 | Unrelated theme; RFC template discipline requires independently reviewable units |
| Defer all three to R3 | R3 is assurance expansion, not obligation closure. Deferring commitments already made is how they became invisible |

## 13. Verification gates

1. `cargo xtask check` — including the new per-example build and
   forbidden-dependency assertion.
2. Focused tests: an example gaining a forbidden dependency fails the gate; an
   example failing to build fails the gate; an absent example directory fails
   rather than silently passing.
3. `cargo xtask check-rfcs`, `doc-currency`, and `link-audit` — OQ-012 status
   transition, `SUMMARY.md` navigation, and all relative links.
4. `cargo fmt --all -- --check`; all-target/all-feature Clippy with warnings
   denied; all-feature workspace tests and doc-tests.
5. Exact Rust 1.85 all-feature workspace check, including each example.
6. `mdbook build docs`.
7. RFC 019's release-candidate gate at closeout.

## 14. Implementation sprint plan

| Sprint | Work | Exit |
|---|---|---|
| S0 | Design freeze: exclusion model, isolation-assertion contract, example scope | Architecture acceptance of this RFC |
| S1 | Add `TERMS_OF_USE.md`; flip requirements OQ-012 to Resolved atomically | Apex reconciliation reviewed |
| S2 | Add the two excluded example crates | Both build under declared feature sets |
| S3 | Implement the example build and forbidden-dependency check in `xtask` | Focused tests pass |
| S4 | Add the three book pages and the user-facing `SUMMARY.md` section | `mdbook build` passes; personas covered |
| S5 | Full gate suite | All gates pass |
| S6 | Changelog and roadmap entries; closeout | Evidence attached; RFC moved on release |

## 15. Dependencies and interactions

RFC 022 owns retirement of the RFC 021 conditional-finalization prose (its §15).
If RFC 023 ships first or alone, that obligation transfers to whichever change
is release-bearing; it must not be dropped, duplicated, or performed twice.

The `0.20.2` artifact is never modified retroactively.

## 16. Exit criteria

RFC 023 is complete only when:

1. both examples exist, build under their declared feature sets, and are
   excluded from the workspace with their own lockfiles;
2. the gate asserts each example's dependency graph is free of forbidden
   crates, and fails closed when an example is missing;
3. `TERMS_OF_USE.md` exists and states no limitation not already recorded
   elsewhere in the project;
4. requirements OQ-012 reads Resolved, reconciled atomically with §4.1 and
   §10.1, and §4.1's example path is amended to the shipped family name;
5. the three book pages exist and `SUMMARY.md` covers the user persona;
6. the device example demonstrably reaches no server crate;
7. no runtime API, behavior, feature, or dependency boundary changes; and
8. roadmap §5.9's example criterion is satisfied rather than waived.
