# RFC 054 — completeness: two sites the survey missed, and the document the gate missed

**Author tier.** `architect`. **Date:** 2026-11-24.
**Governing text.** `rfcs/accepted/054-a-suitability-claim-a-gate-can-assert.md`, **including
`## 0.1 Amendment 1`**, which adds the two sites and the document set entry this slice lands.
Finding is architect review 131 F1 (and F2 for the CHANGELOG clause).
**Base.** `f5a37a1fa6187314c4789431f87e08d6e405bf36` — RFC 054's S1-S4, accepted, not reopened.
**Release.** `0.25.0`. **No version bump.** `cargo xtask release-gate --intended-tag 0.25.0`.
**This is the last slice before the cut.**

Three items, each small. **The defect being fixed is mine**: §2.3's survey was run with a `head`
limit and its table reads as exhaustive. Nothing you did is being corrected.

## 1. Two sites S2 should have named

Both are the same claim S2 already deleted elsewhere. **Delete and point, never rewrite** — a
corrected adjective is still a claim no gate asserts.

### 1.1 `docs/specs/loeres-roadmap-milestones-v1.md:53`

Old, exactly:

```text
3. **Cluster Layer:** dynamic, cloud-native, high-throughput execution.
```

New:

```text
3. **Cluster Layer:** dynamic, server-side execution.
```

This is the sentence shape S2 corrected at `loeres-requirements-v1.md:35` — *"high-throughput,
dynamically allocated, parallel, cloud-native computation"* → *"dynamically allocated, parallel,
server-side computation"*. `cloud-native` goes with `high-throughput` for the same reason it went
there: neither is asserted by anything. **The numbered list and the two sibling layer lines are not
touched.**

### 1.2 `docs/specs/loeres-requirements-v1.md:1225`

Old, exactly:

```text
| AC-014 | Server solver design supports dynamic allocation and high-throughput integration without contaminating edge crates. |
```

New:

```text
| AC-014 | Server solver design supports dynamic allocation and server-side integration without contaminating edge crates. |
```

An acceptance criterion is **asserted**, which is why this one is a defect where
`external-design:829`'s *Purpose* column is not. The zero-bleed half — *"without contaminating edge
crates"* — is gate-asserted and stays exactly as written.

### 1.3 The three that are not defects — do not touch them

Named so the judgement is not re-litigated at the next grep: `loeres-requirements-v1.md:730` (a
requirement about the **caller's** high-throughput paths), `:1287` (a traceability row naming a
requirement theme), `loeres-roadmap-milestones-v1.md:658` (a stated objective). And **two are
disclaimers that must survive**: `loeres-roadmap-milestones-v1.md:730` and
`docs/src/threat-model.md:82`, both of the form *"does not establish high-throughput"*. Deleting a
disclaimer because it contains the word would be the worst possible reading of this slice.

## 2. The gate's document set gains the third apex spec

Add `docs/specs/loeres-roadmap-milestones-v1.md` to `suitability_claim_docs()`.

**Measured, and reproduce it before and after:** a `safety-critical` appended to that file leaves
`cargo xtask doc-currency` **passing** today. After this change it must **fail**, naming the file
and line. Quote both runs.

**`docs/specs/loeres-reconciliation-traceability-v020.md` is deliberately not added.** Its OQ-012
row records that a safety-critical disclaimer is *missing* — naming a term in order to disclaim it,
which is the stated rule that excludes `TERMS_OF_USE.md` and `docs/src/threat-model.md`. Add a
clause to the exclusion's doc comment naming it alongside those two, so the omission reads as the
rule rather than as an oversight.

**One test**, beside the four from the previous slice, which stay unedited: the covered set
contains `docs/specs/loeres-roadmap-milestones-v1.md`, asserted on `suitability_claim_docs()`'s
membership the way the existing exclusion test is.

## 3. The CHANGELOG entry — one clause out, one sentence in

**Out.** The entry credits removing the throughput claim from *"two crate purpose lines and a
published rustdoc line"*. **There is no published rustdoc line**: `crates/loeres-backend-std/src/batch.rs`
left with RFC 055, as your own §1 reported. Delete `and a published rustdoc line`. Your request and
your entry contradicted each other, and the entry is the one a user reads.

**In.** A sentence stating what the slice does **not** claim: the throughput claims surviving at
`loeres-requirements-v1.md:730`, `:1287`, `loeres-roadmap-milestones-v1.md:658` and
`external-design:723`, `:829`, `:1370` are deliberate — requirements about a caller's paths,
traceability rows, stated objectives, design permissions and rationale — and the two disclaimers
must survive. Write it as the judgement it is, not as an apology: the entry should let a reader
grep the word themselves and find the same six sites without concluding the slice was incomplete.

## 4. Verification

1. `cargo xtask doc-currency` → PASS.
2. `cargo xtask check` → **PASS, 20 gates**, on the gate's own exit status, never through a pipe.
3. `cargo xtask release-gate --intended-tag 0.25.0` → PASS, whole stream to a file.
4. **The complete survey, run with no limit and its count stated** — this is the method correction
   Amendment 1 adopts, and this slice is the first to use it:

   ```sh
   git grep -ni "high-throughput\|high throughput" -- . | grep -v '^rfcs/' | grep -v '^CHANGELOG.md' | wc -l
   ```

   It is **12** before this slice. Quote the count and the full listing after it, and confirm every
   remaining line is one of the six §1.3 names.
5. **The gate's new coverage, both directions** (§2): the term in
   `loeres-roadmap-milestones-v1.md` must now fail, and must pass when backticked. Revert both and
   confirm `git diff` on that file is empty — not only `git status`.
6. The review request at
   `.git-exclude/review-request/RFC-054-completeness/REVIEW_REQUEST.md`.

**Not yours:** moving RFC 054 or RFC 055 to `rfcs/done/`, their Status lines naming Amendment 1,
the apex `Implemented scope` field, the release table, the tag, the push, publication, and
`docs/specs/loeres-requirements-v1.md:958`'s backtick — which architect review 131 §1.2 accepted as
landed and which is not reopened.
