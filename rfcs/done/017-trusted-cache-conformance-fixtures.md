# RFC 017 - Trusted/Cache Conformance Fixtures

**Status.** Implemented (v0.20.0) - verification-only conformance hardening.
**Tracks.** Phase 3 / Milestone 3 - conformance hardening for trusted validation cache semantics.
**Touches.** `conformance/`, `xtask/src/checks/conformance.rs`, `loeres-cluster` test harness usage, RFC 013 fixture taxonomy, RFC 015 cache/evidence semantics.

---

### Extended Metadata
* **Rust Edition Compliance:** Rust 2024 Baseline
* **Target Environment:** Host-side verification only. Runtime crates must not parse fixture files or depend on fixture schema types.

## 1. Summary

RFC 015 added a cluster-only validation evidence cache. Its implementation has
unit tests for exact key matching, stale epoch rejection, current-iterate scan
retention, and hot-loop numerical-domain retention. Those tests prove the local
API, but they do not yet put trusted/cache behavior into the repository's
conformance corpus.

RFC 017 extends RFC 013 conformance with trusted/cache fixture states for the
RFC 016 projected-first-order cluster solver and the RFC 015 validation cache.
The goal is to make stale-trust safety a release gate rather than a set of
crate-local tests.

This RFC is verification-only. It adds no solver family, no runtime public API,
no persistent cache, no native gateway adapter, and no device trusted-cache
entrypoint.

## 1.1 Prerequisites

RFC 017 remains verification-only only because the required runtime behavior is
already present in RFC 015's final v0.19.0 implementation. Before RFC 017
implementation starts, these runtime prerequisites must be true:

1. `ValidationEvidenceCache::insert` rejects sentinel
   `ModelIdentity::NON_CACHEABLE` keys.
2. `CacheableProjectedFirstOrderProblem::mutate` advances the mutation epoch
   before exposing mutable model state, or otherwise marks the carrier
   non-cacheable before mutable access.
3. Reusable cache insertion rejects `ProjectedFirstOrderFiniteEvidence::Trusted(..)`.
4. Provided evidence with wrong identity or stale epoch fails closed with
   `SolverError::InvalidInput`.
5. Cached/provided evidence does not suppress current-iterate scans or
   hot-loop numerical-domain checks.

RFC 017 verifies these semantics through conformance fixtures. It must not add
or change runtime APIs or runtime behavior. If any prerequisite is discovered
missing, the correct action is to patch RFC 015 first, not to expand RFC 017
beyond verification.

## 2. Affected crates

| Area | Impact |
|---|---|
| `conformance/` | Add trusted/cache fixture files and suite documentation. |
| `xtask` | Extend the host-only conformance runner to materialize RFC 015 evidence states and check outcomes. |
| `loeres-cluster` | Used by `xtask` as a dependency to run the real cached projected-first-order path. No runtime API change. |
| `loeres-device` | Used as the mathematical parity baseline for fixtures that remain comparable. No runtime API change. |
| `loeres`, `loeres-backend-static`, `loeres-backend-std` | Used through existing public APIs only. |

## 3. Public API boundary impact

No runtime public API is added or changed.

`xtask conformance` gains fixture-state support and may gain internal host-only
schema types. Those schema types are not runtime APIs. Runtime crates must not
depend on `xtask`, fixture TOML, serde schema types, or conformance parser code.

The existing `cargo xtask conformance` command remains valid. The default
invocation stays equivalent to the enforced smoke suite.

## 4. Dependency impact

This RFC permits dependency changes only in `xtask`, if needed to parse and
execute the new host-side fixture states. It does not permit new dependencies in
runtime crates.

| Crate | New dependencies allowed? | Notes |
|---|---:|---|
| `xtask` | yes, host-only if needed | Existing TOML/serde pattern should be reused if possible. |
| `loeres-cluster` | no | Use existing RFC 015 APIs. |
| `loeres-device` | no | Used only by `xtask` for baseline comparison. |
| `loeres-backend-std` | no | Existing dynamic vectors are sufficient. |
| `loeres-backend-static` | no | Existing fixed vectors are sufficient. |
| `loeres` | no | No core impact. |

## 5. `std` / `alloc` impact statement

