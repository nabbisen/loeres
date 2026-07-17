# RFC 021 - Conditional Release Finalization Handoff

**RFC.** [`021-conditional-release-finalization.md`](../../accepted/021-conditional-release-finalization.md)
**Handoff state.** Accepted; architecture review 027 froze the design and the
project owner authorized Q0.5/S0.5 plus bounded S1 on 2026-07-18. S2 and later
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

The current owner authorization is narrower than the complete plan above. After
architecture accepts the Q0.5/S0.5 lifecycle baseline, bounded S1 may implement
only the host-side intended-tag preflight, conditional metadata/lifecycle
validation, focused tests, and normative RFC 000/019/020 protocol amendments
accepted by review 027. S2 version/apex/changelog/roadmap preparation and every
later finalization, tag, distribution, publication, certification, or release
stage remain unauthorized.

## 3. Files changed

The Q0.5/S0.5 lifecycle transition changes only:

- `rfcs/accepted/021-conditional-release-finalization.md`;
- `rfcs/handoffs/021-conditional-release-finalization/implementation-handoff.md`;
- `rfcs/README.md`.

RFC 021 §4 enumerates the complete eventual implementation surface. Only the
bounded S1 subset stated above is owner-authorized, subject to architecture
acceptance of this lifecycle baseline; all S2 and later areas remain
unauthorized.

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

## 5. Tests and gates run

Observed for the Q0.5/S0.5 lifecycle-transition worktree:

- `cargo fmt --all -- --check`: passed;
- `cargo xtask check-rfcs`: passed;
- `cargo xtask doc-currency`: passed;
- `cargo xtask link-audit`: passed; 56 Markdown files scanned;
- `mdbook build docs`: passed; generated `docs/book/` removed; and
- `git diff --check`: passed.

This lifecycle transition contains no S1 implementation and claims no
implementation-gate or release-gate result.

## 6. Generated artifacts

Tracked lifecycle artifacts:

- accepted RFC 021;
- the synchronized RFC index; and
- this synchronized implementation handoff.

No release archive, evidence directory, tag, or generated documentation is
owned by this lifecycle transition. Its private direct-tree review request is
coordination state, not a tracked lifecycle artifact.

## 7. Known limitations

- The conditional lifecycle exception changes current release semantics and
  requires acceptance of this exact lifecycle baseline before S1 begins.
- The exact conditional metadata schema and CLI spelling remain implementation
  details bounded by RFC 021, but schema/version/tag/RFC allowlist/remote/bundle
  bindings are normative.
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

Make the Q0.5/S0.5 lifecycle transition owner-durable and submit its exact
revision for review. After that transition is accepted, implement only S1's
host-side intended-tag preflight, bounded conditional metadata/lifecycle
validation, focused tests, and RFC 000/019/020 protocol amendments. Do not
begin S2 version/apex/changelog/roadmap preparation or any later lifecycle,
tag, push, publication, or release action.
