# RFC 053 — implementation handoff

**Author tier.** `architect`. **Date:** 2026-10-31.
**Governing text.** `rfcs/accepted/053-a-sequence-the-repository-holds.md`, Accepted
(design frozen 2026-10-31). Where this file and the RFC disagree, **the RFC is the contract**;
report the disagreement rather than choosing.
**Release.** `0.24.2`, the version already on `main`. **No slice performs a version bump** —
nothing here adds callable functionality, so `docs/src/development.md`'s first row applies
("nobody; the placeholder was right"). Every slice runs
`cargo xtask release-gate --intended-tag 0.24.2`.
**Owner's record.** `rfcs/handoffs/release-0.24.2/scope-and-decision-points.md`.

**One slice, three parts, in this order: S1, then S2, then S3.** S3 asserts what S1 and S2
establish, so a gate written first would fail on a tree that is still correct.

## 0. Figures, and the commands that produced them

Every number below was measured on `31b66cc`. **Re-derive each one. If a figure comes out
differently, report the discrepancy with its cause rather than adopting either number** — that
handling is what caught the architect's own figures in RFC 050's handoff (architect review 121 §1).

### 0.1 Roman-numeral phase references in tracked Markdown

Measured with the predicate S3 specifies — `collect_md`'s file set, single-backtick code spans
masked, `Phase ` followed by a run of `[IVXLCDM]` terminated by a word boundary:

| figure | value |
| --- | --- |
| raw occurrences | **26** |
| distinct (file, phase) pairs | **12** |
| files containing at least one | **4** |
| distinct phases referenced | **I, II, III, V** |

The four files are `rfcs/README.md`, `rfcs/done/049-the-readme-tells-a-user-what-to-write.md`,
`rfcs/accepted/053-a-sequence-the-repository-holds.md` and
`rfcs/handoffs/release-0.24.2/scope-and-decision-points.md`. The last two are the RFC and its
scope file; RFC 053 §2.1's table counts the four **pre-existing** sites only, which is a different
figure on purpose.

**Phases IV and VI are referenced nowhere yet.** S1 defines them anyway — the file is the
definition of the sequence, not an index of what happens to be cited.

The equivalent one-liner, for the file set alone:

```sh
git ls-files '*.md' | xargs grep -nE 'Phase [IVXLCDM]+\b' | wc -l
```

It over-counts relative to the table because it does not mask code spans; use it only to confirm
the file set, and derive the four table figures from the predicate itself.

### 0.2 ROADMAP.md's stale claims

Three, not two. RFC 053 §2.2 measured the first two; the third was measured while writing this
handoff and is reported here rather than carried:

| site | claim | why it is false |
| --- | --- | --- |
| `ROADMAP.md:88` | "RFCs 001-034 are implemented", plus a five-release enumeration | `rfcs/done/` holds 001-046 and 048-052; the apex block four lines above reads the same, correctly |
| `ROADMAP.md:59` | "**Next: T7 as `0.23.0`**, then T6" | T7 shipped as RFC 041 in `0.23.0` on 2026-10-08, and T6's batch seam as RFC 042 in the same release |
| `ROADMAP.md:83-85` | "`README.md`'s headline already claims \"high-throughput server solving\"" | `grep -c -i throughput README.md` → **0**. The headline no longer makes that claim |

A fourth, inside the same span: `ROADMAP.md:72-73` predicts `E0027` and `E0063` for the RFC 042
break, where the `0.23.0` CHANGELOG entry records `E0638` and `E0027`, and states that `E0639`
fires *instead of* `E0063`. It is a pre-slice prediction, and S2 replaces the span that holds it.

**Note for S1's initial table:** by the time S1 lands, `rfcs/done/` will still hold 048-052 —
RFC 053 is in `accepted/` until its own cut, so the apex scope field does **not** change in this
slice. Do not touch it.

## 1. S1 — `rfcs/destination-sequence.md`

A new tracked file. **It is not an RFC**: no RFC number, no Status line, no amendment sections,
and it is not added to `APEX_DOCS` and carries **no** apex currency block — it makes no release
or scope claim, and adding it to `APEX_DOCS` would require a byte-identical block it has no
business holding.

### 1.1 Required front matter

The file opens with, in substance and in this order:

1. One sentence saying it is the single definition of RFC 047's phases, linking
   `accepted/047-the-state-worth-aiming-at.md`.
