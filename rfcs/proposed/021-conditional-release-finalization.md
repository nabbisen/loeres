# RFC 021 - Conditional Release Finalization

**Status.** Proposed
**Tracks.** Architecture recovery milestone R2; review 025 blocker B6.
**Touches.** RFC lifecycle transition semantics, release-current documentation
metadata, release-candidate evidence sequencing, and owner release decisions.

---

### Extended Metadata

* **Rust Edition Compliance:** No runtime code change; release tooling remains
  Rust 2024 with declared MSRV 1.85.0.
* **Target Environment:** Repository governance and host-side release tooling.
* **Proposed release:** Owner-selected unused corrective version `0.20.2`.
* **Prior blocked candidate:** Local immutable tag `0.20.1`; never distribute,
  publish, move, or reuse it.

## 1. Summary

RFC 019 and RFC 020 require both release-local final state and tag-bound
evidence. The first attempted sequence created canonical tag `0.20.1` before
final lifecycle/current-marker activation. The tag is sound evidence for its
commit, but its immutable artifact explicitly says release readiness is No-Go,
the apex marker is draft, and RFCs 019/020 are Accepted and unshipped. Review
025 therefore rejected `0.20.1` as an actual release.

RFC 021 defines a conditional-finalization protocol for `0.20.2`. One exact
finalization revision contains the eventual release-consistent apex,
changelog, roadmap, handoff, and RFC lifecycle state. Before distribution,
those declarations are explicitly conditional on the named canonical tag,
accepted tag-bound evidence, and the project owner's actual-release decision.
When all conditions hold, the same immutable bytes describe current released
state without a post-tag mutation.

This RFC changes release-governance semantics only. It does not authorize
`0.20.2` preparation, lifecycle activation, tag creation, push, publication,
GitHub release, certification, or actual release until its design and each
later implementation/release gate are separately approved.

## 2. Problem statement

The current rules form a sequencing cycle:

1. RFC 019 requires tagged-revision evidence and an approved release at S6.
2. RFC 020 requires the apex trio and lifecycle state to close jointly with RFC
   019.
3. RFC 000 says `done/` means shipped and the folder is the source of truth.
4. A canonical tag is immutable.
5. Tag-bound evidence can exist only after tag creation.
6. If apex activation and RFC moves happen only after tag-bound evidence, those
   release-local changes are absent from the tagged artifact.
7. If they happen unconditionally before release, repository state falsely
   claims that unshipped work already shipped.

The `0.20.1` sequence chose item 6 and produced blocker B6. Moving the tag is
forbidden. Repeating the same sequence with `0.20.2` is also forbidden.

## 3. Scope

In scope:

- define one exact conditional-finalization revision;
- define the activation predicate and its fail-closed wording;
- define how RFC 000's folder-source-of-truth rule applies during the bounded
  finalization window;
- define the local-evidence, architecture, tag, tag-evidence, owner-decision,
  and distribution order;
- require the eventual `0.20.2` artifact to contain release-consistent state;
- define rollback and abandoned-candidate handling.

Out of scope:

- runtime code, solver behavior, public APIs, features, or dependencies;
- changing RFC 019's gate contents or archive contract;
- publishing automation or registry credentials;
- moving or rewriting `0.20.1`;
- choosing a version other than the owner's selected `0.20.2`;
- generalizing conditional lifecycle state beyond release finalization.

## 4. Affected areas

| Area | Planned impact after design acceptance |
|---|---|
| RFC 000 | Add the narrowly bounded conditional-finalization transition and folder semantics. |
| RFC 019 | Clarify pre-tag identity simulation, exact-finalization review, and tag-bound completion order. |
| RFC 020 | Define conditional apex activation and release-local atomicity. |
| `xtask` | Validate the intended canonical tag/version/revision before tag creation and validate conditional metadata consistently. |
| apex trio | Carry one identical conditional release-current block in the finalization revision. |
| RFC lifecycle/index | Stage RFCs 019/020 as conditionally Implemented in the exact finalization revision. |
| changelog/roadmaps/handoffs | Describe `0.20.2` without an unconditional pre-release shipped claim or permanent No-Go wording. |
| runtime crates | No source change. Version metadata only when later preparation is authorized. |

