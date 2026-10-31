# RFC 053 - A Sequence The Repository Holds

**Status.** Accepted (design frozen 2026-10-31)

**Author tier.** `architect`.

**Governing scoping.** RFC 047 property **G** — *no current-facing document may state a version,
release list or scope the workspace contradicts, and that is asserted mechanically* — turned on
the project's own sequencing. The owner asked on 2026-10-31 whether RFC 047's Phases II–VI are
watched or planned. They are neither: the phases are defined in no tracked file, three tracked
documents cite them anyway, and no gate knows they exist. Every figure below was measured on the
tree at `a699418`, workspace `0.24.2`.

`rfcs/handoffs/release-0.24.2/scope-and-decision-points.md` §1.1 is the owner's record of this
scope; the owner answered its D1 with this RFC alone.

## 0.1 Amendment 1 (2026-10-31)

Made while Accepted, under RFC 000's in-place-amendment rule, and recorded in architect review
122. The Status line carries only the design-freeze date while this RFC is in `accepted/`, because
`doc-currency` requires that form there; **this amendment must be named in the Status line when
the file moves to `rfcs/done/`**, which `check-rfcs` enforces there and only there.

It adds one clause and records one limitation. Where it differs from §3 S3 below, **this section
governs**.

### Clause 4 — the sequence file defines a duplicate-free, contiguous set from I

`rfcs/destination-sequence.md`'s phase numerals must form a contiguous roman sequence starting at
I — I, II, … N — with no numeral defined twice.

Two measurements make it necessary, both in architect review 122:

1. **A duplicate phase section passes clause 1 and clause 2 silently** (review 122 F4): a second
   `## Phase III` with a valid state produced zero findings. The file's own first sentence calls
   itself the single definition, and two sections for one phase are two sources that can disagree
   — the confusion RFC 047 property **D** forbids.
2. **Clause 1 cannot promise that a deleted phase is caught**, because every phase heading cites
   its own phase: the protection comes from *other* files, and for Phases IV and VI those are only
   the sequence file itself and this RFC's handoff. Clause 4 catches a deleted phase whether or not
   anything cites it.

### The plural form is not seen, and the matcher is deliberately not widened

Measured (review 122 F3): `Phases II-VII are unscheduled.` added to a tracked document yields
**zero findings**, although `Phase VII` is undefined, because the matcher keys on the literal
`"Phase "` and cannot match `"Phases "`. `Phases I and II` and `Phases IV and V` are likewise
unseen, and all three forms are already in use in tracked documents.

**This errs in the unsafe direction** — it misses rather than over-reports — unlike RFC 050 §6's
double-backtick limitation. It is recorded rather than fixed, for a reason with a cost attached:
catching `Phases II-VI` means reading every roman run on any line containing `Phases`, and `CI` is
a valid roman run, so the cheap widening buys a false-positive class on precisely the lines that
discuss this mechanism — inside a gate whose own charter is that it *"deliberately does not
attempt to infer arbitrary prose semantics"*. Clause 4 bounds the exposure instead: a missing
definition can no longer hide behind plural-only citations.

**What remains uncovered after clause 4:** a plural-only reference to a phase beyond the defined
contiguous range — a future `Phases VI-VIII` while VII and VIII are undefined. That is the honest
residual, and the first time it costs anything is the time to widen the matcher.

## 1. Summary

RFC 047 is the destination. Its phases — the order in which the destination is approached — live
only in `.git-exclude/reviewed/106-overall-strategy-the-state-worth-aiming-at-2026-10-28.md` §5,
a path `.gitignore:24` excludes from the repository. `rfcs/done/049-the-readme-tells-a-user-what-to-write.md`
defers work to "Phase III" and "Phase V", and the repository contains neither.

That is workflow held in a prompt, which this project's working rule forbids, and it is mine:
`rfcs/accepted/047-the-state-worth-aiming-at.md:152` §5 item 1 put it there deliberately.

This RFC gives the sequence one tracked home that is **not an RFC** — so the argument that put it
outside, *do not freeze an unmeasured order under RFC 000's amendment discipline*, is honoured
rather than overturned — deletes two stale claims the same measurement found in `ROADMAP.md`, and
adds one assertion to `doc-currency` so a reference to a phase cannot outlive the phase.

