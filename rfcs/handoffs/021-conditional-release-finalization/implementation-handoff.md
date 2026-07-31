# RFC 021 - Conditional Release Finalization Handoff

**RFC.** [`021-conditional-release-finalization.md`](../../done/021-conditional-release-finalization.md)
**Handoff state.** Implemented (conditional finalization for 0.20.2).
**Target.** Owner-selected corrective version `0.20.2`; local tag `0.20.1`
remains immutable, unpublished, and unusable as an actual release.

Repository release `0.20.2` is released and carries RFCs 001-021; this tree
is `0.20.3` in development and is not itself a release.

## 1. Summary

Implement a fail-closed conditional-finalization sequence so the exact eventual
tagged tree contains release-consistent apex, lifecycle, changelog, roadmap,
index, and handoff state without making an unconditional pre-release claim.

The key invariant is that no tracked activation edit occurs after the
canonical release tag. The same finalization bytes are conditional before the
authorized distribution event and current when that event makes the complete
activation predicate true.

Review 030 accepted the S1 scope, schema, preflight, RFC amendments, and gate
evidence but rejected two fail-closed details. The corrected validators now
require one canonical conditional apex block with no legacy draft block or
extra statement, and exactly one canonical Status field in each conditionally
staged RFC.

## 2. Scope followed

Planned implementation is limited to:

1. intended-tag preflight for clean untagged finalization revisions;
2. bounded conditional apex/lifecycle metadata and validation;
3. reviewed amendments to RFC 000, RFC 019, and RFC 020;
4. owner-selected `0.20.2` release-local preparation;
5. one atomic exact finalization revision conditionally staging RFCs
   019/020/021;
6. separate local and tagged evidence reviews;
7. explicit owner release authorization followed by an authorized distribution
   event.

Runtime source, APIs, behavior, features, dependencies, publication automation,
credential handling, and tag mutation are out of scope.

S1 and S2 acceptance and Q2 construction authorization are dated historical
inputs to these bytes. Before `P`, later actions require the exact external
evidence and authorizations defined by RFC 021. After `P`, the same tracked
state is the activated release and requires no documentation transition.

## 3. Files changed

Bounded S1 changes:

- `xtask/src/main.rs` passes `release-gate` arguments to the gate;
- `xtask/src/checks.rs` registers the conditional-finalization module;
- `xtask/src/checks/conditional_finalization.rs` defines and validates the
  exact optional schema, lifecycle state, apex constants, and focused predicate
  fixtures;
- `xtask/src/checks/release_gate.rs` adds host-only
  `--intended-tag 0.20.2` preflight, local/remote collision checks, and intended
  candidate identity;
- `xtask/src/checks/check_rfcs.rs` activates exact conditional lifecycle checks
  only when the tracked metadata file exists;
- `xtask/src/checks/doc_currency.rs` activates exact conditional apex checks
  only when that metadata exists;
- `rfcs/done/000-rfc-lifecycle-policy.md` incorporates the narrow lifecycle
  exception and exact Status qualifier;
- `rfcs/done/019-release-integrity-and-msrv-recovery.md` incorporates
  intended-tag and partial-distribution semantics;
- `rfcs/done/020-normative-documentation-authority-and-currency.md`
  incorporates conditional apex/currency semantics; and
- this handoff records S1 traceability and evidence.

No conditional metadata instance, version change, apex/current-state change,
RFC lifecycle move, workflow change, or release artifact is part of S1.

The review-030 correction changes only the two validator modules above plus
this synchronized handoff. It does not revise the accepted RFC semantics or
expand S1 authority.

Bounded S2 changes only:

- `Cargo.toml` and `Cargo.lock` workspace/member version metadata;
- `CHANGELOG.md` with an explicitly untagged, not-released `0.20.2` candidate
  entry;
- the apex trio's identical RFC 020 draft block and draft Status lines;
- `ROADMAP.md`, `docs/src/recovery-roadmap.md`,
  `docs/src/specifications.md`, and the current portions of the detailed
  roadmap; and
- RFC 019/020/021 handoffs.

S2 does not add `release/conditional-finalization.toml`, the canonical Q2 apex
block, RFC lifecycle moves, evidence, tags, workflow changes, or release
actions.

Atomic Q2 changes:

- adds `release/conditional-finalization.toml` with the exact reviewed schema;
- replaces all three legacy draft apex blocks with the canonical RFC 021
  conditional block;
- moves RFCs 019/020/021 from `accepted/` to `done/`, gives each the one exact
  conditional Status, and synchronizes `rfcs/README.md` plus affected links;
- updates the root README, book introduction, changelog,
  root/detailed/recovery roadmaps, traceability, specifications index, threat
  model, and RFC 019/020/021 handoffs to the same external-activation boundary;