The new work is host-side conformance. `xtask` may use `std` and allocation.
No `std` or `alloc` requirement may be introduced into `loeres`,
`loeres-backend-static`, or `loeres-device`.

The zero-bleed gate remains mandatory.

## 6. Device determinism impact statement

Device runtime behavior does not change. Device solves remain validate-inline
and allocation-free.

For fixtures whose mathematical result is comparable, the device projected-first
order path remains the parity baseline against cluster `ValidateAllInputs` and
cluster trusted/cache variants. For fixtures whose purpose is a cluster-only
cache rejection, the device path may be marked `not-applicable`, but the fixture
must say so explicitly.

## 7. Cluster scalability impact statement

Cluster runtime behavior does not change. The conformance runner may exercise
cache-hit and cache-miss paths, but it does not introduce a global cache, LRU,
TTL, persistence, distributed identity, or adapter cache.

The conformance tests should stay small: dimension 2, deterministic data, and no
large generated corpus in the enforced smoke path.

## 8. Error and diagnostic impact

No new `SolverError` variant is introduced.

The conformance runner must compare structured errors, not display strings. The
trusted/cache fixtures use the current RFC 015 mapping:

| Condition | Expected category |
|---|---|
| provided evidence has wrong identity | `SolverError::InvalidInput` |
| provided evidence has stale epoch | `SolverError::InvalidInput` |
| provided evidence has insufficient scope | scan fallback, not immediate failure |
| cache miss | scan fallback, not immediate failure |
| non-finite model data discovered during fallback scan | `SolverError::NonFiniteInput` |
| non-finite current iterate under cached model evidence | `SolverError::NonFiniteInput` |
| non-finite value produced during iteration despite trust/cache | `SolverError::NumericalDomain` |
| attempt to insert reusable `Trusted(..)` evidence | `SolverError::InvalidInput` |
| attempt to insert sentinel identity evidence | `SolverError::InvalidInput` |

RFC 017 does not add an enforced `ClusterValidationPolicy::TrustedByCaller`
solve fixture. It verifies reusable-cache handling and reusable-cache rejection
of `Trusted(..)` evidence. A broader trusted-solve fixture such as
`trusted-finite-bypass` remains later work unless a later design patch adds it
explicitly.

## 9. Feature flag impact

No new runtime feature flag is introduced.

The enforced trusted/cache conformance path runs on the host and uses the
existing `loeres-cluster` projected-first-order `f64` cached entrypoint. It must
not require optional native solver, FFI, tracing, metrics, or async runtime
features.

## 10. Semver impact

This is a minor release because it adds enforced verification behavior and new
conformance fixtures. The expected target is v0.20.0.

Runtime APIs remain source-compatible.

## 11. Concrete technical specification

### 11.1 Suite placement

RFC 017 extends the existing conformance layout without replacing RFC 013 smoke
fixtures:

```text
conformance/
  smoke/
    ... existing RFC 013 fixtures ...
    pfo-cache-match-001.toml
    pfo-cache-miss-valid-001.toml
    pfo-cache-insufficient-scope-valid-001.toml
    pfo-cache-stale-epoch-001.toml
    pfo-cache-wrong-identity-001.toml
    pfo-cache-current-iterate-nonfinite-001.toml
    pfo-cache-hot-loop-numerical-domain-001.toml
    pfo-cache-reject-trusted-evidence-001.toml
    pfo-cache-reject-sentinel-identity-001.toml
  extended/
    README.md
  adversarial/
    README.md
```

The exact filenames may change during implementation if the runner groups them
under a subdirectory, but the fixture IDs above are the required semantic cases
for the enforced RFC 017 slice. Fixture IDs are the stable audit handles.

### 11.2 Fixture family

RFC 017 remains in the same narrow problem family as RFC 013:

| Field | Value |
|---|---|
| `problem_class` | `box_quadratic_diagonal` |
| `solver_family` | `projected_first_order` |
| `dimension` | `2` |
| `scalar_family` | `float` / `f64` |
| device path | RFC 006 `solve_projected_first_order` baseline when comparable |
| cluster uncached path | RFC 016 `solve_projected_first_order_dyn` under `ValidateAllInputs` |
| cluster cached path | RFC 015 `solve_projected_first_order_dyn_cached` |

