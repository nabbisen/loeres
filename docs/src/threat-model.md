# Threat Model

This chapter consolidates the design-level threat model from the requirements
specification (§8) and the external design (§5). It is a design artifact: Loeres
is in early implementation — `loeres` ships the error/diagnostic topology,
the solver outcome/status taxonomy, and the scalar capability contracts, while
the access contracts and all solver engines remain in design or ahead on the
roadmap. The model therefore still describes the controls the implementation
must uphold rather than an assessment of a running solver.

## Boundary validation principle

Public solve entrypoints must validate all externally supplied problem data
before entering private calculation loops. The consequences differ by side:
cluster validation protects multi-tenant availability, confidentiality, and host
resources; device validation protects control-loop availability, bounded
execution, memory integrity, and panic-averse behavior.

## Fail-safe semantics

On invalid or adversarial input, Loeres fails closed: no panic for expected
invalid input, no infinite loop, no hidden allocation on the device path, no
silent dimension truncation, no unchecked division by zero, and no best-effort
solve after validation has identified unsupported structure. Failures are
reported as structured errors. After a failed device solve, the caller-owned
workspace is treated as reset-required unless the solver documents otherwise.

## Server (cluster) threats

| Threat | Design response |
|---|---|
| Memory exhaustion | Batch and model-size limits; allocation-failure reporting; service policy. |
| CPU exhaustion | Execution budgets, cancellation, timeouts, batch fairness. |
| FFI compromise | FFI is cluster-only, default-off, behind an audited feature gate. |
| Cross-tenant leakage | No global mutable solver state by default; observability redaction by RFC. |
| Non-finite / ill-conditioned input | Boundary validation; structured rejection; conditioning policy. |
| Bad item inside a batch | Per-item failure outcome by default, not whole-batch failure. |
| Repeated expensive validation | Explicit validated / trusted input state; never implicit skipping. |

## Device (edge) threats

| Threat | Design response |
|---|---|
| Infinite-loop trigger | Mandatory maximum-iteration cap. |
| Heap exhaustion | No heap allocation. |
| Stack misuse via large copies | Borrowed APIs and explicit `&mut` workspace lifecycle. |
| Buffer overflow | Fallible access contracts and bounded storage wrappers. |
| Division-by-zero | Scalar capability stratification and guarded domain validation. |
| Timing manipulation | Bounded mode and optional constant-iteration mode (not a constant-time claim). |
| Non-finite values | Finite validation where the scalar family supports it. |
| Unsupported structure | Structured rejection before the calculation loop. |

## FFI policy

`loeres`, `loeres-backend-static`, and `loeres-device` must not use FFI.
`loeres-backend-std` and `loeres-cluster` may use FFI only behind explicit,
default-off feature gates, with documented memory ownership, thread-safety, and
failure behavior. FFI results are normalized into Loeres structured
result/error categories at the cluster boundary, and FFI types never appear in
core contracts.

## Disclosure

The public error topology must carry enough information for a safe response
without leaking internal solver invariants. The cluster side may expose richer
diagnostics and observability subject to redaction policy; the device side
exposes only compact, data-oriented diagnostics with no strings or logging.
