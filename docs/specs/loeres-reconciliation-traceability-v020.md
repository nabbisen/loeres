# Loeres v0.20 Architecture Reconciliation Traceability Matrix

**Status:** Historical RFC 020 S1 inventory and S2 edit record; not an apex
currency marker
**Inventory snapshot:** repository release v0.20.0 plus accepted RFC 019/RFC
020 recovery work through commit `1132033`, captured before corrective-version
selection
**Prepared:** 2026-07-15
**Current-state annotation:** 2026-07-17; B5 correction after architecture
review 022
**Scope:** requirements, external design, detailed roadmap, implemented RFCs
001-018, current public crate roots, manifests, verification tooling, threat
model, RFC registry, and concise READMEs

This matrix was the required review boundary before the RFC 020 apex prose was
edited. It preserves the dated S1 inventory, discrepancy classifications, REC
ledger, and S2 instructions as reconciliation provenance; it does not itself
declare the apex trio current. Unless a passage is explicitly labeled as the
current candidate state, “current,” future-tense instructions, and S2
dispositions below describe the 2026-07-15 inventory/reconciliation stage. If
review finds a code-versus-approved-design conflict, reconciliation still stops
under RFC 020 §16.

## 1. Authority and comparison rules

The comparison uses the authority order accepted in RFC 020 §11:

1. requirements and external design provide the high-level constraints;
2. later implemented RFCs govern their specific approved scope;
3. the detailed roadmap governs sequencing and status but cannot override a
   requirement or implemented RFC;
4. code and tests are implementation evidence, not automatic authority;
5. a mismatch without an approving RFC is a blocker, not a prose-edit license.

At the S1 inventory snapshot, the target last-reconciled repository release
remained **v0.20.0**. Recovery implementation after that release was described
separately and could not be represented as shipped. At that time, the project
owner had not selected or authorized a corrective release version.

**Current candidate state (2026-07-22).** Immutable local tag `0.20.1` is a
blocked unpublished candidate under RFC 021. The project owner selected
**0.20.2**; reviews 031/032 accepted S1/S2 and the owner authorized one atomic
Q2 finalization revision. The apex trio now carries the identical canonical
RFC 021 conditional block, and RFCs 019/020/021 are implementation-complete and
conditionally staged in `done/` under the tracked metadata. Before external
predicate `P` succeeds, this is a non-current candidate and shipped scope
remains RFCs 001-018. Exact local/tagged evidence, later architecture
decisions, owner release authorization, and successful distribution remain
pending. This annotation changes no runtime semantics or public API.

## 2. Requirements-section coverage