## 5. Public API and compatibility impact

No runtime public API, solver contract, feature flag, error type, validation
rule, storage boundary, or dependency direction changes.

The host-side release-governance contract gains:

- an intended-tag preflight for a clean untagged finalization revision;
- a machine-checkable conditional-finalization metadata block;
- an exact sequence that distinguishes preparation, activation, and
  distribution.

This is patch-level corrective governance for owner-selected `0.20.2`.

## 6. `std`, allocation, device, and cluster impact

Any tooling implementation is host-only and may use `std` and allocation.
There is no change to edge `no_std`/no-`alloc` boundaries, device determinism,
cluster scalability, numerical behavior, or runtime resource policy.

## 7. Terms

**Finalization revision (`F`).** One clean commit whose tracked tree contains
all bytes intended for the eventual `0.20.2` source artifact, including version
metadata, conditional apex metadata, RFC 019/020 lifecycle paths/status,
changelog, roadmaps, index, and handoffs.

**Intended tag (`T`).** Canonical unprefixed tag `0.20.2`.

**Local finalization evidence (`L`).** Complete non-publishing RFC 019 gate
evidence for `F`, including validation that intended tag `T` is canonical,
matches workspace/changelog/archive version, is unused locally and on the
observed origin, and would target `F`. This preflight must not create `T`.

**Tagged evidence (`E`).** Complete non-publishing RFC 019 evidence after the
owner creates `T`, proving `T^{commit} = F` and repeating the applicable
source/package/clean-extraction suite.

**Architecture acceptance (`A`).** Acceptance of the exact `F`, its local
evidence, conditional semantics, and later its tagged evidence.

**Owner release authorization (`O`).** The project owner's explicit Go to
release/distribute exact tag `T` after tagged evidence is accepted.

**Distribution (`D`).** Owner-authorized external publication of the canonical
tag and/or a release object. Registry publication remains a separately named
decision.

## 8. Conditional authorization and activation predicates

Distribution is authorized only when:

```text
Q = exact_tree(F)
    AND canonical_tag(T = 0.20.2, T^{commit} = F)
    AND tagged_evidence_accepted(E)
    AND architecture_release_go(A)
    AND owner_release_authorization(O)
```

The conditional release state becomes effective only when:

```text
P = Q AND owner_authorized_distribution_of(T, F)
```

Before `P` is true, `F` is a **release-finalization candidate**. Its staged
current-marker and Implemented status are not effective shipped-state claims.
When `Q` is true, the owner may perform exactly the separately authorized
distribution action. That distribution makes `P` true, and the same tracked
declarations become current for released artifact `0.20.2`; no post-tag edit
is needed.

Distribution must not occur before `Q`. Distribution cannot repair a false
`Q`, authorize a different action, or make a failed prerequisite true
retroactively.

## 9. Required release-local wording

The exact finalization revision must not contain an unconditional statement
that `0.20.2` is already released before `P`, and must not contain permanent
candidate wording that contradicts release after `P`.

The shared apex block must express one identical conditional rule in all three
documents, equivalent to:

```text
Release-finalization marker for 0.20.2.
Current when this exact tree is distributed under canonical tag 0.20.2 after
accepted tag-bound evidence, architecture release Go, and project-owner release
authorization; otherwise a non-current finalization candidate.
Implemented scope after activation: RFCs 001-020.
```

The exact wording is implementation-review scope, but the predicate and scope
must not drift among the three apex documents.

The changelog, root/recovery roadmaps, RFC index, and handoffs must use the same
conditional boundary. They may describe evidence already accepted for earlier
steps, but must distinguish it from `P`.

## 10. Conditional RFC lifecycle transition

RFC 000's folder-as-source-of-truth rule remains the default. RFC 021 adds one
narrow release-finalization exception:

1. Architecture must first approve this sequencing design.
2. One atomic finalization revision `F` may move RFCs 019/020 from
   `accepted/` to `done/`, update their Status fields and index rows, and add a
   machine-checkable conditional-activation qualifier.
3. In `F`, `done/` means “implementation complete and staged for the exact
   named release predicate,” not an unconditional claim that distribution
   already occurred.
