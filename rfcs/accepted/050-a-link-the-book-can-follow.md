# RFC 050 - A Link The Book Can Follow

**Status.** Accepted (design frozen 2026-10-30)

**Author tier.** `architect`.

**Governing scoping.** RFC 047 property **B** — *every claim a user can read is asserted by a
gate* — applied to the links in the published book. Architect review 117 F3 found this while
correcting F1, a dead link the twenty gates passed. Every figure here was measured on the tree at
`438c5d3`, `0.24.1`.

RFC 049 made the README's instruction one a user can keep. This makes the book's links ones a
reader can follow.

## 1. Summary

`link-audit` resolves a Markdown link relative to the file that contains it and checks that the
target exists in the **repository**. For a book chapter that is the wrong frame of reference: the
artifact a reader opens is the **rendered book**, where a relative `.md` target has been rewritten
to `.html` and where nothing above `docs/src/` exists at all. So a chapter can link a
repository-root file, resolve correctly in the repository, pass the gate, and ship a link that
404s.

That is not hypothetical. It shipped in `e328aea` and was corrected by hand in `438c5d3`
(architect review 117 F1). This RFC replaces the hand with an assertion, and fixes a second
fidelity defect in the same resolver — a link quoted inside an inline code span is resolved as
though it were a link — which cost three drafts of RFC 049 and will mislead the next author too.

**Both slices adopt with zero document changes.** Measured: the tree already satisfies them.

## 2. The defect, measured

### 2.1 A link valid in the repository and dead in the book

`xtask/src/checks/link_audit.rs` collects every tracked Markdown file and hands it to
`check_markdown_links`, shared with `check-rfcs`, at `xtask/src/checks/check_rfcs.rs:378-411`.
Its resolution is three lines: skip external and anchor-only targets, `base.join(target)` where
`base` is the containing file's parent, then `resolved.exists()`.

For every other document in this repository that is the right rule. For the fourteen Markdown
files under `docs/src/` it is not, because they are not read where they live. Built with the
tree's own `mdbook v0.5.4` against `docs/book.toml`, whose `src = "src"`:

```
$ mdbook build --dest-dir <scratch>/book
$ grep -o 'href="[^"]*README[^"]*"' <scratch>/book/getting-started.html
href="../../README.html"
```

That was the committed state at `e328aea`: a link in `docs/src/getting-started.md` labelled
*README* whose target was `../../README.md`, written here without its bracket-parenthesis syntax
for the reason §2.2 is about. In the repository it resolves to the root `README.md` and
`link-audit` passed. In the book it is `<book-root>/../../README.html` — a file mdBook never
generates, outside the output directory, and outside the site root once the book is served.

**The reader it failed is the one the paragraph was written for.** RFC 049 S3 added that paragraph
to stop the chapter telling a library user to clone the repository; the pointer it offered instead
was dead.

### 2.2 A quoted link is resolved as a link

`markdown_links` at `check_rfcs.rs:413-438` skips fenced code blocks and nothing else. It scans
for the two-character sequence that opens a link target and takes everything to the closing
parenthesis — **inside inline code spans included**.

So a document that *quotes* another document's link, in backticks, as text, has that text
resolved relative to itself. Quoting the README's root-relative link from `rfcs/proposed/` fails
both `check-rfcs` and `link-audit`, and so does a sentence that names the syntax literally. This
is recorded in RFC 049 §2.3, which had to describe its own subject in prose to get past it.

It is a false positive with a real cost: the gate's message says a link is broken when no link was
written, and the author's only remedy is to stop quoting accurately.

### 2.3 What the tree looks like today

Measured over the fourteen Markdown files under `docs/src/`, fenced blocks excluded, after F1's
correction:

| link kind | count |
| --- | --- |
| relative, resolving inside `docs/src/` | 26 |
| relative, resolving outside `docs/src/` | **0** |
| absolute (`http`/`https`) | 19 |
| anchor-only | 1 |

And over every tracked Markdown file in the repository: **zero** links sit inside an inline code
span — because RFC 049 removed the only ones, by rewording.

