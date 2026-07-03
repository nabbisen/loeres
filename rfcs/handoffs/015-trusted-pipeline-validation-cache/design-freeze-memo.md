# RFC 015 - Design-Freeze Memo

**Artifact.** [`rfcs/done/015-trusted-pipeline-validation-cache.md`](../../done/015-trusted-pipeline-validation-cache.md)
**Decision pass.** v1, after RFC 013 v0.18.0 conformance-corpus release.
**Target release.** v0.19.0.
**Scope.** Freeze the trusted-pipeline, validation-cache, model-identity, and
mutation-epoch contract before implementation.

---

## Verdict

RFC 015 is the correct next design item.

The repository now has the prerequisites that were missing when trust/caching
was deferred:

| Prerequisite | Current state | RFC 015 implication |
|---|---|---|
| Core validation vocabulary | RFC 012 implemented `ValidationScope`, `ValidationCoverage`, `TrustedByCaller`, `TrustToken`, and `ValidationState` | RFC 015 must consume this vocabulary rather than inventing a second trust model. |
| Cluster orchestration policy | RFC 008 implemented `ClusterValidationPolicy` but kept `resolve()` pure | RFC 015 must add an explicit evidence channel; policy resolution must not fabricate validation. |
| Typed cluster kernel | RFC 016 implemented `ProjectedFirstOrderSolveRecord { checked_scope, finite }` | RFC 015 has concrete evidence to cache against model identity and epochs. |
| Conformance corpus | RFC 013 implemented enforced smoke fixtures | RFC 015 can define trusted-input fixtures for RFC 013 to add later. |
| Gateway/observability boundary | RFC 009 deferred gateway model identity and cacheability | RFC 015 must define cacheability without leaking raw model data or secrets. |

The design risk is not the cache itself; it is stale trust. RFC 015 must make it
impossible for cached validation evidence to silently apply to a different model,
different mutable state, different solver configuration, or a mutated model
epoch.

## D1 - Ownership and dependency boundary

RFC 015 should be cluster-only for implementation.

| Area | Owner in RFC 015 | Boundary |
|---|---|---|
| Model identity | `loeres-cluster` | `std` allowed; no edge dependency. |
| Mutation epochs | `loeres-cluster` | Attached to mutable model/evidence handles, not core scalar/storage traits. |
| Validation cache | `loeres-cluster` | Host-side cache; may allocate. |
| Core validation vocabulary | `loeres` | Reused unchanged unless a strictly necessary additive core type is justified. |
| Conformance fixtures | RFC 013 consumer | RFC 015 defines fixture semantics; RFC 013 owns fixture file execution. |

Runtime crates outside `loeres-cluster` must not parse cache records or fixture
schema. `loeres-device` and `loeres-backend-static` remain unaffected.

## D2 - Now-slice

The v0.19.0 slice should be intentionally narrow:

1. identify models with a stable in-process model identity;
2. track a mutation epoch for mutable model data;
3. cache model-owned validation evidence only when identity, epoch, solver
   family, problem class, scalar family, and required model scope match;
4. add an explicit provided-evidence path to the typed cluster projected-first-
   order solve path;
5. preserve `ValidateAllInputs` behavior as the always-scanning baseline;
6. keep per-call initial-iterate scans active unless explicitly trusted;
7. keep hot-loop numerical-domain checks non-skippable;
8. add local unit tests and define RFC 013 trusted-input fixture requirements.

Out of scope for v0.19.0:

| Excluded item | Reason |
|---|---|
| Persistent/on-disk cache | Requires serialization, invalidation, and portability policy beyond the immediate stale-trust problem. |
| Distributed cache | Requires cross-process identity and deployment semantics. |
| Device trusted-cache entrypoints | Device APIs already validate inline; no evidence yet that a cache is needed there. |
| Public modeling DSL | RFC 016 deliberately shipped only a trait surface. |
| Gateway native solver adapter cache | RFC 009 gateway adapters can consume the identity policy later, but no native adapter ships now. |

## D3 - Required concepts

RFC 015 should freeze these concepts with explicit names:

