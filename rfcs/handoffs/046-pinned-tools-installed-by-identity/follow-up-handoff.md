# RFC 046 follow-up handoff — C: assert the operative pin table, and stop contradicting it

**To:** implementer (mid-capability model)
**From:** architect
**Authorized by.** Architect review 109 accepted A and B; this handoff carries its F2 and RFC 046
Amendment 1's S6. Read `rfcs/accepted/046-pinned-tools-installed-by-identity.md` §0.1 and
§3.2–§3.3, and review 109 §4 and §5.
**Base revision:** `main` at or after `393957a`.
**Target release:** `0.24.1`. **Nothing here bumps anything.**

**One review request, C.** `cargo xtask release-gate --intended-tag 0.24.1` is required.

**Both items in this handoff are consequences of A and B being right, not of anything being
wrong with them.** F2 is what B's `./`-exemption reasoning left unstated; S6 is a gap in the
*previous handoff*, which named neither file it concerns.

## 1. F2 — assert that the operative pin table exists and still holds the reviewed hashes

The two hashes now live in three tracked places — RFC 046 §3.2, the previous handoff's table, and
`.github/actions/install-pinned-tool/action.yml` — and **only the third is operative**.
`release_workflow_selects_canonical_tags_and_pins_actions` asserts `release.yml` references the
action exactly twice, which pins the path string, so a typo fails. Nothing asserts the referenced
file exists, and nothing asserts its table still holds the hashes that were reviewed.

Delete `action.yml`, or change one hex digit of either hash, and every test passes, `cargo xtask
check` passes, and the failure arrives at a cut — the one moment this project cannot retry
cheaply, which is the risk RFC 046 §5's last row exists to avoid.

**In the test you already touched**, add:

1. `.github/actions/install-pinned-tool/action.yml` exists and is readable.
2. Each pinned SHA-256 from RFC 046 §3.2 occurs in it **exactly twice** —
   `9f12ed4c49936e09b48bf862b595cde2fe64fcbd9d74dfacac6131ca824c8d5f` and
   `5222beabd3e37dc5be0d18ff99b79058469354db5c220153a1b92db5ba12be89`. Twice, not at least once:
   each hash appears in the table comment *and* in its `case` arm, so an exact count also catches
   the two copies inside that one file drifting apart.

**This is not the gate RFC 046 §4 rules out.** That one re-checks a pin against *upstream* and
needs network access from a gate. This compares tracked bytes to tracked bytes. The cost — a
legitimate version bump must update the assertion too — **is the feature**: forcing a human to
touch it is the review §2.3 exists to require.

**Do not add a new gate or a new xtask check.** Extend the existing test.

## 2. S6 — the documented human procedure must stop contradicting CI

Two files, both user-facing in the sense that matters: a contributor reads one and the other reads
back to them when the gate fails.

- **`docs/src/development.md:209`** instructs `cargo install cargo-deny --version 0.20.2 --locked`.
- **`xtask/src/checks/supply_chain.rs:164`**'s `tool_gate` failure hint says
  `cargo install cargo-deny --locked`, **naming no version at all** — looser than the
  documentation it should agree with.

After RFC 046, CI obtains `cargo-deny` by a committed hash while both of these tell a human to
obtain it by a version number, which is precisely what §2.3 argues is not an identity. The hint is
worse than untidy: a developer who follows it installs whatever resolves today and can reach a
**different verdict on the same tree** than CI reaches, because `cargo deny`'s advisory and
license behaviour moves between versions.

**The resolution is not to make a human run a curl-and-hash incantation.** That trades one
confusion for a worse one, and RFC 046 §4 keeps `cargo install` as a legitimate fallback. Instead:

1. `development.md` keeps the `cargo install` form as the local convenience, **says that CI
   installs by pinned hash**, and names `.github/actions/install-pinned-tool`'s table as the
   authority for which version that is. Match the house style of the section it sits in.
2. `supply_chain.rs`'s hint **names the pinned version**, so the two agree and so the developer
   who reads only the error message still installs the right tool.

**Keep the pinned version `0.20.2` everywhere.** If you find yourself wanting to change which
version is pinned, stop and return to the architect — that is a different decision.

## 3. Non-scope

- Changing either pinned version, or the action's logic.
- A gate that checks a pin against upstream (RFC 046 §4) — ruled out, and §1 above is deliberately
  not that.
- Touching `release.yml`, `ci.yml`, `no-std.yml` or `msrv.yml`. C is a test and two strings.
- Creating a tag, for any reason.
- Any library change, any version bump.
- **F1 from review 109 §3** — `find … | head -n1` under `pipefail`. Fails closed, harmless for
  these two archives, and left alone deliberately. If you touch it anyway, `find … -print -quit`
  is the form, and say in the request that you chose to.

## 4. Required evidence

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
3. **That the new assertion fails when it should.** Change one hex digit of a hash in a scratch
   edit, show the test failing, and revert it. The same standard A met for the mismatch path: an
   assertion nobody has watched fail is an assertion nobody has tested.
4. **The exact count you found for each hash in `action.yml`.** If it is not two, say so and stop
   rather than adjusting the expectation to fit.
5. **Both corrected texts**, quoted, so the architect can read them as a contributor would rather
   than as a diff.
6. **A `supply-chain` failure on a dependency you did not touch is not yours.** Report it and fix
   it with a scoped `cargo update -p <crate>`, never a bare one.
7. **Anything in this handoff that does not match the tree.** The previous handoff's gap was found
   by a reviewer and not by its author; the same standard applies to this one.
