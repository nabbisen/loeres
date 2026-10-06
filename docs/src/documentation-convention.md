# Documentation Convention

This chapter is the standard for documentation in this repository, examples
included. It adapts the project owner's documentation guideline to Loeres and
adds the rules RFC 035 requires. Where a rule is mechanically checkable, the
check is named; the rest is guidance, enforced in review.

## 1. Principles

- **The README is a landing page**, not a reference manual. It routes readers
  to the book.
- **Progressive disclosure.** Concepts on the landing page; detail in
  `docs/src/`, navigated by `SUMMARY.md`.
- **Signal over noise.** No release bookkeeping on a landing page. Prefer code,
  tables and short lists to explanatory prose.

## 2. README

The README follows the 3–30–3 rule:

| Part | Content | Rule |
|---|---|---|
| First screen (3 s) | One-line summary, 3–5 features | No release sentences |
| Quick Start (30 s) | One build command; one minimal working example | Example under 20 lines, **copied from a gated example** |
| Navigation and meta (3 min) | Links to the book, RFC index, `CONTRIBUTING.md`, `CHANGELOG.md`, `LICENSE` | No links into empty RFC folders |

**The length is 100–200 lines.** `doc-currency` checks this.

**The Quick Start example is extracted, not written.** In the source of a
gated example, a region is delimited by marker comments:

```rust
// README-EXAMPLE-BEGIN
// ... the lines a first-time reader needs ...
// README-EXAMPLE-END
```

The README block is a verbatim copy of that region. `doc-currency` fails when
the two differ by a single character, and when the region exceeds 19 lines.
Change the example, then copy the region again.

**Release currency has one home:** [Release currency](specifications.md#release-currency).
The README links to it and does not repeat it.

### Where documentation belongs

| Topic | Keep in `README.md` | Offload to `docs/src/` |
|---|---|---|
| Overview and design | Purpose, features | Architecture, design philosophy |
| Setup | One build command | Platform steps, edge cases |
| Usage | One or two common patterns | Full API reference, advanced recipes |
| Development | Link to `CONTRIBUTING.md` | Local setup, test strategy, workflow |
| Releases | Link to the currency page | Version history (`CHANGELOG.md`), migration |

If the README needs a table of contents, the content belongs in the book.

## 3. Examples

**Examples are documents.** They are read by people deciding whether this
library solves their problem, and they are kept correct the way prose is kept
correct. Every new example meets the four criteria below.

1. **The first paragraph states a problem a domain reader recognises**, with no
   `loeres` type named. Types appear only after the problem is stated.
2. **It shows the system shape**: build the model, solve, and **act on the
   result**. It is not a sequence of API calls.
3. **It prints something a reader can interpret** without the source open.
4. **It is workspace-excluded** with its own lockfile and **passes the
   `examples` gate**, which builds it, runs it, and checks its dependency
   isolation.

Each example also says **which problem it solves and for whom**, in its module
comment. The problem is one a reader in that field would name; the audience is
the kind of user who would run it.

The three conformance examples (`cluster-batch-solve`, `cluster-qp-constrained`,
`device-box-pfo`) are API artifacts, kept as they are. Their form is not the
standard for new examples.

### Captured output

A block of output quoted in the book is either **exact** or **described in
prose**. An exact block is marked by an `example-output` comment, naming the
example, on the line immediately before its fence. The comment for
`device-box-pfo` is written as inline code here, not as a line, so the gate does
not read this paragraph as a marker.

The `examples` gate runs the named example and requires the block to be a
contiguous run of that example's own output, byte-for-byte. A block with `...`
in it is refused: quote the complete lines, or describe the output in prose.
Where output depends on the environment, such as timings, describe it; do not
quote part of it. The gate requires a marked block for each example the book
quotes, so one cannot silently lose its check.

The comparison includes every digit of a printed float. This is deliberate: a
float that prints differently is a result that changed, and the gate cannot tell
a last-digit difference that does not matter from one that does. The cost is that
the check is only as portable as the values it quotes, so an example whose output
is not identical across the supported targets describes its output in prose
instead.

Two limits are known and accepted. The gate's registry of captured examples
lives in `xtask/src/checks/examples.rs`, and the symmetry it enforces runs from
that registry to the book: every registered example must have a marked block, and
every marked block must match its run. It does not run the other way. A fence
that shows output but carries no marker is indistinguishable from any other
fence, so quoting output without a marker loses its check — and adding a captured
block means adding its example to that registry. Review is the only backstop for
both, which is why §4 requires a slice to state whether an example is affected.

## 4. Workflow rule

**A slice that changes a public surface or a printed record states, in its
review request, whether an example is affected.** Use the same form as the
evidence line: "Examples affected: none", or the list of examples and what
changed in each. A changed printed record is one a reader could copy.

## 5. What is checked and what is guidance

| Rule | Enforced by |
|---|---|
| README length 100–200 lines | `doc-currency` |
| README Quick Start equals the marked example region, byte-for-byte | `doc-currency` |
| Marked region under 20 lines | `doc-currency` |
| No stale release phrases or release notes in current-facing docs | `doc-currency` |
| Each example builds, runs, and is dependency-isolated | `examples` gate |
| Captured output in the book matches the run | `examples` gate, via the `example-output` marker |
| Every registered captured example has a marked block | `examples` gate |
| An **unmarked** block that shows output | Review only — not enforced |
| The four criteria for an example | Review |
| "Which problem and for whom" in an example | Review |
| The workflow rule in §4 | Review |
| Prose quality (§1) | Review |

The guidance rows are not gates. A coverage rule for examples, asserting that
every kernel has one, is deliberately not enforced. If coverage later drifts,
that is evidence for a registry, and the decision belongs to a new RFC.