2. One sentence saying **order and cost are advisory**: which phase comes next, what it costs and
   what may be cut are architect recommendations and owner authorizations, and this file is
   revised as each is measured. It is deliberately not an RFC so that revising it needs no
   amendment (RFC 047 §5 item 1).
3. **One sentence distinguishing the two numberings**, which is RFC 053 §2.1 item 4's whole
   remedy: *the arabic `Phase 0-3` of `ROADMAP.md` and `docs/specs/loeres-roadmap-milestones-v1.md`
   are the crate-layering milestones and are a different axis from the roman phases below.*
4. One sentence recording the source: the sequence was first written in architect strategy 106 §5,
   which is cited in `rfcs/review-evidence-index.md` and is **not a tracked file**, so this file
   is now the definition rather than a copy with an authority elsewhere.

### 1.2 The three state literals

`deferred`, `scheduled`, `shipped`. **Parsed by equality on the trimmed value. Never `contains`.**

Both halves are load-bearing and both come from a shipped defect:

- `contains` is how RFC 052's clause 5 passed its own gate with a defect in it — architect
  review 119 F1 found `line.contains("published to crates.io")` true of
  `"not published to crates.io"`, and `changelog_distributed_date` had the same shape.
- The obvious word for an unstarted phase, `unscheduled`, **contains `scheduled`**. These three
  literals are pairwise non-substrings, so a `contains` implementation cannot accidentally look
  correct. Equality is still the requirement; the choice of literals is the second line of defence.
- A fourth phrasing **fails closed**, exactly as `is_published` does in `doc_currency.rs`.

### 1.3 Per-phase shape

```text
## Phase I — stop misdirecting

**State.** shipped
**Properties.** B
**RFCs.** 049, 050, 051

<the paragraph from §1.4, verbatim>
```

`**Properties.**` takes the property letters RFC 047 §3 defines, or `—`. `**RFCs.**` takes a
comma-separated list of three-digit numbers, or `—`. Do not invent a property letter for a phase
whose source names none.

### 1.4 The six phases — source text

Architect strategy 106 is **untracked and unreadable to you** (`.gitignore:24`), so its §5 text is
reproduced here. Use it as the paragraph for each phase, lightly re-punctuated to stand alone but
not re-argued:

- **Phase I — stop misdirecting.** A user's "Quick Start"; per-crate READMEs that open with
  purpose and usage; a real minor pin. Not because it is cheap, but because every artefact a
  newcomer meets currently points at the wrong thing, and no later work can be judged while that
  is true.
- **Phase II — decide who we are for, and say it.** The audience question, and a published scope
  statement. Prior to all interface work, because property **D** is impossible without it.
- **Phase III — the front door's contract.** Input format, the refusal vocabulary, the exit
  statuses; its relationship to the conformance corpus **asserted by a gate** rather than
  resembled. Properties **A**, **D**, **E**.
- **Phase IV — identity for what users receive.** Extend RFC 028 to the artifacts users install,
  as **one** identity story rather than two. This gates every real channel, so it precedes the
  channel. Property **C**.
- **Phase V — the channel**, through the user's own package manager and their own trust root.
- **Phase VI — point the evidence outward.** The conformance corpus and the gates become
  something a user or their auditor can run against the artifact they received. Properties **B**
  and **C** completed: a solver whose claims the user can check without trusting us.

Plus, after the six, the two sequencing facts from the same source, as a closing paragraph:
**Phases I and II are not prerequisites for each other and Phase I needs nothing. Phases IV and V
are the expensive half and the half with standing obligations.**

### 1.5 The initial table — derived, and to be re-derived

| phase | state | properties | RFCs |
| --- | --- | --- | --- |
| I | `shipped` | B | 049, 050, 051 |
| II | `deferred` | D | — |
| III | `deferred` | A, D, E | — |
| IV | `deferred` | C | — |
| V | `deferred` | — | — |
| VI | `deferred` | B, C | — |

**How Phase I's row was derived.** Review 106 §5 gives Phase I three items. RFC 049 reordered the
Quick Start and pinned the last released minor, and its own text records the per-crate READMEs as
already opening with purpose; RFC 051 closed the four spec fences that contradicted the pin;
RFC 050 removed the link class that misdirected a book reader. All three items are therefore
satisfied, by RFCs in `rfcs/done/`. **RFC 048 is deliberately not listed** — it is property **G**,
release lists, not user misdirection.

**This derivation is the architect's and it is a measurement.** Re-derive it. If Phase I's items
are not all satisfied, or if a fourth RFC belongs in the row, **report it**; do not adopt either
answer silently.

