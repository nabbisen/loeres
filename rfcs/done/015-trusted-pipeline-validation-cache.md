# RFC 015 — Trusted Pipeline Validation Cache

**Status.** Implemented (v0.19.0) — `loeres-cluster` now owns the cluster-only validation evidence cache, Loeres-generated model identity and mutation epochs, `CacheableProjectedFirstOrderProblem`, and a carrier-only cached projected-first-order solve path for `f64`; final v0.19.0 includes the implementation-review correction that mutation epochs advance before mutable model access.
**Tracks.** Phase 3 / Milestone 3 — trusted cluster validation, cacheability, model identity, and mutation epochs
**Touches.** `loeres-cluster/src/validation_cache.rs` (new), `loeres-cluster/src/model.rs`, `loeres-cluster/src/solve/projected_first_order.rs`, `loeres-cluster/src/runtime.rs`, RFC 013 conformance fixture taxonomy

---

### Extended Metadata
* **Rust Edition Compliance:** Rust 2024 Baseline
* **Target Environment:** `loeres-cluster` (`std`); consumes `loeres` validation vocabulary and the RFC 016 typed projected-first-order cluster kernel. Cluster-only: no edge or `no_std` crate may depend on this cache surface.

## 1. Executive Summary & Problem Statement

Loeres now has a real cluster solver and a real conformance smoke corpus. The
remaining validation gap is repeated validation cost: cluster callers may solve
the same model many times with different iterates, or feed models through a
known upstream validation pipeline. Re-scanning all model data before every solve
can be wasteful, but silently trusting stale data would violate the fail-safe
boundary.

This RFC defines a narrow trusted validation cache for `loeres-cluster`:

1. model identity;
2. mutation epochs;
3. cache keys for validation evidence;
4. provided/cached evidence semantics;
5. integration with the RFC 016 typed projected-first-order kernel;
6. conformance handoff for trusted-input fixtures.

The central rule is: **cached validation evidence is useful only for
model-owned data bound to the same model identity, same mutation epoch, same
solver/problem contract, same scalar family, and sufficient model-owned
validation scope.** A cache entry that cannot prove those facts is not trusted.
It does not validate the caller's current iterate, workspace, numeric config, or
future oracle output.

## 2. Architectural Context & Dependency Alignment

This RFC consumes shipped contracts:

| Contract | Use in RFC 015 |
|---|---|
| RFC 012 validation vocabulary | Reuse `ValidationScope`, `TrustedByCaller`, `TrustToken`, and `ValidationState`; do not create a second trust model. |
| RFC 008 cluster policy | Preserve `ClusterValidationPolicy` meaning and keep policy resolution pure. |
| RFC 016 typed kernel | Cache the evidence represented by `ProjectedFirstOrderSolveRecord { checked_scope, finite }`. |
| RFC 013 conformance corpus | Define trusted-input fixture classes for later conformance expansion. |
| RFC 009 observability/gateway boundary | Keep cache metadata bounded and redacted; do not expose raw model data. |

Dependency alignment:

| Crate | Relationship to this RFC | Dependency impact |
|---|---|---|
| `loeres-cluster` | Owns model identity, mutation epochs, validation cache, and typed-kernel integration | `std` allowed |
| `loeres` | Provides validation vocabulary and `SolverError`; no new dependency required in the default design | No `std`, no `alloc` impact |
| `loeres-backend-std` | Provides dense storage used by the current cluster kernel | No cache dependency |
| `loeres-device` | Out of scope; keeps inline validation | No impact |
| `loeres-backend-static` | Out of scope | No impact |
| `xtask conformance` | Future consumer for trusted-input fixture execution | Host-only |

## 3. Concrete Technical Specification

### 3.1 v0.19.0 scope

The v0.19.0 implementation slice is cluster-only and intentionally narrow:

1. add cache/identity/epoch types in `loeres-cluster`;
2. support provided cached evidence for model-owned finite data in the typed
   projected-first-order cluster solve path;
3. preserve `ValidateAllInputs` as the always-scanning baseline;
4. make stale identity/epoch evidence fail closed;
5. make missing or insufficient model-owned evidence fall back to scanning under
   `RespectBackendValidationState`;
6. keep per-call iterate scans active unless the caller supplies explicit
   `TrustedByCaller` evidence for that finite scope;
