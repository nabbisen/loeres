# RFC 052 implementation handoff — a release table that cannot omit a release

**From:** architect
**To:** implementer
**Date:** 2026-10-30
**Governing text:** `rfcs/accepted/052-a-release-table-that-cannot-omit-a-release.md`, accepted by
the owner on 2026-10-30 and amended in place the same day by its own §0.1, which **corrects three
of its five assertion clauses**. The amended text is the authorized scope; architect review 118
records what was wrong and why.
**Base:** `main` at the revision that accepted RFC 052.

**Scope:** one cell in `docs/src/specifications.md`, one new assertion in
`xtask/src/checks/doc_currency.rs`, one row in `docs/src/documentation-convention.md`, and
`CHANGELOG.md`. **No new gate, no library change, no network access, no version change, no tag.**

**Order:** take this slice **after** RFC 051, which unifies a constant in the same file. RFC 050 is
disjoint from both.

## 0. Read §0.1 of the RFC before anything else

Three clauses changed after acceptance, and two of them would have failed the gate on a correct
tree. **Do not implement from any earlier reading of §3 S2.** In short:

1. **`Shipped` is the *distributed* date, not the tagged date.** Ten of eleven releases were tagged
   and distributed on the same day; `0.20.2` was tagged `2026-07-22` and distributed `2026-07-30`,
   and the release sentence RFC 048 deleted said *shipped 2026-07-30*. S1's cell is **`2026-07-30`**.
2. **The last row's scope is no longer compared to the apex `Implemented scope` for equality.**
   Those track different things and diverge during every release cut. The clause is now version
   equality plus a monotonic bound (§3.4).
3. **The crates.io paragraph is bound to the newest *published* version**, not the newest released
   one. Five releases of this project were distributed and never published.

## 1. What is there now, measured at `e8eff6a`

`docs/src/specifications.md`, section `## Release currency`:

| lines | what |
| --- | --- |
| `:16-20` | the RFC 024 apex block, machine-held in all seven `APEX_DOCS` |
| `:22-25` | the `Publication status` blockquote, which distinguishes *distributed* from *published* |
| `:27-28` | the table's header and separator |
| `:29-39` | the eleven rows |
| `:41-44` | the `What is on crates.io.` paragraph |
| `:46-50` | the `Q32` migration advice — **out of scope**, see §5 |
| `:52-56` | the never-published paragraph — **out of scope** |
| `:58-59` | the apex-specifications pointer |
| `:61` onward | `### Historical release notes` — **out of scope** |

`CHANGELOG.md` has **46** `## [X]` headings. **Thirteen** carry a `**Release status:**` line:

| status | versions |
| --- | --- |
| `released (…); published to crates.io` | `0.20.2`, `0.22.1`, `0.22.2`, `0.22.3`, `0.23.0`, `0.24.0` |
| `released (…); not published to crates.io` | `0.21.0`, `0.21.1`, `0.21.2`, `0.21.3`, `0.22.0` |
| `withdrawn candidate` | `0.20.1` |
| `unreleased` | `0.24.1` |

The other thirty-three — `0.20.0` and below — have **no status line**, and RFC 052 §5 excludes
them: do not assert anything over them and do not add status lines to them.

**The eleven released versions are exactly the table's eleven rows, in the table's order.** Verify
that yourself before writing the parser; if it is no longer true, that is a finding and probably the
defect this slice exists to catch.

## 2. S1 — one cell

`docs/src/specifications.md:29`, the `0.20.2` row: the `Shipped` cell `released` becomes
`2026-07-30`. **Nothing else in the table changes** — not a date, not an RFC range, not the header.

Optionally, one clause in the `Publication status` blockquote at `:22-25` saying the `Shipped`
column is the distribution date. I recommend it: the column's meaning is currently inferable only
from `CHANGELOG.md`, which is how I got it wrong (review 118 F1). Keep the blockquote's existing
two sentences byte-identical and add at most one.

## 3. S2 — the assertion

### 3.0 Shape and placement

One new function in `xtask/src/checks/doc_currency.rs`, called from `validate_repository`. **No new
gate.** It needs three things the file already has or already produces.

**On the line numbers below:** they are from `e8eff6a`, and RFC 051's slice lands in this same file
first. Treat the **names** as authoritative and the numbers as indicative; if one has moved, that
is RFC 051's doing, not a defect.

