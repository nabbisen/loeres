# Release 0.25.0 — scope and decision points

**Author tier.** `architect`. **Date:** 2026-10-31.
**Status.** Planning. No slice has landed; the workspace is at `0.24.2`, a placeholder.
**Authorized by.** The owner authorized this file on 2026-10-31. The scope in §1, each RFC in it,
and the release itself are each authorized separately.

**Whose file this is: the owner's.** It records what is in scope, what is decided, and at what
cost. The cut procedure, when it comes, is its own file (§5). The dev team's instructions are the
per-RFC handoffs, never this file; §4 states the boundary.

**This directory's name is a prediction, and the release convention forbids relying on one.**
See §2 — as scoped in §1.1 alone, the release is `0.24.2`.

## 1. Scope

### 1.1 Recommended, and the only item I would open without a further decision: RFC 053

**RFC 047's phases are not controlled.** Measured 2026-10-31:

1. **They are defined nowhere in the repository.** Phases I–VI exist only in
   `.git-exclude/reviewed/106-overall-strategy-the-state-worth-aiming-at-2026-10-28.md` §5. That
   path is gitignored at `.gitignore:24`.
2. **Three tracked sites cite them anyway** —
   `rfcs/done/049-the-readme-tells-a-user-what-to-write.md:7` ("RFC 047 **Phase I**"), `:188` and
   `:226` (deferring work to "Phase III" and "Phase V"), and `rfcs/README.md:82`. A `done/` RFC
   defers work to a phase the repository does not contain.
3. **The placement was deliberate and it was mine.**
   `rfcs/accepted/047-the-state-worth-aiming-at.md:152` §5 item 1 keeps sequencing "in the review
   corpus as advisory architect material", citing architect review 114 §3. That argument — do not
   freeze an unmeasured order under an Accepted RFC's amendment discipline — is sound, and I
   still hold it. Reading it as a reason to keep the sequence *outside the repository* is a
   different decision, and the wrong one: the third option, a tracked file that is **not** an RFC
   and so carries no amendment discipline, was never priced.
4. **It cannot be corrected where it lives.** Review 106 is cited
   (`rfcs/review-evidence-index.md:190`, ✓) and so is immutable.
5. **Nothing watches it.** No gate reads a phase. Deleting that one untracked file loses the
   sequence, with no failure anywhere and no copy in history.
6. **The numbering already collides.** `docs/specs/loeres-roadmap-milestones-v1.md:64-73` and
   `ROADMAP.md` use *Phase 0–3* for the milestone layering — Phase 2 is the static backend —
   while review 106 uses *Phase II* for an audience decision. Two phase vocabularies, arabic and
   roman, in one project: RFC 047 property **D**'s confusion in our own documents.
7. **A second defect of the same class, in the same file.** `ROADMAP.md` is apex document 4
   (`xtask/src/checks/doc_currency.rs:21`). Its machine-held block correctly reads
   `Implemented scope: **RFCs 001-046, 048-052**`. Four lines below it, `ROADMAP.md:88` says
   "**RFCs 001-034 are implemented**", and `ROADMAP.md:59` says "**Next: T7 as `0.23.0`, then
   T6**" — T7 shipped as RFC 041 in `0.23.0` on 2026-10-08. Stale prose beside a correct checked
   block: **RFC 048's defect recurring**, because RFC 048 scoped only the release-list sentence.

**Shape — three slices, no new gate.**

- **S1 — one home.** `rfcs/destination-sequence.md`: Phases I–VI, each with its title, the
  properties it serves, a state of `unscheduled` / `scheduled` / `shipped`, and the RFCs that
  implemented it. Order and cost are **explicitly advisory** and revisable; the file is not an
  RFC, so RFC 000's amendment discipline does not reach it, which is the whole point of §5 item
  1's argument honoured in a tracked place. One sentence distinguishes the two phase vocabularies
  (defect 6). The RFC tree already holds two non-RFC files at that level — `rfcs/README.md` and
  `rfcs/review-evidence-index.md` — so the implementer verifies the new path against
  `check-rfcs`'s folder expectations rather than assuming it is inert.
- **S2 — delete, do not rewrite.** `ROADMAP.md:59` and `:88` are removed and replaced by
  pointers: `CHANGELOG.md` is the release record, S1's file is the sequence, the apex block is
  the scope. That is RFC 048 S1's remedy, applied to the prose RFC 048 did not scope. Rewriting
  them would restore a hand-maintained claim, which is what rotted.
- **S3 — one assertion in `doc-currency`.** Every `Phase <roman>` reference in a tracked document
  resolves to a phase defined in S1's file; a phase marked `shipped` names at least one RFC, and
  every RFC it names is in `rfcs/done/`; a phase naming an RFC not in `rfcs/done/` cannot be
  `shipped`. Historical records are **in scope by design** — the point of the assertion is that
  `rfcs/done/049-…:188`'s reference resolves. The predicate must mask inline code spans
  (RFC 050 S2) and must not match the arabic milestone phases.

**Why the assertion and not S1–S2 alone.** `ROADMAP.md:88` is the measurement: tracked prose a
human keeps current is insufficient, because we already tried exactly that in exactly that file
and it went stale inside one release with twenty gates green.

**Cost.** One dev-team slice — RFC, implementation, review. The state field is then maintained,
and it joins the cut's couplings (§5). **What it does not do:** it does not schedule anything.
Phase I is `shipped`; II–VI start `unscheduled`, and a state change remains my recommendation and
the owner's authorization.

### 1.2 Candidate A — a release's RFC manifest

