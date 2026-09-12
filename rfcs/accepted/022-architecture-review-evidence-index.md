# RFC 022 - Architecture Review Evidence Index

**Status.** Accepted (design frozen 2026-07-31)
**Design approval.** Author-performed adversarial review pass (see §17 on the
role-separation compromise); project owner authorized the `accepted/` transition
on 2026-07-31. **Amendment 1 (§0, 2026-09-09)** adds author-tier provenance and **Amendment 2 (§0.4, 2026-09-12)** adds corpus↔index coverage symmetry, and **Amendment 3 (§0.5, 2026-09-12)** enforces the cited column and row count to
the index after the reviews 036/037 incident and records that the index binds
non-normative input; it carries the architect's review and awaits no further
design gate. Implementation is authorized for the implementer tier.
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

## 0. Amendment 1 — 2026-09-09

RFC 022 was frozen on 2026-07-31. Between then and implementation, an incident
occurred that its design does not survive unchanged.

**The incident.** Reviews 036 and 037 were requested as independent architecture
reviews, cited as such in RFC 024's Status line and in two commit messages, and
acted upon — one of their recommendations was adopted into a normative document.
The project owner then identified that both were produced by the **implementer
tier**, which reports findings but holds no design-review authority. The adopted
recommendation had to be reversed: it would have violated RFC 020 §11.3.

**Why the frozen design does not catch this.** §11.1 registers reference number,
date, subject, SHA-256, and a cited flag. It binds *which document* a citation
refers to. It records nothing about *who wrote it*, so authority was inferred
from a filename and a requested title. A hash-pinned index would have proved the
right document was cited while saying nothing about whether that document could
approve anything.

### 0.1 The index records provenance

Each row gains an **author tier**, drawn from a closed set:

| Value | Meaning |
|---|---|
| `owner` | The human project owner |
| `architect` | Design authority (high-capability role) |
| `implementer` | Implementer tier — findings and recommendations only |
| `external` | A genuinely independent third party |
| `unrecorded` | Provenance not established |

**`unrecorded` is mandatory where provenance is genuinely unknown, and guessing
is forbidden.** Reviews 036 and 037 are `implementer` on direct owner statement.
For the pre-existing corpus, provenance was not captured at request time and
must not be reconstructed by inference from filenames, titles, or tone. A
retroactively guessed tier would be a fabricated authority claim — the precise
failure this amendment exists to prevent, committed by the fix.

New reviews record tier **at request time**, in the request document, not
afterwards. Provenance is cheap to capture and expensive to reconstruct.

### 0.2 The index binds non-normative input

RFC 020 §11.1 already classifies archived review bundles as *"Historical/private
review input. Not a public normative source."* RFC 022 as frozen never says so,
and its §1 framing — normative documents "cite reviews as the authority for
design acceptance" — sits awkwardly against that classification.

Both are true and the distinction matters: a review is evidence that a decision
was considered, never the source of its authority. Design authority rests with
the architect role and approval with the project owner. The index therefore
binds the identity of **input**, so a reader can reach what a decision rested on
— it does not make that input normative, and no citation may imply it does.

**The rule:** prose must not state that a review *accepted*, *approved*, or
*authorized* anything unless the referenced row's tier is `architect`, `owner`,
or `external`. A row tiered `implementer` or `unrecorded` may only be cited as
having *found*, *reported*, or *recommended*.

Enforcement is bounded, and this is stated honestly rather than overclaimed:
the checker validates that every row carries a tier from the closed set. The
authority-verb rule is human review's to apply, consistent with §11.3's existing
position on what a bounded check can and cannot establish.

### 0.3 Lifecycle note

This was the second in-place amendment of an Accepted RFC, and the case that
showed the condition was broader than RFC 024 §0.4's single sentence recorded:
that exception covered a correction narrowly patching already-committed partial
work, whereas RFC 022 had no implementation at all. The governing conditions are
now RFC 000's *Loeres in-place amendment of Accepted RFCs* section, codified by
RFC 025, and every amendment below is read under it.

### 0.4 Amendment 2 — 2026-09-12: coverage symmetry (review 039 F3)

