# RFC 020 - Normative Documentation Authority and Currency

**Status.** Implemented (v0.20.2)
**Design approval.** Independent architecture R0 re-review accepted; project owner authorized R0.5 activation on 2026-07-15.
**Tracks.** Architecture recovery milestones R0-R2; audit blocker B4 and documentation findings N1, N2, and N6.
**Touches.** `docs/specs/`, `docs/src/`, `README.md`, `ROADMAP.md`, crate READMEs, `rfcs/README.md`, changelog cross-references, and documentation consistency checks.

---

### Extended Metadata

* **Rust Edition Compliance:** No runtime code change; documentation describes the Rust 2024 / MSRV 1.85 project.
* **Target Environment:** Cross-cutting requirements, design, security, roadmap, and maintainer documentation.
* **Proposed release:** Same corrective baseline as RFC 019 unless architecture review separates the closeouts.

## 1. Summary

Loeres declares design specifications to be the source for test design and
points readers from the root roadmap to an authoritative detailed roadmap. At
v0.20.0, the apex requirements, external design, and detailed roadmap still
identify v0.13.1 as current and state that no production std-side solver exists.
The repository has since shipped RFC 009, RFC 011, RFC 013, RFC 015, RFC 016,
RFC 017, and RFC 018.

The contradiction is now a governance blocker: a new RFC author cannot know
whether to follow the stale apex prose, implemented RFCs, current code, or root
roadmap assertions.

RFC 020 establishes a normative hierarchy, refreshes the apex trio to the
corrective baseline, updates the threat model and indexes to actual shipped
behavior, and adds semantic currency checks that complement mechanical link
checks. It does not retroactively rewrite historical decisions or change runtime
contracts.

## 2. Affected areas

| Area | Required impact |
|---|---|
| `docs/specs/loeres-requirements-v1.md` | Reconcile requirements/status through the corrective baseline; retain stable requirement IDs. |
| `docs/specs/loeres-external-design-v1.md` | Describe the actual cluster kernel, observability/gateway seam, validation cache, and current boundaries. |
| `docs/specs/loeres-roadmap-milestones-v1.md` | Reconcile completed RFCs and add recovery milestones without rewriting historical sequencing. |
| `docs/src/threat-model.md` | Replace early-design language with current device/cluster/cache/telemetry/gateway controls and residual risks. |
| `docs/src/specifications.md` | Publish the authority hierarchy and reconciliation metadata. |
| `README.md`, `ROADMAP.md`, crate READMEs | Remove stale phase/status claims and link to current normative sources. |
| `rfcs/README.md` | Correct semantic summaries, including RFC 016's current solve-record shape. |
| `CHANGELOG.md` | Record the documentation/governance correction without claiming runtime changes. |
| `xtask` documentation checks | Add bounded semantic currency assertions where stable and useful. |

## 3. Public API boundary impact

No runtime API changes are allowed. Documentation must describe current public
APIs exactly; it must not introduce aspirational types as shipped.

If reconciliation discovers a real code-versus-approved-RFC mismatch, RFC 020
must record it as a blocker and stop. It may not resolve runtime behavior by
editing prose to bless an accidental implementation.

## 4. Dependency impact

No runtime dependency changes.

Host-only documentation checks should use existing `xtask` capabilities and
simple repository parsing. A new documentation parser dependency requires
separate justification and is not expected for this RFC.

## 5. `std` / `alloc` impact statement

None. Documentation and host tooling only. The normative refresh must preserve
the edge no-`std`/no-`alloc` requirements and zero-bleed dependency direction.

## 6. Device determinism impact statement

No device runtime change. Documentation must accurately state:

- bounded iteration and caller-owned workspace behavior;
- poison-free overwrite-on-entry reuse for the implemented RFC 005/RFC 006
  baseline, rather than presenting reset-required semantics as the only rule;
- target-profile-scoped determinism, not universal bitwise identity;
- mandatory hard-float evidence versus advisory/documented-only profiles;
- panic-averse checks are not formal proof of panic freedom.

## 7. Cluster scalability impact statement

No cluster runtime change. Documentation must distinguish current evidence from
future claims:

- RFC 016 provides one dynamic projected-first-order numerical kernel;
- RFC 008 provides orchestration and bounded parallel/async integration;
- RFC 009 provides metadata observability and a safe mock gateway seam, not a
  concrete native solver adapter;
- RFC 015 provides an in-process validation evidence cache, not a persistent or
  distributed cache;
- there is no broad throughput, large-N, or multi-tenant stress claim yet.

## 8. Error, validation, and diagnostic impact

No runtime changes. The refreshed documents must preserve:

- non-convergence is an `Ok(SolveReport)` status, not `SolverError`;
- trusted/cache evidence never suppresses per-call current-iterate checks or
  hot-loop numerical-domain checks;
- wrong identity or stale epoch fails closed;
- telemetry uses bounded metadata categories and redaction;
- public errors are allocation-free and semver-extensible.

## 9. Feature flag impact

No feature changes. Documentation must mark existing inert/reserved versus
implemented features accurately. In particular, a default-off `ffi-gateway`
feature does not imply that a real native adapter ships.

## 10. Semver impact

Documentation/governance correction only. Stable requirement IDs, external
design decision IDs, and RFC numbers must not be renumbered.

If a normative requirement truly changes, the refresh must identify the change
as an amendment with rationale and compatibility impact. Silent semantic
revision is forbidden.

## 11. Normative authority model

### 11.1 Document roles

| Artifact | Authority | Role |
|---|---|---|
| `docs/specs/loeres-requirements-v1.md` | Normative | Product/environment constraints, goals, non-goals, acceptance requirements. |
| `docs/specs/loeres-external-design-v1.md` | Normative | Public crate/module boundaries, externally visible policies, cross-layer contracts. |
| Implemented RFCs in `rfcs/done/` | Normative and specific | Accepted detailed decisions and implementation departures for their scope. |
| Proposed RFCs in `rfcs/proposed/` | Review contract, not implemented truth | Candidate design; implementation must wait for approval/freeze. |
| Accepted RFCs in `rfcs/accepted/` | Normative frozen implementation contract | Architecture and project-owner review are complete; implementation may start, but shipped behavior must not yet be claimed. |
| `docs/specs/loeres-roadmap-milestones-v1.md` | Normative for sequencing/status | Milestones, dependencies, evidence gates; cannot override requirements or implemented RFC contracts. |
| Root `ROADMAP.md` | Current concise status | Maintainer-facing summary that must link to and agree with the detailed roadmap. |
| `CHANGELOG.md` | Historical release record | What shipped and when; not permission to change architecture. |
| Code and tests | Implemented behavior/evidence | Must conform to approved design; a mismatch triggers review rather than automatically overriding design. |
| Handoffs | Non-normative execution guidance | May sequence work but never override an RFC. |
| Ignored archived review bundles | Historical/private review input | Not a public normative source. |

### 11.2 Conflict rule

When current normative artifacts conflict:

1. stop new implementation in the affected boundary;
2. identify whether the conflict is stale prose, implementation divergence, or
   an intentionally superseded decision;
3. use the later approved/superseding RFC for scope-specific decisions, while
   preserving higher-level requirements unless explicitly amended;
4. patch all affected normative documents in one reviewed reconciliation;
5. record the resolution and evidence in the RFC/changelog.

Code does not silently win over design, and old apex prose does not silently
erase later implemented RFC decisions.

### 11.3 Currency metadata

Each apex specification must state:

- approval status;
- last reconciled repository release;
- last reconciled RFC number/range or explicit list;
- superseded source document, if applicable;
- whether open proposals are included only as roadmap items.

The three apex documents should share one reconciliation table or equivalent
machine-checkable markers so mixed-current headers cannot recur.

RFC 021 adds one bounded conditional form for exact release `0.20.2`. When
`release/conditional-finalization.toml` is present, all three apex documents
must replace the draft block with exactly one identical RFC 021 conditional
block that binds:

- release-finalization version and canonical tag `0.20.2`;
- current state only after the exact tree is distributed under that tag after
  accepted tag-bound evidence, architecture release Go, and project-owner
  release authorization;
- non-current release-finalization-candidate state otherwise;
- Implemented scope RFCs 001-021 only after activation; and
- the rule that stored lifecycle paths do not prove external activation.

The schema, phase, authoritative remote, distribution bundle, exact conditional
RFC set `{019,020,021}`, and 30/120-minute workflow boundaries must agree with
RFC 021 and RFC 000. A tracked document or tool cannot assert that the external
activation predicate occurred.

### 11.4 RFC lifecycle and design-freeze authorization

Loeres adopts RFC 000's five-folder lifecycle variant for recovery work and
later RFCs:

```text
rfcs/proposed/  -> review-active; implementation forbidden
rfcs/accepted/  -> design frozen; implementation authorized
rfcs/done/      -> implemented/shipped
rfcs/archive/   -> withdrawn or superseded
rfcs/draft/     -> optional authoring state
```

The `accepted/` transition is the durable implementation authorization. An
ignored review file or handoff is evidence/input only and cannot authorize the
transition.

Before R1 begins, one atomic R0 lifecycle-activation change must:

1. amend RFC 000's project-adoption section to state that Loeres now uses the
   five-folder variant;
2. create `rfcs/accepted/`;
3. move RFC 019 and RFC 020 from `proposed/` to `accepted/` only after the
   independent architect accepts the patched design and the project owner
   approves the transition;
4. update each RFC Status field to `Accepted` with the design-freeze date and a
   concise tracked approval note;
5. add an Accepted section to `rfcs/README.md` and update all relative links;
6. extend `cargo xtask check-rfcs` to validate accepted-folder status, index
   coverage, unique numbering, and link integrity;
7. run the exact RFC/link/MSRV-relevant checks for that atomic transition.

The transition actor is the project owner or an explicitly authorized
maintainer. The committed folder move, Status metadata, RFC index, and amended
RFC 000 are the normative record. Architect review remains required evidence,
but an ignored private copy is not itself normative.

If either RFC remains in `proposed/`, its implementation must not start. Moving
an RFC to `accepted/` does not claim code exists or gates pass; only movement to
`done/` after closeout records implementation.

## 12. Concrete reconciliation specification

### 12.1 Requirements refresh

The requirements document must be reviewed section-by-section against RFCs
001-018 and current manifests. At minimum it must incorporate or accurately
reference:

- implemented Milestones 1 and 2;
- current Milestone 3 storage, orchestration, kernel, observability/gateway, and
  validation-cache capabilities;
- RFC 010 release/check command distinction as amended by RFC 019;
- RFC 011 target evidence classes;
- RFC 013/RFC 017 conformance scope and limitations;
- explicit continued non-goals: broad solver parity, real native adapter,
  persistent/distributed validation cache, universal deterministic claims;
- current release-readiness blocker/recovery status until R2 closes.

Requirement IDs must be preserved. New requirements receive new IDs; deleted
requirements must instead be marked superseded with a pointer and rationale.

### 12.2 External-design refresh

The external design must update:

- `loeres-cluster::model`, `solve`, `observe`, `gateway`, and
  `validation_cache` public categories;
- current validation/evidence data lifecycle and non-skippable checks;
- cluster batch/runtime and projected-first-order integration;
- device workspace poison-free baseline;
- actual feature posture and reserved integration gates;
- security/redaction boundaries;
- current open public-boundary questions.

Illustrative future categories must be clearly labeled as unimplemented.

### 12.3 Detailed-roadmap refresh

The detailed roadmap must:

- retain historical milestone completion facts;
- record that historical implementation sequencing diverged from the old RFC
  011-before-device recommendation and explain the retrospective mitigation;
- list RFCs 009-018 with correct implemented status;
- include recovery milestones R0-R4 and their entry/exit gates;
- distinguish blocked, proposed, implemented, advisory, and documented-only
  evidence;
- stop claiming a green release gate for v0.20.0 after the audit.

### 12.4 Threat-model refresh

The threat model must describe actual components:

- device projected-first-order boundary validation and bounded loops;
- cluster dynamic solver and batch cancellation/timeout semantics;
- validation evidence cache identity, mutation epoch, cacheability, and stale
  evidence rejection;
- metadata-only telemetry and redaction;
- mock-only gateway state and future real-FFI risk;
- source audit limitations;
- server memory/CPU/multi-tenant residual risks;
- supply-chain checks not yet enforced.

It must separate implemented controls, release gates, residual risks, and future
work rather than blending them.

### 12.5 Index and README reconciliation

The RFC 016 index entry must name the current
`ProjectedFirstOrderSolveRecord { report, checked_scope, finite }` shape and
honest finite-evidence semantics.

Crate READMEs must not call implemented crates Phase 0 skeletons. Each must state
current shipped surfaces and clear environment constraints without duplicating
the full roadmap.

Root README claims, badges, and quick-start version text must be reviewed for
truthfulness. External publication badges must not be read as proof that crates
are published or current without evidence.

### 12.6 Semantic currency checks

RFC 020 extends documentation verification with bounded checks for stable facts:

1. all three apex docs carry the same last-reconciled release marker;
2. no current normative status block claims v0.13.1 after reconciliation;
3. every RFC in `rfcs/done/` appears in `rfcs/README.md` with Implemented status;
4. every RFC in `rfcs/proposed/` appears with Proposed status;
5. every RFC in `rfcs/accepted/` appears with Accepted status and tracked
   design-freeze metadata;
6. root roadmap recovery state matches RFC 019/RFC 020 lifecycle;
7. mdBook navigation includes the recovery roadmap and current threat model;
8. prohibited stale phrases identified during implementation are absent from
   current-status sections.