- `parse_changelog_release_status(source, version)` at `:237`, which returns the bare status token
  for one version. **Reuse it per version; do not write a second status parser.** It does *not*
  enumerate versions, so you add an enumerator over `## [X]` headings — one pass, same heading
  grammar the function already uses (`## [` + version + `]` followed by end-of-line or whitespace).
- `parse_version_triple` at `:131` for ordering.
- `check_apex_currency`'s return value, which RFC 049 S4 already threads through
  `validate_repository` as `last_reconciled`. **Parse the apex block once.** `implemented_scope()`
  at `:219` gives the scope string; take it from there rather than re-reading a document.

Keep the I/O in the caller and the predicate pure, as `readme_requirement_findings` does:

```rust
fn check_release_table(last_reconciled: Option<&str>, errors: &mut Vec<String>)
fn release_table_findings(
    specifications: &str,
    changelog: &str,
    last_reconciled: Option<&str>,
    implemented_scope: Option<&str>,
) -> Vec<String>
```

When `last_reconciled` or `implemented_scope` is `None` the apex block already failed and reported
itself: **skip the clauses that need those values and run the rest.** A second derived complaint is
noise — the same rule RFC 049 S4 applies.

### 3.1 Clause 1 — row-set symmetry, both directions

The table's versions are **exactly** the versions `CHANGELOG.md` marks `released`.

- a released version with no row: a finding naming the version — *this is the defect the slice
  exists for*;
- a row whose version `CHANGELOG.md` does not mark `released`: a finding naming the version and the
  status it actually has.

### 3.2 Clause 2 — ascending order

Consecutive rows ascend by `parse_version_triple`. A finding names the two rows involved.

### 3.3 Clause 3 — each `Shipped` cell is that release's distributed date

Extract the date from the status line's `distributed YYYY-MM-DD`. Validate it with `valid_date` at
`:621` rather than trusting the shape. A finding names the row, the cell and the `CHANGELOG.md`
value.

Note `0.20.2` is the only release where `tagged` and `distributed` differ, so **a parser that reads
`tagged` by mistake passes ten rows and fails one** — and that one row is the one S1 edits. If your
first run fails only on `0.20.2`, suspect the parser before the data.

### 3.4 Clause 4 — lineage, bounded not equated

1. the **last row's version** equals `last_reconciled`;
2. each row's `Carries RFCs` **maximum** is non-decreasing down the table;
3. the last row's maximum is **≤** the maximum in `implemented_scope`.

**Take the largest number appearing in the string**, not the second half of a single `NNN-NNN`
range: `render_scope` at `:158` emits comma-separated parts when `rfcs/done/` has a gap, so
`RFCs 001-040, 042-044` is a shape it can produce and a single-range parser would fail closed on a
correct tree.

Part 3 is an inequality on purpose. During a release cut, RFCs move to `rfcs/done/` before the
release is reconciled, so `implemented_scope` runs ahead of every row in the table; demanding
equality would fail the gate mid-cut and demand a false row (review 118 F2). **Exercise 4 of §7
simulates exactly that window** and is the one result I will read first.

### 3.5 Clause 5 — the crates.io paragraph

The paragraph at `:41-44` names the newest version whose status line says **`published to
crates.io`** — derived, not `last_reconciled` — and the RFC range it states equals **that
version's own row** in the table.

Match the two values by searching the paragraph's text for them, not by parsing its prose into a
grammar: the assertion is that the paragraph *states* the right version and range, and a prose
parser would be a third thing to keep correct. A finding quotes what the paragraph should contain.

### 3.6 Fail closed

A line between `:27` and the paragraph that looks like a table row but does not parse as the
three-column form — version, shipped, carries — is a **finding**, never a skipped line. The same
rule RFC 049 §4.2 states, and the reason is the same: a silently skipped row is how row-set
symmetry becomes vacuous.

### 3.7 Tests

Pure-function tests on literals, in `#[cfg(test)] mod tests`. At minimum:

1. the real table and the real status set — **no findings** (the committed tree);
2. a released version missing from the table — one finding naming it;
3. an extra row for a version marked `unreleased` — one finding naming it and its status;
4. two rows out of order — one finding naming both;
5. a `Shipped` cell carrying the **tagged** date where it differs from the distributed one — one
   finding, which is review 118 F1 as a test;
6. a last row whose version is not the reconciled release — one finding;
7. **the cut window: `implemented_scope` of `RFCs 001-052` with the table's last row at `001-044`
   and `last_reconciled` at `0.24.0` — no findings.** This is F2 as a test and must not be
   skipped;
