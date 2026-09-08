# Developer Handoff — RFC 024 correction series

**Governing RFC.** `rfcs/accepted/024-post-release-documentation-steady-state.md`,
including **Amendment 1 (§0)**. The amendment is normative; where §0 and the
original body disagree, §0 wins.
**Status.** Inherited from RFC 024 (Accepted). Owner approved implementation
2026-07-31.
**Assigned to.** Implementer tier.
**Supersedes.** The partial implementation in `a27c2c7`, which predates
Amendment 1 and does **not** match the current design.

---

## 1. Purpose

Land the amended RFC 024 design and clear the four blockers and one cosmetic
finding raised against the first attempt, so that `main` carries a checker that
agrees with its own governing RFC and an ordinary commit can follow a release.

## 2. Background you need

`0.20.2` released on 2026-07-30. Before that, `doc-currency` admitted only two
apex states — a form pinned to `0.20.2` by a compile-time constant, and an
expired pre-release draft form. Neither describes "released X, developing Y", so
bumping the workspace version failed the gate by construction.

A first implementation (`a27c2c7`) introduced a two-field apex block. Review
found four blockers; two were design defects, corrected by Amendment 1. **Read
§0 of the RFC before touching code** — the field names and invariants changed,
and the committed implementation is stale against them.

## 3. Applicable requirements

- RFC 024 §0 (Amendment 1), §11, §13, §16.
- **RFC 020 §11.3** — each apex specification must state its *last reconciled
  repository release*. This is why the lineage field is a checked currency claim
  and not a stale-tolerant anchor. Do not weaken it.
- **RFC 020 §11.1** — `CHANGELOG.md` is the authority for what shipped and when.
- Roadmap §1.5 — architectural monotonicity. Nothing here may loosen an earlier
  accepted constraint.

## 4. Change scope

### 4.1 Apex block form (`xtask/src/checks/doc_currency.rs` + apex trio)

Marker stays `**Release currency metadata.**`. Fields become:

```text
> **Release currency metadata.**
> This tree: **0.20.3**.
> Last reconciled repository release: **0.20.2**.
> Implemented scope: **RFCs 001-021**.
```

`doc-currency` must assert:

1. *This tree* equals `workspace.package.version`;
2. *Last reconciled* is **strictly less** than *this tree* — equality is never
   valid, there is no release mode, and no authenticated context is to be
   established (RFC 024 §0.1);
3. *Last reconciled* names a version whose `CHANGELOG.md` record is marked
   **released** (§4.2);
4. *Implemented scope* equals the exact derived set (§4.3);
5. all three apex blocks are identical;
6. the retired RFC 020 draft marker and RFC 021 conditional marker are absent.

Delete the `(released)`/`(unreleased)` qualifier logic entirely.

### 4.2 CHANGELOG release-status marker (new, specified here — do not invent one)

A version's release status must be machine-readable. Keyword-sniffing the
heading is **not** acceptable: `## [0.20.1]` carries a date but names a tagged,
quarantined, never-released candidate.

Add a status line as the first line beneath the version heading:

```markdown
## [0.20.2] — 2026-07-22 — RFC 021 conditional finalization
**Release status:** released (tagged 2026-07-22, distributed 2026-07-30)
```

Permitted values: `released`, `unreleased`, `withdrawn candidate`.

**Required on:** the record named by the apex lineage field, and any unreleased
record. Permitted and encouraged elsewhere; retrofitting the full history is
**out of scope** — do not churn old entries.

`0.20.1` must be marked `withdrawn candidate`. It was tagged locally, never
pushed, and must never be reused.

### 4.3 Exact implemented-scope set

Derive from `rfcs/done/`, **excluding RFC 000** (it governs the RFC directory,
not product scope). Render canonically: ascending, three-digit, comma-separated,
**every maximal run of two or more consecutive numbers collapsed to `NNN-NNN`**;
only an isolated number stands alone. `{001..018, 021, 024}` →
`RFCs 001-018, 021, 024`. `{020, 021}` → `RFCs 020-021`, never `RFCs 020, 021`.

A maximum is not a set. Do not re-emit a contiguous range from the highest
number.

### 4.4 Intended-tag preflight (`release_gate.rs`)

Already sources last-reconciled from the apex and requires the tag to advance
past it. Keep that; update it to the renamed field. Add a code comment at
`AUTHORITATIVE_REMOTE` recording that changing the release remote is an
architecture-reviewed governance change, not a routine refactor.

### 4.5 Blocker B1 — finish retiring the conditional state

Only the four exact before/after-`P` sentences were removed. Present-tense
claims that the apex, lifecycle and recovery state are still conditional remain,
and now contradict adjacent released-state prose. Known locations:

- `README.md` ~82-84 — RFCs 019-021 "conditionally staged", "do not prove activation"
- `ROADMAP.md` ~24-26, ~45-57, ~188-190
- `docs/src/specifications.md` ~14-20, ~38-40 — says the apex carries RFC 021's
  conditional block while the apex plainly shows the RFC 024 block
