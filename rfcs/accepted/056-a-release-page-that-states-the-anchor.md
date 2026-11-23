# RFC 056 - A Release Page That States The Anchor

**Status.** Accepted (design frozen 2026-11-23)

**Author tier.** `architect`.

**Governing scoping.** RFC 047 property **C** — *the published artifact is the reviewed
deliverable, and the identity anchor covers what users receive, not only what this project
archives* — together with property **B** (*no claim without a gate*). The owner asked on
2026-11-23 whether CI should create a GitHub Release with release notes; architect review 127
answered with three options and this RFC scopes the recommended one. Every figure was measured on
`7f6a1bc`, workspace `0.25.0`.

This RFC supersedes a decision this repository records in two places, so
`docs/specs/loeres-roadmap-milestones-v1.md` §1.5 requires *"a superseding RFC with an explicit
compatibility and safety analysis"*. That analysis is **§3**.

**Target release: `0.25.1`.** It changes no public surface and forces no position; it lands
**after** `0.25.0` because release machinery must not change in the release that uses it, and it
first runs at the tag after this one.

## 0.1 Amendment 1 (2026-11-23)

Made while Accepted, under RFC 000's in-place-amendment rule, and recorded in the implementation
handoff §0. The Status line carries only the design-freeze date while this RFC is in `accepted/`,
because `doc-currency` requires that form there; **this amendment must be named in the Status line
when the file moves to `rfcs/done/`**.

### S1's status-line assertion: **three** permitted values, not two, and the parser already exists

S1 says the renderer *"asserts that the line it drops is exactly one of the two known literals,
failing if a third phrasing appears"*. Measured instead:
`xtask/src/checks/doc_currency.rs:232` holds

```text
const CHANGELOG_STATUS_VALUES: &[&str] = &["withdrawn candidate", "unreleased", "released"];
```

— **three** values, each also accepted with a trailing ` (` detail form. So the count in S1 is
mine and it is wrong, and the phrasing that must fail closed is a **fourth**, not a third.

**More to the point, the function already exists and must be reused, not rewritten.**
`parse_changelog_release_status` (`doc_currency.rs:240-271`) extracts the value under a version's
heading, validates it against that constant, and fails closed on anything else — and it is already
pure over the supplied source so that every case is testable without a file on disk. S1's
fail-closed requirement is therefore **already implemented once**; the renderer calls it.

Reimplementing it would put two parsers on one record, which is the defect RFC 051 removed by
unifying `CRATE_README_PATHS` and RFC 053 avoided by sharing `collect_md` and `mask_code_spans`.

**Nothing else in S1 changes:** the line is still dropped, for the reason §4 S1 gives, and the
drop still fails closed — through the existing function rather than a new one.

## 1. Summary

**The RFC 028 anchor — the one number that makes this project's central claim checkable — is not
available to a user.** `.github/workflows/release.yml:58-65` retains the release-gate evidence as a
**workflow artifact** with `retention-days: 14`, and downloading a workflow artifact requires a
GitHub account with access to the repository. So for every release this project has shipped, the
uncompressed-tar SHA-256 was computed, reviewed, and then expired unseen.

Meanwhile `git archive --format=tar <tag> | sha256sum` is a command any user can run, against a tag
GitHub already serves. The anchor is *checkable*; it is simply never *stated* anywhere a user can
read.

This RFC has the tagged `release-gate` job render the release notes **into its own evidence
bundle**, and adds a second, narrowly-permissioned CI job that posts that file as a GitHub Release
body after the gate has succeeded. The notes carry the version's `CHANGELOG.md` section, the
uncompressed-tar anchor, the recorded tool versions, and **one sentence naming GitHub's
auto-generated source archives as not the anchor** and why.

**No artifact is attached.** Attaching one is a channel, and *no channel without verifiable
artifact identity* is an anti-goal (§6).

## 2. The defect, measured