| Requirements scope | Stable IDs | Governing RFCs | Current implementation/evidence | S2 disposition |
|---|---|---|---|---|
| Goals, non-goals, crate isolation | G-001..008, NG-001..010 | RFC 000, 001-011, 014, 016 | Five crate manifests; `cargo xtask zero-bleed`, `no-std`, `check-public-api`; crate roots | Preserve; refresh status examples only. |
| Core contracts | CORE-001..014 | RFC 001-003, 012, 014 | `loeres::{scalar,access,dimension,error,diagnostic,solver,validation}`; `problem` remains reserved | Preserve; distinguish implemented modules from the reserved problem module. |
| Scalar capabilities | SCALAR-001..007; ADR-011, ADR-017 | RFC 001 | Six public scalar tiers; `f32`/`f64` implementations and scalar tests | Preserve. |
| Vector/matrix access | VM-001..008; ADR-020 | RFC 002 | Fallible access/mutation, views, dimension types, contiguous fast paths | Preserve exact-size row-major rule and per-axis errors. |
| Allocation-free errors and outcomes | ERR-001..006; ADR-010, ADR-018 | RFC 003, 014 | `SolverError`, compact diagnostics, `SolveReport` status/error split | Preserve; never describe non-convergence as an error. |
| Static storage | BSTATIC-001..012 | RFC 004 | Fixed arrays and contiguous static views; advanced `static-views` still deferred | Refresh status; do not claim advanced views. |
| Device workspace and solve loop | WS-001..008, STEP-001..007, DEVICE-001..013 | RFC 005, 006 | Caller-owned workspace, overwrite-on-entry reuse, bounded projected-first-order kernel, optional constant iteration | Replace reset-required-only threat prose with the approved poison-free baseline. |
| Device threat controls | SEC-D-001..005/007 | RFC 001-006, 010, 011, 014 | Dimension/scalar validation, non-finite rejection, iteration cap, no heap, typed errors, target-scoped timing claims | Implemented for the current device PFO boundary; panic/timing evidence remains non-formal and target-scoped. |
| Device ill-conditioning documentation | SEC-D-006 | RFC 006 | Current PFO documents its supported bounds, step-scale, finite, and numerical-domain rules; it does not claim general conditioning detection | Partial/solver-specific; retain as an obligation for each later solver family. |
| Dynamic storage | BSTD-001..008 | RFC 007 | Dense `Vec` adapters and CSR sparse storage; finite-scan helpers | Preserve; label BLAS/native/SIMD adapters unimplemented. |
| Cluster orchestration | CLUSTER-001..004, CLUSTER-006..007, CLUSTER-010 | RFC 008 | Batch outcomes, config, cancellation, timeout, sequential/parallel/async execution, `ClusterJob` seam | Refresh from orchestration-only snapshot without adding throughput claims. |
| Server budgets, cancellation, and typed failure | SEC-S-001/002/006, CLUSTER-006/010 | RFC 008, 014 | Iteration/deadline budgets, cooperative cancellation, per-item typed outcomes, unwind panic containment | Implemented with explicit limitation: `panic = "abort"` aborts the process; panic containment is not universal. |
| Observability/redaction | CLUSTER-005, SEC-S-004 | RFC 009 | Metadata-only observer API, bounded categories, explicit sinks, redaction | Implemented; no model values or tenant identifiers in baseline telemetry. |
| Multi-tenant isolation documentation | CLUSTER-009, SEC-S-005 | RFC 008, 009 | Per-item outcomes and metadata redaction reduce coupling/leakage, but no broad isolation or stress evidence exists | Residual documentation obligation; do not mark fully satisfied. |
| Edge FFI prohibition | FFI-001..003 | Requirements/ED-008 plus crate-boundary RFCs 001-006 | No FFI dependency or public type in core/static/device; zero-bleed/public scans | Implemented boundary prohibition; not an RFC 009-delivered gateway feature. |
| Server FFI policy/seam | CLUSTER-008, FFI-004..007 | RFC 009 | Default-off `ffi-gateway`, safe gateway categories, `MockGatewayJob`; no concrete native adapter | Policy/seam only. FFI ownership/licensing/thread/failure obligations remain activation gates for a future adapter. |
| Target-scoped determinism | DET-001..009, PANIC-001..006, ADR-014..015 | RFC 006, 010, 011 | Mandatory host/hard-float profiles; advisory-installed soft-float/RISC-V; documented-only WASM/AArch64; panic audit | Publish evidence classes; do not imply universal identity or formal panic proof. |
| Validation vocabulary and reuse | validation rules in §§5.4, 7, 8; AC-014 | RFC 012, 015, 017 | Core coverage/trust vocabulary; model identity, mutation epochs, in-process evidence cache; enforced cache conformance fixtures | Add current cache lifecycle and non-skippable per-call/hot-loop checks. |
| Current numerical kernel/parity | G-004/005, requirements §5.4.3, DEVICE-005, AC-011..014 | RFC 006, 013, 016, 017 | Device and cluster box/bound-constrained projected-first-order paths; dimension-2 smoke parity | One narrow solver family only; no large-N or broad solver claim. |
| Planned problem-family contracts | PF-001..003 | none yet | No public LP, QP, or SOCP core contract. Quadratic smoke fixtures do not create a public QP model | Future/unimplemented; S2 must not mark these IDs satisfied. |
| Generic iterative core contract | PF-004 | RFC 006/016 only for solver-specific oracle traits | `loeres::problem` remains reserved; device/cluster PFO problem traits are solver-specific | Partial, not a generic core family contract. Keep open for a dedicated RFC if needed. |
| CI and release controls | CI-001..010, REL-001..008 | RFC 010, 011, 013, 017, accepted RFC 019 | Developer aggregate passes; RFC 019 S1-S3 restored MSRV and aligned workflows; release package gate remains fail-closed | Remove green-release claim; separate developer evidence from incomplete release evidence. |
| Documentation and acceptance | AC-001..017; §§10, 12-15 | RFC 000-020 as applicable | RFC registry, mdBook, current root roadmap, recovery roadmap | Refresh statuses while preserving IDs and historical facts. |

