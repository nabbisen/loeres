# RFC 046 follow-up F — hold the pin table itself, and stop certifying completeness

**From:** architect
**To:** implementer
**Date:** 2026-10-29
**Governing text:** `rfcs/accepted/046-pinned-tools-installed-by-identity.md`;
architect review 112 §2–§3 and §5.
**Base:** `e3819a1` (RFC 046 E — accepted by architect review 112).

E closed F5: all six of its predicates fail when the claim each names is broken, which I
re-derived myself with the masking occurrences left in place. This slice is **not** a correction
of E. It closes two things E did not cover.

**F7.** `.github/actions/install-pinned-tool/action.yml:9-16` nominates its own comment table as
the source of truth — *"that sidecar is signed by nobody and served by the same party as the
asset, so it is not the source of truth here — this table is."* Follow-up C held that table's
**hash** column, with the stated reason that it catches the two copies in one file drifting
apart. The asset-name and byte-count columns of the same table are held by nothing. Editing the
table row alone, leaving the operative `case` arm correct, passes the test today — I ran it; the
table can be made to lie about which asset was reviewed without failing anything.

**F8.** E's own CHANGELOG entry ends by certifying that *"every tracked claim about it — hash and
version, in all five files — is checked against it by a predicate that names the claim"*. F7 is a
tracked claim about a pinned tool's version, in one of those five files, checked by nothing. That
sentence is the same shape as D's "RFC 046 is complete", which E was written to correct.

Scope: `xtask/src/checks/release_gate.rs`, `CHANGELOG.md`. Nothing else.

## 1. The predicate to add (F7)

One per tool, in `release_workflow_and_its_pinned_tool_table_match_what_was_reviewed`, next to
C's hash loop. It is a **row** predicate, not a count and not a bare-string presence check: find
the table's row for that tool, assert there is exactly one such row, then assert that row carries
all three reviewed cells.

Hold these literals. Every one of them is already in the tree; check each against the file
before you write the assertion (item 6):

| tool | asset cell | bytes cell | hash cell |
| --- | --- | --- | --- |
| `cargo-deny` | `cargo-deny-0.20.2-x86_64-unknown-linux-musl.tar.gz` | `4936832` | `9f12ed4c49936e09b48bf862b595cde2fe64fcbd9d74dfacac6131ca824c8d5f` |
| `mdbook` | `mdbook-v0.5.4-x86_64-unknown-linux-musl.tar.gz` | `4944048` | `5222beabd3e37dc5be0d18ff99b79058469354db5c220153a1b92db5ba12be89` |

Locate the row by its line prefix — the lines are `# | cargo-deny   | …` and
`# | mdbook       | …`, so a trimmed `starts_with("# | cargo-deny")` / `("# | mdbook")` selects
one line each and matches neither the header row nor the `# | ---- |` separator. Do **not** match
on the column padding: a row predicate that encodes how many spaces pad a cell fails the next
time someone realigns the table, which is not a defect.

Three properties this must have, all of them things earlier slices got wrong:

1. **Exactly one row per tool.** Use a `filter(...).count()` of 1, with its own message, before
   reading the row — not `find(...).unwrap()`. A duplicated table is the failure C's exact-count
   form exists to catch, and `find` silently checks only the first copy.
2. **Cells asserted against this handoff's literals, never derived from the file.** Do not parse
   the version out of the URL, or the row out of the `case` arm, to compare the two to each
   other: two copies of the same mistake agree.
3. **The failure message names the cell, not the file.** `"action.yml's pin table row for
   cargo-deny must still carry the reviewed byte count 4936832"`, not `"action.yml is stale"`.
   Review 111's finding was a message that promised more than its predicate delivered; a message
   that says which cell is wrong is the thing that makes the next failure diagnosable.

Keep C's hash exact-count-2 loop exactly as it is. It is not redundant with the row predicate:
the row pins the table's copy and E's URL predicate pins the operative copy, while the count-2
additionally rules out a stray third copy of a hash anywhere in that file. No churn.

## 2. What must stay unasserted, and be said so (F7, second half)