The fixture runner must reject enforced RFC 017 smoke fixtures with unsupported
dimension, solver family, problem class, or scalar family.

### 11.3 Validation/evidence state vocabulary

RFC 013 used only `validation_state = "validate-all-inputs"`. RFC 017 adds a
separate cache/evidence state field so validation policy and evidence setup are
not collapsed into one ambiguous string.

Required new field:

```toml
validation_state = "respect-backend-validation-state"
evidence_state = "cached-validation-match"
execution_mode = "solve"
```

Allowed `evidence_state` values in the enforced RFC 017 slice:

| Value | Meaning |
|---|---|
| `cached-validation-match` | Reusable-cache hit with scanned model-owned evidence under the exact current key and sufficient scope. |
| `cached-validation-miss` | Supply an empty cache or unrelated cache entry; solver must scan and continue/fail according to data. |
| `cached-validation-insufficient-scope` | Provide matching key but insufficient model-owned scope; solver must scan missing finite data. |
| `cached-validation-stale-epoch` | Provide evidence for the same identity before a carrier mutation advanced the epoch; solver must fail closed. |
| `cached-validation-wrong-identity` | Provide evidence with a different model identity; solver must fail closed. |
| `cached-validation-current-iterate-nonfinite` | Matching model evidence exists, but current `x` is non-finite; solver must scan current input and fail. |
| `cached-validation-hot-loop-numerical-domain` | Matching model evidence exists, but the oracle produces non-finite values during iteration; hot-loop fail-safe must fire. |
| `cache-insert-trusted-evidence` | Attempt to insert `Trusted(..)` evidence into the reusable cache; insertion must fail. |
| `cache-insert-sentinel-identity` | Attempt to insert evidence under `ModelIdentity::NON_CACHEABLE`; insertion must fail. |

The provided-evidence happy path is not mandatory in v0.20.0 conformance.
`pfo-cache-match-001` covers the reusable cache lookup path. Provided-evidence
happy path coverage remains in RFC 015 unit tests unless a later RFC 017 design
patch adds a separate `pfo-cache-provided-match-001` fixture.

The `trusted-cache-smoke` conformance group used by v2 fixtures is a
corpus-local review tag. It is not an RFC 011 target-profile group.

Allowed `execution_mode` values:

| Value | Meaning |
|---|---|
| `solve` | Run the solver path and compare parity or fail-closed solve outcome. This is the default mode for all solve fixtures. |
| `cache-insert` | Stop at cache insertion and compare the insertion result. Device baseline, cluster uncached baseline, status comparison, and solution comparison are `not-applicable`; only structured `expected_failure_match` is enforced. |

### 11.4 Fixture schema extension

The v0.20.0 schema remains TOML and host-only. RFC 017 freezes
`schema_version = 2` for every fixture containing `evidence_state`,
`execution_mode`, or cache-specific sections. Existing RFC 013 baseline fixtures
remain `schema_version = 1`.

The parser must reject:

1. `schema_version = 1` with `evidence_state`, `execution_mode`, or `[evidence]`;
2. `schema_version = 2` without a valid `evidence_state`;
3. unknown evidence states;
4. unknown execution modes;
5. `execution_mode = "cache-insert"` with status/solution comparisons marked as
   enforced;
6. contradictions between `evidence_state`, `execution_mode`, and `[evidence]`.

Example:

```toml
schema_version = 2
fixture_id = "pfo-cache-match-001"
suite = "smoke"
execution_mode = "solve"
problem_class = "box_quadratic_diagonal"
solver_family = "projected_first_order"
dimension = 2
scalar_family = "float"
validation_state = "respect-backend-validation-state"
evidence_state = "cached-validation-match"
conformance_groups = ["cluster-reference-smoke"]

[config]
max_iterations = 64
tolerance = 0.000001
step_scale = 0.5

[problem]
lower = [-1.0, -1.0]
upper = [1.0, 1.0]
initial = [0.0, 0.0]
quadratic_diag = [1.0, 1.0]
center = [0.25, -0.5]

[expected]
status = "converged"
termination = "convergence-criterion"
solution = [0.25, -0.5]
error = "none"

[tolerance]
solution_abs = 0.00001
solution_rel = 0.00001
objective_abs = "not-applicable"
residual_abs = "not-applicable"
```

