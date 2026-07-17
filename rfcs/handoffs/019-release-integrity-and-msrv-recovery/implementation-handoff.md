# RFC 019 - Implementation Handoff

**RFC.** [`019-release-integrity-and-msrv-recovery.md`](../../accepted/019-release-integrity-and-msrv-recovery.md)
**Handoff state.** Accepted; S1-S5 and the integrated RFC 020 baseline are
owner-durable. Architecture review 021 accepted revision `525b5fd` and its
retained local evidence as the pre-tag baseline and authorized bounded
`0.20.1` candidate preparation. Tagged evidence and joint S6 closeout remain
pending.
**Target.** Corrective candidate v0.20.1. No commit, tag, push, publication, or
release is authorized by this handoff.

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

The v0.20.1 candidate-preparation step is limited to workspace/lockfile version
metadata, the matching changelog candidate entry, shared draft currency
metadata, current recovery roadmaps, and the two recovery handoffs. RFC files
remain in `accepted/`, and runtime crate production files remain untouched.

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

RFC 019 S3 aligns the workflows without enabling publication or intermediate
artifact claims:

- ordinary CI uses full-SHA action pins, all-target/all-feature clippy, and
  all-feature workspace tests;
- the MSRV workflow uses the exact Rust 1.85 all-feature workspace check;
- the release workflow selects the unprefixed SemVer glob, retains a manual
  non-publishing dispatch, installs stable plus Rust 1.85.0, pins mdBook 0.5.4,
  and invokes the canonical candidate gate once;
- release-workflow actions are pinned to checkout v4.3.1 and the resolved
  stable/1.85.0 rust-toolchain commits; the SHAs were observed directly with
  `git ls-remote` on 2026-07-15;
- no upload or publication step exists while the gate is intentionally
  fail-closed.

RFC 019 S4/S5 completes the non-publishing candidate gate after RFC 020 S5
durability. A clean candidate runs RFC 019 §11.3's source suite, is archived
from the commit-derived tracked manifest at archive root, and has its
path/type/file-set, SHA-256, and normalized manifest validated before
extraction. The gate then verifies extracted Git-object content identity and
executable modes before repeating the applicable suite without recursive
packaging in an owned clean-extraction directory. Successful ignored evidence
records revision, version, local/tag identity, tools, archive digest, manifest,
and evidence-class limitations. Tagged CI retains that directory only after
the gate succeeds; it still performs no publication or GitHub release creation.
The `actions/upload-artifact` v4.6.2 tag resolved directly to the pinned full
commit `ea165f8d65b6e75b540449e92b4886f43607fa02` on 2026-07-16.

Focused S4/S5 evidence observed before a clean candidate commit:

- `cargo test -p xtask checks::release_gate`: passed; 10 tests;
- `cargo +1.85.0 check -p xtask`: passed;
- dirty-tree `cargo xtask release-gate`: failed at tracked-cleanliness
  preflight before the source, package, or extraction phase.

The complete source/package/clean-extraction command cannot be claimed passed
until this candidate is accepted, committed by the owner, and rerun from that
clean revision.

The first S4/S5 tooling review found two evidence-retention blockers. The
workflow now explicitly includes hidden files for its narrowly scoped
gate-owned ignored evidence upload. Retained evidence now advances through
separate source-suite, archive listing/type/file-set/digest, extracted
content/mode, and clean-suite states; no incomplete state promotes a later
validation to pass, and the heading is neutral for local or tagged identity.
Focused tests cover the upload input, pre-extraction wording, post-content
wording, final state, and both candidate identities. Packaging-tool versions
and a second pre-archive tracked-cleanliness check were also added as
defense-in-depth.

The focused workflow-policy test verifies the canonical selector, exact mdBook
install, and 40-hex action references. It passes on stable and Rust 1.85.

After owner durability at `525b5fd`, the complete local gate passed for
manifest version `0.20.0` with local-dry-run identity. It observed the ordered
source suite, 275 workspace unit tests plus doc-test targets, exact Rust 1.85
checking, the architecture aggregate, mdBook, root-layout package validation,
Git-object content/mode identity, and the complete applicable clean-extraction
suite. The retained manifest contained 179 tracked regular files. Review 021
independently matched every archive payload and mode to the commit, confirmed
the archive digest and exclusions, and accepted this evidence for pre-tag use.
It did not accept it as tagged evidence because tag `0.20.0` identifies an
older revision.

The accepted classifications remain explicit: soft-float and RISC-V profiles
were advisory-unavailable; WASM and Linux AArch64 were documented-only; the
size budget was advisory. Candidate v0.20.1 requires new evidence bound to its
exact committed revision, followed by separately authorized tag evidence.

For the dirty v0.20.1 preparation tree, the following were freshly observed:

- `cargo fmt --all -- --check`: passed;
- `cargo clippy --workspace --all-features --all-targets -- -D warnings`:
  passed;
- `cargo test --workspace --all-features`: passed with 275 unit tests and all
  doc-test targets, using a workspace-local temporary directory for rustdoc;
- `cargo +1.85.0 check --workspace --all-features`: passed;
- `cargo xtask check`: passed with mandatory profiles 2/2 and conformance
  12/12, preserving the accepted advisory classes;
- `cargo xtask doc-currency`, `cargo xtask check-rfcs`, and
  `cargo xtask link-audit`: passed;
- `mdbook build docs`: passed, and generated `docs/book/` was removed; and
- `cargo xtask release-gate`: failed at the expected tracked-cleanliness
  precondition before packaging, so no v0.20.1 candidate evidence is claimed.

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

The accepted `525b5fd` local run retained `EVIDENCE.md`, a normalized tracked
content manifest, and `loeres-v0.20.0.tar.gz` in its gate-owned ignored evidence
location. These are historical pre-tag evidence only; they are not v0.20.1
candidate or publication artifacts.

The preparation checks generated only transient build and mdBook output. The
generated book was removed; no v0.20.1 archive or evidence directory exists
before owner durability.

## 7. Known limitations

- RFC 019 does not add vulnerability/license scanning.
- Artifact size remains advisory.
- Advisory/documented-only targets remain accurately labeled and may be
  unavailable.
- A local selector test cannot prove GitHub service behavior by itself; pair it
  with a non-publishing workflow dry run or equivalent review evidence.
- Completion does not authorize publication.
- Candidate v0.20.1 has no clean evidence-bound revision or canonical tag yet.

## 8. Recommended next step

After the owner makes this bounded v0.20.1 preparation durable, run the complete
local gate on that exact clean revision and submit its retained evidence for
review. Only after acceptance may the owner separately authorize canonical tag
creation. Tagged workflow/gate evidence proving tag-to-HEAD identity is still
required before joint S6 closeout. Do not activate apex/lifecycle state,
publish, push, or release while collecting candidate evidence.
