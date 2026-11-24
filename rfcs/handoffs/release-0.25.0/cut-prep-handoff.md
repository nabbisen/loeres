# Release 0.25.0 — cut preparation

**Author tier.** `architect`. **Date:** 2026-11-24.
**Governing text.** `rfcs/handoffs/release-0.25.0/cut-plan.md`, §5 steps 1-3. The owner authorized
the cut and publication on 2026-11-24, and said the dev team may help with the preparation while
**the cut itself stays with the architect and the owner.**
**Base.** `e38fc44` — review 132 and the cut plan.
**Release.** `0.25.0`. **No version bump** — RFC 055's slice performed it; §4 item 1 is a check.

## 0. The boundary, and why it falls where it does

This is **not** a new rule. `docs/src/development.md:482-497` already records it: *"An accepted
slice is pushed; a tag is not the implementer's… Creating a tag is the architect's and the owner's,
never the implementer's… it selects the revision a release is cut from."* Publication is reserved
at `:238-242`. So the mechanical edits below are ordinary slice work, and nothing in this handoff
adjusts the documented convention.

**Yours (this slice):** the three edit groups in §1-§3, the gate runs in §5, and a review request.
Commit and, once accepted, push — as with any slice.

**Not yours, and the reason is in each case what the act means rather than who may perform it:**

| not yours | why |
| --- | --- |
| **dating the `CHANGELOG.md` heading** | the date is the release's own date, and it is not knowable while preparing. §1.3 leaves the heading at `— unreleased` **deliberately**; the architect's cut commit dates it, which is also what makes that commit the finalization revision `development.md:400` describes |
| the RFC 028 anchor, the tag, the tag push | `development.md:482-497` |
| publication, and the whole post-release reconciliation | `development.md:238-242`; the post-release commit also depends on the publication outcome, which has not happened when you finish |

## 1. `CHANGELOG.md` — two content corrections, and the heading left alone

### 1.1 The lead paragraph must name both RFCs

Old, exactly:

```text
Repository release `0.25.0` carries RFC 055 (no public module that declares nothing), and is a
minor because that is a source break (see below).
```

New:

```text
Repository release `0.25.0` carries RFC 054 (a suitability claim a gate can assert) and RFC 055
(no public module that declares nothing), and is a minor because RFC 055 removes four public
paths, which is a source break (see below).
```

Both RFCs ship in this release and the entry currently names one.

### 1.2 Architect review 132 F1 — "exactly these six" is wrong, and invites a reader to check

Old, exactly (the closing sentence of the *"What is left, and why it is not a residue"*
paragraph):

```text
A reader who greps the word themselves will
find exactly these six, deliberately kept.
```

New:

```text
Two more are RFC 008's own section title and its
node in the dependency graph, at `loeres-roadmap-milestones-v1.md:726` and `:1143` — historical
text, not claims. A reader who greps the word themselves will find **ten** occurrences, of which
these **eight** are deliberately kept.
```

**Place it so the sentence it replaces is the paragraph's last**, and check the surrounding
sentence about the two disclaimers still reads correctly after the insertion — the paragraph now
names six non-defects, two disclaimers, and two historical titles, and the count must be the one a
reader actually finds.

**Measured, and the method Amendment 1 adopted — run it with no limit and state the count:**

```sh
git grep -ni "high-throughput\|high throughput" -- . | grep -v '^rfcs/' | grep -v '^CHANGELOG.md' | wc -l
```

→ **10** on this base. Quote the count and the full listing in your request. **If it is not 10,
stop and report** rather than adjusting the sentence to whatever you find: the number in a
user-facing record has to be the number, and a mismatch means something else moved.

### 1.3 The heading stays `## [0.25.0] — unreleased`

Do not date it, do not add a subtitle, and do not touch `**Release status:** unreleased`. §0 says
why.

## 2. RFCs 054 and 055 to `rfcs/done/`

`git mv` both out of `rfcs/accepted/`, and set each Status line to **exactly**:

```text
**Status.** Implemented (v0.25.0). Amendment 1 was made while Accepted, under RFC 000's in-place-amendment rule.
```

**Both carry an Amendment 1, and this is the first cut to move two amended RFCs.** `check-rfcs`
requires a `done/` RFC's Status line to name its amendments, and `doc-currency` **forbids** that
clause while the RFC is in `accepted/` — so the Status edit is correct only as part of the move,
and the move and the edit belong in one commit.

Then `rfcs/README.md`: move both index rows to the Done table, status `Implemented (v0.25.0)`,
rendering their links from `accepted/` to `done/`. **`None currently.` stays under
`## Proposed RFCs`, and the Accepted table keeps 047 and 056** — 047 is a destination, and 056 is
accepted and deliberately sequenced after this release.

## 3. The apex `Implemented scope` in seven documents

`RFCs 001-046, 048-053` → **`RFCs 001-046, 048-055`**, byte-identical in all seven `APEX_DOCS`
(`xtask/src/checks/doc_currency.rs:17-25`), or `check_apex_currency` reports `APEX MISMATCH`.

The third gapped scope string this project has carried; 047's gap is expected and correct.

**`This tree` stays `0.25.0` and `Last reconciled` stays `0.24.2`** in all seven. RFC 024's
inequality is strict and the release has not happened.

## 4. Check, do not edit

Confirm and report each; **none of these is an edit in this slice**:

1. the workspace version and the five internal `[workspace.dependencies]` pins are `0.25.0`;
2. seven apex `This tree: 0.25.0`; seven `Last reconciled: 0.24.2`;
3. exactly one `## [0.25.0]` heading in `CHANGELOG.md`;
4. **`README.md:25,26,29,30` still read `"0.24"`** — they must **not** change in this slice. The
   gate binds them to the `MAJOR.MINOR` of `Last reconciled`, which is still `0.24.2`; they move to
   `"0.25"` in the post-release commit, which is not yours. If you change them here,
   `doc-currency` fails with four `README VERSION` findings.

## 5. Verification

1. `cargo xtask check` → **PASS, 20 gates**, on the gate's own exit status, never through a pipe.
2. `cargo xtask release-gate --intended-tag 0.25.0` → PASS, whole stream to a file. The undated
   heading satisfies it: the preflight requires exactly one `## [0.25.0]` heading matching the
   workspace version, which `## [0.25.0] — unreleased` does.
3. **Do not create a tag.** `release-gate --intended-tag` validates an *unused* intended tag and
   creates nothing; that is the whole reason it takes the flag.
4. `rustc -vV` and `cargo -V` quoted — the last four slices each moved under a toolchain that had
   changed, and the cut will be measured against whatever you report.
5. The review request at
   `.git-exclude/review-request/release-0.25.0-cut-prep/REVIEW_REQUEST.md`, with §1.2's count and
   listing, §4's four checks, and the two Status lines quoted as they landed.

**After I accept it, push it** — an accepted slice is pushed as part of closing it
(`development.md:482`). The cut commit then lands on top of it.