Additional optional sections for RFC 017:

```toml
[evidence]
model_scope = ["finite"]
provided = true
cache = false
mutate_before_solve = false
mutation_result = "ok"
```

The runner may derive these details from `evidence_state` instead of accepting
all fields independently. If both are accepted, contradictions must fail schema
validation.

### 11.5 Required enforced fixture cases

| Fixture ID | Mode | Execution mode | Expected behavior | Required comparisons |
|---|---|---|---|---|
| `pfo-cache-match-001` | parity | `solve` | reusable-cache hit matches `ValidateAllInputs` and device baseline | `status_match`, `solution_within_tolerance` |
| `pfo-cache-miss-valid-001` | parity | `solve` | cache miss scans valid model data and matches baseline | `status_match`, `solution_within_tolerance` |
| `pfo-cache-insufficient-scope-valid-001` | parity | `solve` | insufficient scope scans missing finite data and matches baseline | `status_match`, `solution_within_tolerance` |
| `pfo-cache-stale-epoch-001` | fail-closed solve | `solve` | stale provided evidence fails closed | `expected_failure_match = InvalidInput` |
| `pfo-cache-wrong-identity-001` | fail-closed solve | `solve` | wrong provided identity fails closed | `expected_failure_match = InvalidInput` |
| `pfo-cache-current-iterate-nonfinite-001` | fail-closed solve | `solve` | cache hit does not skip current `x` scan | `expected_failure_match = NonFiniteInput` |
| `pfo-cache-hot-loop-numerical-domain-001` | fail-closed solve | `solve` | cache hit does not suppress in-loop fail-safe | `expected_failure_match = NumericalDomain` |
| `pfo-cache-reject-trusted-evidence-001` | fail-closed cache-insert | `cache-insert` | reusable cache rejects `Trusted(..)` evidence | `expected_failure_match = InvalidInput` |
| `pfo-cache-reject-sentinel-identity-001` | fail-closed cache-insert | `cache-insert` | reusable cache rejects sentinel identity key | `expected_failure_match = InvalidInput` |

### 11.6 Comparison model

There are three comparison modes:

| Mode | Use |
|---|---|
| parity mode | Fixture should solve successfully; compare cluster cached path against cluster `ValidateAllInputs` and device baseline when applicable. |
| fail-closed solve mode | Solver should reject unsafe evidence or input; compare structured `SolverError` category. |
| fail-closed cache-insert mode | Cache insertion should reject unsafe reusable evidence; compare structured `SolverError` category and skip solve/baseline comparisons. |

For parity mode, the tolerance remains RFC 013's `epsilon = 1e-5` unless the
fixture states a tighter tolerance. For fail-closed modes, formatted error text
must not be compared.

### 11.7 Runner behavior

The conformance runner should add an internal execution path:

1. Parse the fixture and validate schema fields.
2. Materialize the diagonal quadratic problem.
3. Run cluster `ValidateAllInputs` as the baseline for parity fixtures.
4. Run the device baseline for parity fixtures that are mathematically
   comparable.
5. Build a `CacheableProjectedFirstOrderProblem`.
6. Construct cache/provided evidence according to `evidence_state`.
7. Run `solve_projected_first_order_dyn_cached` under
   `RespectBackendValidationState`.
8. Compare category outcomes and print per-fixture category results.

For `execution_mode = "cache-insert"` fixtures, the runner must stop at cache
insertion and compare the returned `SolverError`; no solve is run.

## 12. Rejected alternatives

| Alternative | Rejection reason |
|---|---|
| Keep trusted/cache behavior only in unit tests | Unit tests are necessary but do not make stale-trust behavior part of release conformance evidence. |
| Add a native gateway adapter first | Gateway adapters would consume identity/cache behavior; conformance should lock the behavior before adapter work. |
| Add device trusted-cache entrypoints | Device APIs validate inline and are allocation-free; no evidence supports adding cache state to the device boundary. |
| Make all trusted/cache fixtures adversarial-only | Stale trust is a safety boundary and must be in the enforced smoke gate. |
| Store raw model data or hashes in evidence fixtures | RFC 015 identity is process-local and metadata-only; fixture data should remain small and explicit. |