4. `cargo xtask check-rfcs` must recognize this exception only for RFCs named
   by reviewed conditional metadata and only for the exact version/revision
   plan. Ordinary `done/` entries continue to mean shipped.
5. Once `P` becomes true, the staged `done/` state becomes ordinary Implemented
   history without moving or editing the files.
6. If the candidate is abandoned before distribution, a later reviewed
   rollback revision must move conditionally staged RFCs back to `accepted/`,
   restore draft apex state, and record the abandoned version/tag. An immutable
   failed local tag, if one exists, remains unused and unpublished.

This exception may not be used for feature implementation, partial evidence, a
moving branch, an unspecified version, or an unreviewed RFC set.

## 11. Corrected finalization sequence

### Phase Q0 — design approval

1. Review and accept RFC 021.
2. Project owner authorizes its implementation.
3. Move RFC 021 to `accepted/` with design-freeze metadata.

No version bump or lifecycle activation is part of Q0.

### Phase Q1 — tooling and conditional-metadata preparation

1. Implement intended-tag preflight without creating a tag.
2. Implement bounded conditional metadata/lifecycle checks.
3. Update RFC 000, RFC 019, and RFC 020 to the accepted protocol.
4. Prepare version `0.20.2` and the conditional apex/changelog/roadmap/handoff
   content without moving RFCs 019/020 yet.
5. Review and make this baseline owner-durable.

### Phase Q2 — exact finalization revision

1. Create one atomic tracked revision `F` that:
   - contains version `0.20.2`;
   - contains the identical conditional apex block;
   - moves RFCs 019/020 to `done/` with conditional status;
   - updates the RFC index and every affected link;
   - contains release-local changelog, roadmap, recovery, and handoff state;
   - contains no runtime/API change.
2. Run local finalization evidence `L` on clean `F` using intended tag
   `0.20.2` without creating the tag.
3. Submit exact `F` and `L` for architecture review.

### Phase Q3 — canonical tag evidence

Only after architecture accepts Q2 and grants narrow tag authority:

1. The project owner creates immutable local annotated/signed tag `0.20.2` on
   exactly `F`.
2. Run the complete non-publishing tagged gate and retain `E`.
3. Submit `T`, `F`, and `E` for architecture review.
4. Do not push the tag or trigger remote release automation yet.

### Phase Q4 — activation and distribution

Only after architecture accepts Q3:

1. Architecture issues explicit release Go for exact `T`/`F`/`E`.
2. The project owner issues explicit release authorization (`O`) and names the
   allowed distribution action(s); predicate `Q` becomes true.
3. The owner performs an authorized distribution action for exact `T`/`F`;
   predicate `P` becomes true at that release event.
4. Branch/tag push, remote workflow, GitHub release, registry publication, and
   certification remain distinct actions; authority for one is not inferred
   for another.
5. No tracked post-tag activation edit is required or permitted.

## 12. Candidate identity and tag rules

- `0.20.2` is owner-selected and was observed unused locally and on origin on
  2026-07-17. Availability must be rechecked before Q2 and Q3.
- `0.20.1` remains an immutable unpublished historical candidate/evidence tag.
- A canonical tag is never moved, rewritten, deleted for reuse, or force
  pushed.
- If Q3 creates a tag whose evidence fails, that tag is quarantined locally
  and a new owner-selected version is required.
- Artifact filenames may retain `v<version>` independently of unprefixed Git
  tag identity.

## 13. Security and secret handling

- No release credential, registry token, signing secret, or broad environment
  dump is required by the local gates.
- Tag signing may use the owner's configured mechanism; evidence may report
  cryptographic verification but must not overstate trust certification.
- Intended-tag origin availability is a read-only check and is not proof of
  ownership or publication authority.
- Package exclusions, safe path/type validation, and clean-extraction rules
  remain owned by RFC 019.
- Conditional metadata must fail closed on missing, duplicated, mismatched, or
  ambiguous version/tag/revision fields.

## 14. Failure and rollback policy

