# RFC 050 implementation handoff — a link the book can follow

**From:** architect
**To:** implementer
**Date:** 2026-10-30
**Governing text:** `rfcs/accepted/050-a-link-the-book-can-follow.md`, accepted by the owner on
2026-10-30.
**Base:** `main` at the revision that accepted RFC 050.

**Scope:** `xtask/src/checks/link_audit.rs`, two small changes in
`xtask/src/checks/check_rfcs.rs`, and one row plus one sentence in
`docs/src/documentation-convention.md`. **No new gate, no library change, no version change, no
tag.**

**Order:** this slice is disjoint from RFC 051 and RFC 052, which both touch
`xtask/src/checks/doc_currency.rs`. Take it first or in parallel with them; those two run in
sequence with each other.

## 0. Read this first

**0.1 Both assertions are already satisfied.** Measured on the tree at `e8eff6a`: of the links in
the fourteen Markdown files under `docs/src/`, fenced blocks excluded, **26** resolve inside
`docs/src/`, **0** resolve outside it, 19 are absolute and 1 is anchor-only; and across every
tracked Markdown file in the repository, **0** links sit inside an inline code span. So **no
document changes** and `[check]` must still be green at the end with nothing but the convention
edit. If either count differs when you measure it, that is a finding — report it before writing
code.

**0.2 The defect this exists for was real and is already corrected.** At `e328aea`,
`docs/src/getting-started.md` linked the repository root's `README.md` with the target
`../../README.md`; `link-audit` passed and the rendered book carried `href="../../README.html"`.
It was corrected at `438c5d3`. **Exercise 1 of §5 restores that exact target and must fail the
gate** — that is the regression this slice buys, and the one result I will look at first.

**0.3 Everything below was read in the tree before being written.** Line numbers are from
`e8eff6a`. Report anything that does not match rather than working around it — five of my
instructions across the last three slices have been wrong, and every one was caught that way.

## 1. S1 — a book link stays inside the book

### 1.1 Where it goes, and the one visibility change

`check_markdown_links` at `xtask/src/checks/check_rfcs.rs:378-411` is shared: `check-rfcs` calls
it with `rfc_markdown_files()` (RFCs only, `check_rfcs.rs:26`) and `link-audit` calls it with
every tracked Markdown file. **The book boundary is `link-audit`'s alone**, so S1 lives in
`xtask/src/checks/link_audit.rs` and `check_markdown_links` is not modified.

S1 needs two things that file already has:

- `markdown_links` (`check_rfcs.rs:413`) is already `pub(crate)`. Use it; do not write a second
  extractor.
- `external_or_anchor` (`check_rfcs.rs:440-445`) is private. **Change it to `pub(crate)` and
  nothing else.** Re-implementing the predicate would give the two checks two ideas of what an
  external link is, which is the defect class this whole slice is about.

### 1.2 The predicate, exactly

A pure function over a path and a source, so it is testable on literals:

```rust
const BOOK_SRC: &str = "docs/src";

fn book_boundary_findings(path: &Path, source: &str) -> Vec<String>
```

1. If `path` is not inside `BOOK_SRC`, return no findings. **The rule is the book's alone** — every
   other document in the repository is read where it lives, and a relative link out of `rfcs/` or
   the root is correct there.
2. For each target from `markdown_links(source)`: skip it if `external_or_anchor(target)`; take
   the part before `#`; skip it if empty.
3. Resolve it as `check_markdown_links` does — against the containing file's parent — and then
   **normalise lexically**: drop `.` components and pop the previous component on each `..`.
4. If the normalised path is not inside `BOOK_SRC`, that is a finding.

**Normalise lexically, not with `canonicalize`.** `canonicalize` requires the target to exist and
follows symlinks; this assertion must fire on a link that escapes the book *whether or not* the
file it names is present, and must not depend on the filesystem at all — that is what makes it
unit-testable. `Path::join` leaves `..` in place, so the popping is yours to do.

Worked example, which is F1: base `docs/src`, target `../../README.md` → joined
`docs/src/../../README.md` → components `docs`, `src`, `..`, `..` → pop `src`, pop `docs` →
`README.md` → not inside `docs/src` → **finding**.

Note the paths `collect_md` produces begin with `./` (it is called with `Path::new(".")`), so the
normalisation must drop that leading current-directory component before the prefix test, or every
book file will look like it is outside the book.

### 1.3 The message

Follow the file's existing register — `check_markdown_links` prints `  BROKEN LINK at {path}: …`.
Use a distinct prefix, name the containing file, the target **and where it resolved to**, and add
the one clause a maintainer needs to fix it:

```
  BOOK BOUNDARY at docs/src/getting-started.md: `../../README.md` resolves to `README.md`,
  outside docs/src/ — a chapter names a repository path in backticks and links its absolute
  moving-branch URL instead (docs/src/specifications.md does this)
```

One line is fine if it fits; the guidance clause is not decoration. A reader who hits this will
otherwise reach for a deeper relative path, which is the same defect again.

`run()` reports `ok = check_markdown_links(&files) && <the new findings are empty>` — **evaluate
both, do not short-circuit**, so one book-boundary failure does not hide the broken-link report or
the other way round.

### 1.4 Tests

`xtask/src/checks/link_audit.rs` has no `mod tests` today. Add one.

1. a `docs/src/` file linking a sibling `.md` — no finding;
2. **a `docs/src/` file whose target is `../../README.md` — one finding, naming the target.** This
   is F1 as a unit test and is the point of the slice;