7. keep hot-loop numerical-domain checks non-skippable;
8. define RFC 013 trusted-input fixture classes.

Excluded from v0.19.0:

| Excluded item | Reason |
|---|---|
| Persistent/on-disk cache | Requires serialization, portability, and invalidation policy beyond the immediate stale-trust problem. |
| Distributed cache | Requires cross-process identity and deployment semantics. |
| Device trusted-cache entrypoints | Device APIs already validate inline and are allocation-free. |
| Public modeling DSL | RFC 016 intentionally shipped only a trait surface. |
| Gateway/native solver adapter cache | RFC 009 adapters can consume the policy later; no native adapter ships now. |

### 3.2 Model identity and mutation epochs

`loeres-cluster` should define compact, copyable identity types:

```rust
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct ModelIdentity(u64);

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct MutationEpoch(u64);
```

`ModelIdentity` is generated by a Loeres-owned, process-local monotonic
generator used by cacheable carriers. It is not freely supplied as an arbitrary
caller string. This avoids accidental identity collisions and keeps raw model
data out of logs. The identity is an in-process cache key, not a content hash and
not a distributed identity.

`ModelIdentity(0)` is reserved for a sentinel/non-cacheable value. If the
generator exhausts, carrier creation must fail with
`SolverError::InternalInvariantViolation` or create a non-cacheable carrier. It
must never wrap around into an existing identity.

Mutation rules:

1. a new identity starts at a defined epoch, initially `MutationEpoch(0)`;
2. every validation-relevant mutation advances the epoch;
3. a mutation path that cannot advance the epoch makes the model non-cacheable;
4. cloned models get a distinct identity unless they explicitly share one epoch
   owner;
5. read-only solve calls do not advance the epoch;
6. evidence from epoch `n` must never satisfy a solve at epoch `n + 1`.

The initial v0.19.0 cacheable model target is the typed projected-first-order
problem surface used by RFC 016 integration tests and examples. Arbitrary
user-defined trait objects are allowed to solve, but they are cacheable only
when wrapped in a Loeres identity/epoch carrier.

RFC 015 freezes the existence of this minimal carrier:

```rust
pub struct CacheableProjectedFirstOrderProblem<P> {
    // private identity + epoch + inner problem
}
```

Required carrier semantics:

1. identity is generated by Loeres, not supplied as a caller string;
2. epoch starts at `MutationEpoch(0)`;
3. read-only solve access does not advance the epoch;
4. v0.19.0 may choose immutable-after-wrap and expose no mutation API; if
   mutation is exposed, it must be closure-based, e.g. `mutate(|inner| ...)`,
   and advance the epoch before mutable model access;
5. the carrier must not expose `&mut P` directly without epoch advancement;
6. validation-relevant mutation must go through the carrier and advance the
   epoch, or the carrier must mark the model non-cacheable;
7. default clone semantics give the clone a distinct identity; shared-epoch
   clones are deferred unless introduced explicitly by a later RFC;
8. user-defined unwrapped problems remain solveable through existing APIs but
   are non-cacheable.

### 3.3 Evidence key

Validation evidence is addressed by an exact model key plus a separate lookup
requirement. The key identifies the evidence source; the lookup states how much
model-owned coverage the current solve needs.

```rust
pub enum SolverFamilyId {
    ProjectedFirstOrder,
}

pub enum ProblemClassId {
    BoxBoundFirstOrder,
}

pub enum ScalarFamilyId {
    Float64,
}

pub struct ValidationEvidenceKey {
    pub model_identity: ModelIdentity,
    pub mutation_epoch: MutationEpoch,
    pub solver_family: SolverFamilyId,
    pub problem_class: ProblemClassId,
    pub scalar_family: ScalarFamilyId,
}

pub struct ValidationEvidenceLookup {
    pub key: ValidationEvidenceKey,
    pub required_model_scope: ValidationScope,
}
```

These IDs live in `loeres-cluster::validation_cache` for v0.19.0. They are local
bounded IDs, not telemetry labels, though observability may map them to redacted
metadata later.

| Field | v0.19.0 required value |
|---|---|
| solver family | `SolverFamilyId::ProjectedFirstOrder` |
| problem class | `ProblemClassId::BoxBoundFirstOrder` |
| scalar family | `ScalarFamilyId::Float64` only |
| required model scope | model-owned `ValidationScope::FINITE`; structural/problem-config checks remain non-skippable |

