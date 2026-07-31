# RFC 022 - Architecture Review Evidence Index

**Status.** Accepted (design frozen 2026-07-31)
**Design approval.** Author-performed adversarial review pass (see §17 on the
role-separation compromise); project owner authorized the `accepted/` transition
on 2026-07-31.
**Tracks.** Post-recovery traceability defect found during the 0.20.2 onboarding
review; extends RFC 020's normative-authority charge to review evidence.
**Touches.** `rfcs/review-evidence-index.md` (new), `rfcs/README.md`, `xtask`
documentation checks, and the citation surfaces in `rfcs/done/`,
`rfcs/handoffs/`, `docs/specs/`, `docs/src/`, `ROADMAP.md`, and `CHANGELOG.md`.

---

### Extended Metadata

* **Rust Edition Compliance:** No runtime code change; host-side tooling remains
  Rust 2024 with declared MSRV 1.85.0.
* **Target Environment:** Repository governance and host-side verification tooling.
* **Proposed release:** Corrective patch after `0.20.2`; exact version and tag
  require project-owner approval.
* **Prior art:** RFC 000 (lifecycle and index integrity), RFC 019 (release
  evidence), RFC 020 (normative authority and currency), RFC 021 (conditional
  finalization).

## 1. Summary

Eleven tracked normative documents — including `rfcs/done/019`, `020`, and `021`,
`docs/specs/loeres-roadmap-milestones-v1.md`,
`docs/specs/loeres-reconciliation-traceability-v020.md`, `ROADMAP.md`,
`CHANGELOG.md`, `docs/src/recovery-roadmap.md`, and three implementation
handoffs — cite independent architecture reviews `021`, `022`, `024`, `025`,
`027`, `030`, `031`, `032`, and `033` as the authority for design acceptance,
evidence acceptance, and blocker classification. RFC 021's own **Design
approval** line rests on reviews 027 and 031/032.

`git ls-files .git-exclude` returns zero. None of those review documents exists
in the repository, in the `0.20.2` tag, in the release tarball, or in git
history. The released artifact names an authority that a reader of the artifact
cannot reach.

This is the same defect class RFC 020 was chartered to eliminate — normative
statements whose basis cannot be located — and it survived the recovery because
the recovery's own evidence was produced outside the tracked tree.

RFC 022 closes the citation-to-evidence link without publishing deliberation. It
adds one tracked, hash-pinned identity register and a fail-closed check that
every in-repository review citation resolves to a registered row. Review bodies
remain maintainer-held.

## 2. Affected crates

None. No crate under `crates/` is touched. Work is confined to repository
documentation and the host-only `xtask` verification crate.

## 3. Public API boundary impact

No runtime public API changes. No solver type, storage type, problem contract,
validation contract, feature flag, or crate dependency direction changes.

The host-only `xtask` contract gains one enforced assertion inside the existing
documentation-currency check. No new user-facing command is required; adding one
is an implementation-review decision, not a design commitment.

## 4. Dependency impact

No new workspace dependency. The check operates on tracked Markdown and, when
present, on maintainer-held files outside the tracked tree, using facilities
`xtask` already has.

## 5. `std` / `alloc` impact statement

None. `xtask` is a `std` host tool and is never a dependency of a library crate.
The `no_std`, no-`alloc` boundary for `loeres`, `loeres-backend-static`, and
`loeres-device` is untouched.

## 6. Device determinism impact statement

None. No device kernel, workspace, configuration, timing mode, iteration bound,
scalar capability, or target profile is affected.

## 7. Cluster scalability impact statement

None. No orchestration, cancellation, budget, observability, gateway, or
validation-cache behavior is affected.

## 8. Error and diagnostic impact

No change to `SolverError`, `DiagnosticSnapshot`, `SolveStatus`, or any runtime
error or diagnostic surface. Failures introduced by this RFC are host-side gate
failures with actionable messages, not runtime errors.

## 9. Feature flag impact

None.

## 10. Semver impact

None for published crate contracts. The change is documentation and host tooling
only and is appropriate for a patch release.

## 11. Design