3. the same target in a file **not** under `docs/src/` — no finding;
4. an absolute `https://` target and an anchor-only `#section` target — no findings;
5. a target carrying an anchor, `architecture.md#boundaries` — no finding, so the anchor is
   stripped before resolving;
6. a target that escapes and does not exist, `../nonexistent/x.md` — a finding, proving the
   assertion does not depend on the file existing.

## 2. S2 — a link inside an inline code span is not a link

### 2.1 The change

In `markdown_links` (`check_rfcs.rs:413-438`), mask inline code spans before scanning a line for
link targets. Everything else is unchanged: the fenced-block toggle at the top, the scan, and the
`BTreeSet<String>` return.

**The rule, exactly — only matched backtick pairs are masked.** Walk the line; on a backtick, look
for the next backtick; if there is one, the text between them is a code span and is skipped; if
there is not, the rest of the line is ordinary text and **is** scanned. This is the rule the
measurement in RFC 050 §2.3 used, so the figure of zero code-span links in the tree is the figure
for this rule and not for a different one.

**Do not implement CommonMark's backtick-run matching.** A link inside a multi-backtick span stays
resolved, which is the conservative direction — it keeps checking something it checks today — and
§6 of the RFC records it as a known limitation. Pin it with test 6 below rather than fixing it.

### 2.2 Tests

In `check_rfcs.rs`'s existing `mod tests` (`:447`), where `markdown_links` lives:

1. a line containing only a backticked link — no target returned;
2. a line containing a real link — the target returned, unchanged;
3. a line containing both — only the real one;
4. a line with an unmatched backtick before a real link — the real target still returned;
5. a link inside a fenced block — still skipped (the existing behaviour, pinned so S2 cannot break
   it);
6. a link inside a **double**-backtick span — still returned, with a comment naming it as the
   documented limitation of §2.1 rather than an oversight.

## 3. S3 — the record

`docs/src/documentation-convention.md` §5's table gains one row beside the README rows RFC 049
added: a `docs/src/` link resolves inside `docs/src/`, enforced by `link-audit`.

And one sentence where that file states the book's link discipline, saying **why**: a chapter is
read as rendered HTML, so a link out of the book is dead even when the file it names is present in
the repository. Write it in that file's register. Say nothing about mdBook's rewriting mechanics —
the reason a maintainer needs is "the book is not the repository", not the tool's behaviour.

## 4. What must not change

- **`check_markdown_links`'s resolution, its message, or its contract.** Only
  `external_or_anchor`'s visibility and `markdown_links`'s code-span masking.
- **Any document under `docs/src/`.** All 26 relative links already resolve inside the book; if
  you find one that does not, report it (§0.1).
- **`collect_md`, `skip_dir`, and the set of files either gate scans.**
- **No book build, no `mdbook` dependency, no network access, no external-link checking**
  (RFC 050 §4).
- **No anchor validation.**
- **No new gate. No version changed. No tag.**

## 5. Evidence required in the review request

1. The usual five logs, plus `release-gate --intended-tag 0.24.1` exit status checked directly
   rather than through a pipe.
2. **§0.1's four counts, re-measured by you**, with the command you used: 26 inside, 0 outside,
   19 absolute, 1 anchor-only over `docs/src/`, and 0 code-span links repository-wide.
3. The CHANGELOG entry, quoted in full. It says what is now asserted, that the defect it closes
   shipped once and was corrected by hand, and that both assertions were satisfied on adoption.
   **No completeness claim** — in particular, do not say the book's links are now verified; S1
   asserts a boundary, not that nineteen absolute URLs resolve.
4. **The twelve unit tests of §1.4 and §2.2**, named, with tests 2, 4 and 6 of §2.2 and test 2 of
   §1.4 called out — they are the regression test, the two conservative directions, and the
   pinned limitation.
5. **Five exercises against the committed tree**, each reverted, each with the gate's own message
   quoted:

   | # | break | expect |
   | --- | --- | --- |
   | 1 | in `docs/src/getting-started.md`, restore F1: make the README pointer a relative link whose target is `../../README.md` | `BOOK BOUNDARY`, naming that file and target; `link-audit` FAIL |
   | 2 | add a link in any `docs/src/` chapter whose target is `../../CHANGELOG.md` | `BOOK BOUNDARY`, naming that chapter |
   | 3 | add a link in a `docs/src/` chapter to an existing sibling chapter | **PASS** |
   | 4 | create a temporary Markdown file at the repository root containing a bracket-parenthesis link to a nonexistent file, **inside single backticks** | **PASS** — before S2 this was `BROKEN LINK`; delete the file afterwards |
   | 5 | the same temporary file with the same link **not** backticked | `BROKEN LINK`, from the existing check, unchanged by this slice |

   Exercise 1 is the one that matters: it is the committed defect, restored. Exercise 4 is the only
   proof that S2 changed anything, since the tree contains nothing it affects. Exercise 5 is its
   negative control — if it passes, S2 has masked too much.
6. Anything in §1-§3 that does not match the tree, **reported rather than fixed**.
7. If `supply-chain` fails on an advisory unrelated to this slice, report it; change no dependency.

## 6. Push

Per `docs/src/development.md`'s *"An accepted slice is pushed; a tag is not the implementer's"*:
once the review request is accepted, push without asking again. No tag.
