# Developer Handoff — RFC 026 supply-chain gate

**Governing RFC.** `rfcs/done/026-supply-chain-gate.md`. **Assigned to.** Implementer tier. **Release.** `0.21.0`. **Ordering.** Lands before RFC 027 implementation begins.

## Change scope
1. **`deny.toml`** at repo root: `[advisories]` deny vulnerability + unmaintained + unsound, yanked warn→deny; `[licenses]` allow exactly `Apache-2.0`, `MIT`, `BSD-2-Clause`, `BSD-3-Clause`, `ISC`, `Unicode-3.0`, `Zlib` — anything else denied, no `unlicensed` allowed; `[bans]` `multiple-versions = "deny"` with an explicit `skip` list, each entry commented with the reason, `wildcards = "deny"`; `[sources]` crates.io only, `unknown-registry = "deny"`, `unknown-git = "deny"`.
2. **`cargo xtask supply-chain`**: runs `cargo deny --all-features check`; **enforced** in `check` and in `release-gate`'s source-tree and clean-extraction suites. If `cargo-deny` is not on PATH: report `unavailable` **and fail** (RFC 026 §11 — a missing tool is an environment defect). Additionally run `cargo deny check bans` for each edge crate with `--no-default-features` and assert zero external dependencies.
3. **CI**: install a pinned `cargo-deny` version in `ci.yml` and the release workflow; record the version in the evidence bundle's tool list.
4. **Docs**: remove every "cargo audit/deny: N/A — not configured" phrase (grep the tree); document the gate in `docs/src/development.md`; extend `doc-currency`'s stale ledger with that phrase.

## Non-change scope
No workspace dependency added — `cargo-deny` is a tool. No `crates/` change. No policy relaxation to make the current tree pass: if the tree fails `deny`, **that is a finding to report**, not a skip to add.

## Required tests
Planted advisory fixture fails; disallowed license in a fixture manifest fails; missing tool fails; current tree passes; edge no-default-features has zero external deps.

## Evidence
Full suite plus the gate's own output on the real tree and on each fixture. Standard review request; record your tier.