Hash verification as frozen is row→file only: each registered hash must exist
in the corpus. A corpus file that is present but **unregistered** passes
silently. Architect review 039 demonstrated the consequence by accident —
writing that review into the corpus made 52 files against 51 rows and the gate
still passed. The index goes stale on every review written, which is routine.

When the corpus is present, the checker must also assert that every corpus file
is registered: the set of corpus hashes and the set of registered hashes must be
equal. When the corpus is absent this is reported `unavailable`, never passed,
for the same reason as hash verification. The §11.3 table and §13 tests are
updated accordingly. Registering a review remains a hand edit; this makes
forgetting it a gate failure rather than a silent drift.


### 0.5 Amendment 3 — 2026-09-12: cited-column symmetry and row count

Two tightenings, both raised by the implementer tier against the index as
landed and both accepted on their merits.

**Cited-column symmetry.** The checker derives the true cited set at run time
and prints its size, but never compares it to the `Cited in release` ticks. A
citation added to a tracked document left the column stale with no failure —
observed when RFC 024 §0.5 cited review 038 and the count moved 12→13 without
the tick following. That is the hole Amendment 2 closed for rows, still open for
the column. The checker must assert that the set of ticked rows equals the
derived cited set, failing on either direction. This depends only on tracked
bytes, so it holds in a clean extraction and needs no availability branch.

**Row count equals file count.** Set equality lets a duplicated *row* satisfy
symmetry, and a copy-paste while registering a review is a likelier slip than
two byte-identical documents. When the corpus is present, the number of
registered rows must equal the number of corpus files. This compares hash
counts, not reference numbers: §11.7's two `Ref 001` rows carry distinct hashes
and remain legitimate. It is not a uniqueness rule on `Ref`.

Stating a known-unenforced column in the index prose was the right interim
handling; with enforcement the statement is replaced, not kept.

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
**author tier (§0.1)**, and whether the reference is cited by a tracked
normative document.

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
| Every index row carries an author tier from the closed set | **Enforced, fail-closed** | Depends only on tracked bytes (§0.1) |
| No citation asserts approval for an `implementer`/`unrecorded` row | **Human review** | Requires reading the surrounding prose; a bounded checker cannot judge it (§0.2) |
| Each registered SHA-256 matches the corresponding file | **Verified when the corpus is present; reported unavailable otherwise** | The corpus is maintainer-held and legitimately absent from an extraction |
| Every corpus file is registered (corpus↔index coverage symmetry) | **Enforced when the corpus is present; reported unavailable otherwise** | Added by Amendment 2 (§0.4): the index goes stale on every review written, and only a count comparison stops that being remembered rather than enforced |
| The `Cited in release` ticks equal the cited set the checker derives | **Enforced, fail-closed** | Amendment 3 (§0.5): depends only on tracked bytes, holds in a clean extraction; the column had the same remembered-not-enforced hole Amendment 2 closed for rows |
| Registered row count equals corpus file count | **Enforced when the corpus is present; reported unavailable otherwise** | Amendment 3 (§0.5): set equality alone lets a duplicated row pass; compares hash counts, not `Ref` values (§11.7's shared `Ref 001` stays legitimate) |

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
   reported rather than silently ignored; a row with a missing tier, or a tier
   outside the closed set, fails; a corpus file present but unregistered fails,
   and matched corpus/index sets pass (§0.4); a cited-but-unticked row and a
   ticked-but-uncited row each fail; a duplicated row fails the row-count
   check while two distinct-hash rows sharing a `Ref` pass (§0.5).
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
4a. every row carries an author tier from the closed set, `unrecorded` is used
    wherever provenance was not captured, and no tier is inferred retroactively
    (§0.1);
4b. the index states that it binds non-normative input and does not confer
    authority (§0.2, RFC 020 §11.1);
5. `rfcs/README.md` registers the index and all links resolve;
6. the RFC 021 conditional apparatus is retired per §15, in this release or a
   separately reviewed one that precedes it;
7. no runtime API, behavior, feature, or dependency boundary changes; and
8. the `0.20.2` artifact is not retroactively modified.