A cache hit requires exact identity, exact epoch, same solver family, same
problem class, and same scalar family. Coverage is checked outside the key:
`cached.model_checked_scope.contains(lookup.required_model_scope)`. This allows
one wider cache entry to satisfy narrower model-owned validation requests.

### 3.4 Cached evidence

RFC 016 intentionally represents solve evidence as:

```text
checked_scope: ValidationScope
finite: ProjectedFirstOrderFiniteEvidence
```

RFC 015 keeps that representation honest, but narrows what is reusable. Cached
evidence covers **model-owned** validation only. It does not cover the caller's
current `x`, workspace, numeric config, or future oracle output.

```rust
pub struct CachedValidationEvidence {
    pub model_checked_scope: ValidationScope,
    pub finite: ProjectedFirstOrderFiniteEvidence,
}
```

Rules:

| Evidence state | Cache behavior |
|---|---|
| `finite = Scanned` | Cacheable when the key matches and the model-owned checked scope is sufficient. |
| `finite = Trusted(..)` | Not inserted into `ValidationEvidenceCache` in v0.19.0. Caller trust is accepted only as current-solve evidence. |
| `finite = DomainInapplicable` | Cacheable only for scalar/domain contracts that truly cannot represent non-finite values. It is not emitted for `f64` in v0.19.0. |

`FiniteCoverage::NotApplicable` must never mean "scan was trusted away." RFC
016 already corrected that evidence mistake; RFC 015 preserves the split.
`TrustedByCaller` is a responsibility transfer at the current boundary, not a
reusable objective validation result.

### 3.5 Validation cache API

The host-side cache may allocate and should be a small `std` data structure:

```rust
pub struct ValidationEvidenceCache { /* private */ }

impl ValidationEvidenceCache {
    pub fn new() -> Self;
    pub fn get(&self, lookup: &ValidationEvidenceLookup) -> Option<CachedValidationEvidence>;
    pub fn insert(
        &mut self,
        key: ValidationEvidenceKey,
        evidence: CachedValidationEvidence,
    ) -> Result<(), SolverError>;
    pub fn invalidate_model(&mut self, identity: ModelIdentity);
    pub fn clear(&mut self);
}
```

The cache returns evidence only on an exact key match plus a sufficient
model-owned coverage check. It does not search for "close enough" entries across
identities or epochs. `insert` rejects non-cacheable evidence with
`SolverError::InvalidInput`, including `finite = Trusted(..)` in v0.19.0 and any
entry claiming impossible finite semantics for `ScalarFamilyId::Float64`.
Eviction is caller-owned: there is no global cache, LRU, TTL, or size-bound
policy in this slice.

### 3.6 Provided evidence integration

The typed projected-first-order solve path should gain an additive
evidence-aware entrypoint. The existing RFC 016 entrypoint remains valid and
non-cacheable:

```rust
pub fn solve_projected_first_order_dyn<P, S>(
    problem: &P,
    /* existing RFC 016 arguments */
) -> Result<ProjectedFirstOrderSolveRecord, SolverError>
where
    P: ClusterProjectedFirstOrderProblem<S>,
    S: FiniteScalar + MetricScalar;
```

Cached/provided evidence must not be accepted for arbitrary `&P`, because the
solver cannot prove that the evidence belongs to the current model. The
evidence-aware path must accept the Loeres-owned carrier, or a sealed/internal
identity-carrier trait implemented only by that carrier:

Recommended shape:

```rust
pub struct ProvidedValidationEvidence {
    pub key: ValidationEvidenceKey,
    pub evidence: CachedValidationEvidence,
}

pub struct ProjectedFirstOrderSolveOptions<'a> {
    pub provided_evidence: Option<&'a ProvidedValidationEvidence>,
}

pub fn solve_projected_first_order_dyn_cached<P>(
    problem: &CacheableProjectedFirstOrderProblem<P>,
    /* existing RFC 016 arguments */
    options: &ProjectedFirstOrderSolveOptions<'_>,
) -> Result<ProjectedFirstOrderSolveRecord, SolverError>
where
    P: ClusterProjectedFirstOrderProblem<f64>;
```

The evidence-aware cached path is `f64` / `ScalarFamilyId::Float64` only in
v0.19.0. Generic non-cacheable solves remain available through the RFC 016 typed
path. A future RFC may add a sealed scalar-family mapping for additional scalar
families.

