# Architecture Recovery Roadmap

Status: Accepted recovery plan following the independent architecture audit and
R0 design freeze dated 2026-07-15. R0.5 lifecycle activation is complete; this
document does not claim that the R1/R2 blockers are already resolved.

## Purpose

Loeres v0.20.0 has a credible implementation architecture, but its declared
release controls and normative documentation are not currently sufficient for a
release-readiness claim or for starting new public-boundary work. The recovery
program restores a trustworthy baseline before product scope expands.

Scheduling is dependency- and evidence-gated. No calendar dates or duration
estimates are normative. A milestone advances only when its exit evidence has
been observed for the same revision.

## Governing constraints

The recovery must preserve:

- the five-crate server/device boundary and zero-bleed policy;
- Rust 2024 with declared MSRV 1.85 unless a separately approved decision changes it;
- the status/error split and validation-evidence integrity;
- no `std` or `alloc` in the edge baseline;
- RFC-first changes and the RFC 000 lifecycle;
- no commits, tags, publication, or pushes without project-owner authorization.

The independent audit is evidence for planning, not a replacement for project
requirements or RFCs. Accepted RFCs 019 and 020 convert its blocking findings
into frozen implementation contracts.

## Dependency schedule

```text
R0 Recovery design freeze
    |-- RFC 019: MSRV and release integrity
    `-- RFC 020: normative documentation authority and currency
             |
             v
R0.5 Lifecycle activation (`accepted/`)
             |
             v
R1 Corrective implementation and documentation baseline
             |
             v
R2 Tagged-revision and clean-extraction evidence
             |
             v
R3 Assurance expansion
    |-- verification budgets and portability evidence
    |-- supply-chain and operational robustness
    |-- numerical/conformance breadth
    `-- maintainability decomposition
             |
             v
R4 Next public capability RFC
```

RFC 019 and RFC 020 may be implemented in parallel only after both designs are
frozen and moved through the normative `rfcs/accepted/` transition. Their
closeout evidence converges at R2. R3 design may begin earlier, but its
implementation must not weaken or delay the blocking recovery.

## Milestone R0 — Recovery design freeze

Objective: turn the audit blockers into approved, bounded implementation
contracts.

Required deliverables:

1. RFC 019 defines the Rust-1.85 repair, canonical tag format, canonical release
   gate, documentation build, and package/clean-extraction evidence.
2. RFC 020 defines the normative document hierarchy, v0.20.x currency refresh,
   threat-model update, RFC-index correction, and historical-document policy.
3. Each RFC records non-goals, exact acceptance gates, rollback behavior, and
   ownership boundaries.
4. Focused developer handoffs exist for both RFCs.
5. RFC 020 selects RFC 000's five-folder `accepted/` lifecycle variant and
   specifies the atomic transition/checking mechanism.

Exit criteria:

- architecture review accepts both RFCs for design freeze;
- open choices are resolved or explicitly deferred;
- no implementation has been smuggled into the design change;
- `rfcs/README.md` identifies both RFCs as Proposed until the project owner
  authorizes the R0.5 lifecycle transition.

## Milestone R0.5 — Lifecycle activation

Objective: create a durable, policy-valid implementation authorization rather
than relying on ignored review files or an ambiguous frozen-Proposed state.

One atomic governance change must:

1. amend RFC 000 to adopt its documented five-folder variant;
2. create and check `rfcs/accepted/`;
3. move architect-approved RFC 019 and RFC 020 there with Accepted status and
   tracked design-freeze metadata after project-owner approval;
4. update the RFC index and all relative links;
5. extend RFC lifecycle tooling/tests for the Accepted state;
6. pass RFC, link, stable, and exact Rust 1.85 checks applicable to the change.

Exit criteria:

- both RFCs are durably Accepted, not Proposed and not falsely Implemented;
- RFC 000, folder location, Status fields, index, and tooling agree;
- only then may R1 corrective implementation begin.

## Milestone R1 — Corrective baseline

Objective: implement RFC 019 and RFC 020 without changing solver semantics or
runtime public APIs.

Shared integration order:

1. RFC 019 restores the exact Rust 1.85 all-feature workspace check and
   establishes the release-gate/package skeleton.
2. RFC 020 builds the traceability matrix, reconciles the normative documents,
   and adds semantic currency checks to the developer aggregate.
3. Shared `CHANGELOG.md`, `xtask`, evidence, and maintainer wording are
   integrated against one corrective version/revision.
4. RFC 019's final source-tree/package/clean-extraction release gate runs only
   after RFC 020 is integrated.
5. Both closeouts cite the same revision and corrective version.

Parallel authoring is allowed after R0.5, but RFC 019 must not certify a package
that predates RFC 020's normative reconciliation.

Exit criteria:

- B1 through B4 from the audit are demonstrably corrected;
- the exact MSRV, stable-toolchain, and documentation gates pass;
- no tracked documentation calls a contradicted v0.13.1 snapshot current;
- current public type names and shipped RFC status agree across indexes;
- the working tree contains no unrelated changes.

## Milestone R2 — Release evidence closure

Objective: prove that the corrective baseline can be released under the rules
it declares.

Required evidence for one revision:

- format check;
- all-feature/all-target clippy with warnings denied;
- all-feature workspace tests and doc-tests;
- exact Rust 1.85 all-feature workspace check;
- aggregate architecture/zero-bleed/no-std/conformance checks;
- mdBook build;
- package construction with the required root-level layout;
- the same complete gate suite against a clean extraction;
- a non-publishing demonstration that the canonical tag pattern selects the
  release workflow.

Exit criteria:

- evidence is retained in the review/release package;
- RFC 019 and RFC 020 closeout sections identify the observed revision;
- the project owner approves any tag or release operation separately;
- only a later RFC 021 finalization/evidence review may supersede the v0.20.0
  No-Go baseline with the owner-selected `0.20.2` candidate.

## Milestone R3 — Assurance expansion

Objective: address the audit's non-blocking and missing-evidence findings in
small follow-on RFCs. Candidate design rounds, ordered by risk, are:

| Candidate | Scope | Entry condition | Completion signal |
|---|---|---|---|
| Verification budgets and portability | Enforced artifact/monomorphization thresholds; soft-float, RISC-V, WASM, and AArch64 evidence classes | R1 complete; measurement method reviewed | Thresholds and target evidence are machine-enforced or explicitly waived |
| Supply-chain and operational robustness | Dependency vulnerability/license policy; large-model, cancellation-latency, and resource-exhaustion evidence | Canonical release gate exists | Selected security checks and stress profiles are release-visible |
| Numerical conformance expansion | Objective/residual metrics; populated extended/adversarial suites; large-N, ill-conditioned, property/fuzz cases | Normative solver claims are current | Corpus claims match enforced comparisons |
| Maintainability decomposition | ELOC measurement; split oversized verification/test modules; narrow waivers | R1 implementation settles touched files | Files comply or carry reviewed rationale |

These are candidate RFCs, not yet approved scope. They must be drafted
individually after R0 review rather than combined into one unreviewable program.

## Milestone R4 — Next public capability

No new solver family, public modeling surface, real FFI adapter, or edge/cluster
public API expansion may enter implementation before R2 closes. The next
capability RFC must state:

- the user problem and non-goals;
- affected crates and dependency direction;
- scalar, storage, validation, error, and workspace requirements;
- device determinism and cluster scalability impact;
- security/resource boundaries;
- conformance and release evidence;
- compatibility and migration policy.

## Review points and handoffs

A review request package is required at:

1. R0 design freeze;
2. R1 implementation completion before evidence closeout;
3. R2 Go/No-Go review;
4. each later RFC design freeze and implementation closeout.

Developer handoffs are mandatory for RFC 019 and RFC 020 because they coordinate
multiple governance surfaces. Later RFCs need handoffs only when the RFC alone
does not provide a safe implementation sequence.

## Current disposition

The repository remains No-Go for release/readiness claims. R0 design freeze and
the owner-authorized R0.5 `accepted/` transition completed on 2026-07-15. The
R1 corrective implementation, RFC 020 S1-S5 reconciliation, and RFC 019 S1-S5
gate baseline are owner-durable. Architecture review 024 accepted the
non-publishing tagged source/package/clean-extraction evidence for canonical
tag `0.20.1` at `ed282545...`, but RFC 021 later classified that immutable
local tag as an unpublished blocked candidate. Review 031 accepted RFC 021 S1,
and the project owner authorized bounded S2 release-local preparation for
`0.20.2`. The apex shared metadata remains draft, active conditional metadata
is absent, and RFCs 019/020/021 remain Accepted until separately reviewed Q2
atomic finalization and later evidence/authorization stages.
RFC 019 S2 began as a fail-closed candidate/package skeleton. The completed
gate now separates the developer aggregate from `release-gate`, validates
candidate metadata and a commit-derived regular-file manifest, constructs and
checks the root-layout archive, verifies extracted Git-object content/modes,
and repeats the applicable suite in a clean extraction. The accepted tagged
evidence proves the immutable candidate; it does not itself certify or publish
a release.
RFC 019 S3 aligns CI, the exact MSRV workflow, and the non-publishing release
workflow with the accepted command profiles. The release selector uses the
canonical unprefixed tag family, action references are full reviewed SHAs, and
mdBook is pinned to 0.5.4. The workflow may retain successful gate evidence but
still has no publication or GitHub-release step; certification remains No-Go.