8. a last row whose maximum **exceeds** the scope's maximum — one finding;
9. a `Carries RFCs` sequence that decreases — one finding;
10. a scope string with a gap, `RFCs 001-040, 042-044` — parsed to maximum `44`, no spurious
    finding;
11. the crates.io paragraph naming a released-but-unpublished version — one finding, which is F3
    as a test;
12. a malformed table row — the fail-closed finding of §3.6.

## 4. S3 — the record

`docs/src/documentation-convention.md` §5's table gains one row beside the README rows RFC 049
added and the book-boundary row RFC 050 adds: the release table and the crates.io paragraph agree
with `CHANGELOG.md` and the apex block, enforced by `doc-currency`.

`CHANGELOG.md` gains one paragraph: what is now asserted, that these claims sat beside a
machine-held block unheld since RFC 048 designated this section their home, and that the
`Carries RFCs` column is **bounded and ordered but not verified**, because nothing in the
repository records which RFCs a release carried. **No completeness claim beyond that** — in
particular do not write that the release currency page is now verified.

## 5. What must not change

- **`CHANGELOG.md`'s existing entries.** Not one character, including the thirty-three headings
  with no status line (RFC 052 §5). It is the record everything else defers to.
- **`docs/src/specifications.md:46-50`, the `Q32` migration advice.** *"move your requirement to
  `0.24`"* is a true, permanent statement about the release a fix landed in. **No assertion may
  require it to track any current value**, and nothing in this slice edits it.
- **`:52-56`, the never-published paragraph**, and **`### Historical release notes` from `:61`**.
- **The apex block, `APEX_DOCS`, `check_apex_currency`, `parse_changelog_release_status`,
  `implemented_scope`, `render_scope`, `valid_date`, and RFC 049's README predicate** — all reused,
  none edited.
- **No crates.io query and no network.** Clause 5 asserts the paragraph agrees with this
  repository's own record, never that the registry agrees with either.
- **No new gate. No version changed. No tag.**

## 6. One thing to report rather than fix

If you find that a row's `Carries RFCs` cell is **wrong** — not malformed, but stating a scope you
believe that release did not carry — **report it and change nothing.** The corrected clause 4
cannot detect that, §0.1 says so, and correcting release history is not a documentation slice's
business.

## 7. Evidence required in the review request

1. The usual five logs, plus `release-gate --intended-tag 0.24.1` exit status checked directly
   rather than through a pipe.
2. **§1's two measurements, re-run by you**: the thirteen status lines by value, and that the
   eleven `released` versions are exactly the table's eleven rows in order.
3. The CHANGELOG entry, quoted in full, with §4's "bounded and ordered but not verified" limit
   stated.
4. **The twelve tests of §3.7**, named, with 5, 7 and 11 called out — they are review 118's three
   defects as regression tests.
5. **Five exercises against the committed tree**, each reverted, each with the gate's own message
   quoted:

   | # | break | expect |
   | --- | --- | --- |
   | 1 | delete the `0.24.0` row from the table | clause 1, naming `0.24.0` as released with no row |
   | 2 | add a row for `0.24.1` | clause 1, naming `0.24.1` and its `unreleased` status |
   | 3 | set the `0.20.2` cell back to `2026-07-22` | clause 3, naming the row and the distributed date |
   | 4 | **the cut window**: move RFCs 045-052 into `rfcs/done/` and bump nothing else | **PASS** — `doc-currency` reports only the apex scope changes it already reports for `done/` movement, and **no release-table finding** |
   | 5 | change the crates.io paragraph's version to `0.22.0` (released, never published) | clause 5, naming `0.24.0` as the newest published version |

   **Exercise 4 is the decisive one** and the only one that proves F2 is fixed. Note it will change
   `Implemented scope` and so requires the apex block's scope field to move with it in all seven
   `APEX_DOCS` for the apex check to stay green — do that, confirm **no release-table finding
   appears**, and revert all of it. If the apex block's own requirements make the window
   unreproducible, say so and show the attempt; that is a finding about my exercise, not about your
   work.
6. Anything in §1-§4 that does not match the tree, **reported rather than fixed**.
7. If `supply-chain` fails on an advisory unrelated to this slice, report it; change no dependency.

## 8. Push

Per `docs/src/development.md`'s *"An accepted slice is pushed; a tag is not the implementer's"*:
once the review request is accepted, push without asking again. No tag.
