# RFC 019 - Implementation Handoff

**RFC.** [`019-release-integrity-and-msrv-recovery.md`](../../accepted/019-release-integrity-and-msrv-recovery.md)
**Handoff state.** Accepted; architecture design freeze, project-owner
approval, and the RFC 000 `accepted/` transition completed on 2026-07-15.
**Target.** Corrective baseline after v0.20.0. No tag, commit, push, or publication is authorized by this handoff.

## 1. Summary

Restore the declared Rust 1.85 compatibility and make one complete,
non-publishing release gate authoritative for both tagged CI and local release
preparation. The gate must validate the source tree, package layout, and a clean
extraction of the same candidate content.

This is operational hardening. Do not change solver/runtime public APIs.

## 2. Scope followed

Implementation is limited to:

1. rewriting the unstable Rust syntax in `target_profiles.rs` without changing
   profile semantics;
2. separating routine `cargo xtask check` from complete
   `cargo xtask release-gate`;
3. adopting unprefixed SemVer Git tags as canonical;
4. composing fmt, all-target/all-feature clippy, all-feature tests, MSRV,
   architecture checks, mdBook, packaging, and clean-extraction validation;
5. aligning workflows and release/contributor documentation;
6. producing non-publishing evidence.

Out of scope: runtime behavior, dependency-boundary changes, crates.io
publication, a real release/tag, new supply-chain tooling, or new size budgets.

## 3. Files changed

Expected implementation files:

- `xtask/src/checks/target_profiles.rs`
- `xtask/src/checks/release_gate.rs` and focused submodules/tests
- `xtask/src/main.rs` if command routing changes
- `.github/workflows/ci.yml`
- `.github/workflows/msrv.yml`
- `.github/workflows/release.yml`
- `docs/src/development.md`
- `CONTRIBUTING.md`
- `CHANGELOG.md` at closeout

Do not edit runtime crate production files. If implementation needs unrelated
files, stop and request scope review.

## 4. Design decisions and assumptions

- Declared MSRV stays Rust 1.85.0.
- Canonical Git tags are unprefixed SemVer (`0.20.1`), preserving repository
  history. Archive names retain `loeres-v<version>.tar.gz`.
- `cargo xtask check` is the developer aggregate.
- `cargo xtask release-gate` is the complete release-candidate aggregate and
  must avoid recursive repackaging in the extracted tree.
- RFC 019 §11.3 is the only normative complete gate list; this handoff does not
  create a second list.
- Package bytes come from the clean candidate commit's tracked-file set and are
  bound to the recorded commit/tag/version/digest/manifest.
- The source archive contains regular files/directories only and excludes
  generated `docs/book/`.
- Temporary extraction state is workspace-local and ignored.
- Release/publish authority remains with the project owner.
- RFC 020 owns normative-specification content; coordinate shared contributor
  and changelog wording rather than duplicating it.

## 5. Tests and gates run

RFC 019 S1 completed on 2026-07-15. The host-only let-chain was replaced with
Rust-1.85-compatible nested conditionals, and a focused unknown-command
regression test was added. Observed evidence:

- `cargo test -p xtask checks::target_profiles`: passed; 5 tests;
- `cargo +1.85.0 check --workspace --all-features`: passed.

This focused evidence closes the known syntax failure but does not satisfy the
later complete release gate or RFC closeout.

RFC 019 S2 then established the fail-closed release-candidate skeleton. It
separates `check` from `release-gate`, parses stable unprefixed SemVer, checks
version/changelog/tag/cleanliness preconditions, and derives a sorted
regular-file manifest from `git ls-tree -rz --full-tree HEAD`. Unsafe,
excluded, duplicate, link, and special-entry candidates are rejected. Package
construction and clean-extraction certification remain deliberately disabled
until RFC 020 integration.

Additional observed S2 evidence:

- `cargo test -p xtask checks::release_gate`: passed; 3 tests;
- `cargo +1.85.0 test -p xtask checks::release_gate`: passed; 3 tests;
- `cargo +1.85.0 check --workspace --all-features`: passed;
- `cargo clippy -p xtask --all-targets -- -D warnings`: passed;
- dirty-tree `cargo xtask release-gate`: failed closed before packaging, as
  required.

Required during implementation: execute the complete gate list and order in
RFC 019 §11.3. The first focused evidence after the syntax repair is the exact
Rust 1.85 all-feature workspace check; it does not replace the later complete
gate. Record source-tree and clean-extraction results separately. Do not
summarize an advisory/unavailable target as passed enforcement.

## 6. Generated artifacts

Expected only at release-candidate verification:

- `loeres-v<version>.tar.gz` with root-level contents;
- ignored temporary extraction/evidence under the gate-owned workspace path;
- a concise durable review package identifying revision, tools, gates, archive,
  and limitations.

Do not commit generated archives or raw logs unless the project owner approves a
specific durable evidence location.

## 7. Known limitations

- RFC 019 does not add vulnerability/license scanning.
- Artifact size remains advisory.
- Advisory/documented-only targets remain accurately labeled and may be
  unavailable.
- A local selector test cannot prove GitHub service behavior by itself; pair it
  with a non-publishing workflow dry run or equivalent review evidence.
- Completion does not authorize publication.

## 8. Recommended next step

Proceed with S3 workflow alignment while preserving the fail-closed package
boundary, then follow the shared integration order in the recovery roadmap.
Submit a review request after S3 and again with S6 complete source/extraction
evidence.
