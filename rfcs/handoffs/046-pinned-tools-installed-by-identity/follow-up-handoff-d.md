# RFC 046 follow-up handoff — D: assert the pinned *version*, and correct one stale sentence

**To:** implementer (mid-capability model)
**From:** architect
**Authorized by.** Architect review 110 accepted C and raised F4, which this handoff carries. Read
review 110 §2 and §3.
**Base revision:** `main` at or after review 110's commit.
**Target release:** `0.24.1`. **Nothing here bumps anything.**

**One review request, D.** `cargo xtask release-gate --intended-tag 0.24.1` is required.

**D is the last slice of RFC 046.** After it, each pinned tool has exactly one operative home and
every tracked claim about it is asserted. Three slices have each revealed the next; this one does
not, and if you believe it does, say so in the request rather than opening an E.

## 0. Why there is a D at all

C did exactly what its handoff asked — name the pinned version wherever a human is told to install
the tool — and that was right. But naming a version without asserting it moves the defect instead
of removing it: a vague instruction becomes a **confidently wrong** one at the next version bump,
and a confidently wrong instruction gives the reader no cue to go and check.

This is the same omission my previous handoff made about the hash (F2), in the same place, a
second time. It is not a defect in your work.

## 1. F4 — assert the version, in the test that already holds the hashes

Four live claims about which version is pinned exist outside `action.yml`, and none is checked:

| file | line | the claim |
| --- | ---: | --- |
| `docs/src/development.md` | 206 | "CI installs the pinned `0.20.2` release binary by verified SHA-256" |
| `docs/src/development.md` | 213 | `cargo install cargo-deny --version 0.20.2 --locked` |
| `xtask/src/checks/supply_chain.rs` | 164 | the `tool_gate` failure hint's `--version 0.20.2` |
| `docs/src/recovery-roadmap.md` | 242 | "mdBook is pinned to 0.5.4" |

In the same test you extended in C, add one constant per tool — the `cargo-deny` version and the
`mdbook` version as they appear in `action.yml`'s URLs — and assert that each occurs:

1. in `.github/actions/install-pinned-tool/action.yml` (it is part of the pinned URL already);
2. in each file above that claims it.

Hold the constants in the test, next to the hashes it already holds. The point is the same as F2's:
a legitimate version bump must then touch the assertion, and the assertion names every place that
has to move with it. **Do not** parse the version out of the URL at runtime to avoid writing it
twice — a check that derives its expectation from the thing it is checking asserts nothing.

**Do not add a gate or a new `xtask` check.** Extend the existing test, as in C.

Line numbers above are from revision `2d4fff9`; confirm each against the tree before editing, and
report any that moved.

## 2. F4's second half — one sentence that is now wrong in two ways

`docs/src/recovery-roadmap.md:242` reads:

> The release selector uses the canonical unprefixed tag family, action references are full
> reviewed SHAs, and mdBook is pinned to 0.5.4.

This is present-tense narrative about the current workflow, not history, so it is a live claim —
and after RFC 046 it is wrong twice:

- **"action references are full reviewed SHAs"** is no longer true of `release.yml`, which now
  holds two `./.github/actions/install-pinned-tool` references. A local action has no SHA; it is
  pinned by being part of the gated commit itself — the exemption you added to the SHA-pin loop in
  B. Say that, rather than deleting the sentence: the external references *are* still full
  reviewed SHAs, and that remains worth stating.
- **"mdBook is pinned to 0.5.4"** is true but now understated: it is pinned by verified SHA-256,
  not by `cargo install --version`.

Correct both clauses in place, in that document's own register — it is narrative prose, not a
checklist, so match the surrounding sentences rather than importing `development.md`'s style.

## 3. Nit, since you will be in that test anyway

`release_workflow_selects_canonical_tags_and_pins_actions` now asserts the content of a file used
by **both** workflows, so its name no longer describes it. Worth more than tidiness: if
`release.yml`'s call sites were ever removed, the pin-table assertion would leave with a test that
no longer had a reason to exist. Rename it to cover what it actually checks. You cited the
`zero_multipliers_report_no_evidence_via_the_zero_denominator_guard` precedent yourself in B; this
is the same situation.

## 4. Non-scope

- Changing either pinned version, or the action's logic, or any workflow file.
- A gate that checks a pin against upstream (RFC 046 §4) — still ruled out. §1 compares tracked
  bytes to tracked bytes.
- Asserting the *file path* cited in the `tool_gate` hint's prose. Review 110 §4 answer 2
  considered it and declined: one occurrence, and a check there would be out of proportion.
- **F1** — `find … | head -n1` under `pipefail`. Closed as deliberately alone.
- Creating a tag, for any reason.
- Any library change, any version bump.

## 5. Required evidence

```text
cargo fmt --all -- --check          # run last, immediately before committing
cargo clippy --workspace --all-features --all-targets -- -D warnings
cargo test --workspace --all-features
cargo +1.85 check --workspace --all-features
cargo xtask check                   # 20 gates
cargo xtask release-gate --intended-tag 0.24.1      # must PASS
```

**Check `cargo xtask check`'s own exit status.** Never through a pipe.

State in the request:

1. **Examples affected** — expected `none`.
2. **The `CHANGELOG.md` entry you added**, quoted.
3. **That each new assertion fails when it should.** One scratch edit per tool is enough — change
   the version in one claiming file, show the failure, revert. Same standard as A's mismatch path
   and C's hash count: an assertion nobody has watched fail is an assertion nobody has tested.
4. **The occurrence counts you found** for each version string in each file, independently of the
   test's own assertion.
5. **The corrected `recovery-roadmap.md` sentence**, quoted whole, so it can be read as a reader
   of that document would rather than as a diff.
6. **A `supply-chain` failure on a dependency you did not touch is not yours.** Scoped
   `cargo update -p <crate>`, never a bare one.
7. **Anything in this handoff that does not match the tree**, including the four line numbers in
   §1, which are from `2d4fff9`.

**Push it when the review is accepted** — `docs/src/development.md`'s new "An accepted slice is
pushed; a tag is not the implementer's" convention now says so, so this does not need asking each
time. Tags remain the architect's and the owner's.
