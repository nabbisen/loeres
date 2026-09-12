# RFC 024 - Post-Release Documentation Steady State

**Status.** Accepted (design frozen 2026-07-31)
**Design approval.** Architecture review is this project's architect function, not a separate tier: the operating instructions assign design authority *and* review to the same role, and require it to "act as an independent auditor" — independence is a behavior, not a second party. That review has been performed for Amendment 1 (four lenses, Go/No-Go on the record) and its residual conflict of interest — author and reviewer are the same role — is documented in RFC 022 §17 as a standing risk for the project owner to accept or mitigate, not a missing gate. Reviews 036 and 037 were produced by the implementer tier, which reports findings but holds no design-review authority (RFC 020 §11.1 likewise classifies archived review bundles as private input, not a normative source); their findings were verified and largely adopted on their merits, and one was reversed (§0.1). **Implementation of the amended provisions awaits project-owner approval only.**
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

## 0. Amendment 1 — 2026-07-31

Review 036 returned **DESIGN REVISION REQUIRED** against the first implementation
of this RFC. It was requested as an independent architecture review and later
identified as implementer-tier output (§0.3), so its verdict is a finding rather
than an approval — but its four blockers were reproduced directly and are
genuine. Two of them are defects in this frozen design rather than in its
implementation. This amendment corrects
them. The original text is retained below and updated in place; §0 records what
changed and why, so the defect and its correction remain legible.

The other two blockers (B1 stale current-facing conditional prose, B4 missing
`0.20.3` changelog record) are implementation and closeout failures against the
design as already written. They need no design change and are not amended here.

### 0.1 Equality is removed, not made mode-aware (review B3)

**Defect.** §11.1 as frozen made `this tree == last released` with a `(released)`
qualifier the ordinary rule at a release. Review 036 showed this cannot
distinguish the released tree from any later commit that keeps the same version:
such a tree passes the gate while claiming the released identity and containing
unreleased bytes. That is the post-release form of the identity conflation this
RFC exists to remove.

**Rejected correction.** The review proposed mode-aware equality — permit
equality only in an authenticated tag or intended-tag context. That works, but it
requires the checker to authenticate a release context, and every such mechanism
is a new thing to get wrong. It also leaves a `(released)` claim in tracked bytes.

**Adopted correction — the claim is dropped instead.** The apex records
*identity and lineage*, never release status:

```text
This tree: **0.20.3**
Last reconciled repository release: **0.20.2**
Implemented scope: **RFCs 001-021**
```

The invariant becomes **strict**: `this tree > last reconciled`, always, with
no mode, no exception, and no authenticated context to establish. Equality is
never valid in a tracked tree.

This is strictly stronger than what B3 asked for, and it resolves something the
frozen design got wrong at a deeper level. Both original fields were *unstable
across release*: "last released `0.20.2`" and "this tree `0.20.3` (unreleased)"
are true when the revision is tagged and false the moment distribution succeeds.
That instability is exactly why RFC 021 needed conditional wording, and this RFC
inherited the problem while claiming to have escaped it.

The amended fields are stable across `P`. "This tree is `0.20.3`" is a fact about
the bytes. "The last reconciled repository release is `0.20.2`" is a fact fixed at
authoring time that no later distribution changes. The tagged artifact stays true forever with no post-tag
edit — the property RFC 021 fought for, obtained without its machinery.

The repository therefore makes **no claim about whether this tree's own version
shipped**. That is deliberate and follows RFC 021 §13: repository tooling cannot
infer that external distribution occurred from tracked bytes. Whether `0.20.3`
released is answered by the canonical tag on the authoritative remote and its
retained workflow evidence, not by a sentence in a specification. `CHANGELOG.md`
carries the human-readable release narrative.