| Failure point | Required response |
|---|---|
| Q0 design rejected | Make no release-semantic or version change. |
| Q1 tooling/metadata review fails | Keep RFCs 019/020 Accepted and apex draft; revise before Q2. |
| Q2 local evidence or review fails | Do not create tag; correct through a new finalization revision and repeat Q2. |
| Q3 tag evidence fails | Keep tag local and unpublished; never move it; roll back staged lifecycle state through review and select another unused version. |
| Q3 architecture rejects release | Same quarantine/rollback rule; no distribution. |
| Owner declines Q4 | Keep tag and conditional state local/unpublished; `P` remains false. |
| Partial/accidental distribution | Stop further actions, record the incident, and require architecture/owner recovery; do not rewrite the tag. |

Rollback revisions are governance corrections, not mutations of an immutable
tagged artifact.

## 15. Rejected alternatives

| Alternative | Reason rejected |
|---|---|
| Move `0.20.1` | Violates canonical tag immutability and review 025. |
| Publish `0.20.1` with post-tag governance prose | Its artifact explicitly says No-Go/draft/Accepted and cannot be cured externally. |
| Repeat the same post-tag closeout for `0.20.2` | Recreates blocker B6. |
| Unconditionally mark release/current before evidence | Creates a false shipped-state claim and weakens fail-closed governance. |
| Keep RFCs Accepted in the final artifact and move them later | Makes the final artifact's lifecycle state self-inconsistent. |
| Treat branch state as tagged evidence | Does not prove immutable tag-to-commit identity. |
| Add a general `finalizing/` lifecycle folder | Broadens RFC 000 globally when a narrow reviewed exception is sufficient. |
| Automate publication in this correction | Expands credentials and blast radius beyond the sequencing defect. |

## 16. Verification gates

Design-review gates:

1. RFC lifecycle/index and Markdown-link checks.
2. Review of predicates `Q`/`P`, lifecycle exception, rollback, and exact
   ordering.
3. Confirmation that `0.20.1` remains blocked and immutable.
4. Confirmation that no runtime/public API change is authorized.

Implementation gates after acceptance:

- focused unit tests for intended-tag preflight;
- focused unit tests for conditional metadata and lifecycle validation;
- `cargo fmt --all -- --check`;
- all-target/all-feature Clippy with warnings denied;
- all-feature workspace tests and doc-tests;
- exact Rust 1.85 all-feature workspace check;
- `cargo xtask check`;
- `mdbook build docs`;
- RFC 019's complete local and tagged gates at Q2/Q3 respectively.

No gate may be claimed passed unless its output was observed for the reviewed
tree or immutable candidate named by that evidence.

## 17. Implementation plan

| Stage | Work | Review point |
|---|---|---|
| S0 Design freeze | Approve predicate, lifecycle exception, order, rollback, and `0.20.2` plan | Architecture review of RFC 021 |
| S0.5 Activation | Move RFC 021 to Accepted after owner authorization | Lifecycle review |
| S1 Preflight tooling | Add intended-tag and conditional-metadata checks | Focused tooling review |
| S2 Conditional preparation | Prepare `0.20.2` release-local metadata without RFC 019/020 moves | Documentation review |
| S3 Finalization revision | Atomic conditional apex activation and RFC 019/020 moves | Exact-revision/local-evidence review |
| S4 Tagged evidence | Owner tag, non-publishing gate, artifact review | Tag-bound architecture review |
| S5 Owner release | Explicit owner Go and separately authorized distribution actions | Final Go/No-Go |

## 18. Exit criteria

RFC 021 is complete only when:

1. the conditional-finalization semantics are incorporated into RFC 000, RFC
   019, and RFC 020;
2. intended tag/version/revision preflight is machine-checked before tagging;
3. one exact `0.20.2` finalization revision contains all release-local state;
4. local finalization evidence passes and architecture accepts exact `F`;
5. owner-created canonical tag `0.20.2` peels exactly to `F`;
6. complete tag-bound evidence passes and architecture accepts `T`/`F`/`E`;
7. the project owner issues the separate actual-release decision;
8. no post-tag tracked activation edit is required;
9. `0.20.1` remains immutable and unpublished; and
10. runtime APIs, behavior, features, and dependency boundaries remain
    unchanged.