### 2.1 The anchor expires, and is private until it does

| fact | where |
| --- | --- |
| evidence retained as a workflow artifact, **14 days** | `.github/workflows/release.yml:58-65`, `retention-days: 14` |
| the bundle holds the anchor | `EVIDENCE.md`'s `**Uncompressed tar SHA-256:**` line, beside `TRACKED_CONTENT_MANIFEST.tsv` and the archive |
| the anchor is cross-environment by construction | `EVIDENCE.md`: *"The uncompressed tar SHA-256 is the cross-environment identity of this source artifact's content and layout. The archive SHA-256 identifies this build's bytes and is not expected to reproduce in another environment (RFC 028)."* |

So the project computes a number designed to be reproduced by someone else, and then stores it
where only its own maintainers can reach it, for two weeks.

### 2.2 GitHub already shows two archives whose digests are not the anchor

A GitHub Release page carries *Source code (zip)* and *Source code (tar.gz)*, generated by GitHub.
**Neither is the anchor**, and the `tar.gz` cannot be: RFC 028 exists because compressed bytes did
not reproduce between CI and local builds over identical content
(`docs/specs/loeres-roadmap-milestones-v1.md` §7).

This exposure is **not created by this RFC** — GitHub serves tag archives today, with no Release
object — but a Release page puts a download button beside our hash, which is where a reader
concludes they match. **That is the owner's second principle failing at the moment of maximum
attention**, and it is the reason §4's S3 is a required slice rather than a nicety.

### 2.3 The status line is a trap, measured

At the tagged revision the CHANGELOG heading is dated and subtitled, and the line beneath it still
reads

```text
**Release status:** unreleased
```

because the cut keeps it so and the post-release commit marks it `released` afterwards (the
`0.24.2` cut plan §5 step 3 and §6 item 1). **Correct at that revision, and absurd on a Release
page.** A notes renderer that copies the section verbatim publishes "unreleased" on the release
announcement; one that silently drops the line is transforming the record without saying so.

### 2.4 The decision being superseded, stated

Two places record that no GitHub Release is created:

- `.github/workflows/release.yml`'s closing line: `# RFC 019 creates no GitHub release and performs no registry publication.`
- `docs/src/development.md`: *"No workflow in this repository publishes, and the development team does not publish."*

**The second is not superseded and must not be read as such.** *Publication* is crates.io, run by
the architect and the owner under RFC 021 §7. A Release object is **distribution**, which RFC 021
§7 already defines as the tag being accepted and the tagged gate succeeding. This RFC makes the
Release a **consequence** of that definition, never a claim preceding it (§4 S2).

## 3. Compatibility and safety analysis

Required by `docs/specs/loeres-roadmap-milestones-v1.md` §1.5.

### 3.1 What changes for a user

Nothing they depend on. No crate, no API, no feature, no dependency, no solve path. A new public
page appears per release, carrying information this project already computes.

### 3.2 The permission change, and why it is scoped to a second job

Creating a Release needs `contents: write`. The release workflow today declares
`permissions: contents: read` and can write nothing.

**The gate job must keep `contents: read`.** Granting write to the job that runs twenty gates would
widen the blast radius of every one of them. So the Release is created by a **second job**, with
`needs: release-gate` and its own `permissions: contents: write`, consuming the evidence through the
artifact the first job already uploads.

That split buys two things beyond least privilege: the ordering is enforced by `needs:` rather than
by step order, so **a Release cannot exist for a revision whose gate failed**; and the token with
write access is in scope only for a job whose single action is posting a file.

### 3.3 Which tool writes the Release

**`gh release create`, not a third-party action.** RFC 046 established that tools in the release
path are installed by verified identity because `cargo install` pins a crate version but not the
compiler that builds it. That concern is about **a binary whose output enters what we ship**.
Nothing `gh` emits enters the artifact: it posts a file to the platform's own API. A third-party
action would instead put third-party code in the one job holding `contents: write`.

