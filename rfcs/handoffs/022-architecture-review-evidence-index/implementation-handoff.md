# Developer Handoff — RFC 022 architecture review evidence index

**Governing RFC.** `rfcs/accepted/022-architecture-review-evidence-index.md`,
including **Amendment 1 (§0)**. The amendment is normative; where §0 and the
original body disagree, §0 wins.
**Status.** Inherited from RFC 022 (Accepted). Architect review recorded;
implementation authorized for the implementer tier.
**Prerequisite.** RFC 024's implementation (`f63970f`) is accepted and landed.
This handoff builds on that tree.

---

## 1. Purpose

Tracked normative documents cite architecture reviews as the basis for design
and evidence acceptance. `git ls-files .git-exclude` returns **0** — none of
those documents exists in the repository, the `0.20.2` tag, or the release
tarball. A reader of the released artifact cannot reach the authority it names.

Close the citation-to-evidence link without publishing deliberation, and record
enough provenance that authority is never again inferred from a filename.

## 2. Background you need

Nine reviews are cited across eleven tracked files: `021`, `022`, `024`, `025`,
`027`, `030`, `031`, `032`, `033`. All nine exist in the maintainer-held corpus —
**verified, zero dangling citations**. This is a traceability fix, not an
evidence-reconstruction problem.

Amendment 1 exists because of a real incident: two reviews were cited as
independent architecture review, acted upon, and later identified as
implementer-tier output. One adopted recommendation had to be reversed. A
hash-only index would not have caught it.

## 3. Applicable requirements

- RFC 022 §0 (Amendment 1), §11, §13, §18.
- **RFC 020 §11.1** — archived review bundles are *"Historical/private review
  input. Not a public normative source."* The index binds identity of **input**.
  It must not present itself, or be described, as conferring authority.
- RFC 000 — anti-pattern: companion documents acquiring a parallel lifecycle.

## 4. Change scope

### 4.1 The index — `rfcs/review-evidence-index.md` (new, tracked)

One row per review document in the maintainer-held corpus:

| Field | Notes |
|---|---|
| Reference | Numeric prefix used by in-repo citations; `—` for pre-numbering reviews |
| Date | From the filename |
| Subject | From the filename |
| SHA-256 | Of the file contents — the integrity anchor |
| **Author tier** | Closed set: `owner`, `architect`, `implementer`, `external`, `unrecorded` |
| Cited in release | Whether a tracked normative document references it |

**A draft exists** at `rfcs/review-evidence-index.md` (untracked in the working
tree). It predates Amendment 1 and has **no author-tier column** — treat it as a
starting point for the prose and regenerate the table.

**Provenance rules — read §0.1 before filling this column:**

- Reviews **036 and 037 are `implementer`**, on direct project-owner statement.
- Every other existing review is **`unrecorded`**. Provenance was not captured at
  request time.
- **Do not infer a tier** from filename, title, tone, or content. A guessed tier
  is a fabricated authority claim — the exact failure the amendment prevents.
  If you believe a tier is knowable, escalate rather than assign it.

### 4.2 Enforcement — `xtask`

Three assertions with deliberately different strengths (§11.3 as amended):

| Assertion | Strength |
|---|---|
| Every review reference in a tracked normative document resolves to an index row | **Enforced, fail-closed** |
| Every index row carries a tier from the closed set | **Enforced, fail-closed** |
| Each registered SHA-256 matches the file | **Verified when the corpus is present; reported `unavailable` when absent** |
| No citation asserts approval for an `implementer`/`unrecorded` row | **Human review — do not attempt to automate** |

Reporting an absent corpus as `unavailable` rather than `passed` is required.
`.git-exclude/` is legitimately missing from a clean extraction, and promoting an
unverifiable result to a pass is the evidence-integrity failure this project
audits for.

Fold the check into `cargo xtask check`. A new top-level subcommand is optional,
not required.

### 4.3 Citation grammar (§11.4)