### 2.1 Open-question disposition

| ID | Current disposition | Governing evidence | S2/S3 treatment |
|---|---|---|---|
| OQ-001 | Resolved for the baseline | RFC 006 selected a bounded box projected-first-order device family | Record the narrow family; broader QP/SOCP work remains future. |
| OQ-002 | Resolved as target-profile policy | RFC 011 defines mandatory/advisory/documented-only profiles | Name evidence classes; do not imply all profiles are tested. |
| OQ-003 | Resolved for mandatory evidence, not all portability | RFC 011 makes thumbv7em hard-float mandatory; soft-float is advisory-installed | State the distinction rather than a universal hardware-FP requirement. |
| OQ-004 | Deferred/open | RFC 001 reserves fixed-point hooks but ships no fixed-point baseline | Keep feature/integration hooks labeled inert/future. |
| OQ-005 | Resolved | RFC 001 six-tier scalar model | Preserve existing resolution. |
| OQ-006 | Resolved for current access extensions | RFC 002 contiguous fast paths; heavy kernels backend/solver-owned | Preserve existing resolution; later kernels need their own RFCs. |
| OQ-007 | Resolved for the baseline | RFC 005 chooses typed workspace first; raw scratch remains optional/future | State poison-free typed-workspace baseline. |
| OQ-008 | Resolved | RFC 007 selects dynamic dense and CSR sparse adapters | Record implemented adapters and deferred native/SIMD integrations. |
| OQ-009 | Partially resolved/deferred | RFC 009 defines a safe mock gateway and default-off seam only | Concrete native FFI adapter remains unimplemented and separately gated. |
| OQ-010 | Partially resolved/open at formal-proof level | RFC 010/011 panic audit and target gates provide panic-averse evidence | Never claim formal panic freedom; stronger proof needs a later RFC. |
| OQ-011 | Resolved as scoped policy; broad reproducibility remains unsupported | RFC 011 target-scoped determinism | Preserve target-scoped claims and non-goal of universal bit identity. |
| OQ-012 | Open | Requirements call for `TERMS_OF_USE.md` or equivalent, but no dedicated safety-critical disclaimer exists | Retain as a documentation blocker/question; license warranty text alone is not an engineering-use disclaimer. |

## 3. External-design coverage

| External-design scope | Decisions | Governing RFCs | Current evidence | Reconciliation action |
|---|---|---|---|---|
| Workspace, dependencies, import model (§1.1-1.4) | ED-001..003 | RFC 001-011 | Workspace manifests and zero-bleed graph | Preserve. |
| Public module topography (§1.5) | ED-009, ED-015 | RFC 001-009, 012, 014-016 | Current crate roots and re-exports | Add cluster `validation_cache`; mark reserved modules honestly. |
| Feature matrix (§1.6-1.7) | ED-002, ED-008 | RFC 004-011 | Crate manifests | Remove obsolete “no-op skeleton” wording; keep inert/reserved features explicit. |
| Target profiles and automation (§1.8-1.9) | ED-007 | RFC 010, 011, 013, 017, accepted RFC 019 | `target-profiles.toml`, xtask gates, aligned workflows | Add evidence classes and recovery No-Go boundary. |
| Core trait/error/solver topology (§2) | ED-003..004, ED-009..010, ED-014..015 | RFC 001-003, 012, 014 | `loeres` public exports and tests | Preserve with validation module status. |
| Cluster interface (§3) | ED-013 | RFC 007-009, 012, 015, 016 | Dynamic storage, orchestration, PFO kernel, observe/gateway, cache | Replace future-only categories with exact shipped surfaces and limitations. |
| Device interface (§4) | ED-005..007, ED-011..012 | RFC 004-006, 011 | Static storage, typed workspace, bounded device kernel | Preserve poison-free reuse and target-scoped determinism. |
| Threat validation (§5) | ED-006, ED-008, ED-010..011, ED-013..014 | RFC 003, 005-006, 008-009, 012, 014-017 | Structured errors, cancellation, redaction, cache fail-closed behavior | Refresh controls versus residual risks; no formal-security overclaim. |
| Decision register (§6) | ED-001..015 | RFC 001-016 | Implemented RFC decisions and current APIs | Preserve IDs; append reconciliation notes rather than renumber. |
| RFC roadmap (§7-10) | all above | RFC 001-020 | RFC registry and recovery roadmap | Extend implemented list through RFC 018 and recovery status. |

