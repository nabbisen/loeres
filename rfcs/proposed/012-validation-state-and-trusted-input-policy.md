# RFC 012 — Validation State and Trusted Input Policy

**Status.** Proposed
**Tracks.** Cross-cutting input validation, trusted-pipeline responsibility transfer, and fail-safe boundary semantics
**Touches.** `loeres/src/validation.rs`, public solve entrypoints, `loeres-device` boundary APIs, `loeres-cluster` ingestion APIs, conformance fixtures

---

### Extended Metadata
* **Rust Edition Compliance:** Rust 2024 Baseline
* **Target Environment:** Shared policy for `loeres`, `loeres-device`, `loeres-cluster`, `loeres-backend-static`, and `loeres-backend-std`

## 1. Executive Summary & Problem Statement

Loeres must validate public inputs before solver hot paths, but full validation can be expensive. Scanning every scalar in a large sparse cluster model before every solve may be unacceptable, while skipping validation silently would violate fail-safe semantics.

This RFC defines validation state as an explicit public contract. It introduces a responsibility-transfer model that lets callers reuse validation results or assert trusted pipeline boundaries without hiding the risk.

The design goal is to make validation status visible in API shape, logs, diagnostics, and conformance tests.

## 2. Architectural Context & Dependency Alignment

Validation-state types live in `loeres` and must be allocation-free. Cluster-side ingestion may attach richer host diagnostics, but the core state model must remain usable by device code.

Dependency alignment:

| Crate | Relationship to this RFC | Dependency impact |
|---|---|---|
| `loeres` | Owns validation state markers and compact categories | No `std`, no `alloc` |
| `loeres-backend-static` | Provides static storage validation traversal | No `std`, no `alloc` |
| `loeres-device` | Requires validated or explicitly trusted inputs before solving | No `std`, no `alloc` |
| `loeres-backend-std` | Provides dynamic/sparse validation traversal | `std` allowed |
| `loeres-cluster` | May support trusted ingestion pipelines and cached validation | `std` allowed |

## 3. Concrete Technical Specification

### 3.1 Validation state categories

The public model must distinguish at least these states:

| State | Meaning | Allowed solve behavior |
|---|---|---|
| `Unvalidated` | No boundary scan or trusted assertion has been performed | Public solve must validate or reject |
| `Validated` | Loeres validation has checked required invariants | Solver may enter hot path |
| `TrustedByCaller` | Caller explicitly assumes responsibility for invariants | Solver may skip selected validation with visible responsibility transfer |
| `TrustedPipeline` | A named upstream pipeline has validated and preserved invariants | Solver may skip selected validation if pipeline token is accepted |

These may be implemented as marker types, wrapper structs, or state tokens. A casual boolean flag is forbidden.

### 3.2 Required invariants

The baseline validation contract must cover:

1. dimension compatibility;
2. index bounds for sparse structures;
3. duplicate or unsorted sparse entries when relevant to a backend;
4. finite scalar checks where the scalar family supports non-finite values;
5. workspace/problem compatibility for device solve calls;
6. tolerance and iteration configuration sanity;
7. absence of known unsupported problem features for the selected solver.

Solver-specific RFCs may add additional invariants.

### 3.3 Public API shape

Public solve entrypoints must make validation state visible. Acceptable designs include:

```rust
fn solve_validated<P, W>(problem: Validated<P>, workspace: &mut W, config: SolverConfig) -> SolveResult;

fn solve_with_policy<P, W>(problem: P, validation: ValidationPolicy, workspace: &mut W, config: SolverConfig) -> SolveResult;
```

The exact signatures are deferred to RFCs 005, 006, 007, and 008. However, this RFC requires that skipping validation is never an invisible default.

### 3.4 Trusted responsibility transfer

`TrustedByCaller` and `TrustedPipeline` are not “safe because faster.” They mean responsibility has moved out of the solver boundary.

A trusted state must carry compact evidence:

* an audit category or token identifier;
* the validation scope being skipped;
* whether the trust is valid for dimensions only, finite checks only, sparse structure only, or all baseline checks;
* optional corpus/test tag for CI.

In `loeres`, this evidence must remain allocation-free. `loeres-cluster` may attach richer host-side labels outside core types.

### 3.5 Validation caching

Cluster backends may cache validation results for immutable or versioned model data. A cached validation state is valid only if the model identity and relevant mutation epoch match.

Device backends may use static typing to make parts of validation unnecessary, but must still validate runtime scalar values and configuration values when applicable.

### 3.6 Failure semantics

Validation failure must return structured errors, not panic or silently coerce data.

Validation failures must not partially mutate caller-owned device workspace. If validation requires scratch memory, it must use a separate validation workspace or finish before solver workspace mutation begins.

## 4. Rust Systems-Level Nuances & Memory Safety

Validation-state wrappers must avoid large-by-value model copies. They should wrap references, handles, or lightweight state markers.

A type-state design is preferred where it improves API clarity, but the RFC does not require encoding every validation dimension at the type level. Excessive type-state explosion can harm ergonomics and binary size.

No validation-state design may require `dyn Trait` for device-facing hot paths. Cluster orchestration layers may use dynamic dispatch as allowed by RFC 008.

Trusted states must not require `unsafe` to construct. If a future API uses `unsafe` for a highly specialized bypass, it must be introduced by a successor RFC and must name the exact invariants the caller must uphold.

## 5. Algorithmic & Numerical Fail-Safe Guardrails

Validation must execute before numerical iteration begins unless an explicit trusted state is provided.

Minimum fail-safe behavior:

1. non-finite input returns an invalid-input category;
2. invalid dimensions return dimension-mismatch before workspace mutation;
3. unsupported solver/problem pairing returns unsupported-problem before iteration;
4. invalid tolerance or iteration configuration returns invalid-configuration;
5. trusted validation bypass must still allow the solver to return numerical-domain errors discovered during calculation.

Trusted input is not allowed to suppress runtime numerical-domain checks such as checked division failure.

## 6. Verification, Validation, and CI Gates

Acceptance gates:

1. Unit tests demonstrate that unvalidated public inputs are rejected or validated before solve.
2. Tests show that trusted states are visually explicit in public calls.
3. Device tests prove validation failure does not mutate the typed workspace.
4. Cluster tests demonstrate cached validation invalidates after model mutation.
5. `cargo xtask conformance` runs both fully validated and trusted-pipeline fixtures where applicable.
6. Documentation includes a warning that trusted states transfer responsibility and do not prove correctness.
