# RFC 020 - Implementation Handoff

**RFC.** [`020-normative-documentation-authority-and-currency.md`](../../accepted/020-normative-documentation-authority-and-currency.md)
**Handoff state.** Accepted; R0.5 lifecycle activation, S1 traceability, S2
apex reconciliation, corrected S3 supporting artifacts, and corrected S4
bounded semantic currency checks are owner-durable. S5 integrated documentation
evidence is in review preparation; apex currency remains draft.
**Target.** Same corrective baseline as RFC 019 unless architecture review separates them.

## 1. Summary

Reconcile Loeres's normative requirements, external design, detailed roadmap,
threat model, indexes, and concise READMEs with the implemented v0.20.x
architecture. Establish a durable authority/conflict policy so future design
starts from one coherent baseline.

This handoff is a reconciliation exercise, not permission to redesign public
APIs through documentation edits.

## 2. Scope followed

Implementation covers:

1. a traceability pass from requirements and external design through RFCs
   001-018 and current crate roots;
2. apex trio currency refresh with stable IDs;
3. recovery milestones and accurate evidence status;
4. current threat model for device, cluster, validation cache, observability,
   gateway, cancellation, and residual risks;
5. RFC index and crate/root README corrections;
6. bounded semantic currency checks;
7. joint release evidence with RFC 019.

Out of scope: runtime/API changes, silent requirement deletion, broad new
product scope, rewriting historical RFCs to look current, or treating handoffs
as normative design.

## 3. Files changed

Expected implementation files:

- `docs/specs/loeres-requirements-v1.md`
- `docs/specs/loeres-external-design-v1.md`
- `docs/specs/loeres-roadmap-milestones-v1.md`
- `docs/src/threat-model.md`
- `docs/src/specifications.md`
- `docs/src/recovery-roadmap.md`
- `README.md`
- `ROADMAP.md`
- `crates/*/README.md`
- `rfcs/README.md`
- bounded `xtask` documentation-check code/tests
- `CHANGELOG.md` at closeout

Runtime crate production source is evidence to inspect, not a target to edit.

## 4. Design decisions and assumptions

- Requirements and external design remain normative high-level constraints.
- Later approved/superseding RFCs are authoritative for their specific scope.
- The detailed roadmap owns sequencing/status but cannot override contracts.
- Code/test divergence triggers review; code does not silently supersede design.
- Requirement, decision, and RFC IDs remain stable.
- Accepted/frozen designs live in `rfcs/accepted/`; `proposed/` remains
  review-active and implementation-forbidden.
- Historical sequencing differences are recorded honestly.
- Future/aspirational surfaces are labeled unimplemented.
- RFC 019 owns release command/tag/package semantics; reuse its terminology.

## 5. Tests and gates run

RFC 020 S1 produced
`docs/specs/loeres-reconciliation-traceability-v020.md` before editing apex
prose. The matrix maps requirements/design sections, stable ID groups, RFCs
001-018, current crate roots, manifests, tests, and verification tooling. It
also records candidate amendments in RFC 020 §12.8's required schema and keeps
unapproved semantic changes as stop conditions.

S1 inventory commands inspected apex headings/IDs, RFC statuses and summaries,
crate root exports, feature manifests, the threat model, root/crate READMEs,
and known stale phrases. No runtime source was edited.

The first S1 architecture review required bounded traceability corrections.
The matrix now explicitly maps SEC-D-001..007 and OQ-001..012; separates RFC
008 server budgets/failure, RFC 009 observability, residual multi-tenant duties,
edge FFI prohibitions, and future server FFI obligations; marks PF-001..003
unimplemented and PF-004 only partially covered by solver-specific traits; and
adds REC-015 for stale root/crate manifest comments while keeping publication
status externally uncertain.

Required during implementation:

```text
mdbook build docs
cargo xtask check-rfcs
cargo xtask link-audit
cargo xtask check
```

Also run the RFC 019 complete release gate at joint closeout. Human review must
compare public crate re-exports, RFC closeouts, and normative prose; automated
metadata checks are not proof of semantic correctness.

S5 integrated evidence was run from the clean, owner-durable S4 baseline at
`0d76d4e`. The standalone currency, RFC lifecycle/index, 54-file Markdown-link,
and mdBook gates passed. The developer aggregate passed, including mandatory
host/edge profiles and 12/12 bounded conformance fixtures; its soft-float and
RISC-V profiles remained explicitly advisory-unavailable, while wasm32 and
Linux AArch64 remained documented-only. Workspace tests passed 269/269, the
workspace Clippy gate passed with warnings denied, and the Rust 1.85 workspace
check passed. Generated `docs/book/` output was removed after observation.

The first S5 architecture review found three residual semantic conflicts that
bounded automation intentionally did not infer. The correction aligns current
roadmap/book/changelog notices with the owner-durable S3/S4 baseline; describes
the cluster gateway as shipped safe categories plus `MockGatewayJob`, with
`ffi-gateway` inert and reserved for a separately reviewed adapter; adds
`validation_cache` to cluster crate-root topography; and describes the static
`WorkspaceFootprint` surface as implemented rather than a placeholder. No
runtime API, feature definition, dependency, or behavior changed.

## 6. Generated artifacts

Expected review artifacts:

- requirement/design/RFC/current-code traceability matrix;
- list of intentional amendments and historical departures;
- architecture review request package for the refreshed apex trio;
- joint RFC 019/RFC 020 release evidence at closeout.

No generated archive is owned by RFC 020; RFC 019 owns packaging.

## 7. Known limitations

- The refresh documents current limited solver breadth; it does not create new
  large-N, throughput, native-FFI, or cross-target evidence.
- Semantic prose review cannot be fully automated.
- Supply-chain, size-budget, broader target, and conformance expansion remain
  later candidate RFCs.
- External publication badge truth may require network verification; if not
  observed, record uncertainty rather than infer publication state.

## 8. Recommended next step

Submit the cumulative S1-S5 documentation baseline and integrated evidence for
architecture review. Do not activate the apex current marker through evidence
collection alone. After accepted owner durability, prepare S6 joint closeout
with RFC 019's complete tagged-revision, clean-extraction, package, retained
approval, and separate Go/No-Go evidence.