## 4. Implemented RFC-to-code traceability

| RFC | Shipped | Requirements/design scope | Current modules and evidence | Reconciliation note |
|---:|---|---|---|---|
| 001 | v0.6.0 | CORE-005/011, SCALAR-001..007, ED-004 | `loeres::scalar`; six tiers and primitive implementations | Current and consistent. |
| 002 | v0.7.0 | CORE-006/012, VM-001..008, ADR-020, ED-009/015 | `loeres::{access,dimension}`; fallible access/views/fast paths | Current and consistent. |
| 003 | v0.4.0 | CORE-007/010, ERR-001..006, ED-010 | `loeres::{error,diagnostic}`; allocation-free errors | Current and consistent. |
| 004 | v0.8.0 | BSTATIC-001..012 | `loeres-backend-static::{array,dimension,view}` | README status stale; advanced views remain deferred. |
| 005 | v0.9.0 | WS-001..008, ED-005/011/012 | static `workspace`; device `workspace`/`config`; reuse tests | Threat-model reset-required wording stale. |
| 006 | v0.10.0 | DEVICE-001..013, STEP-001..007, DET/PANIC baseline | device `problem`/`solve`; bounded PFO and caller workspace | README status stale; solver breadth remains narrow. |
| 007 | v0.11.0 | BSTD-001..004/007 | backend-std `dense`/`sparse` | Current capability, but manifest comments still say Phase 0/no-op. |
| 008 | v0.13.0 | CLUSTER orchestration, ED-013 | cluster `batch`/`runtime`/`solve`; cancellation and dispatch | Historical orchestration-only claim must be scoped to RFC 008, not current cluster state. |
| 009 | v0.15.0 | CLUSTER-005/008, SEC-S-004, gateway policy portions of FFI-004..007 | cluster `observe`/`gateway`; metadata events and mock gateway | Apex says planned; update while retaining “no native adapter” and residual multi-tenant/FFI obligations. |
| 010 | v0.16.1 | CI-001..010, REL-001..006 | xtask developer aggregate and focused checks | RFC 019 supersedes the old `release-gate` alias semantics. |
| 011 | v0.17.0 | DET-001..009, target portions of CI | target-profile manifest/checker | Add mandatory/advisory/documented-only distinction and sequencing history. |
| 012 | v0.12.0 | validation-state policy | `loeres::validation`; cluster validation policy | Current vocabulary; no implicit validation bypass. |
| 013 | v0.18.0 | parity/conformance acceptance evidence | `conformance/`; `cargo xtask conformance` | Smoke dimension-2 family only; no broad parity claim. |
| 014 | v0.5.0 | ERR/STEP status taxonomy, ADR-018, ED-014 | `loeres::solver`; reports and status/error split | Current and consistent. |
| 015 | v0.19.0 | explicit validation reuse/cache | cluster `validation_cache`; cached `f64` solve path | In-process only; not persistent/distributed. |
| 016 | v0.14.0 | G-004/005, requirements §5.4.3, first server kernel | cluster `model` and `solve::projected_first_order` | Apex “no kernel” claim and RFC index record shape are stale. |
| 017 | v0.20.0 | cache/trust conformance evidence | conformance v2 fixtures and xtask parser/runner | Enforced smoke fixtures only. |
| 018 | v0.20.0 | test maintainability only | cluster solve test modules | No public/normative behavior change. |

## 5. Current public-surface comparison

| Crate | Current public surface | Constraints/limitations to state |
|---|---|---|
| `loeres` | `access`, `diagnostic`, `dimension`, `error`, `problem`, `scalar`, `solver`, `validation`; root re-exports for implemented contracts | `problem` remains reserved; no storage, runtime, allocation, or OS policy. |
| `loeres-backend-static` | `dimension`, `view`, `workspace`; feature-gated `array` | no-std/no-alloc; baseline contiguous views; advanced `static-views` deferred. |
| `loeres-device` | `config`, `diagnostic`, `problem`, `solve`, `workspace` | bounded PFO family; owned-array kernel surface; no broad solver parity. |
| `loeres-backend-std` | `dense`, `sparse`, `adapter`, `batch`, `view`; dense/sparse root exports | dense/sparse implemented; adapter/batch/view categories remain placeholders; no native backend. |
| `loeres-cluster` | `batch`, `gateway`, `model`, `observe`, `runtime`, `solve`, `validation_cache`; corresponding root exports | one dynamic PFO kernel; safe mock gateway only; cache is process-local; no large-N/throughput/multi-tenant stress evidence. |

