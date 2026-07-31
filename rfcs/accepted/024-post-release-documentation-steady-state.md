# RFC 024 - Post-Release Documentation Steady State

**Status.** Accepted (design frozen 2026-07-31)
**Design approval.** Author-performed adversarial review pass (see RFC 022 §17 on the role-separation compromise); project owner authorized the `accepted/` transition on 2026-07-31.
**Tracks.** Blocker discovered during RFC 022 implementation: the documentation
currency checker cannot represent the state of the repository after a release.
Blocks RFC 022 §15/§16 and register item I-6.
**Touches.** `xtask/src/checks/doc_currency.rs`,
`xtask/src/checks/conditional_finalization.rs`,
`xtask/src/checks/release_gate.rs`, `release/conditional-finalization.toml`,
the apex trio's shared currency block, and the RFC 021 conditional prose
surfaces.

---

### Extended Metadata

* **Rust Edition Compliance:** Rust 2024; declared MSRV 1.85.0.
* **Target Environment:** Host-side governance tooling and normative documentation.
* **Proposed release:** Same corrective vehicle as RFC 022 unless review separates them.
* **Prior art:** RFC 020 (currency checks), RFC 021 (conditional finalization).

## 1. Summary

`cargo xtask doc-currency` admits exactly two apex-document states, and **neither
describes the repository after a successful release**:

| Mode | Trigger | What it requires |
|---|---|---|
| Conditional | `release/conditional-finalization.toml` present | Apex block byte-identical to a canonical block generated from the metadata; release marker forced to `v{release_version}`, pinned to `0.20.2` by the compile-time constant `EXPECTED_RELEASE_VERSION` |
| Plain | metadata absent | Apex header must contain `reconciliation draft (not yet the current marker)`, plus `Implemented scope: **RFCs 001-018**`, `Accepted recovery work: **RFC 019 and RFC 020**`, `this work is unshipped and in progress`, and `Activation as the current marker is pending` |

The conditional mode encodes the `0.20.2` candidate. The plain mode encodes the
pre-`0.20.2` RFC 020 draft, whose assertions — RFCs 019/020 accepted and
unshipped, scope ending at 018 — became false when `0.20.2` shipped.

The consequence is concrete and currently blocking: **the project cannot make an
ordinary commit after a release without failing its own gate.** Bumping the
workspace version to `0.20.3` produces

```
APEX RELEASE: docs/specs/loeres-requirements-v1.md has `v0.20.2`, expected workspace `v0.20.3`
```

for all three apex documents, and no documentation edit can fix it while the
metadata exists, because the conditional block is generated from a pinned
constant. Deleting the metadata does not help: it reverts the assertions to
statements the project knows are untrue.

The recovery tooling was built for the recovery. It has no steady state.

## 2. Affected crates

None under `crates/`. Host-only `xtask` changes and normative documentation.

## 3. Public API boundary impact

No runtime public API changes. The `xtask` command surface is unchanged;
`doc-currency` gains a third recognized apex form and the intended-tag preflight
is re-expressed against it.

## 4. Dependency impact

None.

## 5. `std` / `alloc` impact statement

None. Edge `no_std`, no-`alloc` boundaries are untouched.

## 6. Device determinism impact statement

None.

## 7. Cluster scalability impact statement

None.

## 8. Error and diagnostic impact

No runtime error or diagnostic change. Gate failures gain a message that names
which apex field disagrees with which source.

## 9. Feature flag impact

None.

## 10. Semver impact

None for published crate contracts. Patch-level corrective governance.

## 11. Design

### 11.1 The apex block carries two distinct versions

The root defect is that one field, "the release," has been asked to mean both
*what shipped* and *what this tree is*. RFC 021 already proved those must not be
conflated — that conflation was blocker B6 — but it fixed the conflation only
for the duration of one finalization window.

The ordinary apex block therefore carries both, explicitly:

```text
Last released repository release: **0.20.2**
This tree: **0.20.3** (unreleased)
Implemented scope: **RFCs 001-021**
```

`doc-currency` asserts that *this tree* equals the workspace version, that *last
released* is less than or equal to it, that the implemented scope matches the
RFC lifecycle state, and that the block is identical across all three apex
documents.

In a release's finalization revision the two versions become equal, which is
what `F` already does under RFC 021 §7. The release protocol does not change;
it becomes the special case of a rule that also holds between releases.

### 11.2 What this does and does not generalize

RFC 021 §3 excludes "generalizing conditional lifecycle state beyond release
finalization," and that exclusion is respected. This RFC does **not** make the
conditional mechanism permanent, re-target it per release, or introduce a
conditional lifecycle for ordinary work.