**No new gate, no new mechanism, nothing scheduled.** A phase's state changes by the architect
recommending and the owner authorizing, exactly as now; what changes is that the state is written
down and checked.

## 2. The defect, measured

### 2.1 The sequence is not in the repository

| what | where |
| --- | --- |
| Phases I–VI, defined | `.git-exclude/reviewed/106-overall-strategy-the-state-worth-aiming-at-2026-10-28.md` §5 — gitignored at `.gitignore:24` |
| Phase I, cited | `rfcs/done/049-the-readme-tells-a-user-what-to-write.md:7`; `rfcs/README.md:82` |
| Phase III, cited | `rfcs/done/049-the-readme-tells-a-user-what-to-write.md:188`, `:226` |
| Phase V, cited | `rfcs/done/049-the-readme-tells-a-user-what-to-write.md:226` |
| Phases I–VI, defined in any tracked file | nowhere |

Four consequences, each checked rather than argued:

1. **A `done/` RFC defers work to a phase that does not exist in the repository.** A `done/` RFC
   is never amended in place, so those three references are permanent. The sequence must come to
   them; they cannot be edited to meet it.
2. **Review 106 cannot be corrected.** It is cited (`rfcs/review-evidence-index.md:190`, ✓) and
   therefore immutable. A new file is the only way.
3. **Nothing watches it.** No gate reads a phase. Deleting that one untracked file loses the
   sequence silently, with no copy in history.
4. **The numbering collides.** `ROADMAP.md`'s `## Phases` and
   `docs/specs/loeres-roadmap-milestones-v1.md:64-73` use *Phase 0–3* for the crate-layering
   milestones — Phase 2 is the static backend — while review 106 uses *Phase II* for an audience
   decision. Two phase vocabularies, arabic and roman, in one project: RFC 047 property **D**'s
   confusion, in our own documents.

### 2.2 The same class, in an apex document, four lines from the block that holds it

`ROADMAP.md` is apex document 4 (`xtask/src/checks/doc_currency.rs:21`). Its machine-held block
reads `Implemented scope: **RFCs 001-046, 048-052**`, which is correct. Below it:

| site | claim | measured |
| --- | --- | --- |
| `ROADMAP.md:88` | "RFCs 001-034 are implemented", followed by a per-release enumeration | `rfcs/done/` holds 001-046 and 048-052; the enumeration is a release list, which RFC 048 deleted from seven documents in favour of `CHANGELOG.md` |
| `ROADMAP.md:59` | "**Next: T7 as `0.23.0`**, then T6" | T7 shipped as RFC 041 in `0.23.0` on 2026-10-08, three weeks before this measurement |

**This is RFC 048's defect recurring in a file RFC 048 touched**, because RFC 048 scoped the
release-list *sentence* and this is the surrounding prose. It is also the measurement that settles
the design question below: tracked prose a human keeps current is not sufficient, because we
already tried exactly that, in exactly that file, and it went stale inside one release with twenty
gates green.

### 2.3 What the three slices do not reach

There is a **third** sequence axis: the themes `T4`–`T7`. `ROADMAP.md:59-86` cites them; nothing
defines `T1`–`T3`; and `docs/specs/loeres-roadmap-milestones-v1.md` §7, which `ROADMAP.md:47`
offers as "detailed sequencing", is a historical record that ends at C.3 and `0.21.1` and contains
no theme. So the themes have no home either. §5 states why this RFC does not take that on, and §4
states what S2 must therefore leave alone.

## 3. What to do

### S1 — one home, and it is not an RFC

A new tracked file, `rfcs/destination-sequence.md`, is the single definition of RFC 047's phases.
One section per phase, in this shape, with the prose taken from review 106 §5 rather than rewritten:

```text
## Phase I — stop misdirecting

**State.** shipped
**Properties.** B
**RFCs.** 049, 050, 051

<one paragraph: what arriving at this phase means, and why it sits here>
```

Required of the file:

- **It is not an RFC.** It lives beside `rfcs/README.md` and `rfcs/review-evidence-index.md`, which
  are the two existing non-RFC files at that level, so RFC 000's amendment discipline does not
  reach it and the order stays revisable as each phase is measured. That is
  `rfcs/accepted/047-the-state-worth-aiming-at.md:152` §5 item 1 satisfied, not contradicted: what
  that item declines is *freezing* the order, not *recording* it.