Checks should target stable metadata, not parse arbitrary prose or pretend to
prove semantic correctness. Human architecture review remains mandatory.

In RFC 021 conditional mode, the same gate must additionally reject missing or
duplicate conditional blocks, unequal apex blocks, version/tag/schema drift,
scope other than RFCs 001-021, omitted candidate wording, any claim that
`done/` proves activation, and a partial or extra conditional RFC move. The
absence of tracked conditional metadata keeps the existing draft/not-current
rules in force; tooling must not guess the mode from prose or folder paths.

### 12.7 Release-local review links

A release artifact must be self-contained enough to review its requirements,
external design, roadmap, RFC registry, and accepted/implemented RFCs without
network access.

Documentation pages must provide release-local repository paths for normative
sources. Default-branch web links may remain only when labeled as navigation to
the moving development branch; they must not be the sole normative link in a
tagged book or extracted release. Where mdBook cannot render a source-relative
link outside `docs/src/`, the page must print the release-local path and may add
a separately labeled web-navigation link.

### 12.8 Normative amendment record

Every substantive requirements or external-design amendment in the
reconciliation traceability matrix must record at least:

| Field | Meaning |
|---|---|
| affected ID | Stable requirement or external-design decision identifier |
| prior rule | Concise statement of the previously approved rule |
| new rule | Concise statement of the reconciled rule |
| approving RFC | RFC that authorized the semantic change, or `none` for prose-only currency correction |
| compatibility impact | Runtime/API, operational, documentation-only, or none, with rationale |
| reconciled release | Corrective baseline in which the amendment becomes current |

If no approving RFC exists for a substantive change, reconciliation stops and a
separate RFC is required. A prose-only clarification must say why semantics did
not change.

## 13. Security and secret handling

Documentation must not include secrets, real credentials, customer models,
private tenant identifiers, or internal deployment data. Examples use synthetic
metadata and bounded identifiers.

The threat model must not overclaim source scanners, mock gateways, or unit
tests as formal security proofs.

## 14. Rejected alternatives

| Alternative | Reason rejected |
|---|---|
| Declare the v0.13.1 apex trio historical and rely only on code | Violates design-first policy and removes the requirements/external-design layer needed for future RFCs. |
| Treat root `ROADMAP.md` as the sole authority | A release narrative cannot replace detailed requirements and public-boundary design. |
| Rewrite all documents from scratch with new IDs | Breaks traceability and makes historical review unnecessarily difficult. |
| Update only version headers | Leaves substantive contradictions and creates false currency. |
| Let implemented code automatically override RFCs | Rewards accidental divergence and undermines architecture review. |
| Add an elaborate semantic Markdown parser | High maintenance cost and false confidence; bounded metadata checks plus human review are sufficient. |
| Keep accepted/frozen RFCs in `proposed/` | Conflicts with RFC 000's meaning of Proposed and leaves implementation authorization ambiguous. |

## 15. Verification gates

Required design/reconciliation evidence:

1. a requirement-to-RFC/current-code traceability pass for RFCs 001-018;
2. an external-design public-surface comparison against crate roots and READMEs;
3. an RFC lifecycle/status/index pass;
4. a threat-model review against current validation, cache, observe, gateway,
   cancellation, and device solve code;
5. `mdbook build docs`;
6. `cargo xtask check-rfcs`;
7. `cargo xtask link-audit`;
8. the new semantic currency checks;
9. `cargo xtask check`;
10. RFC 019's complete release gate at joint closeout.

Documentation-only checks do not replace runtime tests at release closeout.

## 16. Rollback and partial-reconciliation policy

The authority policy, apex requirements, external design, and detailed roadmap
must be published atomically as one reviewed reconciliation. Supporting threat,
index, README, and currency-check changes must be consistent with that same
baseline before closeout.

If semantic review discovers an unresolved conflict among requirements,
approved RFCs, current public behavior, or security policy:

1. retain the recovery warning and current No-Go status;
2. do not claim the new last-reconciled release marker;
3. do not declare only one or two apex documents current;
4. revert partial currency/status markers or leave the reconciliation explicitly
   draft on its implementation branch until a separate decision resolves the
   conflict;
5. do not edit prose to bless accidental runtime behavior;
6. record the blocker and return the affected boundary to architecture review.

The prior v0.13.1 documents may remain historically inaccurate for current code
during this rollback state, but they must stay visibly marked stale and must not
be relabeled current. RFC 019 release evidence cannot close over a partial RFC
020 reconciliation.

## 17. Implementation sprint plan