**So both assertions below are satisfied by the tree as it stands.** Neither slice changes a
document, and neither is adopted to fix a backlog: they are adopted so the next author cannot
reintroduce what review 117 had to catch by building the book by hand.

## 3. What to do

### S1 — a book link stays inside the book

In `link-audit`, for every Markdown file under `docs/src/`: a link target that is neither external
nor anchor-only must resolve to a path **inside `docs/src/`**. A target that resolves outside it
fails, naming the file, the target and where it resolved to.

**The rule is about the book's own boundary, not about existence**, so it composes with the
existing check rather than replacing it: `check_markdown_links` keeps asserting that the target
exists, and S1 adds that for a book chapter it exists *within the book*. A chapter referring to a
repository file does what `docs/src/specifications.md:80-89` already does — names the path in
backticks and appends the absolute moving-branch URL — which is how F1 was corrected.

Deriving the boundary from `docs/book.toml`'s `src` key rather than hard-coding `docs/src` is
**not** required and I would not do it: the key is one word in a file nobody is proposing to move,
and reading it adds a TOML parse and a failure mode to a check whose whole value is that it is
three lines long.

### S2 — a link inside an inline code span is not a link

In `markdown_links`, exclude inline code spans before scanning a line for link targets: within a
line, the regions delimited by backtick runs are text. Everything else about the function is
unchanged, including its fenced-block handling and its `BTreeSet` return.

This narrows what both `check-rfcs` and `link-audit` check, deliberately. A link inside a code span
is a quotation, not a link — nothing renders it as one, so there is nothing for a reader to
follow and nothing for the gate to protect. Measured: zero links are currently found that way, so
the narrowing removes **no** coverage that exists.

### S3 — say that the book is checked at its own boundary

One row in `docs/src/documentation-convention.md` §5, beside the README rows RFC 049 added:
a `docs/src/` link resolves inside `docs/src/`, enforced by `link-audit`. And one sentence
wherever that file states the book's link discipline, saying why: a chapter is read as rendered
HTML, so a link out of the book is dead even when the file it names is present.

## 4. Non-goals

- **No book build in any gate.** S1 catches this class without rendering anything, and a gate that
  shells out to `mdbook` would pin a second tool, add a build to every run, and still only check
  what S1 checks statically. If a later defect needs the rendered artifact, that is its own RFC.
- **No external-link checking.** The nineteen absolute URLs stay unverified; a gate that reaches
  the network is not a gate this project wants (RFC 026's offline discipline).
- **No change to how `check_markdown_links` resolves or reports a non-book link.**
- **No new gate.** S1 and S2 are both inside `link-audit` and its shared resolver.
- **No rewrite of any book chapter**, and no change to the three absolute references in
  `docs/src/specifications.md`.
- **No anchor validation.** Whether `#release-currency` exists in its target is a different
  assertion with a different parser.

## 5. Risks

- **Functional — Low.** Two predicates in a documentation gate. No library change.
- **Business — Low, and this is the point.** The user-visible effect is that a link in the book
  goes where it says.
- **Operational — Low.** S1's boundary is a path prefix; S2 removes a false positive that
  currently costs an author drafts. Neither needs a value that moves per release.
- **Security — Low.** No channel, no artifact, no trust root. One sentence only because S2
  narrows an assertion: it stops resolving text that was never a link, and the measurement shows
  the narrowing covers nothing real today.

## 6. What I could be wrong about

- **S2 could hide a typo in a document that means a code span to be a link.** I judge that
  impossible to care about — such a link does not render — but it is a narrowing, and a narrowing
  deserves naming. The honest statement of its cost: after S2, a broken link written inside
  backticks is invisible to the gate forever.
- **S1 assumes the book is only ever read as rendered HTML.** A maintainer reading `docs/src/`
  in an editor follows the relative path happily, and for them F1 was not broken. I am choosing
  the published reader over the maintainer, which is RFC 047's order, not a neutral fact.
- **Hard-coding `docs/src` means a book moved elsewhere silently loses the assertion.** §3's
  reasoning for that trade is explicit and I still prefer it to a TOML read, but it is a trade.
