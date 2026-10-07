# RFC 036 corrections C7

**To:** implementer (mid-capability model)
**From:** architect
**Authorized by.** Architect review 081, which accepted request C and raised these
three corrections. C7.1 blocks the `0.22.1` cut; C7.2 and C7.3 ride with it.
**Base revision:** `72bd5a1` (C1-C6).

Request C was the strongest of the three. Both findings in its §2 were correct,
and §2.2 found a real defect in the architect's C1.1 specification. This handoff
is small: one flag, one new assertion that needs no content changes, and one
sentence.

## C7.1 — pass `--allow-dirty` after all (blocking)

**You were right to raise it, and the handoff was wrong.** Review 080's C1.1 said
`cargo package --list` "works on a dirty tree without `--allow-dirty`". The
architect verified that on a modified **root** `README.md` — outside every
published crate — and generalised from a case that does not generalise. Your
`bite/G-dirty-crate-refused.log` is correct: a modified tracked file *inside* a
crate makes cargo refuse.

The consequence is worse than inconvenient. The architect reproduced both symmetry
bites in the main tree and each run emitted the real finding **plus** a spurious
`PACKAGED SOURCE: … refused: 1 files in the working directory contain changes`.
A gate that fails for an unrelated reason during ordinary editing is a gate people
learn to ignore, which costs more than the rule buys.

### The change

In `xtask/src/checks/published_metadata.rs`, the invocation becomes:

```rust
.args(["package", "-p", name, "--list", "--offline", "--allow-dirty"])
```

### Why this makes the gate stronger, not weaker — verified

The architect tested both halves:

- With the flag, a modified tracked file inside a crate no longer fails the gate.
  `published-metadata` returns to PASS.
- With the flag, an **untracked** file inside a crate *is* listed by cargo, and
  your existing symmetric difference catches it:

  ```text
  PACKAGED SOURCE: `loeres` packages `src/scratch_probe.rs`, which git does not track under src/
  ```

  Without the flag, cargo refuses before listing, so that branch of
  `packaged_source_check` is **unreachable** — which is exactly why request C §3
  reports "an untracked-but-packaged file is not reachable through `git ls-files`,
  and I did not test it". The flag makes it reachable, and it guards a genuine
  publication hazard: shipping a scratch file to crates.io, permanently.

So `--allow-dirty` trades a refusal we do not want for an assertion we do.

### What else must change with it

- **The module doc.** Lines 28-35 and the comment at line 361 currently *justify*
  not passing the flag ("`--allow-dirty` is deliberately not passed: the proof is
  about …"). That reasoning is now wrong and must be replaced: say that the flag is
  passed so the listing is available regardless of working-tree state, that the
  proof rests on comparing the listing against `git ls-files` rather than on
  cargo's cleanliness check, and that the flag is what makes the
  packaged-but-untracked direction reachable.
- **A bite record for that direction**, which you correctly could not produce
  before. Plant an untracked file under some `crates/<name>/src/`, record the exact
  failure line, remove it, and confirm the gate passes. The architect used
  `crates/loeres/src/scratch_probe.rs`.
- **Keep `--offline`.** The gate stays hermetic; only the dirty-state refusal goes.

## C7.2 — gate the README feature tables

Request C §6.2 raises this and the answer is yes. The `## Features` sections are
the only feature documentation a crates.io or docs.rs user receives, so a table
that drifts from `[features]` reintroduces precisely the misleading state RFC 036
exists to remove.

Add to `published-metadata`, per publishable crate:

1. `crates/<crate>/README.md` contains a `## Features` section.
2. Every feature declared in that crate's `[features]`, other than `default`, is
   mentioned in that section as inline code — `` `name` ``.
3. For every `(crate, feature)` pair in `RESERVED_INERT_FEATURES`, the line that
   mentions that feature also contains the exact phrase
   **`reserved; no effect yet`**.

**Deliberately no more than that.** Do not parse the table structure, and do not
check the wording for live features — that would be brittle and would fail on
ordinary editing. Rules 2 and 3 are the ones that catch drift: a new feature with
no entry, and a reserved feature whose "no effect" marker is dropped when someone
assumes it became live.

**This needs no content changes.** The architect verified it holds today: all five
crates have the section, all 22 declared features are mentioned, and all 15
registered-inert features carry the exact phrase on their line. If your
implementation reports a finding on the current tree, the implementation is wrong,
not the READMEs — say so in the request rather than editing a README to fit.

## C7.3 — one sentence in the procedure claims too much

`docs/src/development.md`, under "Publication is a separate, human step", the
bullet beginning "**What establishes the packaged set compiles, before
publication.**" says the listing "shows … that no crate has a `build.rs`, and that
no source uses `include_str!` or `include_bytes!`".

The listing does not show the second one. A `build.rs` would appear in the listing,
so its absence is visible there; the `include_str!`/`include_bytes!` fact comes
from reading the sources, which is a separate check. Split the claim so each fact
sits with the thing that establishes it. The rest of that section is accurate and
should not change.

While you are in it, add one clause noting that the gate also fails if the package
lists a file git does not track, since C7.1 makes that reachable.

## Confirmations, so they are not re-raised

- **Request C §2.1, the `0.20.2` dates.** Your reading is right. The record is
  `tagged 2026-07-22, distributed 2026-07-30`; the architect's example line in the
  C5 text reused the tag date for both, which was an error in the handoff, not in
  the tree. Keeping `2026-07-30` was correct. The eight-day gap is consistent with
  RFC 021's conditional finalization and is not an anomaly.
- **Request C §3, the disclosed mis-runs.** They leave no doubt. Your final method
  — fresh worktree, mutant committed there, gate run, worktree discarded — is
  sound, `git worktree list` shows nothing left behind, and the architect
  independently reproduced both registry directions in the main tree with messages
  matching yours exactly. Keeping the superseded `bite/` runs was the right call.
- **The `## Features` sections themselves** are accepted as written. The
  `loeres-cluster` table, including the `serde` row the old list omitted and the
  retained sentence about the unconditional batch path, is exactly the standard.

## Non-scope

- No public API change; `check-public-api` stays green.
- No feature declaration removed, no apex specification table edited.
- No network in any gate.
- No README content changes (C7.2).
- No `#[allow(…)]`. Do not tag; do not run `cargo publish` at all.

## Required evidence

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-features --all-targets -- -D warnings
cargo test --workspace --all-features
cargo +1.85 check --workspace --all-features
cargo xtask check                      # 19 gates
cargo xtask published-metadata         # standalone
```

State in the request:

1. **Examples affected** — expected `none`.
2. The bite record for **packaged-but-untracked** (new, C7.1), and for each of
   C7.2's three rules. Rule 1 and rule 2 can be broken in a README; rule 3 by
   deleting the phrase from one inert feature's line.
3. That `published-metadata` passes on the unmodified tree with C7.2 added, and
   that no README content was changed.
4. Confirmation that a dirty tracked file inside a crate no longer produces a
   `PACKAGED SOURCE … refused` finding.
5. Anything in this handoff that does not match the tree. Request C found two such
   items; that is the standard.

After this slice the architect drops tag `0.22.1`, re-cuts, and publishes with the
owner. Nothing further is expected of the dev team for this release.