Phase V's `Properties.` is `—` because its source names none. Do not supply one.

## 2. S2 — delete the stale claims; do not rewrite them

Three replacements in `ROADMAP.md`. **Exact** old text, **exact** new text. Nothing else in the
file changes.

**Match on the text, not the line numbers.** Every line number in this handoff is from `31b66cc`
and all three replacements shift the ones below them. Each old text below was verified **unique**
in `ROADMAP.md` at that revision, so a literal match is unambiguous; if any of them is not unique
on your tree, stop and report it rather than guessing which occurrence was meant.

### 2.1 The preamble gains the sequence file

Old:

```text
this file is a short summary. This roadmap's milestones are milestones toward
[`rfcs/accepted/047-the-state-worth-aiming-at.md`](rfcs/accepted/047-the-state-worth-aiming-at.md)
— the project's stated destination and the properties that would constitute
arriving at it.
```

New:

```text
this file is a short summary. This roadmap's milestones are milestones toward
[`rfcs/accepted/047-the-state-worth-aiming-at.md`](rfcs/accepted/047-the-state-worth-aiming-at.md)
— the project's stated destination and the properties that would constitute
arriving at it. The order in which that destination is approached, and the state
of each of its phases, is
[`rfcs/destination-sequence.md`](rfcs/destination-sequence.md); the arabic
`Phase 0-3` below are the crate-layering milestones and a different axis.
```

### 2.2 The `## Current status` opening claim

Old:

```text
RFCs 001-034 are implemented: 001-026 shipped in `0.21.0`, 027-029 shipped in
`0.21.1`, RFC 030 (randomized differential testing for numerical kernels) shipped
in `0.21.2`, RFCs 031-033 (adversarial and extended conformance, step-size
guidance, an exact projection in the `Converged` claim) shipped in `0.21.3`, and
RFC 034 (a heuristic `infeasibility_evidence` field, wrong in both directions)
shipped in `0.22.0`. The current numerical breadth is one
```

New:

```text
The implemented RFC scope is the `Implemented scope` field of the currency block
above, and which release carried what is `CHANGELOG.md`. Neither is restated
here: a hand-maintained copy of either is what RFC 048 deleted from seven
documents and what RFC 053 found still here. The current numerical breadth is one
```

**Delete and point. Do not write the corrected range.** Writing `RFCs 001-046, 048-052` into the
prose would restore a hand-maintained claim four lines from the machine-held field that already
states it — the defect, with a fresher number.

### 2.3 The forward theme claims

Replace **the whole span** from `**Next: T7 as `0.23.0`, then T6.**` through
`T5 (LP), T6 (server-side maturity), T7 (fixed-point scalars).` inclusive — in the measured tree,
lines 59-86 — with:

```text
**T7 and T6's batch seam shipped.** T7 — a fixed-point scalar baseline — landed as
RFC 041, and T6's batch seam (L10) as RFC 042. Both are in `rfcs/done/`, and
`CHANGELOG.md` records which release carried them. T6's other three items —
metadata-only observability, the mock-only `ffi-gateway` seam, and the
process-local validation cache — **stay closed until a real consumer exists**, the
gateway especially, being external surface with security weight.

**T4, a benchmark harness, is the open theme.** The owner's fourth visitor
question, "how effective or powerful", is the one RFC 035 §4 deferred, and it is
still unanswered. Architect review 086 scopes it: effectiveness and counted-work
cost are deterministic and reproducible — RFC 031's difficulty reporting already
measures deviation from the exact optimum — while wall-time throughput is
environment-dependent and can only ever be advisory. The themes still open are T4,
T5 (LP) and T6's three closed items.

**No release position is claimed here.** Which release carries which theme follows
from the slices, under `docs/src/development.md`'s release version convention;
what `main` carries between releases is a placeholder, not a prediction.
```

**Why the whole span and not a trim.** Everything removed is either false (§0.2's second and third
rows, and the `E0027`/`E0063` prediction) or a pre-slice rationale now held in its own RFC and in
reviews 056, 065 and 086 — `rfcs/done/041-a-fixed-point-scalar-baseline.md` and
`rfcs/done/042-the-batch-seam-carries-what-the-solve-found.md` are the record of what was decided
and why. Nothing unique to `ROADMAP.md` is lost. **If you find a claim in that span that is both
true and recorded nowhere else, stop and report it** rather than deleting it.

## 3. S3 — one assertion in `doc-currency`

In `xtask/src/checks/doc_currency.rs`. **No new gate, no new command, no network, no new file
sweep of its own.**