Provided cached evidence can skip only model-owned finite scans. The typed solve
path still scans the current initial iterate `x` unless the caller separately
uses explicit `TrustedByCaller` evidence for that finite scope. Numeric config,
workspace compatibility, current `step_scale`, cancellation, and hot-loop
checks remain non-cacheable.

Provided evidence is classified against the current carrier-derived
`ValidationEvidenceLookup` before cache lookup or use. Cache misses are not
errors; caller-provided wrong identity or stale epoch evidence is an error.

### 3.7 Policy behavior

Existing policy meanings are preserved:

| Policy/input | Required behavior |
|---|---|
| `ValidateAllInputs` | Run scans and produce fresh evidence. The caller may insert only the model-owned evidence portion into a cache after success. |
| `RespectBackendValidationState` + no provided evidence and no cache hit | Run scans and continue. |
| `RespectBackendValidationState` + matching sufficient provided/cache evidence | Use it only for skippable model-owned scans; still run per-call and non-skippable checks. |
| `RespectBackendValidationState` + matching evidence with insufficient model-owned scope | Scan missing model-owned scopes and continue. |
| `RespectBackendValidationState` + provided evidence with wrong identity or wrong epoch | Return `SolverError::InvalidInput`; do not silently treat it as absence. |
| cache lookup miss | Not an error; scan. |
| `TrustedByCaller` | Skip only asserted skippable scopes, returning visible `TrustedByCaller` evidence. |

`ClusterValidationPolicy::resolve` remains pure. Cache lookup, stale-evidence
classification, and scan fallback live at the typed kernel/model boundary, where
identity, epoch, provided evidence, and required scope are available.

### 3.8 Non-skippable checks

Cached/provided evidence never skips:

1. cancellation before expensive work;
2. numeric config validation;
3. dimension compatibility among current `x`, workspace, bounds, and problem;
4. current `step_scale` finite and `> 0`;
5. current initial iterate finite scan, unless separately covered by explicit
   `TrustedByCaller`;
6. hot-loop gradient finiteness;
7. hot-loop candidate finiteness;
8. cancellation polling during iteration.

The cache speaks only for model data keyed by identity and epoch. It cannot prove
that the caller's current iterate, workspace, config, or future oracle output is
valid. Numeric solve configuration such as tolerance and max-iteration count is
validated every solve. Changing numeric config may change status or solution,
but it does not invalidate model-owned finite evidence by itself.

### 3.9 Error mapping

RFC 015 introduces no new `SolverError` variant.

| Condition | Error |
|---|---|
| stale identity or stale epoch in provided evidence | `SolverError::InvalidInput` |
| provided evidence has insufficient model-owned scope under `RespectBackendValidationState` | run missing scans and continue |
| cache entry claims impossible finite semantics for the scalar family | `SolverError::InvalidInput` |
| unsupported scalar family for cached/evidence-aware path | `SolverError::UnsupportedProblemStructure` |
| internal cache invariant failure | `SolverError::InternalInvariantViolation` |
| non-finite discovered while scanning | `SolverError::NonFiniteInput` |
| non-finite discovered during iteration despite trust/cache | `SolverError::NumericalDomain` |

Stale evidence is distinct from absent evidence. Absence may fall back to scans;
staleness means the caller supplied evidence that claims to apply but is bound to
the wrong identity or epoch, so it fails closed.

### 3.10 Observability and gateway constraints

Cache observability may emit bounded metadata only.

| Allowed | Forbidden |
|---|---|
| cache hit/miss/stale category | raw model data |
| solver family/problem class | raw vectors, matrices, objective values |
| scope category | caller secrets, tenant IDs, request IDs |
| identity token class | full opaque identity bytes if they can correlate sensitive models |

Gateway/native adapter caches remain out of the v0.19.0 implementation slice,
but RFC 015 defines the identity/epoch vocabulary such adapters can consume
later.

## 4. Rust Systems-Level Nuances & Memory Safety

The cache is cluster-only and may allocate, but it must not introduce a path from
edge crates to `std` or `alloc`. Zero-bleed remains a release gate.

Identity and epoch types are copyable value types. Cache storage owns only small
keys and evidence values, not model buffers.

No `unsafe` is required. If a future API introduces an unsafe validation bypass,
it must be owned by a successor RFC and must name the exact invariants the caller
must uphold.