### 11.1 The index

A single tracked file, `rfcs/review-evidence-index.md`, registers one row per
review document: reference number, date, subject, SHA-256 of the file contents,
and whether the reference is cited by a tracked normative document.

The SHA-256 is the integrity anchor. It lets a future maintainer — or an auditor
holding a copy of the corpus from any source — prove that the document in hand is
the one the released artifact cited, without the repository carrying the body.

### 11.2 What the index must never become

RFC 000 records an anti-pattern: handoffs acquiring a parallel lifecycle. The
same failure is available here. The index therefore:

- records evidence identity only;
- carries no status, approval, or state column;
- never gains its own state folders;
- grants, withdraws, and modifies nothing.

RFC status remains governed exclusively by RFC 000's folder-as-source-of-truth
rule.

### 11.3 Enforcement, and its honest limits

Two distinct assertions, with deliberately different strengths:

| Assertion | Strength | Rationale |
|---|---|---|
| Every review reference in a tracked normative document resolves to an index row | **Enforced, fail-closed** | Depends only on tracked bytes; holds in a clean extraction |
| Each registered SHA-256 matches the corresponding file | **Verified when the corpus is present; reported unavailable otherwise** | The corpus is maintainer-held and legitimately absent from an extraction |

Reporting an absent corpus as *unavailable* rather than *passed* follows RFC
011's evidence-class discipline and RFC 019's refusal to promote advisory
results to enforced evidence.

### 11.4 Citation grammar is normative

The enforcement in §11.3 is only as strong as its ability to recognize a
citation. If an author writes a reference the checker's pattern does not match,
the citation escapes enforcement silently — the check passes while the defect it
exists to catch is present. That failure mode is worse than no check, because it
manufactures false confidence.

The citation form is therefore fixed: **`architecture review NNN`**, with `NNN`
zero-padded to three digits, and multiple references written as a
slash-separated list (`reviews 031/032`). The checker must additionally **report**
any near-miss — a `review` token adjacent to digits that does not match the
canonical form — as a finding rather than ignoring it. Silence must mean "no
citations present," never "no citations recognized."

### 11.5 Cited reviews are immutable

A hash-pinned index is only meaningful if the pinned document does not move. Once
a review is cited by a tracked normative document, its file is immutable: no
edits, including typographical corrections. A correction is issued as a new
review document with its own reference number, exactly as RFC 000 forbids
renumbering and RFC 021 forbids moving a canonical tag.

This is the same principle applied one layer down. If a cited review could be
edited, the index would record a hash that silently stops matching, and the
project would be unable to distinguish an innocent typo fix from evidence
tampering.

### 11.6 Disposition deferred

v1 registers identity, not outcome. Transcribing a review's disposition from the
prose that cites it would be circular — the citation is the thing being verified.
Populating a disposition column requires reading each review and is a separate
reviewed pass, out of scope here.

### 11.7 Known corpus irregularities

Two preparation reviews share reference `001`, disambiguated by date.
Pre-recovery reviews predate the numeric scheme, carry no reference number, and
are registered for completeness; none is cited by a tracked document. The index
records these as they are rather than renumbering — RFC 000 forbids renumbering
for exactly the reason that external references would break silently.

## 12. Rejected alternatives

| Alternative | Reason rejected |
|---|---|
| Track the full review corpus | Closes the link, but pushes ~48 deliberation documents into every release tarball, expands the artifact well beyond its purpose, and publishes review-round prose that was written as working material |
| Do nothing; treat citations as maintainer-private shorthand | Leaves released normative documents citing unreachable authority — the RFC 020 defect, restated |
| Embed each hash inline at every citation site | Duplicates integrity data across eleven files, drifts on the first edit, and has no single point of verification |
| Store evidence in an external system and link by URL | Adds an availability dependency the repository does not control and cannot verify offline or from an extraction |
| Fold this into RFC 020 as an amendment | RFC 020 is Implemented and shipped in `0.20.2`; amending a shipped RFC to cover a defect found afterwards rewrites history. RFC 000 requires a superseding or follow-on RFC |