- **It states, in one sentence, that the arabic `Phase 0-3` of `ROADMAP.md` and
  `docs/specs/loeres-roadmap-milestones-v1.md` are the crate-layering milestones and a different
  axis from these roman phases.** §2.1 item 4 is the reason.
- **It is advisory on order and cost, and says so.** Which phase comes next, what it costs and what
  may be cut remain architect recommendations and owner authorizations.
- The implementer verifies the new path against `check-rfcs`'s folder expectations and
  `link-audit`, rather than assuming a top-level file there is inert.

**The three state literals are `deferred`, `scheduled`, `shipped`,** and they are parsed by
**equality on the trimmed value, never by `contains`**. Both halves of that sentence are
load-bearing:

- `contains` is how RFC 052's clause 5 shipped a defect that passed its own gate — architect
  review 119 F1 found `line.contains("published to crates.io")` true of
  `"not published to crates.io"`, and `changelog_distributed_date` had the same shape.
- The obvious literal for an unstarted phase, `unscheduled`, **contains `scheduled`**. Equality
  removes the trap; literals that are pairwise non-substrings remove it twice, so that a `contains`
  implementation cannot accidentally look correct.
- A fourth phrasing **fails closed**, as `is_published` does in `doc_currency.rs`.

**The initial table, derived — reproduce it before adopting it.** Review 106 §5 gives Phase I three
items: a user's Quick Start, per-crate READMEs that open with purpose and usage, and a real minor
pin. RFC 049 reordered the Quick Start and pinned the last released minor; its own text records the
per-crate READMEs as already opening with purpose; RFC 051 closed the four spec fences that
contradicted the pin, and RFC 050 removed the link class that misdirected a book reader. So
**Phase I is `shipped`, by RFCs 049, 050 and 051**, and **Phases II–VI are `deferred`**. This
derivation is mine and it is a measurement, not an instruction: re-derive it, and if it comes out
differently, **report the discrepancy rather than adopt either answer**. (That handling is what
caught my figures in RFC 050's handoff §0.1 — architect review 121 §1.)

### S2 — delete the two stale claims; do not rewrite them

- `ROADMAP.md:88`: the RFC-range claim and its per-release enumeration go. The apex block four
  lines above states the scope, and `CHANGELOG.md` is the release record. **Delete and point** —
  RFC 048 S1's remedy. Rewriting the numbers would restore a hand-maintained claim, which is what
  rotted.
- `ROADMAP.md:59`: the **version** attached to a forward theme claim goes. "Next: T7 as `0.23.0`,
  then T6" becomes a statement of theme order with **no version and no scope in it**, because a
  version claim about an unshipped release is a prediction and `docs/src/development.md` already
  holds that `main` carries a placeholder, not a prediction.

The rest of `ROADMAP.md`'s narrative is history and is left alone (§4).

### S3 — one assertion, inside `doc-currency`

Three clauses. No new gate, no new command, no network.

1. **Every roman-numeral phase reference in a tracked Markdown file resolves** to a phase defined
   in S1's file. The match is `Phase ` followed by a run of roman-numeral characters, **not an
   enumeration of I–VI**: a future `Phase VII` must fail for want of a definition rather than pass
   unnoticed. Inline code spans are masked, per RFC 050 S2 — and `mask_code_spans` substitutes one
   space per span, which is load-bearing (architect review 121 §3).
2. **Every phase's `State.` equals one of the three literals**, by equality, failing closed on a
   fourth.
3. **A `shipped` phase names at least one RFC, and every RFC it names resolves to a file in
   `rfcs/done/`.** Equivalently: a phase naming an RFC that is not in `rfcs/done/` cannot be
   `shipped`. `deferred` and `scheduled` phases may name no RFC.

**Historical documents are in scope by design.** The point of clause 1 is precisely that
`rfcs/done/049-…:188`'s reference resolves; a predicate that exempted `done/` RFCs would exempt the
three references that motivated the RFC.

## 4. What must not be swept in

Named, because S2 edits an apex document and the temptation is to tidy:

1. **`ROADMAP.md`'s architecture-recovery notice** and every historical completion narrative. They
   are dated records of a revision, and the file already says so: *"The detailed entries below are
   chronological release history."* Not currency, not touched.
2. **The `## Phases` section and its arabic Phase 0–3.** They are the milestone axis and they are
   correct. S1 distinguishes them in one sentence; S2 does not renumber them. Renumbering would
   orphan `docs/specs/loeres-roadmap-milestones-v1.md:64-73`.
3. **The themes `T4`–`T7`.** S2 removes the *version* from the forward claim and nothing else. The
   themes keep their content and their order; §5 and §2.3 record that they have no definitional
   home, which is a separate RFC's problem.
4. **The apex currency block.** Machine-held, byte-identical across seven documents, and correct.
5. **RFC 047 itself.** Accepted, and this RFC adds no provision to it. §5 item 1 of RFC 047 stays
   true: sequencing is not frozen under amendment discipline. The new file is where it is recorded.

## 5. Non-goals

- **Scheduling any phase.** Phases II–VI are `deferred` on the day this ships, and a state change
  is an architect recommendation and an owner authorization, as now.
- **Answering Phase II's audience question.** The owner's, put and not answered on 2026-10-31. It
  is the reason Phase III is unschedulable (review 106 §5 puts property **D** behind it), and this
  RFC only makes that visible.
- **A home for the themes `T4`–`T7`** (§2.3). Taking it on would mean deciding whether themes and
  destination phases are one axis or two — a design question worth its own RFC, and worth nothing
  while both are stale.
- **Verifying which RFCs a release carried.** `docs/src/specifications.md`'s `Carries RFCs` column
  stays bounded and ordered but not verified (RFC 052 §0.1). Clause 3 binds a phase's RFCs to
  `rfcs/done/`, which is a different claim.
- **Any change to the five crates.** No crate, no public API, no dependency, no feature, no `std`
  or `alloc` posture, no device determinism, no cluster behaviour, no error or diagnostic surface
  is touched. **Semver: none** — documents and one assertion inside an existing gate, so the
  release stays the patch already on `main` (`0.24.2`).

## 6. Risks

- **A third hand-maintained field.** The `State.` line joins the apex scope field and the release
  table as something a cut may have to move. Bounded by clause 3: a `shipped` phase whose RFCs are
  not in `rfcs/done/` fails the gate, so the field cannot drift *upward* unnoticed. It can still
  sit at `deferred` after a phase has in fact shipped, which no mechanism can detect, and which is
  the honest limit of this RFC.
- **Clause 1 could fire on prose that means nothing by it** — a sentence about "Phase I" in a
  quoted external document, say. **Measured under the predicate itself**, not by eye: **26 raw
  occurrences in 4 tracked files, 12 distinct (file, phase) pairs, referencing phases I, II, III
  and V only** — every one intended, and all four phases will be defined by S1. The command that
  produced those figures is in the implementation handoff §0.1; §2.1's table counts the four
  *pre-existing* sites, and this RFC and its scope file are the other two files. If a false
  positive ever appears, the fix is the one RFC 050 took for code spans, not an exemption list.
- **The file could become a plan nobody reads.** It is three screens at most, it is cited from
  `ROADMAP.md`'s destination sentence, and clause 1 gives it one reader that cannot forget.

## 7. What I could be wrong about

- **That `deferred` is the honest word for Phases II–VI.** It asserts a decision to postpone.
  Phases IV–VI are closer to *not yet costed*. I chose three literals over four because a fourth
  state nobody can define is how a field acquires a meaning per writer; if the owner prefers
  `open`, the change is one word in three places and no logic.
- **That the themes are a separate axis.** If the owner's answer to Phase II's audience question
  makes the themes a restatement of the phases, §2.3's problem dissolves and S1's file should
  absorb them. I would rather find that out from the audience answer than guess it now.
- **That S2's deletions are enough.** I measured two stale claims in `ROADMAP.md`. I did not sweep
  the other six apex documents' prose for the same class, because RFC 048 did that sweep for
  release lists and `check_apex_currency` holds the result. Prose *around* a correct block is the
  gap RFC 048 left and §2.2 found once; I do not claim `ROADMAP.md` is the only instance, and I am
  not proposing the sweep inside this RFC.
