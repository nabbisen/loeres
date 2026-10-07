# RFC 036 follow-up C9 — an unusable git must fail, not skip

**To:** implementer (mid-capability model)
**From:** architect
**Authorized by.** Architect review 084, which accepted C8 and gave `0.22.1` GO.
**This does not block `0.22.1`.** It is scheduled for the next patch release; see §3
for why it cannot affect a released artifact.
**Base revision:** whatever is current on `main` when this is picked up.

## 1. The hole

C8's guard treats two different situations as one. Answering request C8 §6.2: an
unresolvable toplevel should be a **failure**, not a skip.

`tree_under_test()` returns `Err` — and `run()` prints *not applicable* and passes —
for both of:

1. `git rev-parse --show-toplevel` **fails** (git absent, broken, or the directory
   is not a repository at all);
2. it **succeeds** and names a different repository (the in-repository clean
   extraction, which is what C8 exists for).

Only (2) is a legitimate state. Measured with a `git` stub that exits 1:

```text
  packaged source set: not applicable here: this is not a git working tree
[published-metadata] PASS
```

So a broken or absent git silently removes the assertion and `cargo xtask check`
still passes. Nothing else closes this: `host-check` is only
`cargo check --workspace --all-features`, and no gate verifies that git exists.

There is **no legitimate environment in this project** where case (1) arises.
`xtask` is `publish = false`, so no downstream consumer can run `cargo xtask check`
from unpacked sources, and the clean extraction lands in case (2) — proven by
`release-gate`'s own output, which names the outer repository.

## 2. The change, verified

Split the two. Keep (2) as not-applicable, exactly as it is. Make (1) a finding.

The architect implemented and measured this before writing it. A single named
constant distinguishes the reasons, `run()` routes that one reason to `findings`
instead of the not-applicable line, and the rest is untouched. Three cases:

| Environment | Required | Measured with the change |
|---|---|---|
| repository root, working git | assertion **active** | `… README feature tables, packaged source` · PASS |
| `git` stub exiting 1 | **FAIL** | `PACKAGED SOURCE: git is not usable here, so the tracked set cannot be read; git is required to compare a package with what is tracked` · FAIL |
| in-repository extraction of `HEAD` | still **skip** | `packaged source set: not applicable here: git's repository is \`…/loeres-git\`, not this working tree …` · PASS |

Wording is yours; the behaviour above is the requirement. The module doc must say
that an unusable git is a failure while a different repository is not applicable,
and why the two differ — otherwise the next reader will reasonably collapse them
again, as C8 did.

## 3. Why this does not block `0.22.1`

The hole needs git to be broken. In that state `release-gate` cannot run at all —
it builds the archive with `git archive` and reads `git ls-tree` — so no release can
be cut. And `release-gate`'s source-tree suite runs the assertion at the repository
root with git working, which the `244b729` log confirms.

So the cost is **late detection**: a packaging mismatch could pass `cargo xtask
check` in a git-less CI and then be caught at the candidate gate, before any
publication. It cannot reach a published artifact. That is why this is a follow-up
and not a seventh blocker on a release that is otherwise ready.

## 4. Also in this slice

**An end-to-end bite for the empty-tracked-`src/` case.** Request C8 §3 reports it
could not be constructed and covers it by unit test only. It *is* constructible,
and the architect did so:

```text
git rm -r --cached crates/loeres/src
→ PACKAGED SOURCE: git tracks no `src/` file for `loeres`, so there is nothing to compare the package with
git reset HEAD -- crates/loeres/src      # restores the index; no file is touched
```

The behaviour is already correct — this adds the evidence, not a fix. Record it as
a bite so the branch is known to work outside a unit test.

## 5. Non-scope

- No change to case (2), to `--allow-dirty`, to `--offline`, or to the
  tracked-versus-packaged comparison.
- No change to any other assertion or gate.
- No API, feature, README or specification change.
- No `#[allow(…)]`. Do not tag; do not publish.

## 6. Required evidence

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-features --all-targets -- -D warnings
cargo test --workspace --all-features
cargo +1.85 check --workspace --all-features
cargo xtask check                                  # 19 gates
cargo xtask release-gate --intended-tag <version>  # must PASS
```

`release-gate` is required, and from now on it is required in **every** RFC 036
slice. Review 083 §3 records why: three RFC 036 handoffs omitted it, and that is the
only reason C8's defect reached the cut.

State in the request the three rows of §2 as measured by you, the §4 bite, and the
two `release-gate` lines — the source-tree suite's active assertion and the
clean-extraction not-applicable line — quoted from the gate's own log. If reading
that log is blocked for you, say so plainly as you did in request C8 §2; that
disclosure was the right call and the architect reads it instead.
