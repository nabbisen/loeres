# RFC 036 correction C8 — the packaged-source check is not clean-extraction-safe

**To:** implementer (mid-capability model)
**From:** architect
**Authorized by.** Architect review 083, which retracts review 082's GO for
`0.22.1`. This is the last blocker; nothing else in RFC 036 is outstanding.
**Base revision:** `220746f` on `main`.

## 1. What happened

`cargo xtask check` passes with nineteen gates. `cargo xtask release-gate
--intended-tag 0.22.1` **fails**, on `220746f`, in the clean-extraction suite:

```text
[release-gate:clean-extraction] …
  PACKAGED SOURCE: `loeres` packages `src/access.rs`, which git does not track under src/
  … 74 findings, all PACKAGED SOURCE …
  [architecture] command exited with exit status: 1
[release-gate] FAIL
```

All 74 findings are the same assertion — the one the architect specified in C1.1.
Every other gate passes in the extraction, so the defect is isolated.

**This is the architect's error twice over.** The assertion compares
`cargo package --list` against `git ls-files`, and the architect never considered
that `release-gate` runs `cargo xtask check` a second time inside an extracted
archive. Worse, the RFC 036 handoffs required `cargo xtask check` but **not**
`cargo xtask release-gate` — unlike the earlier C1 handoff of this cycle, which did
require it. So the evidence the architect asked for could not have caught this. You
did nothing wrong, and no value you reported was false.

## 2. The cause, measured — and why the obvious guard is wrong

`release-gate` extracts the archive to
`.git-exclude/tmp/release-gate/extracted/`, which is **inside this repository**.
So git commands run there do not fail; they resolve against the *outer* repository,
where `crates/loeres/src` relative to that working directory matches nothing
tracked, because `.git-exclude/` is ignored.

Reproduced by extracting `HEAD` into a directory inside the repo and one outside:

| Location | `git rev-parse --is-inside-work-tree` | `--show-toplevel` | `git ls-files crates/loeres/src` |
|---|---|---|---|
| repository root | `true` | the repository root | 18 files |
| copy **inside** `.git-exclude/tmp/` | **`true`** | the **outer** repository root | **0 files**, exit 0 |
| copy outside any repository | fails | `fatal: not a git repository` | exit 128 |

**So `--is-inside-work-tree` is not a usable guard**: it answers `true` in the
extraction and the listing comes back empty and plausible. Do not use it. The
failure mode here is a command that succeeds with a silently wrong answer, which is
exactly the shape of defect RFC 036 exists to prevent.

## 3. The change

In `xtask/src/checks/published_metadata.rs`, guard the packaged-source assertion on
the git working tree being **the tree under test**:

- Run `git rev-parse --show-toplevel`.
- If it fails, or its canonicalized value is not the current working directory,
  the assertion is **not applicable**. Report that explicitly — one line, naming
  the reason — and do not emit a finding.
- Otherwise run the assertion exactly as it does today.
- If `git rev-parse` succeeds and the toplevel matches but `git ls-files` then
  fails, that is still a **failure**, not a skip. An empty tracked set for a crate
  that has sources is also a failure. Keep those fail-closed.

Use the existing `super::util::command_stdout` helper, as the current code does.

### Why skipping loses nothing

The assertion is a statement about repository-versus-package consistency, and it is
checked in the **source-tree** suite of the same `release-gate` run, at the
repository root, where it applies. The clean-extraction suite exists to prove the
archive builds and passes from nothing; it has no git history to be consistent
with. Reporting not-applicable there is correct, not a loophole.

The project already has this idiom: `review-evidence` reports the review corpus as
`unavailable` rather than failing when it is absent, and `target-profiles` reports
`advisory-unavailable`. Follow that wording style so the output reads consistently.

### The module doc

The packaged-source bullet must say that the assertion requires the git working
tree to be the tree under test, that it is skipped as not-applicable otherwise,
and **why `--is-inside-work-tree` is not the guard** — with the in-repo extraction
named as the concrete case. The next person to touch this will otherwise reach for
exactly that function.

## 4. Non-scope

- No change to any other assertion; all of them pass in the extraction.
- No change to `--allow-dirty` or `--offline`, both of which stay (C7.1).
- No change to the tracked-versus-packaged comparison itself, which is correct.
- No API, feature, README or specification change. `check-public-api` stays green.
- No `#[allow(…)]`. Do not tag; do not publish.

## 5. Required evidence

**`cargo xtask release-gate --intended-tag 0.22.1` is required this time**, and is
the point of the slice:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-features --all-targets -- -D warnings
cargo test --workspace --all-features
cargo +1.85 check --workspace --all-features
cargo xtask check                                  # 19 gates, assertion ACTIVE
cargo xtask release-gate --intended-tag 0.22.1     # must PASS
```

Redirect the gate's whole stream to a file; never pipe it through `head` or `tail`.
If you must repeat a run on an unchanged revision, first remove only
`.git-exclude/release-evidence/<revision>-v0.22.1/`.

State in the request:

1. **Examples affected** — expected `none`.
2. That the assertion is **active** in the source-tree suite and reported
   **not-applicable** in the clean-extraction suite, quoting both lines from
   `release-gate`'s own output. This is the whole proof; do not substitute
   `cargo xtask check` for it.
3. A bite record showing the assertion still bites at the repository root after the
   guard is added — plant an untracked file under some `crates/<name>/src/`, record
   the failure, remove it. A guard that accidentally skips everywhere would
   otherwise look like success.
4. That `git ls-files` failing at a matching toplevel is still a failure, with a
   bite for it if you can construct one; if you cannot, say so rather than claim it.
5. Anything in this handoff that does not match the tree.