- corrects the conditional allowlist scan to inspect actual top metadata
  Status fields, so RFC 000 may normatively document the exact qualifier
  without being misclassified as an unreviewed conditional RFC. A focused
  regression test retains rejection of a real unreviewed Status use.

Q2 makes no runtime/API, dependency, workflow, credential, tag, distribution,
publication, certification, or actual-release change.

## 4. Design decisions and assumptions

- Distribution-authorization predicate `Q` requires matching conditional
  metadata, accepted local evidence and `A_L`, exact local tag peel, accepted
  tagged evidence and `A_E`, a fresh authoritative-remote collision check, and
  owner authorization for the named bundle.
- Activation predicate `P` additionally requires successful `D_release`
  strictly after `Q`. Under the current workflow, `D_release` inseparably
  includes remote acceptance of tag `0.20.2`, the automatically triggered
  release workflow starting within 30 minutes, its release gate and required
  evidence upload, and successful terminal completion within 120 minutes.
- `0.20.2` is owner-selected; availability must be rechecked before
  finalization and tag creation.
- `0.20.1` stays local, immutable, unpublished, and blocked.
- The final tagged tree must contain conditional release-current state with
  activated scope RFCs 001-021 and conditionally staged RFC 019/020/021
  `done/` paths.
- RFC 000's ordinary folder semantics remain unchanged outside the narrow
  reviewed exception.
- Failure after local tag creation quarantines the tag and requires another
  owner-selected version; tags are never repaired by movement.
- Tag push and its automatic workflow/upload are one bundle. Branch push,
  GitHub release, registry publication, and certification remain separately
  authorized actions.
- A fresh exact direct/peeled ref query against one authoritative release
  remote is mandatory immediately before distribution and fails closed on
  collision, ambiguity, multiple candidate remotes, or network failure.
- Tooling validates conditional structure but must never infer external `P`
  from tracked `done/` paths alone.
- The optional schema path is
  `release/conditional-finalization.toml`. Its reviewed implementation values
  are schema `1`, version/tag `0.20.2`, phase
  `release-finalization-candidate`, remote `origin`, bundle
  `tag-push-release-workflow-v1`, RFC set `[19, 20, 21]`, and 30/120-minute
  workflow boundaries. Unknown fields, including a self-referential revision,
  fail closed.
- The optional file must be tracked in `HEAD` before intended-tag mode can
  pass. This prevents an untracked worktree-only file from influencing release
  evidence.
- Conditional apex parsing uses canonical normalized-block equality, not
  required-substring presence. It rejects the legacy RFC 020 draft marker,
  duplicate markers, extra or unconditional claims, and fields displaced
  outside the bounded block.
- Conditional lifecycle parsing counts every `**Status.**` field and requires
  exactly one field equal to the canonical conditional Status. Duplicate,
  conflicting, missing, and ordinary Implemented Status values fail closed.

## 5. Tests and gates run

Observed for the bounded S1 implementation worktree and retained as historical
tooling evidence:

- `cargo fmt --all -- --check`: passed;
- `cargo test -p xtask`: passed, 59 tests;
- `cargo xtask check-rfcs`: passed;
- `cargo xtask doc-currency`: passed;
- `cargo +stable clippy --workspace --all-features --all-targets -- -D
  warnings`: passed;
- `TMPDIR="$PWD/target/tmp" cargo +stable test --workspace --all-features`:
  passed, including all unit and doc-test targets;
- `cargo +1.85.0 check --workspace --all-features`: passed;
- `cargo xtask check`: passed, including link audit of 56 Markdown files; and
- `mdbook build docs`: passed; generated `docs/book/` was removed.

The 59 focused tests include the B11 retained-legacy-block, identical extra
statement, unconditional release claim, displaced field cases; the B12
conflicting, duplicate, missing, and ordinary Status cases; and the
non-blocking no-finish distribution observation suggested by review 030.

The first S1 workspace-test invocation reached successful unit suites but
rustdoc could not write to the environment's read-only `/tmp`. The same
command was rerun with a workspace-local `TMPDIR` and passed completely.

Observed for the bounded S2 preparation worktree on 2026-07-22:

- `cargo fmt --all -- --check`: passed;
- `cargo test -p xtask`: passed, 59 tests;
- `cargo xtask doc-currency`: passed;
- `cargo xtask check-rfcs`: passed;
- `cargo xtask link-audit`: passed, 56 Markdown files;
- `cargo +stable clippy --workspace --all-features --all-targets -- -D
  warnings`: passed;
- `TMPDIR="$PWD/target/tmp" cargo +stable test --workspace --all-features`:
  passed, including all unit and doc-test targets;
