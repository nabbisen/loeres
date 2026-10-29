# RFC 048 implementation handoff — one release list that cannot rot

**From:** architect
**To:** implementer
**Date:** 2026-10-29
**Governing text:** `rfcs/accepted/048-one-release-list-that-cannot-rot.md`, accepted by the owner
on 2026-10-29 **as written** — which decides §7's open question in favour of the wider reading:
S2 covers all four documents, book chapters included.
**Base:** `main` at the revision that accepted RFC 048.

**Scope:** `xtask/src/checks/doc_currency.rs` (one constant), seven Markdown documents, and
`ROADMAP.md`'s one new sentence. **No new gate, no new predicate, no library change, no version
change, no tag.**

Everything here is deletion plus a constant plus four copies of an existing four-line block. If
you find yourself editing `parse_ordinary_apex_currency`, `extract_currency_block`,
`check_apex_currency` or any other function, **stop and report it** — RFC 048 §5 makes that out of
scope, and it would mean the mechanism is not what §3 says it is.

## 0. Read this first: the one thing that already exists

`check_apex_currency` already holds every path in `APEX_DOCS` to seven properties (RFC 048 §3).
`APEX_DOCS` is three paths, all under `docs/specs/`. **The four documents that went stale are the
four that were never in it.** Your job is to put them in it and give them the block, not to build
anything.

One fact worth knowing before you start, because it explains the defect: `docs/src/specifications.md`
already carries a heading `## Release currency` whose first line reads *"This is the one home for
release currency. The README links here (RFC 035 §3.3)."* That decision was made and then the list
was duplicated into seven documents anyway. The deletion in S1 is what makes that sentence true.

## 1. S1 — delete the run, at eight sites in seven documents

The sentence is an append-only run: *"Repository release `X` shipped `DATE` and carries RFCs
`NNN-NNN`."* repeated on one physical line. There are **eight sites** and **four textual
variants**. Five of the eight sit **mid-paragraph**, so a blanket substitution would damage the
prose around them. Delete the run **and the single space before it**, and leave every adjacent
sentence byte-identical.

### 1.1 The three standalone paragraphs — delete the paragraph

In each of these the run is the whole paragraph. Delete the paragraph line and exactly one of the
blank lines around it, so the surrounding blank-line structure is unchanged:

| document | line | what precedes | what follows |
| --- | --- | --- | --- |
| `docs/specs/loeres-requirements-v1.md` | 29 | blank, after `> gate now uses the ordinary steady-state form.` | blank, then `---` |
| `docs/specs/loeres-external-design-v1.md` | 23 | blank, after `> uses the ordinary steady-state form.` | blank, then `---` |
| `docs/specs/loeres-roadmap-milestones-v1.md` | 20 | blank, after `> ordinary steady-state form.` | blank, then `---` |

Each of those three paragraphs begins `Repository release \`0.20.2\` shipped 2026-07-30 and carries
RFCs 001-021.` and ends `Repository release \`0.22.0\` shipped 2026-09-24 and carries RFCs 001-034.`

### 1.2 The five embedded sites — delete the run, keep its neighbours

For each: the text **immediately before** the run must survive unchanged including its trailing
period, the run goes, and the text **immediately after** must survive unchanged. Then **rewrap the
affected paragraph** to that document's existing width, **changing no word**.

| # | document | lines | text ending immediately before the run | first words immediately after the run |
| --- | --- | --- | --- | --- |
| 1 | `ROADMAP.md` | 27–29 | `…conclusion, and its required evidence upload succeeded.` | `RFC 024 subsequently retired the one-release conditional apparatus…` |
| 2 | `ROADMAP.md` | 112–113 | `…evidence only and are never reused.` | `Currency lives in the apex trio's RFC 024 block, which records…` |
| 3 | `docs/src/introduction.md` | 38 | *(the run starts the paragraph)* | `Current solver breadth is one projected-first-order family…` |
| 4 | `docs/src/specifications.md` | 65–66 | `…reconciled against, never release status.` | `On conflict, stop affected public-boundary work…` |
| 5 | `docs/src/threat-model.md` | 156–157 | `…and follows RFC 021's conditional boundary.` | *(the run ends the paragraph)* |