**The residual, stated rather than hidden:** the runner's `gh` version is not pinned. That is
accepted here precisely because its output is not an artifact; if a future slice ever attaches a
file to a Release, this trade must be re-derived, not recalled.

### 3.4 What a reader of the page can verify, and what they cannot

**Can:** `git archive --format=tar <tag> | sha256sum` against the stated anchor — the same three-way
check the architect runs at every cut, reduced to one command.

**Cannot:** that GitHub's own `tar.gz` matches anything. S3 says so on the page.

### 3.5 Monotonicity

None of §1.5's six rules is touched: no trait, no `alloc`, no workspace change, no cluster concept
in core, no dynamic dispatch, no reinterpretation of "deterministic". The change is additive and
outside the crates entirely.

### 3.6 Reversibility

A Release object can be deleted; a crates.io version cannot. This is the reversible half of
distribution, which is part of why it is the right first step of Phase IV.

## 4. Design

### S1 — the notes are an artifact of the gate, not of CI

`cargo xtask release-gate` writes **`RELEASE_NOTES.md`** into the evidence directory it already
creates, beside `EVIDENCE.md`, `TRACKED_CONTENT_MANIFEST.tsv` and the archive.

**Rendered, never typed**, from three sources the gate already holds:

1. the version's `CHANGELOG.md` section, from its dated heading to the next `## [`;
2. the **uncompressed-tar SHA-256** the gate just computed;
3. the tool versions the gate already records in `EVIDENCE.md`.

Consequences, each deliberate:

- **The notes are reviewed with everything else.** They appear in the candidate evidence of every
  slice, so a malformed CHANGELOG section fails the gate *before* a tag is pushed rather than at
  Release time.
- **There is no second source of truth.** A hand-written release note beside `CHANGELOG.md` is the
  defect RFC 048 and RFC 053 spent two releases removing.
- **CI renders nothing.** S2's job posts a file.

**The status line (§2.3) is dropped, and the drop fails closed.** The renderer omits the
`**Release status:**` line, and **asserts that the line it drops is exactly one of the two known
literals**, failing if a third phrasing appears — the `is_published` pattern, equality and fail
closed, because RFC 052's clause 5 shipped a defect through a `contains` on exactly this field
(architect review 119 F1). The RFC's reason for dropping rather than keeping: that line states the
**CHANGELOG's** status at that revision, and the Release page's own existence is the distribution
fact; publishing "unreleased" on a release page is the confusion property **D** forbids.

### S2 — a second CI job, after the gate, narrowly permissioned

In `.github/workflows/release.yml`, a `github-release` job: `needs: release-gate`,
`permissions: contents: write`, downloading the evidence artifact (SHA-pinned
`actions/download-artifact`, as the repository already pins `upload-artifact`), then
`gh release create "$TAG" --notes-file <bundle>/RELEASE_NOTES.md --verify-tag`.

- **No assets.** `--notes-file` only.
- **Not a draft**, because the authorized act is the tag push and the gate has already passed.
- **GitHub's `prerelease` flag is not set.** Nothing in this project defines it, and an undefined
  flag on a user-facing page is a claim without a gate. The CHANGELOG preamble already states what
  pre-`1.0` means here.
- **Idempotence:** a re-run must not fail the workflow on an existing Release. The handoff decides
  between `gh release edit` on conflict and a guard, with the behaviour of each measured rather
  than assumed.

### S3 — the page says what the anchor is, and what it is not

`RELEASE_NOTES.md` carries, beneath the CHANGELOG section, a short block of text this RFC fixes in
substance:

- the **uncompressed-tar SHA-256**, named as the RFC 028 cross-environment anchor, with the exact
  command that reproduces it;
- one sentence that **GitHub's auto-generated *Source code (zip)* and *(tar.gz)* assets are not
  this anchor**, because gzip bytes do not reproduce across environments — the reason RFC 028
  anchors the uncompressed tar;
