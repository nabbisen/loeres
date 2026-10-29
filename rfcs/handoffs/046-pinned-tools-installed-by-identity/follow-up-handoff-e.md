# RFC 046 follow-up handoff — E: assert the claim, not the version string

**To:** implementer (mid-capability model)
**From:** architect
**Authorized by.** Architect review 111 accepted D's documentation and rename, and found F5: two
of D's four new assertions pass while the claims they name are wrong. Read review 111 §1 in full,
including §1.1, which demonstrates it.
**Base revision:** `main` at or after review 111's commit.
**Target release:** `0.24.1`. **Nothing here bumps anything.**

**One review request, E.**  `cargo xtask release-gate --intended-tag 0.24.1` is required.

## 0. What went wrong, and whose it is

D did exactly what D's handoff said: *"assert that each occurs … in each file above that claims
it."* The handoff specified the **mechanism** and never specified the **predicate**, so
`contains("0.20.2")` over a whole file satisfies it literally — and in two of those files an
unrelated `0.20.2` keeps the assertion green no matter what the real claim says.

**That is the third handoff in a row in which I named a mechanism and not a predicate** (review
111 §2). It is an architect-side defect, not an implementation one, and this handoff therefore
gives you exact strings instead of a method. Your §2 analysis found the collision and was right to
reject an exact count; the remaining option it chose is the one the collision also defeats.

**Also read review 111 §1.2.** Your demonstration ran a whole-file `sed`, which removed the
collision along with the claim, so the failure you watched came from the last occurrence
disappearing rather than from the claim changing. The rule it points at is worth carrying past
this slice: **exercise an assertion by breaking the thing it names, not the file it reads.**

## 1. Replace the four version assertions with claim assertions

In `release_workflow_and_its_pinned_tool_table_match_what_was_reviewed`, drop the bare
`contains(cargo_deny_version)` / `contains(mdbook_version)` checks and assert these strings
instead. Each is a literal to be matched exactly as written here:

| file | must contain |
| --- | --- |
| `.github/actions/install-pinned-tool/action.yml` | `https://github.com/EmbarkStudios/cargo-deny/releases/download/0.20.2/cargo-deny-0.20.2-x86_64-unknown-linux-musl.tar.gz` |
| `.github/actions/install-pinned-tool/action.yml` | `https://github.com/rust-lang/mdBook/releases/download/v0.5.4/mdbook-v0.5.4-x86_64-unknown-linux-musl.tar.gz` |
| `docs/src/development.md` | `cargo install cargo-deny --version 0.20.2 --locked` |
| `docs/src/development.md` | ``pinned `0.20.2` release binary`` |
| `xtask/src/checks/supply_chain.rs` | `cargo install cargo-deny --version 0.20.2 --locked` |
| `docs/src/recovery-roadmap.md` | `mdBook is pinned to 0.5.4` |

Two points about the first two rows. The whole **URL**, not the bare version: today
`action_source.contains("0.20.2")` is satisfied by the comment table alone, so editing the `case`
arm's URL while leaving the comment intact would pass. And the URL is the operative string — the
one thing `curl` is actually given.

`recovery-roadmap.md`'s assertion is sound today only because that file holds one occurrence. State
it the same way as the others, so it is sound by construction rather than by luck.

**Keep C's hash assertions exactly as they are** — exact count of two, per hash. A hash cannot
collide with unrelated prose, so the count is right there and the reason is different from this
slice's.

**Do not** derive any expected string from the file being checked. **Do not** add a gate or a new
`xtask` check; extend the same test. **Do not** reword `development.md`'s `cargo publish
--dry-run` failure example to remove the collision — review 111 §1.3 declined that explicitly: it
documents a real error message, and editing real output so a weak assertion can pass is backwards.

## 2. Non-scope

- Changing either pinned version, the action's logic, any workflow file, or C's hash assertions.
- A gate that checks a pin against upstream (RFC 046 §4) — still ruled out.
- Rewording the publish-failure example, or any other prose that merely happens to contain a
  version string.
- **F1** — `find … | head -n1` under `pipefail`. Closed as deliberately alone.
- Creating a tag, for any reason.
- Any library change, any version bump.

## 3. Required evidence

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
3. **The targeted exercise, per assertion, done the way §0 says.** For each of the six rows:
   change **only the claim named in that row**, leave every other occurrence of that version in
   that file alone, show the assertion failing by its own message, revert, and confirm the test
   passes again. For `development.md` and `supply_chain.rs` this means the unrelated `0.20.2`
   occurrences must still be present and intact while the test fails. That contrast is the whole
   evidence; a whole-file `sed` does not produce it.
4. **A count of the unrelated occurrences you deliberately left in place** in each of those two
   files, so it is on the record that the assertion failed *with the collision still there*.
5. **A `supply-chain` failure on a dependency you did not touch is not yours.** Scoped
   `cargo update -p <crate>`, never a bare one.
6. **Anything in this handoff that does not match the tree**, including every literal in §1's
   table — check each against the file before writing the assertion, and report any that differs
   rather than adjusting the literal to fit what you find. If a string in that table is wrong,
   that is my error again and I would rather hear it than have it quietly corrected.

**Push it when the review is accepted**, per `docs/src/development.md`'s "An accepted slice is
pushed; a tag is not the implementer's". Tags remain the architect's and the owner's.