Sites 1 and 4 are inside blockquotes (`> ` prefixed); keep the prefix on every surviving line.

The four variants differ only in their opening clause — `\`0.20.2\` carries`, `\`0.21.0\` shipped`,
`\`0.20.2\` is released and carries`, `\`0.20.2\` shipped 2026-07-30 and carries` — so **read each
site before editing it** rather than reusing one pattern.

### 1.3 The completion predicate for S1

```sh
git grep -n 'Repository release `0\.2' -- '*.md'
```

must return matches in **exactly two** files and nowhere else:

- `CHANGELOG.md` — it **is** the record; untouched by this slice;
- `docs/src/recovery-roadmap.md` — one mention, a historical narrative about a past recovery, **not**
  a currency claim. **Leave it exactly as it is** (RFC 048 §7).

Report the count you found at each of the eight sites before deleting it, and the final output of
that command.

## 2. S2 — give four documents the block, and add them to `APEX_DOCS`

### 2.1 The block, verbatim

Insert exactly this, including the trailing bare `>`:

```markdown
> **Release currency metadata.**
> This tree: **0.24.1**.
> Last reconciled repository release: **0.24.0**.
> Implemented scope: **RFCs 001-044**.
>
```

**It must be byte-identical to the three specs' blocks.** That is not a style preference:
`check_apex_currency` compares the whitespace-normalized block across every apex document and
reports `APEX MISMATCH` on any difference (RFC 048 §3, property 6). Copy it from
`docs/specs/loeres-requirements-v1.md:12-16` rather than retyping it.

`extract_currency_block` stops at the first blank blockquote line, so **anything you write after
the bare `>` is free prose and is not compared.** You may add a document-specific sentence there;
you are not required to, and I would not for these four.

### 2.2 Where each block goes

| document | placement |
| --- | --- |
| `ROADMAP.md` | immediately after the four-line preamble that ends `this file is a short summary.` (line 6), separated by a blank line, before the `> **Architecture recovery notice…` blockquote |
| `docs/src/introduction.md` | immediately before the document's first `##` heading |
| `docs/src/specifications.md` | under the existing `## Release currency` heading, as the first block beneath its `This is the one home for release currency.` line |
| `docs/src/threat-model.md` | immediately before the document's first `##` heading (`## Trust boundaries`) |

### 2.3 The constant

`xtask/src/checks/doc_currency.rs:11-15`. Add the four paths **after** the three existing ones, so
the comparison base stays a normative spec:

```rust
const APEX_DOCS: &[&str] = &[
    "docs/specs/loeres-requirements-v1.md",
    "docs/specs/loeres-external-design-v1.md",
    "docs/specs/loeres-roadmap-milestones-v1.md",
    "ROADMAP.md",
    "docs/src/introduction.md",
    "docs/src/specifications.md",
    "docs/src/threat-model.md",
];
```

Extend the constant's own comment to say why these four are here — that they make release-scope
claims and had none of this held — in that file's register.

Verified before prescribing: none of the four contains `**RFC 020 shared currency metadata
(draft).**` or `**RFC 021 conditional release-finalization metadata.**`, and none already contains
a `Release currency metadata` block, so each addition is clean.

## 3. S3 — `docs/src/threat-model.md`'s opening sentence

Lines 3–8 currently read:

> This chapter describes the implemented v0.20.0 device, cluster, validation, telemetry, and
> gateway boundaries plus RFCs 019/020/021, which shipped in repository release `0.20.2` (released
> 2026-07-30). It separates implemented controls from release gates, residual risks, and future
> work. It is not a formal security proof, safety certification, or claim that the repository is
> release-ready.

