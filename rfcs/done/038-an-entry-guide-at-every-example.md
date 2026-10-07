# RFC 038 - An Entry Guide at Every Example

**Status.** Implemented (v0.22.3). Amendment 1 was made while Accepted, under RFC 000's in-place-amendment rule.

**Author tier.** `architect`.

**Governing question.** The owner asked, on 2026-10-07, why no example project has a
`README.md` or an entry guide. The answer was partly "by design" and partly "nobody
noticed", and the second half is the reason for this RFC.

## 0.1 Amendment 1 (2026-10-07)

Made while Accepted, under RFC 000's in-place-amendment rule. The Status line carries only
the design-freeze date while this RFC is in `accepted/`, because `doc-currency` requires
that exact form there; **the amendment must be named in the Status line when this RFC moves
to `rfcs/done/`**, which `check-rfcs` enforces for `done/` only.

**§2.1's marker comments are withdrawn in favour of structural extraction.** The architect
measured the tree before writing the handoff and found the marker mechanism to be the wrong
borrowing.

The existing `README-EXAMPLE-BEGIN` / `README-EXAMPLE-END` pair delimits **code**, and
`marked_region` compares raw lines byte-for-byte (`doc_currency.rs:871-880`). A problem
statement lives inside a `//!` module comment, so it would need prefix stripping, and worse,
a plain `// MARKER` line placed between two `//!` lines contributes nothing to rustdoc —
which means the paragraphs either side would **join into one** in the rendered output. The
`doc-build` gate runs rustdoc under `-D warnings` and would not necessarily catch a silent
paragraph merge.

Structural extraction needs no markers and no source edit at all. Measured across all six
examples: each has a one-line first paragraph (its title), and the **second** paragraph is
either the problem statement or, for the three API artifacts, the single line "What this
example demonstrates (RFC …):". So the parts this RFC needs are identifiable from the
document's own shape, and the shape is the one
`docs/src/documentation-convention.md` §3.1 already requires.

The two categories are distinguished by an explicit **registry**, not by a heuristic on
prose, in the idiom RFC 022, RFC 030 and RFC 036 already use, with symmetry asserted in both
directions.

## 1. Summary

RFC 035 made examples documents. It put each example's entry guide in **two** places and
a per-example `README.md` in neither:

- each example's **module comment** — `docs/src/documentation-convention.md` §3: "Each
  example also says which problem it solves and for whom, in its module comment";
- a **table in the root `README.md`**, one row per example with its problem and target.

The reasoning was sound: an example's first paragraph *is* its guide, and a second copy
would be a thing that rots. Both halves of that reasoning have now failed in practice.

**It rots anyway.** `cluster-counted-work` was added by RFC 037, declared in its review
request, accepted by the architect in review 087, and **never added to the root table** —
five rows against six directories. It appears nowhere a visitor reads. No gate asserts
that table's completeness, so the single mechanism compensating for the absent READMEs
went stale after one cycle, silently. That is precisely the failure RFC 035 built the
`doc-currency` snippet assertion to prevent, left unbuilt one level up.

**And the guide is not where the link lands.** The table's links point at the example
*directory*. GitHub renders `README.md` in a directory view; with none, a visitor who
follows the link sees `Cargo.lock  Cargo.toml  src` and must know to open
`src/main.rs`. Browsing to `examples/` shows six bare directory names with no indication
of where to start.

This RFC puts a guide where a visitor arrives, and gates both directions so neither can
drift.

## 2. Design

### 2.1 A `README.md` in every example, extracted, not written

Each `examples/<name>/README.md` carries the example's problem statement, how to run it,
and what it prints. **Its problem statement is the second paragraph of the module
comment**, taken structurally rather than by marker comments (Amendment 1): the `//!` block
is read from the top, its prefixes stripped, and its paragraphs split on blank doc lines.
The first paragraph is the title; the second is the problem.

`doc-currency` gains the corresponding assertion, per example in the worked-problem
category: the README's problem paragraph equals that extracted paragraph. A second copy
that cannot drift is not the thing RFC 035 was avoiding.

What a per-example README adds beyond the module comment, and so must be written rather
than extracted: the run command, and a pointer back to the root README and the relevant
user guide. Keep it short — this is a signpost, not a chapter.

### 2.2 An index at `examples/README.md`

One table: every example, its problem in a clause, and its path — the root README's table,
at the place a visitor browsing `examples/` actually lands. The root README keeps its own
table; this RFC does not move it, because a reader on the landing page should not have to
navigate to learn what exists.

### 2.3 Symmetry, in both directions, gated

A new assertion in `doc-currency` (it already owns the README landing checks):

1. every directory under `examples/` has a row in the root `README.md` table **and** in
   `examples/README.md`;
2. every row in either table names a directory that exists;
3. every example directory has a `README.md`.

Fail closed on all three. This is the same coverage symmetry as RFC 022's citations,
RFC 030's differential registry and RFC 036's reserved-feature registry, and it is what
makes §2.1 and §2.2 durable rather than another thing to remember.

Adding `cluster-counted-work` to the tables is part of the slice, not a prerequisite for
it: the gate should fail on the current tree before the row is added, and the
break-and-restore record should show that.

## 3. The three conformance examples

`cluster-batch-solve`, `cluster-qp-constrained` and `device-box-pfo` are, in the
convention's words, "API artifacts, kept as they are. Their form is not the standard for
new examples."

They still get a `README.md` and a table row, because a visitor browsing to them deserves
to know what they are — including that they are API artifacts rather than worked problems.
Their module comments are not required to gain a marked problem region; where one is
absent, the README's description is written, and the gate's §2.1 assertion applies only to
examples that carry the markers.

## 4. Explicit non-scope

- **No change to any example's code or output.** No `src/` edit beyond adding marker
  comments around text that already exists.
- No change to the `examples` gate, to captured-output checking, or to
  `published-metadata`. The per-example READMEs are **not** packaged — they are not inside
  a published crate — so RFC 036's packaged-README rules do not apply to them, and the
  relative-link prohibition in particular does not: these READMEs are read on GitHub,
  where relative links work.
- No move of the root README's table.
- No new dependency, no public API change, no kernel change.

## 5. Risks

| Risk | Mitigation |
|---|---|
| Six more documents to keep correct | §2.3 is the point: the problem statement is extracted and gated, the tables are gated for symmetry, so the parts that can rot are checked rather than trusted. |
| The root README is near its floor | It is **101 lines** against `doc-currency`'s 100-200 band after the duplicate licence line was removed. This RFC adds nothing to it except one table row, but the margin is one line, and a later slice that removes anything will breach. Recorded as a known constraint; revisiting the band is a separate decision for the owner. |
| Marker comments clutter the source | They are two lines per example, and the mechanism is already in use and understood. |
| A README duplicating the module comment drifts in the written parts | Only the problem statement is extracted and gated. The run command is one line and is checked by the reader who runs it; the pointers are checked by `link-audit`. |

## 6. Exit criteria

1. Every directory under `examples/` has a `README.md`.
2. Each such README's problem statement is a marked region of its own source, asserted
   byte-for-byte by `doc-currency` where the markers exist.
3. `examples/README.md` indexes every example.
4. The root `README.md` table and `examples/README.md` both list every example directory,
   and name no directory that does not exist — asserted in both directions.
5. `cluster-counted-work` appears in both tables.
6. `cargo xtask check` passes with twenty gates; `cargo xtask release-gate` passes.