## 13. Verification gates

Implementation must run and record:

1. `cargo fmt --all --check`
2. `cargo check --workspace --all-features`
3. `cargo test --workspace --all-features`
4. `cargo clippy --workspace --all-features --all-targets -- -D warnings`
5. `cargo xtask conformance`
6. `cargo xtask conformance --suite smoke`
7. `cargo xtask check-rfcs`
8. `cargo xtask link-audit`
9. `cargo xtask check`
10. `cargo xtask release-gate`
11. Clean extraction `cargo xtask check`

The implementation must also include a stale-reference scan with these targets:

1. fail if runtime crates import `xtask::*`;
2. fail if runtime crates reference conformance schema modules or schema types;
3. fail if runtime crate `Cargo.toml` files gain fixture/parser dependencies;
4. allow `xtask -> runtime crates` path dependencies, because the runner is
   host-only.

## 14. Implementation sprint plan

| Sprint | Work |
|---|---|
| S0 Design Freeze | Review this RFC, freeze schema v2, execution modes, and required fixture IDs. |
| S1 Fixture Skeleton | Add fixture files or fixture-state table and parser enums without changing runtime crates. |
| S2 Contract Tests | Add xtask parser/schema tests for evidence states and contradiction rejection. |
| S3 Runner Integration | Materialize cache/provided evidence and call the real RFC 015 cached solve path. |
| S4 Verification | Enforce the new smoke cases in `cargo xtask conformance` and aggregate gates. |
| S5 Documentation | Update `conformance/README.md`, RFC 013 references if needed, roadmap, and changelog. |
| S6 Closeout | Move RFC 017 to `done/`, record gate evidence, and leave adapter/cache expansion as future work. |

## 15. Exit criteria

RFC 017 is complete when:

1. enforced smoke conformance includes all required RFC 017 fixture IDs or their
   exact semantic equivalents;
2. cache-hit, cache-miss, insufficient-scope, stale-epoch, wrong-identity,
   current-iterate-scan, hot-loop-fail-safe, trusted-evidence-rejection, and
   sentinel-identity-rejection cases are all covered;
3. successful cache variants match the `ValidateAllInputs` baseline and device
   baseline where applicable;
4. fail-closed variants compare structured `SolverError` categories;
5. runtime crates do not parse fixture files or depend on fixture schema types;
6. runtime crates do not import `xtask`, conformance schema types, or parser
   dependencies;
7. zero-bleed and no-std gates still pass;
8. `cargo xtask conformance` and `cargo xtask conformance --suite smoke` both
   pass and remain equivalent for the enforced smoke suite;
9. `cargo xtask check` and `cargo xtask release-gate` pass in the working tree
   and in a clean extraction.

## 16. Implementation closeout

RFC 017 ships in v0.20.0 as a host-side conformance release. Runtime crate APIs
are unchanged.

The implemented smoke corpus contains the nine required cache fixture IDs:
`pfo-cache-match-001`, `pfo-cache-miss-valid-001`,
`pfo-cache-insufficient-scope-valid-001`, `pfo-cache-stale-epoch-001`,
`pfo-cache-wrong-identity-001`,
`pfo-cache-current-iterate-nonfinite-001`,
`pfo-cache-hot-loop-numerical-domain-001`,
`pfo-cache-reject-trusted-evidence-001`, and
`pfo-cache-reject-sentinel-identity-001`.

`xtask conformance` now supports `schema_version = 2`,
`execution_mode = "solve" | "cache-insert"`, and explicit `evidence_state`
setup for RFC 015 validation-cache cases. Successful cache fixtures compare
device, cluster `ValidateAllInputs`, and cluster cached outcomes. Fail-closed
solve and cache-insert fixtures compare structured `SolverError` categories.

Failed and panicking mutation-closure epoch safety remains an RFC 015 runtime
prerequisite covered by `loeres-cluster` unit tests:
`carrier_mutate_advances_epoch_before_returned_error` and
`carrier_mutate_advances_epoch_before_caught_panic`.