**The lineage field must remain a currency claim (finding 037-N2, resolved
against the reviewer's recommendation).** The field as first amended was hand-asserted and checked only by the strict inequality, so it could
understate how many releases had occurred. Review 037 offered two resolutions —
derive it, or narrow it to a mere ordering anchor — and preferred narrowing. The
first amendment adopted that. **It is wrong, and is reversed here.**

`RFC 020 §11.3` requires that each apex specification state its **last reconciled
repository release**. That is an unconditional currency obligation from a shipped
`done/` RFC. A field explicitly permitted to go stale does not satisfy it, so
narrowing would have silently loosened an earlier accepted constraint — the
precise thing roadmap §1.5 forbids, and the same defect class as B2. Neither
review caught this; the reviewer recommended the option that causes it.

The cost argument for narrowing was also wrong. It held that deriving from
`CHANGELOG.md` needs a new released/unreleased marker convention, because
`## [0.20.1]` names a version that was tagged, quarantined, and **never
released**, so "most recent heading" is wrong today. The premise is correct — but
review 036's own blocker B4 *already* requires "the correct current/unreleased
changelog record for `0.20.3`" and a changelog that "cleanly separates released
`0.20.2` history from unreleased `0.20.3` work." The convention is therefore
already a required deliverable, and deriving against it is nearly free.

**Adopted resolution.** The field takes RFC 020 §11.3's own name and meaning:

```text
This tree: **0.20.3**
Last reconciled repository release: **0.20.2**
Implemented scope: **RFCs 001-021**
```

It must name a version carried by a `CHANGELOG.md` record marked released, it
must be strictly less than *this tree*, and `doc-currency` must check both. This
is stable across `P` in the sense §0.1 requires — "the release this document was
last reconciled against" is a fact fixed at authoring time that no later
distribution changes — while satisfying §11.3 by construction and applying B2's
own diagnosis to the sibling field.

The tag-collision checks in the intended-tag preflight remain the authoritative
protection against re-releasing a shipped version; this field is defense in depth
and a currency marker, not the primary guard.

### 0.2 Implemented scope binds an exact set (review B2)

**Defect.** §11.4 required scope to be derived rather than hard-coded, and the
implementation derived it as the highest number in `rfcs/done/` rendered as a
contiguous range. A maximum is not a set: with RFC 023 withdrawn and RFC 024
implemented, `RFCs 001-024` would certify RFC 023 as implemented. Derivation is
necessary but not sufficient; the field must bind the exact lifecycle set.

**Correction.** The field carries a canonical compact set: ascending, three-digit,
comma-separated, with contiguous runs collapsed to `NNN-NNN`.

```text
Implemented scope: **RFCs 001-018, 021, 024**
```

Withdrawal and supersession are ordinary lifecycle events under RFC 000, so the
representation must express gaps rather than treat one as a gate failure. A
checker that hard-fails on a legitimate event gets worked around.

**RFC 000 is excluded** from the field, and the rule is stated here so it is not
rediscovered as a bug: RFC 000 governs the RFC directory itself, not the
product's implemented scope. Its number is reserved and never re-used, but it is
not a scope member.

**Canonical rendering rule (re-review 037, N1).** The prose above did not state
the minimum contiguous-run length, so `{020, 021}` could be rendered either
`020-021` or `020, 021` and two defensible readings could disagree byte-for-byte
against a checker that compares exactly. The rule is therefore fixed: **every
maximal run of two or more consecutive numbers collapses to `NNN-NNN`**; only an
isolated number stands alone. There is no threshold to remember and exactly one
rendering per set. `{001..018, 021, 024}` renders as `RFCs 001-018, 021, 024`;
`{020, 021}` renders as `RFCs 020-021`, never `RFCs 020, 021`.

Today's derived value is unchanged (`RFCs 001-021`), so this changes no
documentation now. It changes what happens the first time an RFC is withdrawn.

### 0.3 Provenance correction for reviews 036 and 037

Reviews 036 and 037 were requested as independent architecture reviews and cited
as such in this RFC's earlier Status line and in commits `5defae0` and `55554df`.
The project owner subsequently identified that both were produced by the
**implementer tier**. Those citations were therefore inaccurate.

The record is corrected forward rather than rewritten. What changes:

- their verdicts ("DESIGN REVISION REQUIRED", "CONDITIONALLY APPROVED") are
  **findings and recommendations, not approvals**. The architecture-review
  function belongs to the architect role, which has performed it; the owner
  approves. Routing the request to a second party was an optional mitigation for
  the author-reviewer overlap, not a required tier;
- their technical content stands or falls on verification, not authorship. Every
  checkable claim in both was re-verified: RFC 020 §11.1's CHANGELOG role is
  quoted accurately, `5defae0` touches only this file, `xtask/` is unchanged
  since `a27c2c7`, and no existing scope statement in the repository counts RFC
  000. B1-B4 and N1 were reproduced directly and are genuine;
- one recommendation is **reversed** on architectural grounds (037-N2, §0.1),
  which is the concrete reason the distinction matters: the implementer tier
  proposed a resolution that would have violated RFC 020 §11.3, and adopting it
  on the strength of its verdict would have shipped a monotonicity regression.

### 0.4 The lifecycle exception this amendment exercised

Amending in place was accepted by re-review 037 as a practical necessity:
returning RFC 024 to `proposed/` while its rejected implementation sits committed
would create an RFC 000 contradiction — implementation existing against a
Proposed RFC, which that folder's rule forbids.

The re-review was also right that the exception was, at the time, written down
nowhere. It now is: the governing conditions are RFC 000's *Loeres in-place
amendment of Accepted RFCs* section, codified by RFC 025, and this amendment is
read under it rather than under a condition this RFC states for itself.

### 0.5 Editing shipped RFC Status lines (review 038 F1)

The implementation changed RFCs 019/020/021 from `Implemented (conditional
finalization for 0.20.2)` to `Implemented (v0.20.2)`. RFC 021 §11 Q4 step 8
says "No tracked post-tag activation edit is required or permitted." The edit
is correct, and the reason is recorded here so a later reader does not read that
sentence as a permanent prohibition:

- the prohibition is scoped to the Q4 finalization sequence, which closed on
  2026-07-30 when `P` became true;
- the `0.20.2` tagged artifact is immutable and untouched — the edit lives on
  the `0.20.3` development line;
- `Implemented (vX.Y.Z)` is RFC 000's prescribed form once shipped;
- leaving a conditional claim in a shipped Status line indefinitely would be
  the B1 defect this RFC exists to retire.

### 0.6 Amendment 2 — 2026-09-12: remove the retired module (review 038 F2)

§11.3 as written retained `conditional_finalization.rs` "as the implemented
record of RFC 021", and the implementation kept its `APEX_MARKER` alive under
`#[allow(dead_code)]` beside a duplicate literal in `doc_currency`. The project
owner's rule is explicit: **remove unused code; do not suppress it.** Dead code
is technical debt, and a lint suppression is a promise to carry it.

The record of RFC 021 is RFC 021's own document and git history, not live code.
`check_rfcs` calls the module only when `release/conditional-finalization.toml`
exists, and RFC 024 deleted that file permanently, so the entire module (≈500
lines) and its call site are unreachable. Both are removed. `doc_currency` keeps
its single `RETIRED_CONDITIONAL_APEX_MARKER` constant as the guard that the
retired marker never reappears — one owner for that string, no duplication, no
suppression. §11.3 and §16 item 5 are updated to match.

### 0.7 Consequences for the queued correction series

§11.1, §11.4, §13, and §16 are updated in place to match. Implementation of the
amended provisions is blocked until independent re-review accepts this amendment.
Blockers B1 and B4 may be corrected in parallel, since neither depends on it.

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

The ordinary apex block therefore carries both, explicitly *(amended by §0.1 —
the block records identity and lineage, never release status)*:

```text
This tree: **0.20.3**
Last reconciled repository release: **0.20.2**
Implemented scope: **RFCs 001-021**
```

`doc-currency` asserts that *this tree* equals the workspace version, that *last
reconciled repository release* is **strictly less** than it and names a version
carried by a `CHANGELOG.md` record marked released (§0.1), that the implemented scope
matches the exact RFC lifecycle set (§11.4), and that the block is identical
across all three apex documents.

Equality is never valid, so there is no release-mode special case and no
authenticated context for the checker to establish. Both fields stay true after
distribution succeeds, so the tagged artifact needs no post-tag edit — the
property RFC 021 required, without its machinery. The repository makes no claim
about whether its own version shipped; the canonical tag and its retained
workflow evidence answer that, and `CHANGELOG.md` carries the narrative.

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
that it be present. The `conditional_finalization` module, its tests, and its `check_rfcs` call site are
removed (Amendment 2, §0.6); RFC 021's document and git history are its record.

The intended-tag preflight in `release_gate` currently hard-fails when the
metadata is absent. It is re-expressed against the ordinary apex form: the
intended tag must equal the workspace version, must equal *this tree*, must be
absent locally and on the authoritative remote, and must exceed *last released*.
Every property RFC 021 §7 required of `L` is preserved; only its source changes.

### 11.4 Scope binding

`Implemented scope` is checked against the RFC lifecycle rather than a literal.
Hard-coding `RFCs 001-018` is precisely why the plain mode expired. The scope
field must be derived from what is in `done/`, so it cannot silently rot again.

*Amended by §0.2.* Derivation alone is insufficient: the field must bind the
**exact set** in `done/`, rendered canonically as ascending three-digit numbers,
comma-separated, with contiguous runs collapsed to `NNN-NNN`. A maximum is not a
set. RFC 000 is excluded from the field because it governs the RFC directory
itself, not the product's implemented scope. Withdrawal and supersession are
ordinary RFC 000 lifecycle events, so a gap must be representable rather than a
gate failure. Every maximal run of two or more consecutive numbers collapses to
`NNN-NNN`; only an isolated number stands alone (§0.2).

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
   strictly exceeds *last reconciled*.
2. Focused tests: apex/workspace version disagreement fails; *preceding
   released* equal to or exceeding *this tree* fails (equality is never valid,
   §0.1); apex blocks differing across the trio fails; implemented scope
   disagreeing with the exact `done/` set fails; absent apex block fails rather
   than passing vacuously.
3. Scope-set tests: an accepted gap, an archived gap, a missing number, and
   malformed filenames each fail; a contiguous set and a set with gaps each
   render and validate canonically; the two-element boundary run renders as
   `NNN-NNN` and the comma-separated pair is rejected; RFC 000 is excluded.
4. Intended-tag preflight tests: tag equal to workspace version and exceeding
   the last reconciled release passes; tag not exceeding it fails; existing local or
   remote tag fails; all without the conditional metadata present. **The
   end-to-end `cargo xtask release-gate --intended-tag <version>` command must
   be observed reaching and passing apex binding and both collision checks** —
   focused unit tests on the binding function alone are not sufficient evidence
   (review 036 B4).
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
   tree whose version strictly exceeds the last reconciled repository release;
2. the apex block records this-tree and last-reconciled versions distinctly,
   both are asserted, and no tracked tree can assert equality or claim its own
   release status;
3. implemented scope binds the exact `done/` set, excluding RFC 000, and can
   represent gaps;
4. the intended-tag preflight preserves every RFC 021 §7 property without
   requiring the conditional metadata;
5. the conditional metadata file and the `conditional_finalization` module are
   removed, with no `#[allow(dead_code)]` left behind (§0.6);
6. the conditional apparatus is not generalized to ordinary development;
7. no runtime API, behavior, feature, or dependency boundary changes; and
8. the `0.20.2` artifact is unmodified;
9. every current-facing conditional claim is retired, with RFC 021 protocol
   history preserved as explicitly historical (review 036 B1); and
10. `CHANGELOG.md` carries an unreleased record for this tree's version and the
    observed `0.20.2` release chronology (review 036 B4);
11. the canonical set rendering admits exactly one representation per set, with
    the two-element boundary fixed (re-review 037 N1); and
12. the lineage field satisfies RFC 020 §11.3 as a checked currency claim —
    named per that section, strictly less than this tree, and bound to a
    released `CHANGELOG.md` record (finding 037 N2, resolved against the
    reviewer's recommendation); and
13. Amendment 1 carries the architect's recorded review with a Go/No-Go
    recommendation, and the project owner has approved implementation (§0.3).