## 13. Verification gates

1. `cargo xtask check` — with the new citation-resolution assertion enforced.
2. Focused tests: a citation with no index row fails; an index row whose hash
   mismatches a present corpus fails; an absent corpus reports unavailable and
   does not pass; a reference format not matching the citation grammar is
   reported rather than silently ignored.
3. `cargo xtask check-rfcs` and `link-audit` — index registered in
   `rfcs/README.md`, all relative links resolve.
4. `cargo fmt --all -- --check`; all-target/all-feature Clippy with warnings
   denied; all-feature workspace tests and doc-tests.
5. Exact Rust 1.85 all-feature workspace check.
6. `mdbook build docs`.
7. RFC 019's release-candidate gate at closeout.

## 14. Implementation sprint plan

| Sprint | Work | Exit |
|---|---|---|
| S0 | Design freeze: index schema, enforcement split, non-lifecycle constraint | Architecture acceptance of this RFC |
| S1 | Add `rfcs/review-evidence-index.md` and register it in `rfcs/README.md` | Index complete; every current citation resolves |
| S2 | Implement the citation-resolution assertion and hash verification with the availability split | Focused tests pass |
| S3 | Sweep citation surfaces for reference-format consistency | No unparseable citation remains |
| S4 | Full gate suite | All gates pass |
| S5 | Changelog and roadmap entries; retire the RFC 021 conditional block per the dependency in §15 | Documentation reconciled atomically |
| S6 | Closeout | Evidence attached; RFC moved on release |

## 15. Incidental currency corrections

Two stale items on the release-governance surface ride this RFC, since it
already owns that surface and RFC 020 requires reconciliation rather than
drive-by edits:

- `Cargo.toml:13` states that member crates expose APIs "through RFCs 001-018";
  `0.20.2` shipped 001-021.
- The post-release version convention adopted after `0.20.2` — bump `main` to
  the next patch immediately after a release, with the release version set
  authoritatively in the finalization revision — is recorded nowhere. It belongs
  in `docs/src/development.md` and `CONTRIBUTING.md`.

Neither changes runtime behavior.

## 16. Dependencies and interactions

**This RFC is the first release-bearing change after `0.20.2`.** It therefore
inherits the obligation to retire RFC 021's conditional-finalization apparatus:
the before-`P`/after-`P` block in the apex trio, `README.md`, `ROADMAP.md`,
`CHANGELOG.md`, `rfcs/README.md`, `docs/src/specifications.md`, and
`docs/src/threat-model.md` describes a condition that has since resolved. It
must be replaced with plain released-state wording, not copied forward and not
edited into the `0.20.2` artifact retroactively.

This is called out as an explicit dependency rather than absorbed silently,
because it is a separate concern that merely shares a vehicle. If the project
prefers, it may land as its own RFC ahead of this one; it may not land as an
unreviewed side effect.

## 17. Review independence

The project lifecycle requires independent architecture acceptance for the
`accepted/` transition. This RFC was authored and reviewed by the same agent.
That is a weaker evidence class than an independent pass, and it is recorded
here rather than left implicit — it is a concrete instance of register item
I-12, the single-maintainer role-separation risk inherited from the 2026-07-17
preparation review.

The self-review was adversarial and produced material changes: §11.4 and §11.5
did not exist in the reviewed draft and close a silent-pass failure mode and an
evidence-mutability hole respectively. It is not, however, a substitute for a
second reviewer, and the project should not treat it as one by precedent.

## 18. Exit criteria

RFC 022 is complete only when:

1. every review reference in a tracked normative document resolves to a
   registered index row;
2. the resolution assertion is enforced and fails closed;
3. hash verification runs when the corpus is present and reports unavailable —
   never passed — when it is absent;
4. the index carries no lifecycle, status, or approval semantics;
5. `rfcs/README.md` registers the index and all links resolve;
6. the RFC 021 conditional apparatus is retired per §15, in this release or a
   separately reviewed one that precedes it;
7. no runtime API, behavior, feature, or dependency boundary changes; and
8. the `0.20.2` artifact is not retroactively modified.