| Concept | Required property |
|---|---|
| `ModelIdentity` | Stable for one logical model instance; cheap to copy/compare; not derived from raw model data in logs. |
| `MutationEpoch` | Monotonically changes whenever validation-relevant model data mutates. |
| `ValidationEvidenceKey` | Includes identity, epoch, solver family, problem class, and scalar family. Required scope is part of lookup, not the exact key. |
| `ValidationEvidenceLookup` | Pairs a key with the required model-owned scope for this solve. |
| `CachedValidationEvidence` | Stores scanned model-owned coverage, not a generic "valid" boolean and not reusable caller trust. |
| `ValidationEvidenceCache` | Host-side cache API that returns evidence only on an exact key match. |
| `ProvidedValidationEvidence` | Explicit evidence passed into a solve; absence must not mean trust. |
| `CacheableProjectedFirstOrderProblem<P>` | Loeres-owned identity/epoch carrier for the v0.19.0 projected-first-order slice. |

The RFC should prefer compact newtypes over casual `u64` parameters:

```rust
pub struct ModelIdentity(/* opaque */);
pub struct MutationEpoch(/* monotonic counter */);
pub struct ValidationEvidenceKey { /* identity + epoch + solver/problem metadata */ }
pub struct ValidationEvidenceLookup { /* key + required model scope */ }
pub struct CachedValidationEvidence { /* scanned model-owned coverage */ }
```

The exact field spelling belongs in the RFC text, but the key requirement is
semantic: a cache hit must prove the evidence was produced for the same model
identity and epoch under the same solver/problem/scalar contract, and the cached
model-owned coverage must contain the requested model-owned scope.

## D4 - Mutation epoch policy

Mutation epochs are the central stale-cache guard.

Required rules:

1. creating a model identity starts at a defined epoch;
2. every validation-relevant mutation increments or replaces the epoch;
3. mutations that cannot update the epoch must make the model non-cacheable;
4. cloned models must either get a distinct identity or a clearly shared
   identity/epoch owner;
5. read-only solve calls must not advance the epoch;
6. validation evidence from epoch `n` must never satisfy a solve at epoch `n+1`.

RFC 015 freezes a minimal v0.19.0 carrier:

```rust
pub struct CacheableProjectedFirstOrderProblem<P> {
    // private identity + epoch + inner problem
}
```

The carrier generates identity through a Loeres-owned, process-local monotonic
generator, starts at `MutationEpoch(0)`, does not advance on read-only solve
access, advances on validation-relevant mutation or marks the model
non-cacheable, and gives clones distinct identities. `ModelIdentity(0)` is
reserved; generator exhaustion fails closed with `InternalInvariantViolation` or
creates a non-cacheable carrier. Arbitrary caller-provided identities are not
accepted. Unwrapped user-defined problems remain solveable but non-cacheable.

For v0.19.0 the carrier should be immutable-after-wrap unless implementation
explicitly adds a closure-based `mutate(|inner| ...)` API that advances the
epoch before mutable model access. It must not expose raw `&mut P` without epoch
advancement.

## D5 - Validation evidence semantics

RFC 016 intentionally split evidence into:

```text
checked_scope: ValidationScope
finite: ProjectedFirstOrderFiniteEvidence
```

RFC 015 must preserve that honesty:

| Evidence state | Cache behavior |
|---|---|
| `finite = Scanned` | Cacheable when key matches and model-owned scope is sufficient. |
| `finite = Trusted(..)` | Not inserted into the reusable cache in v0.19.0; accepted only as current-solve evidence. |
| `finite = DomainInapplicable` | Cacheable only for scalar/domain contracts that really make non-finite values impossible. |

`ValidationCoverage::new(..., FiniteCoverage::NotApplicable)` must not be used
to encode "trusted away" finite scans. RFC 016 already corrected this; RFC 015
must not regress it.

Cached evidence may satisfy only model-owned finite data for the identified
model/epoch. The caller's current `x` remains a per-call input and must be
scanned unless covered by explicit `TrustedByCaller` evidence for that solve.

## D6 - Cluster validation-policy integration

RFC 015 should extend policy integration without changing the meaning of the
existing modes:

| Policy/input | Required behavior |
|---|---|
| `ValidateAllInputs` | Run scans and produce fresh evidence. Only the model-owned evidence portion may be cached after success. |
| `RespectBackendValidationState` with no provided evidence and no cache hit | Run scans and continue. |
| `RespectBackendValidationState` with matching sufficient evidence | Use evidence only for skippable model-owned scans; still run per-call/non-skippable checks. |
| `RespectBackendValidationState` with insufficient model-owned scope | Scan missing scopes and continue. |
| `RespectBackendValidationState` with wrong identity or stale epoch | Return `SolverError::InvalidInput`; never treat stale evidence as absence. |
| `TrustedByCaller` | Skip only asserted skippable scopes, with visible `TrustedByCaller` evidence. |

`ClusterValidationPolicy::resolve` should remain pure. The actual validation
scan/cache lookup must live at the typed kernel or model/evidence boundary where
identity, epoch, and required scope are available.

The cached/evidence-aware solve path must accept
`&CacheableProjectedFirstOrderProblem<P>` or a sealed/internal identity-carrier
trait implemented only by that carrier. It must not accept arbitrary `&P` with
provided evidence. The existing RFC 016 `solve_projected_first_order_dyn(&P, ...)`
stays source-compatible and non-cacheable.

The cached/evidence-aware path is `f64` / `ScalarFamilyId::Float64` only in
v0.19.0. Generic non-cacheable solves remain available through the RFC 016 typed
path.

## D7 - Non-skippable checks

RFC 015 must make these checks non-cacheable and non-skippable:

1. cancellation before expensive work;
2. numeric config validity;
3. dimension compatibility among current `x`, workspace, bounds, and problem;
4. finite/positive `step_scale` when it is read from the current model;
5. current initial iterate finite scan, unless separately covered by explicit
   `TrustedByCaller`;
6. hot-loop gradient and candidate finiteness;
7. cancellation polling during iteration.

The reason is simple: cached evidence can only speak for model data it keyed.
It cannot prove that a caller's current mutable iterate, workspace, config, or
future oracle output is valid.

Numeric solve configuration is not part of the cache key in v0.19.0. Tolerance
and max-iteration count are validated every solve; changing them may change
status or solution, but does not invalidate model-owned finite evidence by
itself.

## D8 - Error mapping

The RFC should use existing `SolverError` variants unless a separate RFC 003
amendment is justified.

Recommended mapping:

| Condition | Error |
|---|---|
| stale identity/epoch evidence | `InvalidInput` |
| provided evidence has insufficient model-owned scope under fallback mode | scan missing scopes and continue |
| cache entry claims impossible finite semantics for the scalar family | `InvalidInput` |
| unsupported scalar family for cached/evidence-aware path | `UnsupportedProblemStructure` |
| cache backend/internal invariant failure | `InternalInvariantViolation` |
| non-finite discovered while scanning | `NonFiniteInput` |
| non-finite discovered during iteration despite trust/cache | `NumericalDomain` |

The design should avoid a new "stale cache" error unless review finds that the
existing taxonomy cannot express the boundary safely.

## D9 - RFC 013 conformance handoff

RFC 015 must define the trusted-input fixtures that RFC 013 can add after the
policy ships.

Minimum future fixture classes:

| Fixture class | Expected result |
|---|---|
| matching cached evidence | Same status/solution as `ValidateAllInputs`; model-owned finite scan may be skipped while per-call checks remain active. |
| stale epoch evidence | Structured failure, not solved. |
| wrong model identity | Structured failure, not solved. |
| trusted finite bypass with NaN produced in hot loop | `SolverError::NumericalDomain`, not solved. |
| insufficient model-owned scope | Missing scopes are scanned and the solve proceeds if scans pass. |

The fixture schema must keep `validation_state = "validate-all-inputs"` for the
existing smoke suite and add explicit states for cached/trusted evidence only
after RFC 015 lands.

## D10 - Observability and gateway notes

RFC 009's redaction rule still applies. Validation cache observability may emit
bounded metadata only:

| Allowed | Forbidden |
|---|---|
| cache hit/miss/stale category | raw model data |
| solver family/problem class | raw vectors, matrices, objective values |
| scope category | caller secrets, tenant IDs, request IDs |
| identity category/token class | full opaque identity bytes if they can correlate sensitive models |

Gateway/native adapter caches remain out of the v0.19.0 implementation slice,
but RFC 015 should give them the identity/epoch vocabulary they will consume.

## D11 - Acceptance gates

Before implementation starts, RFC 015 text should define acceptance gates for:

1. no runtime crate outside `loeres-cluster` gains cache/parser dependencies;
2. stale identity and stale epoch tests fail closed with `InvalidInput`;
3. matching evidence can skip only model-owned finite scans without changing
   solution/status semantics;
4. `ValidateAllInputs` still scans and remains the baseline behavior;
5. trusted/cached evidence never suppresses hot-loop `NumericalDomain`;
6. `ClusterValidationPolicy::resolve` remains pure;
7. `TrustedByCaller` evidence is not inserted into the reusable cache;
8. mutable carrier operations advance the epoch or mark the model non-cacheable;
9. unwrapped user-defined problems remain solveable but non-cacheable;
10. the cached/evidence-aware solve path accepts the cacheable carrier, not
   arbitrary `&P`;
11. `ValidationEvidenceCache::insert` returns `Result<(), SolverError>` and
   rejects non-cacheable evidence;
12. model identity generation is Loeres-owned, monotonic, process-local, and
   fails closed on exhaustion;
13. cached/evidence-aware solving is `f64` / `ScalarFamilyId::Float64` only in
   v0.19.0;
14. `cargo xtask conformance` remains green, with new trusted fixtures added only
   if RFC 015 implements their full semantics;
15. `cargo xtask check`, `cargo xtask release-gate`, clippy, tests, RFC checks,
   and link audit pass on the working tree and clean extraction.

## Required RFC 015 Text Patch

Patch `rfcs/done/015-trusted-pipeline-validation-cache.md` before coding.
The RFC text should:

1. state the v0.19.0 now-slice and explicit non-goals;
2. define model identity, mutation epochs, evidence keys, cached evidence, and
   provided evidence;
3. freeze how provided/cache evidence integrates with the RFC 016 typed
   projected-first-order entrypoint;
4. preserve `ValidateAllInputs` as the default scanning baseline;
5. specify stale-cache fail-closed behavior and error mapping;
6. preserve non-skippable structural and hot-loop checks;
7. define the RFC 013 trusted-input conformance handoff;
8. keep observability metadata redacted and bounded.

## Review-Settled Decisions

1. Missing or insufficient model-owned scope under
   `RespectBackendValidationState` scans and continues; wrong identity or wrong
   epoch rejects with `SolverError::InvalidInput`.
2. `ModelIdentity` is generated by a Loeres-owned wrapper/carrier. Arbitrary
   caller-provided identity values are not accepted in v0.19.0.
3. No core `TrustKind` variant is added in v0.19.0.
4. Cache replacement/eviction is caller-owned. The cache provides `new`, `get`,
   `insert`, `invalidate_model`, and `clear`; there is no global cache, LRU,
   TTL, or size-bound policy in this slice.
5. Stable local IDs are `SolverFamilyId::ProjectedFirstOrder`,
   `ProblemClassId::BoxBoundFirstOrder`, and `ScalarFamilyId::Float64`.
6. The cached/evidence-aware entrypoint accepts the cacheable carrier, not
   arbitrary unwrapped problems.
7. `ValidationEvidenceCache::insert` is fallible and rejects non-cacheable
   evidence with `SolverError::InvalidInput`.
8. Identity generation is Loeres-owned, monotonic, process-local, reserves
   `ModelIdentity(0)`, and fails closed on exhaustion.
9. The v0.19.0 cached/evidence-aware path is `f64` only.
10. The carrier is immutable-after-wrap unless implementation explicitly adds a
   closure-based mutation API that advances the epoch before mutable model
   access.

## Non-Goals

This memo does not implement RFC 015, change code, or alter release status. It
records the design-freeze and review-settled questions that the RFC 015 proposed
text must answer before implementation proceeds.