## 6. Candidate amendment and correction ledger

“Prose-only” means the approved rule and runtime/API remain unchanged. It does
not mean the edit is optional.

| ID | Affected ID/section | Prior rule or claim | Reconciled rule or claim | Approving RFC | Compatibility impact | Reconciled release |
|---|---|---|---|---|---|---|
| REC-001 | All apex currency blocks | v0.13.1 is current; RFC 009/010 follow | RFC 020 reconciled implemented RFC 001-018 through v0.20.0; RFC 021 conditionally stages the exact v0.20.2 marker and RFCs 019/020/021 without inferring external activation | RFC 009-021 | Documentation/governance conditional-finalization state; no runtime/API change | v0.20.0 snapshot; conditional v0.20.2 finalization |
| REC-002 | Roadmap §§1.1-1.2 | Accepted/frozen RFC remains Proposed in `proposed/` | Loeres uses Proposed → Accepted → Implemented and the `accepted/` folder | RFC 000 as amended; RFC 020 §11.4 | Governance/documentation; no runtime/API impact | corrective baseline after v0.20.0 |
| REC-003 | CI-001..010, REL-001..008; external design §1.9; roadmap §5 | `check`/`release-gate` alias and green release gate | `check` is the developer aggregate; RFC 019 candidate gate is separate and remains fail-closed until joint closeout | RFC 019 | Operational tooling already implemented; no runtime API impact | corrective baseline after v0.20.0 |
| REC-004 | G-004/005; requirements §5.4.3; external design §3.5; roadmap Milestone 3 | No std-side numerical kernel exists | One dynamic box projected-first-order kernel ships through RFC 016 | RFC 016 | Additive cluster API already shipped; prose reconciliation | v0.14.0 / reconciled at v0.20.0 |
| REC-005 | CLUSTER-005/008, SEC-S-004, gateway-policy portions of FFI-004..007; external design §§3.6-3.7 | Observe/gateway categories are planned | Metadata observability and a safe mock gateway seam ship; no concrete native adapter ships; SEC-S-005 and concrete FFI obligations remain residual | RFC 009 | Additive cluster API already shipped; no new FFI or isolation claim | v0.15.0 / reconciled at v0.20.0 |
| REC-006 | Validation policy in requirements §§5.4/8; external design §§3.4/5 | Validated/trusted reuse is abstract | RFC 012 vocabulary plus RFC 015 process-local identity/epoch cache; wrong/stale evidence fails and per-call/hot-loop checks remain | RFC 012, 015, 017 | Additive cluster API already shipped; security clarification | v0.19.0-v0.20.0 |
| REC-007 | WS-007; ED-011; threat model fail-safe section | Failed device workspace is described as reset-required | RFC 005/006 baseline is always reusable because required scratch is normalized on entry | RFC 005, 006 | Prose-only correction to already-approved lifecycle | v0.9.0-v0.10.1 / reconciled at v0.20.0 |
| REC-008 | DET-001..009, ADR-015, external design §1.8 | Target evidence is described as one future/reference profile | Mandatory host/hard-float, advisory-installed soft-float/RISC-V, and documented-only WASM/AArch64 are distinct evidence classes | RFC 011 | Documentation/evidence classification only | v0.17.0 / reconciled at v0.20.0 |
| REC-009 | RFC index entry 016 | Two-field record (`checked`/`trust`) | `ProjectedFirstOrderSolveRecord { report, checked_scope, finite }` with named finite evidence | RFC 016 implementation reconciliation | Documentation-only correction to shipped API | v0.14.1 / reconciled at v0.20.0 |
| REC-010 | RFC 013/017 verification descriptions | Conformance is absent or future | Enforced smoke parity/cache fixtures exist, but broad/adversarial/large-N evidence does not | RFC 013, 017 | Documentation/evidence correction | v0.18.0-v0.20.0 |
| REC-011 | Static/device crate README status | Crates are Phase 0 skeletons | RFC 004-006 storage/workspace/device PFO surfaces are implemented | RFC 004-006 | Documentation-only status correction | v0.8.0-v0.10.1 / reconciled at v0.20.0 |
| REC-012 | Threat model | Early design controls; access/solvers “ahead on roadmap” | Separate implemented device/cluster/cache/telemetry controls, release gates, residual risks, and future native-FFI/supply-chain work | RFC 005-017, accepted RFC 019/020 | Security documentation correction; no new assurance claim | v0.20.0 + recovery state |
| REC-013 | Root publication badges | Moving external badges may imply current publication | Badge truth must be verified externally or labeled navigation; repository state alone does not prove publication | none; prose-only evidence rule | Documentation-only; unresolved external observation | TBD |
| REC-014 | Detailed roadmap sequencing history | Target-profile RFC appears after the device kernel despite earlier recommendation | Record that RFC 006 shipped before RFC 011 and that later target-profile/conformance gates provide retrospective mitigation, without rewriting history | RFC 006, 011, 013 | Historical documentation only | v0.17.0-v0.18.0 / reconciled at v0.20.0 |
| REC-015 | Root `Cargo.toml` comments; `crates/loeres/Cargo.toml`; `crates/loeres-backend-std/Cargo.toml` | All members are an unpublished Phase 0 skeleton with no public API; implemented features are blanket no-ops | Tracked code/RFCs prove public APIs and implemented dense/sparse features. Inert/reserved features remain labeled as such. External publication status stays unverified unless separately observed | RFC 001-018 as mapped above; publication claim has no local proof | Comment/documentation correction only; no manifest key, feature, dependency, runtime, or API change | v0.20.0 + recovery state |