`action.yml:14-15` (*"the tag forms and asset-name forms differ between the two tools (`0.20.2`
vs `v0.5.4`)"*) and `:25` (*"`cargo-deny-0.20.2-x86_64-unknown-linux-musl/cargo-deny`"*) restate
the table in explanatory prose. **Leave them unasserted.** Holding prose verbatim means the next
person to improve that paragraph fails a test for improving it.

Say so in the test's own comment, in two sentences at most: that the table row and the operative
URL are held, that the surrounding explanatory comments are deliberately not, and why.

**Do not add this to `docs/src/development.md`'s "What is deliberately exempt" list.** I
considered it and decided against it: that list's six items are consequences a contributor must
know, and a test-design note would dilute a list whose value is that every line earns its place.
The comment beside the predicate is where the next person to touch the predicate will read it.

## 3. The CHANGELOG (F8, and review 112 §5 nit 1)

Three edits to `CHANGELOG.md`, all inside the unreleased section:

1. **Delete E's closing completeness sentence** — *"After this slice, each pinned tool has exactly
   one operative home and every tracked claim about it … rather than merely the string."* Replace
   it with what is actually held, in the register of the rest of the entry, and **stop there**. A
   CHANGELOG entry says what changed; it is not the place to certify that nothing remains. No
   sentence in F's own entry may assert that RFC 046 is complete, or that every claim is now
   checked, or anything of that shape — including about F itself.
2. **Rewrap D's entry.** Correcting it in place left `(C) named` / `the pinned version everywhere
   a human is told to install the` / `tool,` ragged where the neighbouring entries wrap evenly. No
   gate covers this; it is cosmetic and it is in the file you are already editing. Change no word
   of its content.
3. **Add F's own entry**, naming review 112 F7 and F8, and saying plainly that E's completeness
   sentence was wrong and is removed rather than amended — the same honest form E used for D's.

## 4. What must not change

- **Either pinned version.** `cargo-deny` stays `0.20.2`, `mdbook` stays `0.5.4`.
- **`action.yml`'s logic, or any of its text.** This slice asserts that file; it does not edit it.
  If a literal in §1's table does not match the file, **stop and report it** — do not edit the
  file to fit the assertion.
- **`.github/workflows/`**, C's hash assertions, E's six predicates, and the `cargo publish
  --dry-run` failure example in `docs/src/development.md` (it documents a real error message).
- **No new gate, no tag.** Both remain out of scope; the tag is the architect's and the owner's.

## 5. Evidence required in the review request

1. The usual five logs plus `release-gate --intended-tag 0.24.1` exit status, checked directly
   rather than through a pipe.
2. The CHANGELOG entry, quoted in full, including the rewrapped D entry and E's entry with the
   deleted sentence shown as deleted.
3. **Each cell exercised individually** — six breaking edits, two tools × three cells. For each:
   the single line edited, the test's failure message, and the confirmation that the operative
   `case` arm (`:44`, `:48`) and the other two cells of that row were left untouched. Print the
   untouched lines **before** each run, not after. An assertion is exercised by breaking the thing
   it names, not the file it reads — this is the third time that sentence appears in an RFC 046
   handoff and the reason the previous two rounds happened.
4. **A seventh exercise:** delete the `cargo-deny` table row entirely and confirm the
   exactly-one-row assertion fires with its own message, not an index panic or an `unwrap`.
5. The six literals in §1's table, each checked against the tree before any assertion was written,
   with the occurrence count found for each.
6. Anything in §1 that does not match the tree, reported rather than fixed.
7. If `supply-chain` fails on an advisory unrelated to this slice, report it; do not change a
   dependency.

## 6. On whether this is the last slice

Do not say. I told the owner D was the last slice and was wrong, then did not say it of E and was
right not to. The question has an answer in the tree, not in a closing sentence: after this slice
every cell of the pin table, both operative URLs and all four human-facing claims are held by a
predicate that names them, and two explanatory comments are exempt in writing. Whether that is
complete is mine to judge against the tree when I review it — state what you held and what you
left, and let me do that.
