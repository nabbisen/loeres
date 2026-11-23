# RFC 056 — implementation handoff

**Author tier.** `architect`. **Date:** 2026-11-23.
**Governing text.** `rfcs/accepted/056-a-release-page-that-states-the-anchor.md`, Accepted
(design frozen 2026-11-23), **including `## 0.1 Amendment 1`**, which corrects S1's
status-value count and names the function to reuse. Where this file and the RFC disagree, **the
RFC is the contract**; report it rather than choosing.
**Release.** `0.25.1`, **not `0.25.0`** — release machinery must not change in the release that
uses it (RFC 056 header). So this slice lands **after** the `0.25.0` cut, and the workspace
version it runs against is the one the post-release commit sets. **Do not start it before
`0.25.0` is tagged and reconciled**; if you are handed it earlier, say so and stop.
**Order: S1, S4, then S2 and S3.** The renderer and its tests come first; the workflow job posts a
file that must already exist and already be right.

## 0. Amendment 1, and the figure I had wrong

S1 said the dropped `**Release status:**` line must match *"one of the two known literals"*. It is
**three** — `doc_currency.rs:232`:

```text
const CHANGELOG_STATUS_VALUES: &[&str] = &["withdrawn candidate", "unreleased", "released"];
```

each also accepted with a trailing ` (` detail form, so what must fail closed is a **fourth**
phrasing. My count, not derived; corrected in Amendment 1.

**And the parser already exists.** `parse_changelog_release_status` (`doc_currency.rs:240-271`) is
pure over the supplied source, validates against that constant, and fails closed. **Call it. Do
not write a second one.** Two parsers over one record is what RFC 051 removed by unifying
`CRATE_README_PATHS` and RFC 053 avoided by sharing `collect_md` and `mask_code_spans`. It is
private to `doc_currency`; make it `pub(crate)` the way RFC 050's slice did for
`external_or_anchor` and RFC 053's for `mask_code_spans` — and change nothing else about it.

## 1. S1 — `RELEASE_NOTES.md`, rendered by the gate into its own evidence bundle

### 1.1 Where

`xtask/src/checks/release_gate/package.rs`. `write_evidence` at `:480-491` writes `EVIDENCE.md`
into the evidence directory from `render_evidence`'s string; add the notes beside it, from the same
inputs, written in the same place and failing the same way (`cannot write …` → `Err`).

**Follow `render_evidence`'s shape exactly:** a pure `render_release_notes(...) -> String` over
already-gathered inputs, and one `fs::write`. The whole point is that the body is testable on
literals.

### 1.2 What goes in it, in this order

1. **The version's `CHANGELOG.md` section**, from its `## [<version>]` heading line to the line
   before the next `## [`, **with the `**Release status:**` line and the blank line that follows it
   removed** — and with `parse_changelog_release_status` called first, so a fourth phrasing fails
   the gate rather than being silently dropped (§0).
2. **The anchor block** of §2 below, verbatim in substance.
3. **The tool versions** the gate already observes — `ToolVersions::observe` at `:470`, the same
   values `EVIDENCE.md` prints. Render them in the same fenced form, so the two files cannot
   disagree about what produced the evidence.

### 1.3 Three things the renderer must not do

- **Not reformat the CHANGELOG section.** Copy the lines. The record's shape is the record's.
- **Not invent a heading.** The CHANGELOG heading line — dated and subtitled by the cut — is the
  notes' own first line. GitHub shows the tag name separately; a second title is noise.
- **Not read the tag.** The gate already knows the candidate version; the notes are rendered for
  it. Nothing in the renderer may depend on a tag existing, because `release-gate` runs on every
  slice, long before any tag.

## 2. S3 — the anchor block, written out

Append this, with `<version>`, `<uncompressed-tar-sha256>` and `<archive-sha256>` substituted from
the values the gate just computed. **Substituted, never typed by a human.**

```text
## Verifying this release

The identity of this release's source is the SHA-256 of its **uncompressed** tar
archive (RFC 028):

    <uncompressed-tar-sha256>

Reproduce it from this tag:

    git archive --format=tar <version> | sha256sum

**GitHub's own "Source code (zip)" and "Source code (tar.gz)" downloads on this
page are not that identity.** Gzip output is not byte-reproducible across
environments — two builds of identical content differ — which is why this
project anchors the uncompressed tar instead. For the record, this build's
compressed archive hashed `<archive-sha256>`, and that value is not expected to
reproduce elsewhere.

This page records distribution, not publication. Whether this version is on
crates.io is stated in `CHANGELOG.md`; five releases of this project were
distributed and never published.
```

**Why the `git archive` command and not a download instruction:** it is the one check a reader can
run without trusting anything this page serves. That is the whole of property **C** in one line, so
it stays first.

**If the gate has no uncompressed-tar digest** — the stage where `digests` is `None` at
`render_evidence:493-499` — **render no anchor block at all** rather than an empty one, and say in
the notes nothing about identity. A notes file that states a blank hash is worse than one that
states none.