**The requirement:** the sentence must stop naming a version or a release date. The block you add
in S2 states the tree, the last reconciled release and the implemented scope, all three held by a
gate; the sentence's job is to say what the chapter covers.

**Its second and third sentences must survive word-for-word.** *"It is not a formal security proof,
safety certification, or claim that the repository is release-ready"* is correct, is load-bearing,
and is the reason this document may say what it says. Do not touch it.

Suggested, not prescribed — write it in the document's own register if you can do better:

> This chapter describes the device, cluster, validation, telemetry and gateway boundaries this
> tree implements, and the release gates around them. The currency block above states which tree
> and which reconciled release that is. It separates implemented controls from release gates,
> residual risks, and future work. It is not a formal security proof, safety certification, or
> claim that the repository is release-ready.

## 4. S4 — `ROADMAP.md` cites its destination

One sentence, in `ROADMAP.md`'s preamble, naming
`rfcs/accepted/047-the-state-worth-aiming-at.md` as the destination this roadmap's milestones are
milestones toward. Link it the way that file links its other RFC references.

This is the only additive item in the slice. Say what RFC 047 is — a destination and the properties
that would constitute arriving — and do **not** restate its goal sentence; a paraphrase in a second
document is the duplication this whole RFC is about.

## 5. What must not change

- **`CHANGELOG.md`.** Not one character. It is the record everything else now defers to.
- **`docs/src/recovery-roadmap.md`.** Its single mention is historical narrative (§1.3).
- **The block's format**, its field names, its three values, or any function in
  `doc_currency.rs`. Only `APEX_DOCS` and its comment.
- **`threat-model.md`'s disclaimer sentences** (§3), and its residual-risk list — including *"No
  dedicated safety-critical engineering-use disclaimer currently exists"*, which is correct.
- **Any RFC, any file under `rfcs/`.** The index row for RFC 048 is already written.
- **No gate added. No version changed. No tag.**

## 6. Evidence required in the review request

1. The usual five logs, plus `release-gate --intended-tag 0.24.1` exit status checked directly
   rather than through a pipe.
2. The CHANGELOG entry, quoted in full. It states what was deleted, where the block now lives, and
   the measured defect — five releases missing from seven documents. **No completeness claim**: say
   what is now held and stop.
3. **S1's count at each of the eight sites before deletion**, and the final
   `git grep -n 'Repository release \`0\.2' -- '*.md'` output showing matches in `CHANGELOG.md` and
   `docs/src/recovery-roadmap.md` only.
4. **Proof the four new blocks are byte-identical to the three existing ones**, by the same
   normalization the gate uses — not by eye.
5. **Five targeted exercises against the committed tree**, each breaking the thing one assertion
   names, each reverted, each with the gate's own message quoted:

   | # | break | expect |
   | --- | --- | --- |
   | 1 | `This tree: **0.24.1**` → `**0.24.2**` in **one** new document | `APEX RELEASE: <that path> …expected workspace …` |
   | 2 | `Last reconciled repository release: **0.24.0**` → `**0.24.1**` in one new document | the strict-inequality failure, naming that path |
   | 3 | `Implemented scope: **RFCs 001-044**` → `**RFCs 001-043**` in one new document | the stale-scope failure, naming that path |
   | 4 | add one word inside the block's four lines in one new document, leaving all three field values correct | `APEX MISMATCH: <that path> …` |
   | 5 | delete the whole block from one new document | the missing-marker failure, naming that path |

   Exercise **each one in a different document of the four**, so all four are shown to be under the
   gate rather than one of them being under it four times. Exercise 4 is the one that proves the
   cross-document identity check reaches the new paths; do not skip it because exercises 1–3
   already failed.
6. Anything in §1 or §2 that does not match the tree, **reported rather than fixed.**
7. If `supply-chain` fails on an advisory unrelated to this slice, report it; change no dependency.

## 7. Push

Per `docs/src/development.md`'s "An accepted slice is pushed; a tag is not the implementer's":
once the review request is accepted, push without asking again. No tag.
