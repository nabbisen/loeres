# RFC 012 — Validation State and Trusted Input Policy

**Status.** Implemented (v0.12.0). Core-first `loeres::validation` vocabulary (state categories, coverage descriptor, allocation-free trusted-by-caller evidence); cluster trusted-pipeline / caching deferred to RFC 008, shared conformance corpus to RFC 013.
**Tracks.** Cross-cutting input validation and fail-safe boundary semantics — core validation-state vocabulary (this RFC); cluster trusted-pipeline / caching deferred to RFC 008; shared conformance corpus deferred to RFC 013.
**Touches.** `loeres/src/validation.rs` (splitting to `loeres/src/validation/{state,evidence,scope}.rs` if it crosses the 300-ELOC threshold). Device / cluster / conformance integration is **out of this RFC's now-slice** — RFC 012 formalizes the *existing* device inline checks without changing shipped signatures, and names the later owners for the deferred surface.

---

### Extended Metadata
* **Rust Edition Compliance:** Rust 2024 Baseline
* **Target Environment:** Shared policy for `loeres`, `loeres-device`, `loeres-cluster`, `loeres-backend-static`, and `loeres-backend-std`

## 1. Executive Summary & Problem Statement

Loeres must validate public inputs before solver hot paths, but full validation can be expensive. Scanning every scalar in a large model before every solve may be unacceptable, while skipping validation silently would violate fail-safe semantics.

This RFC defines validation state as an explicit public contract **in `loeres` core**: an allocation-free vocabulary of validation-state categories and a coverage/evidence model that makes validation status visible in API shape, logs, and diagnostics. It introduces a caller-side responsibility-transfer category (`TrustedByCaller`) that lets callers assert trusted boundaries without hiding the risk.

**Scope (F1, core-first).** This RFC owns the core vocabulary now: the `loeres::validation` module, the state categories, the invariant-coverage model, and allocation-free trusted-by-caller evidence, with local tests. It does **not** implement cluster trusted-pipeline mechanics, validation caching, model identity, or mutation epochs (RFC 008-owned), nor the shared conformance corpus / `xtask conformance` fixtures (RFC 013-owned). Construction-time structural checks remain owned by the storage constructors (RFC 004 / RFC 007) and are not re-performed by this state model.

## 2. Architectural Context & Dependency Alignment

Validation-state types live in `loeres` and must be allocation-free. Cluster-side ingestion may attach richer host diagnostics, but the core state model must remain usable by device code.

Dependency alignment:

| Crate | Relationship to this RFC | Dependency impact |
|---|---|---|
| `loeres` | Owns the validation-state vocabulary, coverage model, and trusted-by-caller evidence (this RFC's now-slice) | No `std`, no `alloc` |
| `loeres-backend-static` | Construction-time structural validity (RFC 004); finite scan where `FiniteScalar` | No `std`, no `alloc` |
| `loeres-device` | Existing inline pre-iteration checks (RFC 006) are formalized as satisfying the contract; **no signature change** (F3c) | No `std`, no `alloc` |
| `loeres-backend-std` | Construction-time structural validity + `validate_finite` (RFC 007) | `std` allowed |
| `loeres-cluster` | Trusted-pipeline mechanics, caching, model identity, mutation epochs — **deferred to RFC 008** | `std` allowed (later) |

The now-slice is the `loeres::validation` vocabulary plus the rule that construction-time structural checks (dimensions, sparse coordinate bounds, duplicate rejection) are owned by the storage constructors and are a precondition, not a re-scan. Cluster trusted-pipeline / caching (RFC 008) and the shared conformance corpus (RFC 013) consume this vocabulary later.

## 3. Concrete Technical Specification

### 3.1 Validation state categories

The core model distinguishes these states. **Structural validity is a construction precondition (F2), not a state**: RFC 004 / RFC 007 constructors already enforce dimensions, sparse coordinate bounds, and duplicate rejection, so a constructed adapter cannot be structurally invalid. Validation state therefore covers the *remaining* runtime / semantic checks (finite scans, problem/config pairing, solver-family pre-loop invariants) and responsibility transfer.

| State | Meaning | Allowed solve behavior |
|---|---|---|
| `Unvalidated` | The remaining runtime / semantic checks have not been performed (structure is already construction-guaranteed) | Public solve must validate or reject before the hot path |
| `Validated` | Loeres has checked the applicable remaining invariants; the state carries a coverage descriptor (§3.4) recording, per invariant, checked vs not-applicable | Solver may enter the hot path |
| `TrustedByCaller` | Caller explicitly assumes responsibility for a named coverage scope | Solver may skip the asserted checks with visible responsibility transfer |

`TrustedPipeline` (a named upstream pipeline asserting trust via a host-side token) is **deferred to RFC 008** and is not part of this RFC's now-slice; the core evidence primitives (§3.4) are designed so RFC 008 can add it without a core migration.

These are implemented as marker / wrapper types carrying the coverage descriptor, never a casual boolean flag.

### 3.2 Invariant ownership

The baseline invariants split by owner. **Construction-time invariants** are enforced by the storage constructors (RFC 004 / RFC 007) and are a precondition; this RFC's state model does not re-scan them:

1. dimension compatibility;
2. index bounds for sparse structures;
3. duplicate / unsorted sparse entries (rejected at construction by the relevant backend).

**Validation-state invariants** are the remaining runtime / semantic checks this RFC's `Validated` state covers (each recorded in the coverage descriptor as checked or not-applicable):

4. finite-scalar checks where `S: FiniteScalar` (not-applicable for scalar families without non-finite values — see §3.7);
5. workspace / problem compatibility for device solve calls (already performed inline by RFC 006; formalized here, not re-implemented);
6. tolerance and iteration configuration sanity (already performed by `DeviceSolveConfig::validate`, RFC 005);
7. absence of known unsupported problem features for the selected solver.

Solver-specific RFCs may add additional validation-state invariants. The coverage descriptor (§3.4) records which of (4)–(7) were checked, not-applicable, or asserted under `TrustedByCaller`.

### 3.3 Public API shape (F3c — internal formalization, no retrofit)

The shipped RFC 006 device entrypoint `solve_projected_first_order` already validates its inputs inline before iteration (`config.validate`, `problem.validate_boundary`, finite checks). RFC 012 **formalizes those existing checks as satisfying the validation contract internally**; it does **not** change that signature and does **not** add a required `Validated<P>` parameter to the shipped solver. The v0.10.x / v0.11.x device surface is preserved.

The core principle still holds: **skipping validation is never an invisible default.** Where a future, additive device RFC introduces a parallel validated/trusted entrypoint (only on a measured need to skip repeated checks), it would consume this RFC's vocabulary — for example accepting a `TrustedByCaller` coverage scope — and make the responsibility transfer visible in the signature:

```rust
// Illustrative future additive entrypoint (NOT introduced by this RFC):
// fn solve_*_trusted<P, W>(problem: P, trust: TrustedByCaller, workspace: &mut W, config: ...) -> ...;
```

This RFC's deliverable is the `loeres::validation` vocabulary those entrypoints would use, not a change to any existing solver signature.

### 3.4 Coverage descriptor and trusted-by-caller evidence (F6)

Both `Validated` and `TrustedByCaller` carry an allocation-free **coverage descriptor** recording, per validation-state invariant, whether it was checked, not-applicable, or asserted. The core representation is compact and `Copy` — no heap, no `String`, no `Vec`:

- a **scope bitset** — a transparent integer newtype (`ValidationScope(u8)`) with one bit per coverage dimension: finite values, problem/config pairing, and solver-family pre-loop invariants. An `ALL` constant denotes *all validation dimensions known to this RFC's surface / release* (R1) — not a forever-complete claim; later RFCs adding dimensions redefine `ALL` for their release. (Structural dimensions / bounds are construction-owned and are not bits here.)
- a **finite-coverage variant** — `FiniteCoverage::{Checked, NotApplicable}` (§3.7) — so a `Validated` value is never ambiguous about whether a finite scan ran. Trusted bypass and unavailable are not `FiniteCoverage` variants (§3.7).
- for `TrustedByCaller`: a `#[non_exhaustive]` `TrustKind` enum carrying `CallerAssertion` only — RFC 008 pipeline-trust categories are added later, not reserved as a concrete variant now (R2) — plus a compact **numeric audit token** (an integer category, not a string).
- an optional `&'static str` label, only where a static tag is useful; never an owned `String`.

`TrustedByCaller` is not "safe because faster": it means responsibility for the asserted scope has moved out of the solver boundary, and the asserted scope is visible in the evidence. In `loeres`, this evidence remains allocation-free; `loeres-cluster` (RFC 008) may attach richer host-side labels outside the core types.

### 3.5 Validation caching — deferred to RFC 008 (non-normative note)

Validation caching is **out of this RFC's now-slice**. Cluster backends may later cache validation results for immutable or versioned model data, where a cached state is valid only if the model identity and relevant mutation epoch match — but model identity, mutation epochs, and cache behavior are **RFC 008-owned** and are neither specified nor implemented here. This RFC only provides the allocation-free coverage descriptor (§3.4) that such a cache would carry.

(The device observation is in-slice: device backends may use static typing to make parts of validation unnecessary, but must still validate runtime scalar values and configuration when applicable — which RFC 006 already does inline.)

### 3.6 Failure semantics

Validation failure must return structured errors, not panic or silently coerce data.

Validation failures must not partially mutate caller-owned device workspace. If validation requires scratch memory, it must use a separate validation workspace or finish before solver workspace mutation begins.

### 3.7 Finite checking and not-applicable (F7, P1)

Finite scans require `S: FiniteScalar`. The finite invariant is **not-applicable** only when the scalar/domain is **explicitly known to be non-finite-incapable** by its type/domain contract — *not* merely because a `FiniteScalar` impl is absent. A missing `FiniteScalar` impl is an *unavailable validation capability*, not proof that non-finite values cannot occur; treating it as not-applicable would let an unscannable scalar silently look validated.

If a solver requires finite validation and the scalar can prove neither *finite-checked* nor *finite-not-applicable*, the path must reject with a structured error or require explicit `TrustedByCaller` evidence.

The four conditions are distinct and must not be conflated:

- *finite-checked* — a finite scan ran (`S: FiniteScalar`) and passed;
- *finite-not-applicable* — non-finite values are impossible by the scalar's domain/type contract, so no scan is required;
- *trusted bypass* — the caller asserted responsibility for the finite scope under `TrustedByCaller`;
- *unavailable* — finite validation capability is absent for this scalar; this is **not** validated and **not** not-applicable.

`Validated` records *finite-checked* or *finite-not-applicable* only (via the finite-coverage variant in the coverage descriptor, §3.4); *trusted bypass* lives in `TrustedByCaller`; *unavailable* is rejected rather than admitted as `Validated`.

## 4. Rust Systems-Level Nuances & Memory Safety

Validation-state wrappers must avoid large-by-value model copies. They should wrap references, handles, or lightweight state markers. The coverage descriptor (§3.4) is a `Copy` value built from a transparent integer-newtype scope bitset and small enums — no heap allocation, no `String`, no `Vec` — so it composes into device-facing markers without binary-size or `no_alloc` cost.

A type-state design is preferred where it improves API clarity, but the RFC does not require encoding every validation dimension at the type level. Excessive type-state explosion can harm ergonomics and binary size.

No validation-state design may require `dyn Trait` for device-facing hot paths. Cluster orchestration layers may use dynamic dispatch as allowed by RFC 008.

Trusted states must not require `unsafe` to construct. If a future API uses `unsafe` for a highly specialized bypass, it must be introduced by a successor RFC and must name the exact invariants the caller must uphold.

## 5. Algorithmic & Numerical Fail-Safe Guardrails

Validation must execute before numerical iteration begins unless an explicit trusted state is provided.

Minimum fail-safe behavior, mapped to the existing `SolverError` topology (RFC 003 — RFC 012 introduces no new category):

1. non-finite input → `SolverError::NonFiniteInput`;
2. invalid dimensions / shape disagreement → `SolverError::InvalidDimension` or `SolverError::DimensionMismatch`, per the owning constructor/access contract, before workspace mutation;
3. unsupported solver/problem pairing → `SolverError::UnsupportedProblemStructure` before iteration;
4. invalid tolerance / iteration configuration → the existing RFC 005 mapping (`SolverError::InvalidInput` / `NonFiniteInput` / `InvalidDimension` per the specific failure) — **not** a new "invalid-configuration" category;
5. trusted validation bypass must still allow the solver to return `SolverError::NumericalDomain` errors discovered during calculation.

Trusted input is not allowed to suppress runtime numerical-domain checks such as checked-division failure. Any new error category would require an explicit RFC 003 amendment, which this RFC does not make.

## 6. Verification, Validation, and CI Gates

Acceptance gates are **RFC-012-local** (F5). The shared cross-backend validation corpus and `cargo xtask conformance` fixtures remain **RFC 013-owned** and are not a gate for this RFC.

1. Tests cover state construction and transition APIs (`Unvalidated` → `Validated` / `TrustedByCaller`).
2. Tests cover the allocation-free evidence types (scope bitset, finite tri-state, trust kind/token) — `Copy`, no heap.
3. Tests show `TrustedByCaller` construction is visible in API shape and carries its asserted scope.
4. Tests cover `Validated`'s finite-checked vs finite-not-applicable representation (§3.7).
5. Tests show validation failure returns structured errors and does not leave validation state in an incorrect partial form.
6. The core types build under `no_std` / no-`alloc` (verified on `thumbv7em-none-eabihf`); documentation warns that trusted states transfer responsibility and do not prove correctness.

Cluster cache-invalidation tests and the device-workspace-no-mutation conformance fixtures are deferred to their owning RFCs (008 / 013); the device no-partial-mutation property itself is already upheld by RFC 006's pre-iteration checks and is restated in §3.6.