### 3.1 Reuse, do not reimplement

- **The file set** is `link_audit::collect_md(Path::new("."), &mut files)` — already `pub(crate)`
  and already shared with `review_evidence` for exactly this reason (its doc comment says so). It
  skips `.git`, `.git-exclude` and `target`.
- **Code-span masking** is `check_rfcs::mask_code_spans`, which is currently private; make it
  `pub(crate)` the way RFC 050's slice made `external_or_anchor` `pub(crate)`, and **do not copy
  it**. Its single-space substitution is load-bearing (architect review 121 §3): deleting a span
  instead would let a `]` before it and a `(` after it become an adjacent `](`.
- Its documented limitation stands: a **double**-backtick span is not masked (RFC 050 §6). That is
  the conservative direction — it reports rather than hides — so inherit it, and do not widen the
  masker in this slice.

### 3.2 The three clauses

1. **Every roman phase reference resolves.** In every file from `collect_md`, per line, after
   masking: match `Phase ` followed by a run of `[IVXLCDM]`, terminated by a word boundary. Each
   matched numeral must name a phase defined in `rfcs/destination-sequence.md`. The match is a
   **roman-numeral run, not an enumeration of I-VI** — a future `Phase VII` must fail for want of
   a definition rather than pass unnoticed. Arabic `Phase 0-3` must not match.
2. **Every phase's state is one of the three literals**, by equality on the trimmed value, failing
   closed on anything else — `unscheduled` included.
3. **A `shipped` phase names at least one RFC, and every RFC it names resolves to a file in
   `rfcs/done/`.** A `deferred` or `scheduled` phase may name none. Equivalently: a phase naming
   an RFC that is not in `rfcs/done/` cannot be `shipped`.

**Historical documents are in scope, by design.** The point of clause 1 is that
`rfcs/done/049-the-readme-tells-a-user-what-to-write.md:188` resolves. A predicate that exempted
`done/` RFCs would exempt the three references that caused this RFC to exist.

### 3.3 Findings

Follow the register convention already in the file (`APEX MISMATCH`, `RELEASE TABLE`,
`STALE CURRENT PROSE`, `RFC SCOPE`): `PHASE REFERENCE` for clause 1 and `PHASE STATE` for clauses
2 and 3. Each finding names the file, the line where a line number exists, and the value found.

**One finding per defect, not per occurrence.** Architect review 119 F3 is the precedent: a
renamed header produced twelve findings for one cause. An undefined `Phase VII` cited in five
places is one undefined phase — report the phase once, with the sites it is cited from.

### 3.4 Tests, in `doc_currency.rs`'s test module

Eight, each pinning one decision above:

1. a reference to a phase the sequence file does not define → fail;
2. `Phase VII` → fail, proving the predicate is a roman run and not an enumeration;
3. the **negative-control pair**: the same undefined reference inside a single-backtick code span
   → pass, and not backticked → fail;
4. `Phase 2` → no match (the arabic milestone axis);
5. a state of `in progress` → fail closed;
6. a state of `unscheduled` → fail. **This is the substring trap pinned**: it must fail, and an
   implementation using `contains` would pass it;
7. a `shipped` phase naming an RFC that is in `accepted/` rather than `done/` → fail;
8. a `deferred` phase naming no RFC → pass.

## 4. Verification, and what to hand back

1. `cargo xtask doc-currency` → PASS on the tree with S1 and S2 landed.
2. `cargo xtask check` → **PASS, 20 gates**, on the gate's own exit status, never through a pipe.
3. `cargo xtask release-gate --intended-tag 0.24.2` → PASS. Whole stream to a file.
4. A `CHANGELOG.md` entry under `## [0.24.2] — unreleased`.
5. **Prove clause 1 catches the real thing**, the way RFC 050's slice proved its own: temporarily
   mark Phase III `deferred` → `removed`, or delete Phase III's section, and record the finding
   text that `doc-currency` prints for `rfcs/done/049-…:188`. That reference is the defect this
   RFC exists to stop outliving its phase; show it caught.
6. The review request at
   `.git-exclude/review-request/RFC-053-a-sequence-the-repository-holds/REVIEW_REQUEST.md`, naming
   the revision and listing any figure in §0 that came out differently.

**Not yours:** moving RFC 053 to `rfcs/done/`, the apex `Implemented scope` field, the release
table, the tag, the push, and publication. Those are the cut's, and the cut is the architect's and
the owner's (`docs/src/development.md`).