No candidate in this ledger changes a runtime contract without an approving
RFC. If S2 review identifies such a change, it must be removed from this
reconciliation and handled by a separate RFC.

## 7. Known limitations that must remain explicit

- Only one box/bound-constrained projected-first-order family has device and
  cluster implementations; this is not broad LP/QP/SOCP parity.
- The enforced conformance corpus is a bounded smoke corpus, not universal
  numerical equivalence.
- No concrete native solver adapter ships; `ffi-gateway` is a default-off seam.
- The validation evidence cache is in-process and model-carrier-scoped, not
  persistent or distributed.
- No broad throughput, large-N, multi-tenant, memory-pressure, or denial-of-
  service stress evidence exists.
- Mandatory target evidence covers the host and thumbv7em hard-float profiles;
  other profiles retain their advisory/documented-only labels.
- Panic audits and tests are not formal proof of panic freedom.
- Supply-chain vulnerability/license scanning and enforced artifact-size
  thresholds remain outside the corrective baseline.
- At the S1 inventory snapshot, RFC 019 package construction, clean extraction,
  tagged-revision evidence, and release/readiness approval were incomplete.
  Local package and clean-extraction mechanics were subsequently accepted, but
  corrected exact-revision evidence, tagged-revision evidence, and
  release/readiness approval remain pending and fail-closed.

## 8. Historical S2 edit boundary recorded at matrix approval

The following instructions governed the S2 update after the original matrix
was accepted:

1. use identical currency metadata for the then-current S2 snapshot: last
   reconciled release v0.20.0, RFCs 001-018 implemented, RFC 019/020 recovery
   tracked but not shipped;
2. apply apex-owned REC-001 through REC-010 and REC-014 without changing stable
   IDs; retain REC-011 through REC-013 and REC-015 for S3 supporting artifacts;
3. preserve historical milestone facts while clearly separating current state;
4. add the authority/conflict rule and release-local normative paths;
5. retain explicit limitations and recovery No-Go status;
6. leave supporting threat model, READMEs, RFC index, and currency automation to
   S3/S4, but do not mark the apex trio current until those supporting changes
   are consistent.

The later owner-selected `0.20.2` finalization supersedes only item 1's
historical version marker. The apex trio now consistently carries RFC 021's
conditional block, while RFCs 019/020/021 are conditionally staged and the
shipped scope remains RFCs 001-018 until external predicate `P` succeeds.
Exact local/tagged evidence, architecture decisions, owner authorization, and
successful distribution remain required before current-marker activation.