Canonical form is `architecture review NNN`, three digits, multiples as
`reviews 031/032`. The checker must additionally **report near-misses** — a
`review` token adjacent to digits not matching the canonical form — as a
finding. Silence must mean "no citations present," never "none recognized."

### 4.4 Immutability (§11.5)

Once cited, a review file is immutable, including typo fixes. A correction is a
new numbered document. Record this rule where a maintainer will meet it — the
index prose is the right place.

### 4.5 Incidental currency corrections (§15)

Three stale items ride this RFC:

1. `Cargo.toml` — the comment says APIs are exposed "through RFCs 001-018";
   `0.20.2` shipped 001-021.
2. The post-release version convention — bump `main` to the next patch after a
   release, with the release version set in the finalization revision — is
   recorded nowhere. Add it to `docs/src/development.md` and `CONTRIBUTING.md`.
3. `/docs/book/` is now gitignored; confirm no tracked build output remains.

## 5. Explicit non-change scope

Do **not**: track the review corpus itself (§12 rejected it — it would push ~48
deliberation documents into every release tarball); add lifecycle, status, or
approval semantics to the index (§11.2 — this is RFC 000's handoff anti-pattern
one layer over); add a disposition column (§11.6 defers it deliberately —
transcribing an outcome from the prose that cites the review is circular);
modify any crate under `crates/`; touch `.github/workflows/`, the `0.20.2`
artifact, or the `0.20.1` tag.

If the design appears wrong, **stop and escalate**. Do not adjust the RFC to
match the code.

## 6. Required tests

- a citation with no index row fails;
- a row with a missing tier, or a tier outside the closed set, fails;
- a hash mismatch against a present corpus fails;
- an absent corpus reports `unavailable` — and specifically **does not pass**;
- a near-miss citation format is reported, not silently ignored;
- a canonical multi-reference (`reviews 031/032`) resolves to both rows.

## 7. Required evidence

Observed on the final revision, none reported from memory:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-features --all-targets -- -D warnings
cargo test --workspace --all-features
cargo +1.85.0 check --workspace --all-features
cargo xtask check
mdbook build docs
```

Plus a demonstration that the corpus-absent path reports `unavailable`: run the
check with `.git-exclude/` temporarily moved aside, or via a fixture. Do not
report the corpus-present run as evidence for the absent case.

`release-gate --intended-tag` is **not** required here — no release surface
changes. If you run it, note that `docs/book/` must be removed first or the
mdbook step refuses to proceed.

## 8. Prohibited shortcuts

- **Do not infer an author tier.** `unrecorded` is the correct, honest value.
- **Do not report an unavailable check as passed.**
- **Do not scope the citation check to only the citations you already know
  about.** The first RFC 024 implementation enumerated exactly the phrases it
  had removed and passed while the defect remained. Derive from the documents.
- Do not automate the authority-verb rule. It needs a reader; a keyword matcher
  would produce false confidence, which is worse than no check.
- Do not let the index acquire status columns later "for convenience."

## 9. Known risks

| Risk | Mitigation |
|---|---|
| Tier column invites retroactive guessing | §0.1 forbids it; `unrecorded` is expected to dominate |
| Index read as conferring authority | §0.2 wording is mandatory in the index prose, not optional |
| Citation grammar drift | Near-miss reporting is required, not optional |
| Corpus absent in CI/extraction | `unavailable`, never `pass` |

## 10. Acceptance criteria

RFC 022 §18, including amended items 4a and 4b.

## 11. Required review-request content

1. implementation summary; 2. addressed requirements mapped to §4 items;
3. changed files; 4. important implementation decisions; 5. any difference from
RFC 022 §0, with justification; 6. tests added and executed; 7. test results;
8. build and static-analysis results, including the corpus-absent demonstration;
9. unresolved issues; 10. known limitations; 11. requested review focus.

**Record your own author tier in the request.** Under Amendment 1 §0.1 that is
now captured at request time, and this request is the first to do so.

Send it to the architect. Do not move RFC 022 to `done/` — that follows the
release that carries it.