- the tool versions the gate recorded, so a reader knows what produced the evidence;
- **what is not claimed**: that this release is on crates.io. Publication is separate (RFC 021 §7),
  and five of this project's releases were distributed and never published.

### S4 — assertions

- **Unit tests on the renderer**, pure over CHANGELOG text: a well-formed section renders; a
  version with no section fails; two sections for one version fails; a third `Release status:`
  phrasing fails closed; the dropped line does not appear in the output.
- **`release-gate` already fails if the file cannot be written** — no new gate, and the existing
  `if-no-files-found: error` on the upload keeps an empty bundle from reaching S2.
- **No assertion is made about the Release object itself.** CI posting a file is not something this
  repository can assert from inside; what it can assert is that the file is correct, and S1 puts it
  where every slice's review sees it.

## 5. What must not be swept in

1. **`docs/src/development.md`'s publication sentence.** Not superseded (§2.4). Publication stays
   the architect's and the owner's, and no workflow performs it.
2. **The gate job's `contents: read`.** S2 adds a job; it does not widen the gate's permissions.
3. **`retention-days: 14` and the evidence upload.** They stay as they are — the bundle remains the
   maintainers' record. This RFC adds a public statement of one number from it, not a replacement
   for it.
4. **`EVIDENCE.md`'s content and wording.** The renderer reads it; it does not rewrite it.
5. **`CHANGELOG.md`'s format.** The renderer adapts to the record, never the reverse.

## 6. Non-goals

- **No attached artifact, and therefore no channel.** *No channel without verifiable artifact
  identity* is an anti-goal, and attaching a tarball beside GitHub's two would put three source
  artifacts with three digests on one page — the two-identity-stories problem Phase IV exists to
  prevent. Architect review 127 declined this explicitly as option C.
- **Phase IV is not completed.** This is its cheapest first step: the anchor of the **source**
  artifact, stated publicly. Binary artifact identity, which RFC 047 §5 item 2 records as an open
  question, is untouched.
- **No publication, no registry action, no signing, no attestation.**
- **No change to what "distributed" means** (RFC 021 §7).
- **Semver: none.** Documents, `xtask`, and CI.

## 7. Risks

- **A reader downloads GitHub's `tar.gz` and compares it to our hash.** S3's sentence is the whole
  mitigation, and it is prose on a page rather than a gate. It is the risk I would most want
  measured after the first use: if the page reads as two competing hashes, it needs a different
  shape, not a longer sentence.
- **`contents: write` exists in the workflow where it did not before.** Scoped to a job whose only
  action is posting a file (§3.2), and the token is GitHub's own job token, not a stored
  credential.
- **A failed `gh` call after a successful gate** leaves a distributed release with no page. That is
  the correct failure direction — distribution is defined by the tag and the gate, not by the page
  — and the workflow's failure is visible.
- **The renderer could drift from `CHANGELOG.md`'s shape** if the record's format changes. S4's
  tests pin the shape, and the gate renders on every slice, so drift fails early rather than at a
  tag.

## 8. What I could be wrong about

- **That dropping the status line is better than transforming it.** The alternative is to render it
  as *"distributed by the tagged release-gate job; publication to crates.io is separate"* — more
  informative, and one more sentence this project would have to keep true. I chose the smaller
  claim; §4 S1's fail-closed assertion is what makes either choice safe.
- **That `gh` is the right tool.** §3.3 reasons from RFC 046's purpose rather than its letter. A
  reviewer who reads RFC 046 as requiring identity for *every* tool in the release path would
  reach a SHA-pinned action instead, and would be applying a defensible rule more strictly than I
  have.
- **That the notes belong in the evidence bundle.** It couples the gate to a presentation concern.
  The alternative — rendering in CI — decouples them and gives up the review and the early failure.
  I would rather the gate carry one more file than have a release page nobody reviewed.