| Sprint | Work | Review point |
|---|---|---|
| S0 Design freeze | Approve authority hierarchy, accepted-folder lifecycle, conflict rule, and currency metadata | Architecture review of RFC 020 |
| S0.5 Lifecycle activation | Atomically amend RFC 000, enable `accepted/`, move frozen RFCs, and extend lifecycle checks | Project-owner approval and lifecycle gate review |
| S1 Traceability matrix | Map requirements/design sections to RFCs 001-018 and current modules | Review before prose edits |
| S2 Apex reconciliation | Refresh requirements, external design, detailed roadmap | Apex-document architecture review |
| S3 Supporting docs | Update threat model, root/crate READMEs, RFC index, book pages | Security and maintainer review |
| S4 Currency checks | Add bounded metadata/status checks | Tooling review for false positives/negatives |
| S5 Full documentation build | Run links, mdBook, RFC, and aggregate gates | Documentation review request package |
| S6 Joint closeout | Combine with RFC 019 release evidence and move to `done/` | Go/No-Go review |

## 18. Exit criteria

RFC 020 is complete only when:

1. the five-folder lifecycle is active and RFC 019/RFC 020 entered R1 from
   `rfcs/accepted/` under §11.4;
2. the normative hierarchy and conflict rule are published;
3. the apex trio is reconciled through the chosen corrective baseline;
4. requirements and decision IDs retain traceability and amendment records;
5. no current normative text claims the absence of shipped RFC 009/RFC 016/RFC
   015/RFC 017 capabilities;
6. the threat model describes actual controls and residual risks;
7. RFC 016's index summary matches the current public solve record;
8. crate READMEs no longer claim Phase 0 skeleton status where implementation exists;
9. historical sequencing divergence is recorded, not erased;
10. release-local paths make the normative baseline self-contained;
11. semantic currency, RFC, link, mdBook, and aggregate checks pass;
12. an architecture review accepts the refreshed documents;
13. RFC 019's tagged-revision/clean-extraction evidence closes before any new
    public-boundary implementation begins.

## 19. Historical 0.20.1 S6 closeout preparation

This section records the earlier `0.20.1` joint S6/R2 preparation; it does not
describe the current lifecycle state. At that stage RFC 020 remained in
`accepted/`, the shared apex metadata remained draft, and new public-boundary
implementation remained blocked.

Architecture review 024 accepted RFC 019's non-publishing evidence for
canonical unprefixed tag `0.20.1`, which peels to
`ed282545fe12de7827377c690a3e7024f0f4fbeb`. That tagged archive contains the
integrated RFC 020 S1-S5 reconciliation: the shared draft apex trio, authority
and conflict rules, stable traceability/amendment records, current threat and
supporting documentation, RFC/index lifecycle state, release-local paths, and
bounded semantic currency checks. The accepted artifact has 179 tracked
regular files; independent review matched its manifest, contents, modes,
required documentation, exclusions, and safe paths/types to the tagged commit.
RFC 019 §18 records the package identity, digest, tools, gate scope, and
evidence-class limitations without creating a second normative gate list.

The tag is immutable at `ed282545...`; this closeout-preparation section and
the accompanying status refresh are post-tag repository-governance records and
are not files in the accepted archive. They must not be presented as tagged
artifact content. Their purpose is to make the later owner and architecture
decisions auditable while preserving the distinction between artifact evidence
and repository lifecycle state.

For this post-tag closeout-preparation worktree, lifecycle, semantic currency,
54-file link, mdBook, aggregate, formatting, all-target/all-feature Clippy,
275-test plus doc-test, exact Rust 1.85, and diff gates were freshly observed
passing. The aggregate reported mandatory profiles 2/2 and conformance 12/12,
with the accepted advisory classifications unchanged. Generated `docs/book/`
output was removed.

At that historical stage, joint S6/R2 closure still required owner durability,
architecture review, and a separate project-owner decision.

## 20. RFC 021 conditional finalization for 0.20.2

RFC 020 was staged with RFCs 019/021 in `done/` under the exact tracked
`0.20.2` conditional metadata. The apex trio carried one canonical RFC 021
block: before external predicate `P` succeeded it was a non-current
release-finalization candidate, and stored documentation or lifecycle state
did not by itself prove activation.

The atomic Q2 tree required one exact clean revision, complete local and
intended-tag evidence, and architecture acceptance — all completed. Tag
creation, tag-bound evidence, release Go, owner distribution authorization,
and successful distribution followed.

**Resolution (2026-07-30).** Repository release `0.20.2` released
successfully under RFC 021's protocol, carrying RFCs 001-021. RFC 024
subsequently retired the conditional-finalization apparatus from ordinary
post-release development; publication, GitHub release, and certification
remain separately authorized and have not been sought.