The existing `solve_projected_first_order_dyn` API must remain source-compatible
and non-cacheable. RFC 015 adds a carrier-based cached path rather than
retrofitting a required evidence parameter into the existing function or
allowing arbitrary unwrapped problems to consume cached evidence.

The cached/evidence-aware path is `f64`-only in v0.19.0. `DomainInapplicable` is
not emitted for `f64` and is not cacheable for any scalar family until a later
scalar/domain contract proves that non-finite values are impossible.

## 5. Algorithmic & Numerical Fail-Safe Guardrails

Trust and caching are pre-loop optimizations only.

Minimum fail-safe behavior:

1. stale evidence never yields `Solved`;
2. wrong identity never yields `Solved`;
3. wrong epoch never yields `Solved`;
4. absent or insufficient model-owned evidence under
   `RespectBackendValidationState` scans missing scopes and continues when data
   is valid;
5. hot-loop numerical-domain checks still run and still return
   `SolverError::NumericalDomain`;
6. non-convergence at the iteration cap remains `Ok(SolveStatus::NotConverged)`,
   not an error;
7. matching cached evidence skips only model-owned finite scans; per-call
   initial-iterate checks remain active unless explicitly trusted;
8. cache hit/miss behavior must not change mathematical status or solution for
   equivalent validated inputs.

## 6. RFC 013 Conformance Handoff

RFC 013 owns fixture execution. RFC 015 defines the future trusted/cache fixture
classes that RFC 013 should add after this policy lands.

Minimum future fixture classes:

| Fixture class | Expected result |
|---|---|
| matching cached evidence | Same status/solution as `ValidateAllInputs`; eligible model-owned finite scan may be skipped while per-call checks remain active. |
| stale epoch evidence | Structured `InvalidInput` failure, not solved. |
| wrong model identity | Structured `InvalidInput` failure, not solved. |
| trusted finite bypass with NaN produced in hot loop | `SolverError::NumericalDomain`, not solved. |
| insufficient model-owned scope under fallback mode | Missing scopes are scanned and the solve proceeds if scans pass. |

Existing smoke fixtures keep `validation_state = "validate-all-inputs"`.
Trusted/cache fixture states must be explicit, for example:

| Fixture field value | Meaning |
|---|---|
| `validate-all-inputs` | Current RFC 013 scanning baseline. |
| `cached-validation-match` | Provided evidence matches identity/epoch/scope. |
| `cached-validation-stale-epoch` | Provided evidence has stale epoch and must fail. |
| `cached-validation-wrong-identity` | Provided evidence has wrong identity and must fail. |
| `trusted-finite-bypass` | Caller asserts finite scope; hot-loop checks remain active. |

The exact TOML schema extension is RFC 013-owned when fixtures are added.

## 7. Verification, Validation, and CI Gates

Acceptance gates for the RFC 015 implementation:

1. Enforced: no runtime crate outside `loeres-cluster` gains validation-cache
   dependencies.
2. Enforced: stale identity and stale epoch tests fail closed.
3. Enforced: matching cached evidence skips only model-owned finite scans and
   leaves per-call initial-iterate checks active unless separately trusted.
4. Enforced: `ValidateAllInputs` still scans and remains the default baseline.
5. Enforced: `RespectBackendValidationState` with absent/insufficient evidence
   scans missing scopes rather than silently trusting them.
6. Enforced: `ClusterValidationPolicy::resolve` remains pure.
7. Enforced: trusted/cached evidence never suppresses hot-loop
   `NumericalDomain`.
8. Enforced: existing RFC 013 smoke conformance remains green.
9. Enforced: `TrustedByCaller` evidence is not inserted into the reusable cache
   in v0.19.0.
10. Enforced: mutable carrier operations advance the epoch or mark the model
    non-cacheable.
11. Enforced: unwrapped user-defined problems remain solveable but
    non-cacheable.
12. Enforced: the cached/evidence-aware solve path accepts
    `CacheableProjectedFirstOrderProblem<P>` or a sealed identity carrier, not
    arbitrary `&P`.
13. Enforced: `ValidationEvidenceCache::insert` returns `Result<(), SolverError>`
    and rejects non-cacheable evidence.
14. Enforced: model identity generation never accepts caller-supplied identity
    values and fails closed on exhaustion.