## 3. S2 — the second workflow job

In `.github/workflows/release.yml`. The existing `release-gate` job is **unchanged**, including its
`permissions: contents: read` at workflow level and its evidence upload.

Add a `github-release` job:

- `needs: release-gate` — the ordering is the point (RFC 056 §3.2): no Release may exist for a
  revision whose gate failed;
- `permissions: contents: write` **on the job**, not at workflow level;
- `if: startsWith(github.ref, 'refs/tags/')`, so a `workflow_dispatch` run cannot create a Release
  for a non-tag ref;
- download the evidence artifact uploaded by job 1 — `name: loeres-release-evidence-${{ github.sha }}` —
  with `actions/download-artifact`, **pinned by commit SHA with the version in a trailing comment,
  exactly as `actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02 # v4.6.2` is**;
- then one step:

```yaml
        env:
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        run: |
          gh release create "${GITHUB_REF_NAME}" \
            --notes-file "<evidence-dir>/RELEASE_NOTES.md" \
            --verify-tag \
            --title "${GITHUB_REF_NAME}"
```

**`<evidence-dir>`** is the one directory the artifact contains; resolve it rather than hardcoding
a revision-and-version name, and **fail if there is not exactly one** — a bundle with two
directories means something unexpected happened upstream and guessing is not the response.

### 3.1 Four decisions, fixed, with their reasons

- **No assets.** `--notes-file` only. Attaching one is a channel, and *no channel without
  verifiable artifact identity* is an anti-goal (RFC 056 §6).
- **Not a draft.** The authorized act is the tag push, and the gate has passed.
- **`--prerelease` is never passed.** Nothing in this project defines that flag, and an undefined
  flag on a user-facing page is a claim without a gate.
- **`gh`, not a third-party action** (RFC 056 §3.3). Do not add one. If `gh` is absent from the
  runner image, stop and report — do not install a tool into the job that holds `contents: write`
  without the identity discipline RFC 046 requires.

### 3.2 Pin the action yourself, by identity

**I am not giving you a commit SHA for `download-artifact`, because I would be inventing it.**
Obtain it from the upstream release you intend to use, and **verify that the SHA you write resolves
to the tag you name in the comment** before committing — that verification is the whole of RFC 046's
discipline applied to an action rather than a binary. Record in your request how you obtained and
checked it.

### 3.3 A re-run must not fail the workflow

`gh release create` fails if a Release already exists for the tag, and `workflow_dispatch` plus a
re-run make that reachable. **Measure the behaviour before choosing**: either guard with
`gh release view "$TAG"` and `gh release edit --notes-file` on the existing one, or accept the
failure as correct. Pick one, state which, and say what you observed — do not implement both.

## 4. S4 — tests

On `render_release_notes` and the section extractor, pure over CHANGELOG text, in
`package.rs`'s test module:

1. a well-formed section renders, and **the output contains no `Release status:` line**;
2. the section ends **before** the next `## [` heading — assert the following version's subtitle is
   absent, not merely that the length looks right;
3. a version with **no** section → `Err`;
4. **two** sections for one version → `Err` (the shape `release-gate`'s own preflight already
   rejects for headings, asserted here for the renderer too);
5. a **fourth** `Release status:` phrasing → `Err`, through `parse_changelog_release_status`, not
   through a second check;
6. `digests == None` → the rendered notes contain **no** anchor block and no empty hash (§2).

## 5. Verification, and what to hand back

1. `cargo xtask check` → **PASS, 20 gates**, on the gate's own exit status, never through a pipe.
   **Note:** 19 of 20 pass before architect review 127 E1's fix lands; if that slice has not
   landed, `size-budget` fails for an unrelated reason — say so rather than treating it as yours.
2. `cargo xtask release-gate --intended-tag <the then-current version>` → PASS, whole stream to a
   file, **and quote `RELEASE_NOTES.md` in full in your request.** It is user-facing text; it is
   reviewed as text, not as a file that exists.
3. `actionlint`, or at minimum `gh workflow view release`, to confirm the YAML parses — a broken
   release workflow is only discovered at a tag otherwise.
4. **Prove the ordering holds without pushing a tag.** State how `needs:` and the `if:` guard make a
   Release unreachable when the gate fails and when the ref is not a tag; a dry-run on a scratch
   branch with the create step replaced by `echo` is acceptable evidence, and say that is what it is.
5. **Do not create a real GitHub Release.** The first real one happens at the next authorized tag,
   by the architect and the owner. If you believe the job cannot be reviewed without one, say so and
   stop.
6. A `CHANGELOG.md` entry under the current unreleased heading, stating what the page will carry and
   that no artifact is attached.
7. The review request at
   `.git-exclude/review-request/RFC-056-a-release-page/REVIEW_REQUEST.md`, with §3.2's pin
   verification, §3.3's measured choice, and the rendered notes.

**Not yours:** moving RFC 056 to `rfcs/done/`, its Status line naming Amendment 1, the apex
`Implemented scope` field, the release table, any tag, any push, publication, and the first real
Release.
