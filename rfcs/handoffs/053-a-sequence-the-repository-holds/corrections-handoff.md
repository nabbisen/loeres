# RFC 053 — corrections handoff

**Author tier.** `architect`. **Date:** 2026-10-31.
**Governing text.** `rfcs/accepted/053-a-sequence-the-repository-holds.md`, **including
`## 0.1 Amendment 1`**, which governs where it differs from §3 S3. Findings are architect review
122.
**Base.** `3a2f8caedd5e00f3fa9c8b7a63370a13b5ea0b34` — accepted; the implementation needs no
rework, and nothing below reverses a decision in it.
**Release.** Still `0.24.2`. No version bump.

Three items. **C1 and C3 are a sentence each; C2 is one clause and two tests.** Nothing else in the slice is
reopened — the `done_rfc_numbers` design choice flagged in the request §3.2 is **confirmed
correct** (review 122 §5) and must not be changed.

## C1 — the CHANGELOG count is low by one

The `## [0.24.2] — unreleased` entry says:

```text
Two stale claims recurring in `ROADMAP.md` — a hand-maintained RFC range
RFC 048 had already deleted from seven other documents, and a shipped release still described as
upcoming — are deleted and replaced with a pointer to the fields that already hold them,
```

**Three** false claims were deleted, not two. The third is in the span S2.3 replaced: it asserted
that `README.md`'s headline claims "high-throughput server solving", where
`grep -c -i throughput README.md` is **0** — the headline no longer says it. The implementation
handoff §0.2 lists all three.

Say three and name the third. A count in a user-facing record must not be low. The
`E0027`/`E0063` prediction that went with the same span is **not** to be counted — it was a
pre-slice prediction, not a claim of current state — so do not make it four.

## C2 — clause 4: the sequence file defines a duplicate-free, contiguous set from I

Per Amendment 1. `rfcs/destination-sequence.md`'s phase numerals must form a contiguous roman
sequence starting at I — I, II, … N — with **no numeral defined twice**.

**Why, measured.** A second `## Phase III — a second, conflicting section` with
`**State.** deferred`, inserted before Phase IV, produced **zero findings** (review 122 F4). And
because every phase heading cites its own phase, clause 1's protection against a *deleted* section
comes only from other files — which for Phases IV and VI is just the sequence file and this RFC's
handoff. Clause 4 catches a deleted phase whether or not anything cites it.

**Where.** `check_phase_sequence` in `xtask/src/checks/doc_currency.rs`, over the numerals
`parse_destination_phases` already returns. Register `PHASE STATE`, as clauses 2 and 3 use.

**Shape.** A small function over the parsed numerals, pure, so it is testable on literals the way
`phase_state_findings` is. It needs a roman-numeral-to-value conversion for the contiguity check;
keep it local, minimal and total over the numerals the file can hold — a numeral it cannot read is
a finding, not a skip. **Fail closed**, as `is_published` does.

**One finding per defect, not per phase** (review 119 F3): a duplicated numeral is one finding
naming it; a gap in the sequence is one finding naming the missing numeral.

**Tests — two, named for what they pin:**

1. a numeral defined twice → one finding naming it;
2. a sequence missing an interior numeral (I, II, IV) → one finding naming the missing one.

Add them beside the eight already in `doc_currency.rs`'s test module, which stay as they are.

**What clause 4 does not do, and must not be made to do.** It does not widen the reference matcher
to the plural form. Amendment 1 records why: `Phases II-VI` would require reading every roman run
on any line containing `Phases`, and `CI` is a valid roman run, so it buys a false-positive class
on exactly the lines that discuss this mechanism. The residual — a plural-only reference beyond
the defined range — is stated in the amendment and is accepted.

## C3 — one sentence in the sequence file, added after the review

Architect review 122 §5.1 called this a nit needing no action. **A datum arrived immediately
after**, and it changes the call: writing Amendment 1 — the amendment that *documents* the plural
form — put the words `Phase VII` into prose unbackticked, and `doc-currency` failed on the
architect's own text within minutes:

```text
PHASE REFERENCE: `Phase VII` is not defined in rfcs/destination-sequence.md; cited at
./rfcs/accepted/053-a-sequence-the-repository-holds.md:46
```

The gate was right, the fix was one pair of backticks, and nothing was wrong with the
implementation. But the reach of clause 1 — **every** tracked Markdown line, fenced code blocks
included, inline code spans excepted — is now something a writer meets by surprise rather than by
reading.

Add one sentence to `rfcs/destination-sequence.md`'s front matter, after the numbering-distinction
sentence: a mention of a phase this file does not define must be written inside single backticks,
because `doc-currency` resolves every `Phase <roman>` reference in a tracked document, in prose
and in fenced blocks alike. State it as the mechanism's reach, not as an apology for it.

No code change. The behaviour is correct and conservative; only its visibility is being fixed.

## Verification

1. `cargo xtask doc-currency` → PASS.
2. `cargo xtask check` → **PASS, 20 gates**, on the gate's own exit status, never through a pipe.
3. `cargo xtask release-gate --intended-tag 0.24.2` → PASS. Whole stream to a file.
4. **Prove clause 4 catches both halves on the real file**, not only in tests: duplicate Phase III,
   record the finding, revert; delete Phase V's section **and** confirm the finding names the
   missing numeral rather than relying on a citation elsewhere, revert. `git status --short` empty
   after each.
5. The review request at
   `.git-exclude/review-request/RFC-053-clause-4-corrections/REVIEW_REQUEST.md`, which should
   state whether C3's sentence reads as the mechanism's reach or as an apology for it.

**Not yours**, unchanged: moving RFC 053 to `rfcs/done/`, its Status line naming Amendment 1, the
apex `Implemented scope` field, the release table, the tag, the push.