- `docs/specs/loeres-requirements-v1.md` ~8, ~24-28, ~1121-1124
- `docs/specs/loeres-external-design-v1.md` ~3, ~19-23, ~1386-1401
- `docs/specs/loeres-roadmap-milestones-v1.md` ~3, ~16-20, ~45, ~1041-1048, ~1059-1075

Treat that list as a starting point, not a complete inventory. Rewrite
current-facing claims to the post-release state; **preserve RFC 021 protocol
history where it is explicitly historical**. Then extend the bounded stale-phrase
checks to reject the representative markers you found, and add regression
fixtures proving they fail.

### 4.6 Blocker B4 — `0.20.3` changelog record and chronology

Add `## [0.20.3]` marked `unreleased`, describing this correction series.
Correct the `0.20.2` narration: tagged 2026-07-22, **distributed 2026-07-30**.
Keep the historical conditional-protocol description intact.

### 4.7 Finding N4 — reflow damage

`docs/src/introduction.md:38` — `projected-first- order` → `projected-first-order`.
Restore the missing trailing newline in `docs/src/introduction.md` and
`docs/src/recovery-roadmap.md`. (`dependency- and evidence-gated` and
`documentation- and build-metadata` are correct suspended hyphens — leave them.)

## 5. Explicit non-change scope

Do **not** touch: any crate under `crates/`; public APIs, features, dependencies,
solver behaviour; `.github/workflows/`; the `0.20.2` released artifact; the
`0.20.1` tag; RFC 024's design text. Do not push, tag, or publish anything.

If the design appears wrong, **stop and escalate** — do not adjust the RFC to
match the code.

## 6. Required tests

Per RFC 024 §13 as amended:

- apex/workspace version disagreement fails;
- *last reconciled* equal to or greater than *this tree* fails;
- *last reconciled* naming a record marked unreleased or withdrawn fails;
- *last reconciled* naming a version with no changelog record fails;
- apex blocks differing across the trio fail;
- scope set: accepted gap, archived gap, missing number, malformed filenames each
  fail; contiguous and gapped sets each render canonically; the **two-element
  boundary** renders `NNN-NNN` and the comma-separated pair is rejected; RFC 000
  excluded;
- intended-tag: tag equal to workspace version and exceeding last reconciled
  passes; not exceeding fails; existing local or remote tag fails;
- B1 regression fixtures: representative stale current-facing phrases fail.

## 7. Required evidence

All observed on the final revision, none reported from memory:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-features --all-targets -- -D warnings
cargo test --workspace --all-features
cargo +1.85.0 check --workspace --all-features
cargo xtask check
mdbook build docs
cargo xtask release-gate --intended-tag 0.20.4    # must reach and pass apex
                                                  # binding and both collision checks
```

The last command is **mandatory**. Its absence was blocker B4: unit tests on
`validate_intended_binding` alone are not end-to-end evidence. Use a version
above the current workspace version so the preflight is exercised without
implying a release intent.

## 8. Prohibited shortcuts

- **Do not scope a check to only what you changed.** The first implementation's
  stale-prose check enumerated exactly the phrases already removed, so it passed
  while the defect it existed to catch was present. That produced B1.
- Do not weaken or make advisory any check to obtain a pass.
- Do not report focused unit tests as end-to-end gate evidence.
- Do not keyword-sniff changelog headings for release status.
- Do not retrofit or reword historical changelog entries beyond §4.6.
- Do not delete prose to make a check pass. An earlier attempt at §4.5 replaced
  whole paragraphs and destroyed recovery history and scope disclosures; work at
  sentence granularity and diff every file you touch.

## 9. Known risks

| Risk | Mitigation |
|---|---|
| §4.5 is a judgement-heavy sweep; the location list is incomplete | Diff every touched file; flag anything ambiguous rather than guessing |
| Reflow can silently damage prose | Check for broken hyphenated compounds and trailing newlines |
| The stale `a27c2c7` implementation may be mistaken for current | Re-read RFC 024 §0 first; expect to change field names and delete the qualifier |
| Scope-set rendering has one boundary case | The two-element run test is mandatory |

## 10. Acceptance criteria

RFC 024 §16 items 1-12. Item 13 (owner approval) is already satisfied for
implementation; closeout review remains.

## 11. Required review-request content

Per the organization's handoff format:

1. implementation summary;
2. addressed requirements, mapped to §4 items and blocker IDs;
3. changed files;
4. important implementation decisions;
5. any difference from RFC 024 §0 — with justification, or none;
6. tests added and executed;
7. test results;
8. build and static-analysis results, including the mandatory end-to-end
   preflight output;
9. unresolved issues;
10. known limitations;
11. where you want review focused.

Send it to the architect. Do not move RFC 024 to `done/` — that is a closeout
decision after review and owner approval.
