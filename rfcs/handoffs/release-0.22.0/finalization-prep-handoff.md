# Developer Handoff — `0.22.0` finalization preparation

**Assigned to.** Implementer tier.
**Authorized.** Project owner, 2026-09-24, for the **preparation only**.
**Scope boundary.** **You prepare; the architect and the owner cut.** Do **not**
run `release-gate` as the release act, do **not** create or push a tag, and do
**not** touch `.git-exclude/release-evidence/`. Commit the prepared tree and send
the review request; the architect verifies, tags and publishes.

---

## 1. Why this is `0.22.0` and not `0.21.4`

`ConstrainedSolveRecord<S>` gained a field and `#[non_exhaustive]`. It shipped in
`0.21.1` with public fields and no such attribute, so this is **source-breaking**
(`E0063` / `E0639` / `E0638`). Under Cargo's `0.x` rules the **minor** is the
breaking position (RFC 034 Amendment 3 §0.3.1). This is the first minor bump in
this sequence — every release since `0.21.0` has been a patch.

## 2. RFC 034 to `done/`

`git mv rfcs/accepted/034-infeasibility-detection.md rfcs/done/`, and set:

```text
**Status.** Implemented (v0.22.0). Amendments 1-3 were made while Accepted, under RFC 000's in-place-amendment rule.
```

RFC 034 carries **three** amendment headings, and `check-rfcs` asserts a `done/`
RFC's Status line accounts for every one (RFC 025 §13). Then move its row in
`rfcs/README.md` from the Accepted table to the Done table with status
`Implemented (v0.22.0)`, and leave the Accepted table reading `None currently.`

## 3. Version bump — `0.21.4` → `0.22.0`, four lockfiles

`Cargo.toml`'s workspace `version`, then refresh **all four** lockfiles by running
a check in each location: the workspace, plus `examples/cluster-batch-solve`,
`examples/cluster-qp-constrained` and `examples/device-box-pfo`. All four must
read `0.22.0`.

## 4. Scope prose — change exactly these five, leave the other nine alone

This is the step that has gone wrong before. `0.21.3` **genuinely carries RFCs
001-033**, so every sentence saying so is a **historical fact and must not be
touched**.

**CHANGE to `001-034` (current-scope claims) — five sites:**

| File | What it says now |
|---|---|
| `Cargo.toml:25` | `# Member crates expose implemented public APIs through RFCs 001-033.` |
| `ROADMAP.md:61` | `RFCs 001-033 are implemented: …` — also add RFC 034 to the list, as in-this-`0.22.0`-tree, not yet released |
| `docs/specs/loeres-external-design-v1.md:11` | `> Implemented scope: **RFCs 001-033**.` |
| `docs/specs/loeres-requirements-v1.md:15` | `> Implemented scope: **RFCs 001-033**.` |
| `docs/specs/loeres-roadmap-milestones-v1.md:10` | `> Implemented scope: **RFCs 001-033**.` |

**DO NOT CHANGE — nine sites**, each of the form *"Repository release `0.21.3`
shipped 2026-09-24 and carries RFCs 001-033."*: `README.md`, `ROADMAP.md` (two
occurrences, one of them **line-wrapped**), `docs/specs/loeres-external-design-v1.md`,
`docs/specs/loeres-requirements-v1.md`, `docs/specs/loeres-roadmap-milestones-v1.md`,
`docs/src/introduction.md`, `docs/src/specifications.md`, `docs/src/threat-model.md`.

## 5. The release-candidate clause — append to those same nine

Immediately after each of the nine sentences in §4, append:

```text
 This tree is `0.22.0` and is not itself a release until it is tagged and distributed.
```

**Sweep wrap-tolerantly.** One of ROADMAP's two occurrences wraps across a line
break, so a line-based `git grep` finds only one of them. Normalise whitespace
before matching. Expect **exactly nine** insertions, and report the count.

## 6. Apex currency blocks — `This tree` only

Set `> This tree: **0.22.0**.` in the three apex specs.

**Leave `> Last reconciled repository release: **0.21.3**.` alone.** RFC 024's
invariant is strict — `this tree > last reconciled`, and **equality is never
valid**. Reconciliation to `0.22.0` belongs to the post-release commit, which the
architect writes.

## 7. CHANGELOG

Rename the heading `## [0.21.4] — unreleased — post-0.21.3 development` to:

```text
## [0.22.0] — 2026-09-24 — <a short release theme>
```

**Keep `**Release status:** unreleased`.** Nothing may claim a tag or a
distribution that has not happened; the architect sets `released (…)` in the
post-release commit. Write a release summary above the existing RFC 034
subsections, and it **must** state, in its own right and not only in the
breaking-change bullet:

- the **source break** and who it affects (anyone constructing
  `ConstrainedSolveRecord` by struct literal), with the fix;
- that `infeasibility_evidence` is a **heuristic, wrong in both directions**,
  with the measured rates;
- that no answer changes and no other type changed shape.

## 8. Explicit non-scope

- **No tag, no push of a tag, no `release-gate` run as a release act, no writing
  under `.git-exclude/release-evidence/`.** Running `cargo xtask check` and
  `cargo xtask conformance` to verify your own work is expected and fine.
- No `rfcs/done/` file edited (RFC 025 §11).
- No kernel, test or behaviour change of any kind — this slice is metadata and
  prose only.
- Do not set `Last reconciled` (§6) or the release status (§7).

## 9. Required evidence

`cargo fmt --check`, clippy `-D warnings`, `cargo test --workspace --all-features`,
MSRV 1.85, `cargo xtask check` (**17 gates**), `cargo xtask conformance` plus
`--suite extended` and `--suite adversarial` (24/24, 6/6, **26 of 31** — the same
five), `mdbook build docs --dest-dir target/xtask-book/local`, and:

- all four lockfiles showing `0.22.0`;
- the count of release-candidate clauses inserted (**expect nine**);
- a wrap-tolerant grep showing the nine historical `001-033` sentences are
  **unchanged** and that `Last reconciled` still reads `0.21.3`.

## 10. Review request

Standard eleven-item format; **record your author tier**; send to the architect.
Do not move RFC 034 to `done/` in a separate commit from the rest — one
finalization commit, as the previous three releases did.
