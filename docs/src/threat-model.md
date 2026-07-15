# Threat Model

This chapter describes the implemented v0.20.0 device, cluster, validation,
telemetry, and gateway boundaries plus the accepted RFC 019/RFC 020 recovery
state. It separates implemented controls from release gates, residual risks,
and future work. It is not a formal security proof, safety certification, or
claim that the repository is release-ready.

Release-local normative sources are
`docs/specs/loeres-requirements-v1.md`,
`docs/specs/loeres-external-design-v1.md`, implemented RFCs under
`rfcs/done/`, and accepted recovery RFCs under `rfcs/accepted/`. If those
sources conflict, affected implementation stops under RFC 020's conflict rule.

## Trust boundaries

- `loeres`, `loeres-backend-static`, and `loeres-device` are the edge boundary:
  `no_std`, no `alloc`, no FFI, no server-runtime types, and no hidden heap.
- `loeres-backend-std` and `loeres-cluster` are server-only and may allocate,
  schedule work, and use optional runtime integrations.
- User problem data, current iterates, validation evidence, gateway responses,
  cancellation state, and observability sinks cross explicit public boundaries.
- `xtask`, CI, and package checks provide repository evidence. They do not run
  in solver processes and do not prove semantic or operational security.

## Implemented device controls

The RFC 006 box/bound-constrained projected-first-order device path provides:

- structural and scalar validation before critical loops;
- finite-value and positive-step-scale rejection for its supported scalar
  boundary;
- a mandatory maximum-iteration cap and optional constant-iteration mode;
- caller-owned typed workspace with no heap allocation;
- overwrite-on-entry normalization, so workspace remains reusable after
  success, failure, or non-convergence without a separate caller reset;
- structured `SolverError` failure and `Ok(SolveReport)` non-convergence;
- hot-loop numerical-domain checks and bounded indexed storage access.

The current kernel documents its bounds, step-scale, finite-input, and domain
rules, but it is not a general ill-conditioning detector. Determinism evidence
is target-profile-scoped. Constant iteration is not cryptographic constant time,
and panic audits/target builds are not formal proof of panic freedom or WCET.

## Implemented cluster controls

RFC 008 orchestration supplies per-item batch outcomes, iteration/deadline
budgets, cooperative cancellation, and sequential/parallel/async execution.
One malformed or failed item need not collapse unrelated items. Worker-panic
containment applies only with `panic = "unwind"`; `panic = "abort"` terminates
the process.

RFC 016 supplies one dynamic box/bound-constrained projected-first-order
kernel. Structural checks remain mandatory. Policy-governed finite scans may
use explicit trust/cache evidence, but current-iterate scans and hot-loop
numerical-domain checks remain non-skippable. Non-convergence remains a solved
status, not an error.

These controls do not establish broad LP/QP/SOCP support, high-throughput or
large-N capacity, memory-pressure behavior, denial-of-service resistance, or
multi-tenant isolation. Deployments must add service-level request, memory,
concurrency, queue, identity, authorization, and tenant-isolation controls.

## Validation evidence and cache lifecycle

RFC 012 defines validation scope/trust vocabulary. RFC 015 adds a process-local,
model-carrier-scoped validation evidence cache for the supported `f64` cluster
PFO path.

- Evidence is keyed by model identity, mutation epoch, solver/problem/scalar
  family, and scope.
- Mutation advances the epoch before user mutation code runs, including error
  and unwind paths, so prior evidence becomes stale.
- Wrong identity, stale epoch, sentinel/non-cacheable identity, and invalid
  reusable-cache evidence such as trusted/domain-inapplicable evidence fail
  closed.
- A cache miss or insufficient scope is not trusted: the implementation scans
  the missing eligible model-owned data and continues only if validation
  succeeds. Invalid scanned data produces the normal structured error.
- Cache hits may avoid eligible model-owned finite rescans only. They never
  suppress current-iterate, workspace/config, cancellation, step-scale, or
  hot-loop numerical-domain checks.
- The cache is in-process and non-persistent. It is not distributed, durable,
  cross-process, or a tenant-isolation mechanism.

## Telemetry and disclosure

RFC 009 observation is metadata-only. Events use bounded solver/problem IDs,
dimension/iteration/elapsed buckets, outcome categories, and static redacted
labels through explicit observer sinks. Baseline telemetry excludes model
values and tenant identifiers.

Callers still control their sink, surrounding spans/logs, and application
metadata. Redacted Loeres events do not prove that a host application cannot
leak tenant or model data. Public errors and compact diagnostics likewise avoid
string-rich model disclosure, but service error handling must not expose
untrusted panic text or surrounding application state.

## Gateway and FFI boundary

The shipped gateway is a pure-Rust `MockGatewayJob` plus safe policy/failure/
thread-safety categories. No concrete native solver adapter ships.
`ffi-gateway` is default-off and does not by itself activate or validate a
native integration.

Any future concrete adapter requires a separately reviewed contract covering
memory ownership and lifetimes, thread safety, native panic/exception/failure
translation, cancellation, licensing and distribution, version compatibility,
untrusted outputs, unsafe-code review, and containment of the expanded trusted
computing base. FFI remains forbidden in core/static/device crates.

## Repository and release gates

The developer aggregate checks dependency isolation, edge target builds,
feature profiles, RFC/index links, public APIs, panic-prone source patterns,
unsafe markers, bounded conformance fixtures, and documentation links. Mandatory
host/hard-float target evidence is distinct from advisory-installed soft-float
and RISC-V profiles and documented-only WASM/AArch64 profiles.

These source scans can miss generated code, build-script behavior, external
tool behavior, runtime resource exhaustion, and semantic defects. Size evidence
is advisory until owner RFCs freeze thresholds. Dependency vulnerability and
license-policy scanning are not enforced in the corrective baseline.

RFC 019's package/readiness gate is distinct from the developer aggregate and
remains intentionally fail-closed until integrated documentation, preflight,
tagged-revision, clean-extraction, retained-evidence, and approval requirements
are met. The repository remains No-Go for release/readiness claims.

## Residual risks and future work

- Broad throughput, large-N, multi-tenant, memory-pressure, cancellation-latency,
  and denial-of-service stress evidence is absent.
- Supply-chain vulnerability/license checks and enforced artifact/
  monomorphization thresholds remain future assurance work.
- Extended/adversarial, ill-conditioned, property/fuzz, objective, and residual
  conformance coverage remains limited or absent.
- Advisory/documented-only target profiles are not mandatory tested support.
- A real FFI adapter would materially expand the trusted computing base.
- No dedicated safety-critical engineering-use disclaimer currently exists.
  OQ-012 therefore remains open; Apache-2.0 warranty language is not treated as
  an engineering-use or deployment-suitability assessment.

Users are responsible for system-level hazard analysis, independent validation,
safe fallback behavior, watchdogs, resource limits, and regulatory or assurance
activities appropriate to their deployment. Loeres must not be treated as
certified for safety-critical control merely because its device path is bounded
and panic-averse.
