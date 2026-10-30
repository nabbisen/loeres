# RFC 052 - A Release Table That Cannot Omit A Release

**Status.** Proposed

**Author tier.** `architect`.

**Governing scoping.** RFC 047 property **G** — *no current-facing document may state a version,
release list or scope the workspace contradicts, and that is asserted mechanically* — applied to
the one page the README sends a reader to for release currency. Architect review 115 §5 F5 found
these claims unheld and verified a bounded predicate satisfiable; it has been carried, unhanded,
through two slices. Every figure here was measured on the tree at `438c5d3`, `0.24.1`.

RFC 048 deleted a hand-maintained release list from seven documents because it had rotted. It left
the one home that list was deleted *in favour of* hand-maintained.

## 1. Summary

`docs/src/specifications.md`'s `## Release currency` is, by its own first line, *"the one home for
release currency"*, and the README links to it. Directly beneath its machine-held apex block sit an
**eleven-row release table** and a **`What is on crates.io.`** paragraph, both maintained by hand,
both making exactly the class of claim RFC 048 found five releases out of date in seven other
documents.

They are correct today. Nothing asserts that they stay correct, and the apparatus that would have
caught a stale row is four lines away in the same file, reading a different part of it.

**Measured, and this is why the slice is small:** the table's eleven rows are *exactly* the
versions `CHANGELOG.md` marks `released`, in ascending order, and ten of eleven `Shipped` cells
already equal that release's tag date. The assertion below is satisfiable by changing **one table
cell**.

## 2. The defect, measured

### 2.1 What is held and what is not

`docs/src/specifications.md:16-20` carries the RFC 024 apex block, machine-held by `doc-currency`
in all seven `APEX_DOCS` since RFC 048: this tree `0.24.1`, last reconciled `0.24.0`, implemented
scope `RFCs 001-044`.

`:27-39` is the release table. `:41-44` is the `What is on crates.io.` paragraph. Neither is read
by any gate. `doc-currency`'s only coverage of this file is `check_stale_ledger`'s blocklist of
phrases — each added after an earlier incident — and `check_release_local_paths`, which asserts
that six marker strings are *present*. **A row stating the wrong scope, or a missing row, is
invisible.**

That is the RFC 048 defect, one layer down: twenty gates pass while a hand-maintained release list
sits beside a correct machine-held block. RFC 048 removed the duplicated list from seven documents
and designated this section its home; designating a home does not assert what the home says.

### 2.2 The table's rows are already a derivable set

Measured over `CHANGELOG.md`'s `**Release status:**` lines, whose three permitted values
`doc_currency.rs:229` already owns:

| status | versions |
| --- | --- |
| `released` | `0.20.2`, `0.21.0`, `0.21.1`, `0.21.2`, `0.21.3`, `0.22.0`, `0.22.1`, `0.22.2`, `0.22.3`, `0.23.0`, `0.24.0` |
| `withdrawn candidate` | `0.20.1` |
| `unreleased` | `0.24.1` |

Thirty-three further `CHANGELOG.md` headings — `0.20.0` and below — carry **no** status line at
all.

The table's eleven rows are that `released` set exactly, in that order. So the predicate needs no
window constant, no date list and no hand-written expectation: **the row set is the released set,
and the gate can derive it.**

### 2.3 The `Shipped` column is already the tag date, in ten of eleven cells

| table row | `Shipped` cell | `CHANGELOG.md` tag date | |
| --- | --- | --- | --- |
| `0.20.2` | `released` | `2026-07-22` | **differs** |
| `0.21.0` … `0.24.0` (ten rows) | the date | the same date | match |

One cell says `released` where the other ten carry a date. That single inconsistency is the whole
document change this RFC needs.

### 2.4 The last row and the paragraph restate the apex block

The table's last row is `0.24.0` carrying `001-044`; the apex block says last reconciled `0.24.0`,
scope `RFCs 001-044`. The paragraph says the newest published version of every crate is `0.24.0`
and that it carries RFCs `001-044`. Three statements of two values, one of them machine-held and
two not.

## 3. What to do

### S1 — one cell

`docs/src/specifications.md`'s `0.20.2` row: `released` becomes `2026-07-22`, the tag date
`CHANGELOG.md` records. Nothing else in the table changes. The column header already reads
`Shipped`; if a word is needed to say the cell is the tag date, put it in the `Publication status`
blockquote above the table, which already distinguishes *distributed* from *published*.

### S2 — the assertion

One function in `xtask/src/checks/doc_currency.rs`, joining the gate that already parses both
sources. **No new gate.** It reads `docs/src/specifications.md`'s release table and asserts:

1. **Row set symmetry, both directions.** The table's versions are exactly the versions
   `CHANGELOG.md` marks `released` — a missing row and an extra row both fail, each naming the
   version. Reuse `parse_changelog_release_status`; do not write a second status parser.
2. **Ascending order**, by `parse_version_triple`, which the file already has.
3. **Each `Shipped` cell is that release's tag date** as `CHANGELOG.md` records it.
4. **The last row is the apex block's lineage.** Its version equals `Last reconciled repository
   release` and its `Carries RFCs` cell equals the range in `Implemented scope`. The value comes
   from `check_apex_currency`'s return, which RFC 049 S4 already threaded through
   `validate_repository` — parse the block once, not twice.
5. **The `What is on crates.io.` paragraph names the same two values** — that version and that RFC
   range.

Every failure names the row or the paragraph and the two values it is between. **Fail closed on a
table shape the parser does not recognise**: a row that does not match the expected three-column form
is a finding, never a skipped line, for the reason RFC 049 §4.2 gives.

### S3 — the record

One row in `docs/src/documentation-convention.md` §5, beside the README rows RFC 049 added: the
release table and the crates.io paragraph agree with `CHANGELOG.md` and the apex block, enforced by
`doc-currency`.

## 4. What must not be swept in

Three claims in that same section are **not** currency and must survive untouched. Naming them is
half this RFC's value, because a predicate written without reading them would break true
statements:

- **`:46-50`, the `Q32` migration advice.** *"If you use `Q32` with the constrained kernel, move
  your requirement to `0.24`"* is a true, permanent statement about the release a fix landed in. It
  names a minor and must keep naming that one. **It is not a currency claim and no assertion may
  require it to track the released minor.**
- **`:52-56`, the never-published paragraph.** `0.21.0` through `0.22.0` were tagged and
  distributed but never published, and the paragraph says why they stay that way. Historical, and
  partly about crates.io's own behaviour, which this repository cannot assert.
- **`### Historical release notes` at `:61` onward**, including the `v0.20.0` note and the sentence
  that already says the shipped-contract list has grown since. Explicitly historical; RFC 024's
  current-before-historical boundary covers it and `current_before_historical` in
  `doc_currency.rs:785` is how the existing checks respect it.

## 5. Non-goals

- **No crates.io query, no network.** Clause 5 asserts that the paragraph agrees with this
  repository's own record, never that the registry agrees with either. What is actually on
  crates.io is outside anything a gate here can know (RFC 026's offline discipline).
- **No change to `CHANGELOG.md`.** It is the record the rest now defers to, as RFC 048 left it.
- **No new gate, no new status vocabulary, no version change, no tag.**
- **No change to the apex block, `APEX_DOCS`, or RFC 049's README predicate.**
- **No restructuring of `## Release currency`** beyond S1's one cell. Its blockquote, its headings
  and its link targets stay.
- **No assertion over the thirty-three statusless `CHANGELOG.md` headings.** They predate the
  status convention; demanding one would rewrite release history to satisfy a gate.

## 6. Risks

- **Functional — Low.** One table cell and one predicate.
- **Business — Low.** The page a user is sent to for "which release carries what" becomes one that
  cannot silently omit a release.
- **Operational — Low, and lower afterwards.** Today a release cut must remember to add a row;
  afterwards the gate says so. The cost is that a cut cannot land until the row is added — which is
  the point, and is the same trade RFC 048 took.
- **Security — Low.** No channel, no artifact, no trust root. One sentence only because the
  crates.io paragraph is where a user learns what to install: a stale version there sends someone
  to an older release than they think, which is a supply-chain-adjacent confusion even though no
  control changes.

## 7. What I could be wrong about

- **Clause 3 assumes `Shipped` means the tag date.** Ten of eleven cells support that reading and
  the eleventh says `released`, so the column's meaning is inferred from its contents, not from a
  statement anywhere. If `Shipped` was meant to be the *distribution* date, the two coincide in
  every row today and the clause is still satisfiable — but it would be asserting the wrong thing
  for the right result, and the blockquote above the table should then say which it is.
- **Clause 1's symmetry assumes the table is meant to be complete over released versions.** It is
  complete today. If a future release were deliberately left out of the table, this assertion would
  have to be withdrawn rather than worked around.
- **This has been carried for two slices.** Review 115 §5 found it on 2026-10-29 and I twice judged
  other work ahead of it. Both judgements were about an instruction wrong *now* outranking claims
  correct now, which I still think was right — but the honest statement is that the claims have
  been unheld for the whole of that time, not that the delay was free.
