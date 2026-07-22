# RFC 021 - Conditional Release Finalization Handoff

**RFC.** [`021-conditional-release-finalization.md`](../../accepted/021-conditional-release-finalization.md)
**Handoff state.** Corrected S1 accepted by architecture review 031 at
`77484c51...`; B11/B12 are closed. The project owner authorized bounded S2
release-local preparation on 2026-07-22. S2 review is pending; Q2 and later
stages remain unauthorized.
**Target.** Owner-selected corrective version `0.20.2`; local tag `0.20.1`
remains immutable, unpublished, and unusable as an actual release.

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

The current owner authorization is narrower than the complete plan above. S1
is accepted. Bounded S2 may prepare workspace/lock version `0.20.2`, the
matching changelog candidate, draft/not-current apex metadata, current
roadmaps, and RFC 019/020/021 handoffs for review. It must keep active
conditional metadata absent and RFCs 019/020/021 in `accepted/`. Q2 atomic
finalization, intended-tag evidence, tags, distribution, publication,
certification, and release remain unauthorized.

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
- `rfcs/accepted/019-release-integrity-and-msrv-recovery.md` incorporates
  intended-tag and partial-distribution semantics;
- `rfcs/accepted/020-normative-documentation-authority-and-currency.md`
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

## 6. Generated artifacts

No release archive, evidence directory, metadata instance, tag, or generated
documentation is owned by S1 or S2. Test fixture directories are gate-local
temporary state and are removed by their test guards. The generated mdBook
output was removed after validation.

## 7. Known limitations

- S2 documentation/version preparation requires review before Q2 can be
  considered.
- A conditionally staged `done/` state is intentionally narrow and must not
  become a general substitute for shipped lifecycle state.
- Tag-bound evidence still occurs after tag creation; the protocol reduces
  risk through exact local evidence and architecture review but cannot make tag
  creation reversible.
- Remote tag acceptance followed by workflow no-start, cancellation, timeout,
  gate failure, or artifact-upload failure is a partial-distribution incident;
  `P` remains false and the remote tag is not rewritten.
- No actual release is authorized.

## 8. Recommended next step

Make the bounded S2 preparation owner-durable, then submit the exact revision
for focused documentation/architecture review against RFC 021, review 031, the
amended RFC 000/019/020 protocol, and this handoff. Do not add active
conditional metadata or begin Q2 lifecycle/apex finalization, intended-tag
evidence, tag creation, push, publication, certification, or release action
until separately reviewed and authorized.