- `cargo +1.85.0 check --workspace --all-features`: passed;
- `cargo xtask check`: passed, including 12/12 conformance fixtures and the
  advisory size baseline;
- `mdbook build docs`: passed; generated `docs/book/` was removed; and
- `git diff --check`: passed after the S2 documentation update.

The S2 checks observed workspace/member version `0.20.2`, the identical legacy
RFC 020 draft apex grammar, all three RFCs still in `accepted/`, and active
conditional metadata absent. `cargo xtask release-gate --intended-tag 0.20.2`
was deliberately not run or claimed: its successful use belongs to clean exact
Q2 finalization evidence after the tracked metadata, canonical conditional
apex block, and atomic lifecycle state exist.

Historical construction evidence observed on 2026-07-22 for the first Q2
revision, before architecture review 033 identified the B13 prose defect:

- `cargo test -p xtask conditional_finalization`: passed, 8 focused tests;
- `cargo fmt --all -- --check`: passed;
- `cargo test -p xtask`: passed, 59 tests;
- `cargo +stable clippy --workspace --all-features --all-targets -- -D
  warnings`: passed;
- `TMPDIR="$PWD/target/tmp" cargo +stable test --workspace --all-features`:
  passed, including all unit and doc-test targets;
- `cargo +1.85.0 check --workspace --all-features`: passed;
- `cargo xtask check-rfcs`: passed and reported conditional structure staged
  without inferring external activation; and
- `cargo xtask doc-currency`: passed with the same non-inference report;
- `cargo xtask check`: passed, including 12/12 conformance fixtures and the
  advisory size baseline;
- `mdbook build docs`: passed; generated `docs/book/` was removed; and
- `git diff --check`: passed.

Those results were worktree construction checks, not exact clean-revision
evidence. The first exact revision later received its own complete evidence;
review 033 rejected its release-local prose and required fresh evidence for
the corrected bytes.

Observed for the review-033 B13 correction worktree on 2026-07-22:

- the bounded conditional-current-prose regression adds representative
  before-`P`/after-`P`, tracked-state non-inference, separately authorized
  optional-action, and stale-unconditional-phrase assertions;
- `cargo fmt --all -- --check`: passed;
- `cargo test -p xtask`: passed, 61 tests;
- `cargo xtask check-rfcs`, `cargo xtask doc-currency`, and
  `cargo xtask link-audit`: passed; the link audit scanned 56 Markdown files;
- `mdbook build docs`: passed and generated `docs/book/` was removed;
- all-target/all-feature workspace Clippy with warnings denied: passed;
- all-feature workspace tests and all doc-test targets: passed, including 71
  core, 22 static-backend, 23 std-backend, 85 cluster, 32 device, and 61 xtask
  unit tests;
- exact Rust 1.85 all-feature workspace checking: passed;
- `cargo xtask check`: passed with mandatory profiles 2/2, conformance 12/12,
  advisory-unavailable soft-float/RISC-V, and documented-only WASM/AArch64;
  and
- `git diff --check`: passed before this evidence note was added.

These are correction-worktree observations. Fresh exact local evidence belongs
to the new clean owner-durable revision and cannot reuse the evidence for the
review-033-rejected revision.

## 6. Generated artifacts

No release archive, evidence directory, metadata instance, tag, or generated
documentation is owned by S1 or S2. Test fixture directories are gate-local
temporary state and are removed by their test guards. The generated mdBook
output was removed after validation.

Q2 adds the tracked conditional metadata instance and the synchronized
release-local finalization state. Release archives and private evidence are
external artifacts generated only for an exact clean revision; tracked bytes
do not reveal whether they exist. Q2 itself creates no tag or distribution
artifact.

## 7. Known limitations

- Tracked state is not Q2, tag-bound, or distribution evidence; those facts are
  established externally against the exact revision or tag.
- A conditionally staged `done/` state is intentionally narrow and must not
  become a general substitute for shipped lifecycle state.
- Tag-bound evidence still occurs after tag creation; the protocol reduces
  risk through exact local evidence and architecture review but cannot make tag
  creation reversible.
- Remote tag acceptance followed by workflow no-start, cancellation, timeout,
  gate failure, or artifact-upload failure is a partial-distribution incident;
  `P` remains false and the remote tag is not rewritten.
- Optional post-activation actions remain separately authorized.

## 8. Conditional operation boundary

Before `P`, follow RFC 021's accepted-evidence and owner-authorization sequence
without moving `0.20.1` or inferring readiness from tracked state. After `P`,
the same bytes require no activation edit. Branch pushes, GitHub release
creation, registry publication, and certification remain separate actions.
