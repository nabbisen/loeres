# RFC 053 — clause 4's reporting, bounded

**Author tier.** `architect`. **Date:** 2026-10-31.
**Governing text.** `rfcs/accepted/053-a-sequence-the-repository-holds.md` including
`## 0.1 Amendment 1` — **unchanged by this slice**. Clause 4's requirement is exactly what it was;
only how it reports is corrected. Finding is architect review 123 F1.
**Base.** `560a386357e4af7ff592f51f329be56b80946f73` — C1, C2 and C3 are accepted and are not
reopened.
**Release.** Still `0.24.2`. No version bump. **This is the last slice before the cut.**

One item, in one function. Ten lines and one test.

## The defect, measured

Changing one heading in `rfcs/destination-sequence.md` from `## Phase VI` to `## Phase M`
produces **994 `PHASE STATE` findings**: one *is missing Phase N, between the defined numerals*
line for every absent numeral from `Phase VI` to `Phase CMXCIX`.

`roman_value("M")` is 1000, and `for value in 1..=max` enumerates every absent numeral below it.
Each line is true; together they bury the one datum a reader needs.

**The numerals above are in backticks, and that is not cosmetic.** A fenced block is not
masked — only inline code spans are — so pasting that output verbatim into a tracked document
makes every line of it a phase reference the gate then tries to resolve. This handoff failed
`doc-currency` on its own first draft for exactly that reason: the second time in this slice that
a document *about* the mechanism tripped it. The rule is the sentence C3 added to
`rfcs/destination-sequence.md`, and it is followed here rather than widened, because
fence-awareness would put block state into a per-line masker that `check_rfcs` shares.

**Two consequences for your own review request**, both learned the same way:

1. Write an undefined phase name inside single backticks, or do not write it — a fenced transcript
   is checked like prose.
2. **Do not let a code span wrap a line.** `mask_code_spans` runs per line, so an opening backtick
   on one line and its closing backtick on the next flips the parity of every backtick after it on
   that second line, and text you meant to quote is scanned. That cost this handoff its second
   draft. It over-reports rather than under-reports, which is RFC 050 §6's direction, so it is a
   writing rule and not a defect to fix in this slice.

This is architect review 119 F3 — a renamed table header producing twelve findings for one cause —
at eighty-three times the scale, and it contradicts the instruction in the previous handoff's C2:
*"one finding per defect, not per phase"*.

**The defect is in my specification.** The same C2 also said *"a gap in the sequence is one finding
naming the missing numeral"*, singular. You implemented that sentence literally, which is correct
for a one-numeral gap and contradicts the other sentence for anything else. Nothing you wrote
departs from what I asked.

## The correction

In `phase_numeral_sequence_findings`, in `xtask/src/checks/doc_currency.rs`.

Duplicates already have their own branch and keep it unchanged, so contiguity from I is exactly
**`max == distinct_value_count`** over the readable numerals. Replace the `1..=max` enumeration
with that comparison, and when it fails emit **one** finding carrying the three facts that locate
the cause:

- the highest numeral present,
- how many phases are defined,
- the **lowest** missing numeral.

So the ordinary case still names the actionable thing — deleting Phase V reports a lowest missing
of `Phase V` — and the pathological case reports once rather than 994 times. Keep `render_roman`
for naming, keep the unreadable-numeral branch exactly as it is (review 123 §3 records that it is
why the typo failed loudly at all), and keep the `PHASE STATE` register.

**Do not** reintroduce a per-missing-numeral list, capped or otherwise: a cap is an arbitrary
number in a gate, and the lowest missing numeral is the only one a reader needs to act on.

## The test

One, beside the two from the previous slice, which stay unedited:

- numerals `I`, `II`, `III`, `IV`, `V` plus `M` → **exactly one** gap finding, naming `VI` as the
  lowest missing and `M` as the highest present. `assert_eq!(findings.len(), 1)` is the point of
  the test, so assert it before asserting the contents.

## Verification

1. `cargo xtask doc-currency` → PASS.
2. `cargo xtask check` → **PASS, 20 gates**, on the gate's own exit status, never through a pipe.
3. `cargo xtask release-gate --intended-tag 0.24.2` → PASS. Whole stream to a file.
4. **On the real file, both ends of the range:** `## Phase VI` → `## Phase M` must now produce
   **one** finding — quote it — and deleting Phase V's section must still produce a finding naming
   `Phase V` as the lowest missing. Revert each, and confirm `git diff rfcs/destination-sequence.md`
   is empty afterwards rather than only `git status`.
5. The review request at
   `.git-exclude/review-request/RFC-053-clause-4-bound/REVIEW_REQUEST.md`.

**Not yours**, unchanged: moving RFC 053 to `rfcs/done/`, its Status line naming Amendment 1, the
apex `Implemented scope` field, the release table, the tag, and the push.