RFC 052 §0.1 and the `0.24.1` cut plan §9 record that `docs/src/specifications.md`'s
`Carries RFCs` column is bounded and ordered but **not verified**: nothing in this repository
records which RFCs a release carried, so this release's own cell was written by hand from the cut
plan's table. **Cost:** a per-release record — most plausibly a field in each `CHANGELOG.md`
entry — plus one clause. **Buys:** truth for one table column, and a source S1's `shipped` state
could read instead of being asserted twice. **Recommendation:** take it only together with §1.1,
and only if the owner wants that column load-bearing; on its own it is the smaller problem.

### 1.3 Candidate B — T4, a benchmark harness

The owner's fourth visitor question, "how effective or powerful", deferred by RFC 035 §4 and
scoped by architect review 086. **Review 086's first obligation is already discharged**: `README.md`
no longer claims throughput (measured 2026-10-31; the headline review 086 objected to is gone).
What remains is the question itself: effectiveness and counted-work cost are deterministic and
reproducible, wall-time throughput is environment-dependent and can only ever be advisory.
**Cost:** much the largest item here, and the only one that **adds callable surface** — so it is
the item that decides §2.

### 1.4 Candidate C — the permanent-gap question

The `0.24.1` cut plan §2.1: if RFC 047 is never implemented as a unit, `RFCs 001-046, 048-052`
carries that gap forever. **Leave it.** It is true, and it has not yet cost anything. I will not
propose changing what the scope field means until it does.

### 1.5 Not in this release

- **Phase II's audience question.** The owner's to answer; see §3 D2. I can cost it, not decide it.
- **T6's three remaining items** — metadata-only observability, the mock-only `ffi-gateway` seam,
  the process-local validation cache. Closed until a real consumer exists; the gateway especially,
  being external surface with security weight.
- **T5 (LP).**

## 2. The position is derived from the slices, and this directory's name is a prediction

`docs/src/development.md` §"A slice that moves the release's position bumps the version before the
cut" states it: what `main` carries between releases is **a placeholder, not a prediction**, and
its table derives the position — *only measures or documents* → the patch already on `main`;
*adds functionality a caller can call* → a minor, bumped by the first such slice.

**As scoped in §1.1 alone — documents plus one assertion in an existing gate — nothing adds
callable functionality, so the release is `0.24.2`,** the placeholder already on `main`, and this
directory should be `rfcs/handoffs/release-0.24.2/`. It becomes `0.25.0` only if the owner adds
candidate B (§1.3) or another additive slice.

I proposed the `0.25.0` name before deriving the position. That is the same error as review 121 §5
item 1, where I named `0.25.0` for what the convention made `0.24.1`, from the size of the
sequence rather than from the rule. **This directory is renamed the moment §3 D1 is answered**,
and no slice runs `release-gate --intended-tag` against a version this file has guessed.

## 3. Decision points — what the owner is asked, and when

**D1 — the scope. Now.** Three forms, costs in §1:

| choice | cost | what it buys |
| --- | --- | --- |
| **§1.1 alone** (recommended) | one dev-team slice; a maintained state field | the sequence enters the repository and a gate holds it; `ROADMAP.md`'s two stale claims go; release is `0.24.2` |
| §1.1 + candidate A | a second slice, and a new per-release field at every cut | one more hand-held column becomes checked; release is `0.24.2` |
| §1.1 + candidate B | much the largest; design, harness, and an advisory/deterministic split | answers the fourth visitor question; **forces the minor**, release is `0.25.0` |

**D2 — Phase II's audience question. Not blocking, and not mine.** Nothing in §1 depends on it;
RFC 047 property **D** does, and `rfcs/done/049-…:226` records it undecided. **Cost of deferring:**
Phase III — the front door's contract — stays unschedulable, because review 106 §5 puts property
**D** behind the audience statement. **Cost of answering now:** it commits a published scope
statement that later capability work must stay inside.

**D3 — the release, and publication.** Separately authorized, publication the more so
(RFC 021 §7), and irreversible.

## 4. What the dev team preps, and what it does not

**Prepped by the dev team:** every slice in the authorized scope, its review request, its
`CHANGELOG.md` entry, `cargo xtask check`, and
`cargo xtask release-gate --intended-tag <version>` once §2's position is fixed.

**Not prepped by the dev team:** the finalization revision, the tag, the push, the RFC 028
anchor, publication, and the post-release reconciliation. That boundary is
`docs/src/development.md`, which states that no workflow in this repository publishes and the
development team does not publish.

**This file is the owner's record, not a dev-team instruction.**

## 5. The cut, when it comes

Its own file, `rfcs/handoffs/release-<version>/cut-plan.md`, written before execution rather than
recalled during it. One coupling is already known: if §1.1 S3 ships and any phase's state changes
at the cut, that change joins the single post-release commit that already has to carry the
`CHANGELOG.md` release mark, the release-table row and the apex `Last reconciled` move together
(RFC 052 clauses 1 and 4).

## 6. Open after this release

- **RFC 047 Phases II–VI stay unscheduled** — visibly, in S1's file, rather than in my memory.
- The permanent-gap question (§1.4).
- T4 (unless D1 takes candidate B), T5, and T6's three remaining items.
- Carried unchanged from `rfcs/handoffs/release-0.24.0/scope-and-decision-points.md` §6: the
  `f64`-only disagreement direction at `FRAC_BITS = 24`; whether `check-public-api` should carry
  an API-shape baseline; and a distance-aware default step for the curvature-free case
  (RFC 039's open item).