It generalizes the *lesson* — never let the tree claim a release it has not had —
into a plain two-field statement that needs no bespoke machinery. Between
releases the tree says plainly that it is unreleased. That is a weaker,
cheaper, always-true mechanism, and it removes the reason to reach for the
conditional apparatus again.

### 11.3 Retiring the one-release apparatus

`release/conditional-finalization.toml` is removed, and with it the requirement
that it be present. The `conditional_finalization` module and its tests are
retained as the implemented record of RFC 021, but nothing in the steady-state
gate path requires it.

The intended-tag preflight in `release_gate` currently hard-fails when the
metadata is absent. It is re-expressed against the ordinary apex form: the
intended tag must equal the workspace version, must equal *this tree*, must be
absent locally and on the authoritative remote, and must exceed *last released*.
Every property RFC 021 §7 required of `L` is preserved; only its source changes.

### 11.4 Scope binding

`Implemented scope` is checked against the RFC lifecycle rather than a literal.
Hard-coding `RFCs 001-018` is precisely why the plain mode expired. The scope
field must be derived from what is in `done/`, so it cannot silently rot again.

## 12. Rejected alternatives

| Alternative | Reason rejected |
|---|---|
| Re-target the conditional metadata at each release | Makes a one-release correction mechanism permanent; excluded by RFC 021 §3; the "process becomes the product" failure the 2026-07-17 review warned about |
| Keep the workspace version at the last released value indefinitely | The I-6 hazard: every commit then claims to be the release. The mirror image of this was flagged as a live foot-gun when the manifest sat at the blocked `0.20.1` |
| Make `doc-currency` advisory instead of fail-closed | Weakens an enforced governance check to make a defect disappear; roadmap §1.5 forbids later work loosening earlier constraints |
| Record only the last released version in the apex block | Then nothing binds the apex to the working tree, and apex drift — audit blocker B4 — returns |
| Amend Accepted RFC 022 to absorb this | RFC 022 is design-frozen and scoped to evidence identity. RFC 021 set the precedent: when execution reveals a sequencing defect, create a focused RFC rather than widen a frozen one |
| Edit the `0.20.2` artifact to fit the new form | Forbidden. The released artifact is never modified retroactively |

## 13. Verification gates

1. `cargo xtask doc-currency` — passing on a post-release tree where *this tree*
   exceeds *last released*.
2. Focused tests: apex/workspace version disagreement fails; *last released*
   exceeding *this tree* fails; apex blocks differing across the trio fails;
   implemented scope disagreeing with `done/` fails; absent apex block fails
   rather than passing vacuously.
3. Intended-tag preflight tests: tag equal to workspace version and exceeding
   last released passes; tag not exceeding last released fails; existing local
   or remote tag fails; all without the conditional metadata present.
4. `cargo xtask check`, `check-rfcs`, `link-audit`.
5. `cargo fmt --all -- --check`; all-target/all-feature Clippy with warnings
   denied; all-feature tests and doc-tests.
6. Exact Rust 1.85 all-feature workspace check.
7. `mdbook build docs`.

## 14. Implementation sprint plan

| Sprint | Work | Exit |
|---|---|---|
| S0 | Design freeze: two-field apex form, scope derivation, preflight re-expression | Architecture acceptance |
| S1 | Implement the ordinary apex parser and its assertions | Focused tests pass |
| S2 | Derive implemented scope from the RFC lifecycle | Scope cannot be hard-coded |
| S3 | Re-express the intended-tag preflight; drop the metadata requirement | Preflight tests pass without metadata |
| S4 | Rewrite the apex trio and the RFC 021 conditional prose surfaces to the ordinary form; remove the metadata file | `doc-currency` green at `0.20.3` |
| S5 | Full gate suite | All gates pass |
| S6 | Changelog, roadmap, closeout | Evidence attached |

## 15. Dependencies and interactions

This RFC **blocks** RFC 022 §15/§16 and register item I-6. RFC 022's
prose-retirement obligation transfers here, since retirement turns out to be a
tooling contract change rather than an edit; RFC 022 §16 criterion 6 is
satisfied by this RFC completing.

The `0.20.2` artifact is not modified. Its conditional block remains correct for
the tree it describes; the ordinary form applies from the next release forward.

## 16. Exit criteria

RFC 024 is complete only when:

1. `doc-currency` recognizes an ordinary post-release apex form and passes on a
   tree whose version exceeds the last released version;
2. the apex block records last-released and this-tree versions distinctly, and
   both are asserted;
3. implemented scope is derived from the RFC lifecycle, not hard-coded;
4. the intended-tag preflight preserves every RFC 021 §7 property without
   requiring the conditional metadata;
5. the conditional metadata file is removed and no steady-state check requires
   it;
6. the conditional apparatus is not generalized to ordinary development;
7. no runtime API, behavior, feature, or dependency boundary changes; and
8. the `0.20.2` artifact is unmodified.
