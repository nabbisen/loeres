# RFC 021 - Conditional Release Finalization Handoff

**RFC.** [`021-conditional-release-finalization.md`](../../proposed/021-conditional-release-finalization.md)
**Handoff state.** Design proposed; implementation forbidden until independent
architecture acceptance and project-owner lifecycle authorization.
**Target.** Owner-selected corrective version `0.20.2`; local tag `0.20.1`
remains immutable, unpublished, and unusable as an actual release.

## 1. Summary

Implement a fail-closed conditional-finalization sequence so the exact eventual
tagged tree contains release-consistent apex, lifecycle, changelog, roadmap,
index, and handoff state without making an unconditional pre-release claim.

The key invariant is that no tracked activation edit occurs after the
canonical release tag. The same finalization bytes are conditional before the
owner release decision and current after the complete predicate becomes true.

## 2. Scope followed

Planned implementation is limited to:

1. intended-tag preflight for clean untagged finalization revisions;
2. bounded conditional apex/lifecycle metadata and validation;
3. reviewed amendments to RFC 000, RFC 019, and RFC 020;
4. owner-selected `0.20.2` release-local preparation;
5. one atomic exact finalization revision;
6. separate local and tagged evidence reviews;
7. an explicit owner actual-release decision.

Runtime source, APIs, behavior, features, dependencies, publication automation,
credential handling, and tag mutation are out of scope.

## 3. Files changed

This design round changes only:

- `rfcs/proposed/021-conditional-release-finalization.md`;
- `rfcs/handoffs/021-conditional-release-finalization/implementation-handoff.md`;
- `rfcs/README.md`.

Expected later implementation areas are enumerated by RFC 021 §4. They are not
authorized by this proposed handoff.

## 4. Design decisions and assumptions

- Conditional-finalization predicate `P` requires exact tree/tag identity,
  accepted tagged evidence, architecture release Go, and owner actual-release
  Go.
- `0.20.2` is owner-selected; availability must be rechecked before
  finalization and tag creation.
- `0.20.1` stays local, immutable, unpublished, and blocked.
- The final tagged tree must contain conditional release-current state and
  conditionally staged RFC 019/020 `done/` paths.
- RFC 000's ordinary folder semantics remain unchanged outside the narrow
  reviewed exception.
- Failure after tag creation quarantines the tag and requires another
  owner-selected version; tags are never repaired by movement.
- Push, GitHub release, registry publication, certification, and actual release
  remain separately authorized actions.

## 5. Tests and gates run

Observed for this design worktree:

- `cargo fmt --all -- --check`: passed;
- `cargo xtask check-rfcs`: passed;
- `cargo xtask doc-currency`: passed;
- `cargo xtask link-audit`: passed; 56 Markdown files scanned;
- `mdbook build docs`: passed; generated `docs/book/` removed; and
- `git diff --check`: passed.

No implementation or release gate is claimed by this proposed design.

## 6. Generated artifacts

Expected only:

- the tracked RFC 021 design;
- this tracked implementation handoff;
- a private direct-tree architecture review request.

No release archive, evidence directory, tag, or generated documentation is
owned by this design round.

## 7. Known limitations

- The conditional lifecycle exception changes current release semantics and
  requires architecture acceptance before implementation.
- The exact conditional metadata schema and CLI spelling remain implementation
  details bounded by RFC 021.
- A conditionally staged `done/` state is intentionally narrow and must not
  become a general substitute for shipped lifecycle state.
- Tag-bound evidence still occurs after tag creation; the protocol reduces
  risk through exact local evidence and architecture review but cannot make tag
  creation reversible.
- No actual release is authorized.

## 8. Recommended next step

Submit RFC 021 and this handoff for architecture design review. If accepted,
wait for project-owner authorization before moving RFC 021 to `accepted/` or
implementing any tooling, version, apex, lifecycle, or release-state change.