15. Enforced: cached/evidence-aware solving is `f64` /
    `ScalarFamilyId::Float64` only in v0.19.0.
16. Enforced: zero-bleed proves only `loeres-cluster` gains cache types.
17. Enforced: `cargo fmt --all --check`, `cargo clippy --workspace
   --all-features --all-targets -- -D warnings`, `cargo test --workspace
   --all-features`, `cargo xtask check`, `cargo xtask release-gate`,
   `cargo xtask check-rfcs`, and `cargo xtask link-audit` pass.
18. Enforced: a clean extraction runs `cargo xtask check` successfully before
    RFC 015 moves to `done/`.

## 8. Review-Settled Decisions

The design-freeze review settled these points for v0.19.0:

1. Missing or insufficient model-owned scope under
   `RespectBackendValidationState` scans and continues; wrong identity or wrong
   epoch rejects with `SolverError::InvalidInput`.
2. `ModelIdentity` is generated by a Loeres-owned carrier. Arbitrary
   caller-provided identity values are not accepted in v0.19.0.
3. No core `TrustKind` variant is added in v0.19.0. Pipeline/cache trust remains
   cluster-side.
4. Cache replacement/eviction is caller-owned. The cache provides `new`, `get`,
   `insert`, `invalidate_model`, and `clear`; no global cache, LRU, TTL, or
   size-bound policy ships in this slice.
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

## 9. Implementation Closeout

RFC 015 ships in v0.19.0 as a cluster-only validation-cache release. The
pre-release implementation reviewed as `0.19.0-pre.1` was corrected before
publication. This RFC does not add cache dependencies to `loeres`,
`loeres-device`, or `loeres-backend-static`.

Implemented pieces:

1. Added `loeres_cluster::validation_cache` with `ModelIdentity`,
   `MutationEpoch`, `ValidationEvidenceKey`, `ValidationEvidenceLookup`,
   `CachedValidationEvidence`, `ProvidedValidationEvidence`,
   `ValidationEvidenceCache`, and local bounded IDs
   `SolverFamilyId::ProjectedFirstOrder`, `ProblemClassId::BoxBoundFirstOrder`,
   and `ScalarFamilyId::Float64`.
2. Added `CacheableProjectedFirstOrderProblem<P>`, a Loeres-owned identity/epoch
   carrier. Identity is process-local and generated by Loeres; read-only solves
   do not advance epochs; mutation is available only through an epoch-bumping
   closure.
3. Added `solve_projected_first_order_dyn_cached`, a carrier-only `f64` cached
   solve entrypoint. The existing `solve_projected_first_order_dyn(&P, ...)`
   remains source-compatible and non-cacheable.
4. Kept reusable cache evidence model-owned only. Cache hits may skip
   model-owned finite scans, while current iterate scans, workspace/config
   checks, step-scale checks, cancellation, and hot-loop finite checks remain
   active unless explicitly trusted for the current solve.
5. Made `ValidationEvidenceCache::insert` fallible and reject non-cacheable
   evidence such as `Trusted(..)` and `DomainInapplicable` for `f64`.
6. Added tests for exact-key/coverage lookup, trusted-evidence rejection,
   invalidate/clear behavior, carrier identity/epoch/clone semantics, matching
   cached evidence, cache miss fallback, insufficient-scope fallback,
   wrong-identity/stale-epoch rejection, per-call iterate scanning, and
   hot-loop `NumericalDomain` preservation.

RFC 013 trusted/cache fixture execution remains future work; this RFC defines
the semantics those fixtures should exercise.

### 9.1 Pre-release implementation-review correction

The final v0.19.0 patch closes the stale-evidence mutation hole identified
during implementation review:

1. `CacheableProjectedFirstOrderProblem::mutate` advances the mutation epoch
   before exposing mutable access to the inner model. If the closure mutates and
   then returns `Err` or unwinds under `catch_unwind`, evidence for the previous
   epoch no longer matches the carrier's current lookup key.
2. `ValidationEvidenceCache::insert` rejects the sentinel
   `ModelIdentity::NON_CACHEABLE`.
3. `SolverFamilyId`, `ProblemClassId`, and `ScalarFamilyId` are
   `#[non_exhaustive]`.
4. Regression tests cover failed mutation, caught-panic mutation, sentinel-key
   insertion, cache miss over non-finite bounds, and insufficient-scope fallback
   over non-finite bounds.
