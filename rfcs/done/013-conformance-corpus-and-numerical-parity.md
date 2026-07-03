# RFC 013 — Conformance Corpus and Numerical Parity Policy

**Status.** Implemented (v0.18.0) — `conformance/` now contains the enforced smoke corpus and `cargo xtask conformance` runs the RFC 013 device/cluster parity gate.
**Tracks.** Cross-cutting numerical evidence, cluster/device compatibility, regression testing, and release validation
**Touches.** `conformance/`, `xtask/src/checks/conformance.rs`, `xtask/Cargo.toml`, `loeres-device` examples, `loeres-cluster` examples, solver-specific test fixtures, CI workflows

---

### Extended Metadata
* **Rust Edition Compliance:** Rust 2024 Baseline
* **Target Environment:** Shared verification layer spanning `loeres-device`, `loeres-cluster`, `loeres-backend-static`, and `loeres-backend-std`

## 1. Executive Summary & Problem Statement

Loeres intentionally supports two different execution families: static device solving and dynamic cluster solving. These paths must share mathematical contracts without promising bitwise-identical outputs across every CPU, FPU, compiler profile, scalar family, and solver configuration.

The project now has both sides of the first comparable solver family: RFC 006 shipped the device projected-first-order kernel and RFC 016 shipped the dynamic cluster analog. RFC 011 then supplied stable target/profile names and `conformance_group` metadata. This RFC turns the existing RFC 010 `conformance` hook into a real smoke gate for that shared projected-first-order family.

The v0.18.0 implementation slice is deliberately narrow: enforce a small smoke suite for a dimension-2 diagonal box quadratic family, report per-category outcomes, and keep extended/adversarial breadth staged.

## 2. Architectural Context & Dependency Alignment

The conformance corpus is a verification artifact, not a runtime dependency. Runtime crates do not parse fixture files or depend on fixture schema types.

Dependency alignment:

| Component | Relationship to this RFC | Dependency impact |
|---|---|---|
| `conformance/` | Stores fixture data and suite documentation | Repository test data only |
| `xtask conformance` | Loads fixtures and runs comparisons | Host-only; may use `std`, workspace crate APIs, and TOML/serde parsing dependencies |
| `loeres-device` | Provides the real device projected-first-order entrypoint | No parser dependency |
| `loeres-cluster` | Provides the real dynamic projected-first-order entrypoint | No parser dependency |
| `loeres-backend-static` | Provides host-materialized fixed vectors for device fixtures | No parser dependency |
| `loeres-backend-std` | Provides dynamic vectors for cluster fixtures | No parser dependency |
| `loeres` | Owns shared status/error categories | No `std`, no `alloc` |

`xtask` may link to workspace crates as a host-side verification crate. No runtime crate may depend on `xtask`, fixture TOML, or the fixture schema.

## 3. Concrete Technical Specification

### 3.1 Corpus directory layout

The v0.18.0 repository layout is:

```text
conformance/
  README.md
  smoke/
    pfo-box-converged-001.toml
    pfo-box-not-converged-001.toml
    pfo-box-invalid-bound-001.toml
  extended/
    README.md
  adversarial/
    README.md
```

The smoke suite is enforced. `extended/` and `adversarial/` are README-backed placeholders in v0.18.0; they may report not-enforced/advisory until populated by later patches.

### 3.2 v0.18.0 fixture family

The first fixture family is the only problem/solver family implemented on both sides:

| Field | Value |
|---|---|
| `problem_class` | `box_quadratic_diagonal` |
| `solver_family` | `projected_first_order` |
| device path | `loeres-device::solve_projected_first_order` over `FixedVector<f64, 2>` |
| cluster path | `loeres-cluster::solve_projected_first_order_dyn` over `DenseVector<f64>` |
| dimension | `2` only in v0.18.0 |
| scalar family | `float` / `f64` |
| target groups | `device-reference-smoke`, `cluster-reference-smoke` |
| validation state | `validate-all-inputs` only in v0.18.0 |

The diagonal quadratic problem is:

```text
f(x) = 0.5 * Σ q_i * (x_i - c_i)^2
grad_i(x) = q_i * (x_i - c_i)
x_next_i = clamp(x_i - step_scale * grad_i(x), lower_i, upper_i)
```

The runner must reject any enforced smoke fixture with `dimension != 2`. It must not silently skip unsupported dimensions.

Trusted-input fixtures are deferred until RFC 015 defines trusted-pipeline, cached validation, model identity, and mutation epoch semantics. RFC 013's general policy still requires trusted fixtures to surface numerical-domain failures once such fixtures exist, but v0.18.0 smoke uses `ValidateAllInputs` only.

### 3.3 Required smoke fixtures

The first smoke suite must contain exactly these required fixtures:

| Fixture ID | Expected result | Enforced categories |
|---|---|---|
| `pfo-box-converged-001` | `SolveStatus::Converged` / `TerminationReason::ConvergenceCriterion` | `status_match`, `solution_within_tolerance` |
| `pfo-box-not-converged-001` | `SolveStatus::NotConverged` / `TerminationReason::IterationCap` | `status_match` |
| `pfo-box-invalid-bound-001` | `SolverError::InvalidInput` | `expected_failure_match` |

For the not-converged fixture, use a deliberately small cap such as `max_iterations = 1` with a nonzero initial distance so accidental convergence is unlikely. Failure comparison must use structured `SolverError` categories, not formatted strings.

### 3.4 Fixture schema

The fixture format is stable, host-only TOML:

```toml
schema_version = 1
fixture_id = "pfo-box-converged-001"
suite = "smoke"
problem_class = "box_quadratic_diagonal"
solver_family = "projected_first_order"
dimension = 2
scalar_family = "float"
validation_state = "validate-all-inputs"
conformance_groups = ["device-reference-smoke", "cluster-reference-smoke"]

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

Required top-level fields:

| Field | Required | Notes |
|---|---:|---|
| `schema_version` | yes | Must be `1` for v0.18.0. |
| `fixture_id` | yes | Stable ID; semantic changes require a new ID. |
| `suite` | yes | `smoke`, `extended`, `adversarial`, or `regression-only`. |
| `problem_class` | yes | v0.18.0 supports only `box_quadratic_diagonal`. |
| `solver_family` | yes | v0.18.0 supports only `projected_first_order`. |
| `dimension` | yes | v0.18.0 supports only `2`. |
| `scalar_family` | yes | v0.18.0 supports `float` / `f64` fixtures. |
| `validation_state` | yes | v0.18.0 supports only `validate-all-inputs`. |
| `conformance_groups` | yes | Must include groups emitted by RFC 011 target profiles. |

Required sections:

| Section | Required | Notes |
|---|---:|---|
| `[config]` | yes | `max_iterations`, `tolerance`, `step_scale`. |
| `[problem]` | yes | `lower`, `upper`, `initial`, `quadratic_diag`, `center`; each vector length must equal `dimension`. |
| `[expected]` | yes | Expected status/error/solution metadata. |
| `[tolerance]` | yes | Solution tolerance required; objective/residual may be `"not-applicable"`. |

Validation rules:

1. Unknown or malformed required fields fail schema validation.
2. Every vector length must equal `dimension`.
3. `quadratic_diag` entries must be finite and positive for converged/not-converged fixtures.
4. `step_scale`, `tolerance`, and `max_iterations` must map to both solver configs without lossy interpretation.
5. Unsupported `problem_class`, `solver_family`, `scalar_family`, `validation_state`, or `dimension` values fail for enforced smoke fixtures.
6. Generated fixtures must record seed and generator version; v0.18.0 hand-authored fixtures may omit seed fields.

`xtask` should use a host-only TOML parser (`toml` / `serde` or equivalent). This is permitted because `xtask` is host-only. Runtime crates must not parse fixture TOML or depend on parser crates.

### 3.5 Outcome comparison model

The default parity rule is tolerance-based convergence, not bitwise equality.

The baseline cross-path tolerance is:

```text
epsilon = 1e-5
```

Solver-specific RFCs may define tighter or looser tolerances, but they must justify the value and identify whether it applies to objective value, primal residual, dual residual, variable vector distance, or status classification.

### 3.6 Required comparison categories

`cargo xtask conformance` must report per-fixture category results:

| Category | v0.18.0 behavior |
|---|---|
| `status_match` | Enforced for all non-error fixtures. |
| `solution_within_tolerance` | Enforced for converged fixtures with an expected solution. |
| `expected_failure_match` | Enforced for error fixtures; compares structured `SolverError` categories, not display strings. |
| `objective_within_tolerance` | Report `not-applicable`; shipped reports do not carry objective values. |
| `residual_within_tolerance` | Report `not-applicable`; the projected-first-order contract has no residual output. |
| `not_comparable` | Allowed only when explicitly declared; not used by the required smoke fixtures. |

A single scalar pass/fail output is insufficient. The command must expose the category that failed.

### 3.7 Suite CLI behavior

RFC 013 freezes this command contract:

```text
cargo xtask conformance
cargo xtask conformance --suite smoke
cargo xtask conformance --suite extended
cargo xtask conformance --suite adversarial
```

Behavior:

| Invocation | v0.18.0 behavior |
|---|---|
| no args | same as `--suite smoke` |
| `--suite smoke` | enforced; used by `cargo xtask check` |
| `--suite extended` | if fixtures exist, run them; if only the README placeholder exists, report not-enforced/advisory |
| `--suite adversarial` | if fixtures exist, run them; if only the README placeholder exists, report not-enforced/advisory |
| unknown args/suite | fail with usage |

After RFC 013 lands, the aggregate gate must no longer report the default conformance path as a not-enforced hook. Placeholder extended/adversarial suites may still report not-enforced/advisory honestly.

### 3.8 Runner materialization

The v0.18.0 runner materializes only dimension-2 fixtures:

| Path | Materialization |
|---|---|
| device | `FixedVector<f64, 2>` and the RFC 006 device projected-first-order API |
| cluster | `DenseVector<f64>` and the RFC 016 typed dynamic projected-first-order API |

The runner should not generate Rust source files in v0.18.0. Host-side materialization is sufficient and reviewable.

### 3.9 Smoke, extended, and adversarial staging

The smoke suite is release-gated and must run quickly.

The extended suite may later include larger matrices, sparse structures, target-specific comparisons, conditioning stress cases, and generated fixtures.

The adversarial suite may later cover:

* invalid dimensions;
* non-finite scalar input for float scalars;
* singular or ill-conditioned structures;
* invalid tolerance configuration;
* max-iteration exhaustion;
* workspace/problem mismatch for device cases;
* trusted-input numerical-domain behavior after RFC 015.

The v0.18.0 invalid-bound smoke fixture is sufficient to satisfy the immediate "at least one expected failure behavior" acceptance gate.

## 4. Rust Systems-Level Nuances & Memory Safety

The corpus runner may allocate on the host. Device crates may not depend on fixture parsers.

For device tests, host-side tooling may materialize static arrays from fixtures and call real device APIs. Generated Rust files are not part of v0.18.0.

Large fixture data should not be embedded into device firmware by default. Device conformance builds should use tiny smoke fixtures unless an explicit extended profile is selected.

Comparing floating-point vectors must avoid naive absolute-only checks where scale matters. The comparison policy supports absolute and relative tolerance for solution vectors in v0.18.0.

## 5. Algorithmic & Numerical Fail-Safe Guardrails

The corpus must validate failure behavior as well as success behavior. A solver that panics, hangs, allocates unexpectedly, or returns an unstructured error fails conformance even if normal cases converge.

Minimum fail-safe expectations:

1. invalid fixtures return structured error categories;
2. ill-conditioned fixtures do not cause unbounded iteration;
3. max-iteration fixtures terminate with `Ok(SolveReport)` carrying `SolveStatus::NotConverged` and `TerminationReason::IterationCap` (RFC 014), not a `SolverError`;
4. trusted-input fixtures still surface numerical-domain errors discovered during computation once RFC 015-owned trust channels exist;
5. unsupported problem classes return unsupported-problem rather than attempting best-effort execution.

## 6. Verification, Validation, and CI Gates

Acceptance gates for the RFC 013 implementation:

1. Enforced: the repository contains `conformance/README.md`, the three required smoke fixture TOML files, and README placeholders for `extended/` and `adversarial/`.
2. Enforced: `cargo xtask conformance` and `cargo xtask conformance --suite smoke` run the smoke suite and fail on schema, runner, status, solution, or expected-failure mismatches.
3. Enforced: `cargo xtask check` runs the enforced smoke suite and no longer reports default conformance as a not-enforced hook.
4. Enforced: the runner reports per-fixture and per-category results.
5. Enforced: objective and residual categories appear as `not-applicable` in v0.18.0, not as hidden skipped checks.
6. Enforced: at least one fixture checks expected failure behavior using structured `SolverError` category comparison.
7. Enforced: cross-path numerical comparison uses tolerance-based rules, never bitwise identity.
8. Advisory/not-enforced: `extended` and `adversarial` suites may report placeholders until populated by later work.
9. Future-owner: solver-specific RFCs must add or update fixtures when they introduce new supported problem classes.

## 7. Implementation Closeout

RFC 013 shipped in v0.18.0 as a repository-verification release. Runtime crates
do not parse fixture TOML and did not gain fixture-schema dependencies.

Implemented pieces:

1. Added `conformance/README.md`, three required `conformance/smoke/*.toml`
   fixtures, and README placeholders for `extended/` and `adversarial/`.
2. Promoted `cargo xtask conformance` from an RFC 010 placeholder hook to the
   enforced smoke runner. The default invocation is the smoke suite.
3. Added a host-only TOML/serde fixture loader in `xtask`, materializing the
   dimension-2 diagonal quadratic family against the real RFC 006 device solver
   and the RFC 016 cluster solver.
4. Reported per-fixture and per-category outcomes:
   `status_match`, `solution_within_tolerance`, `expected_failure_match`,
   `objective_within_tolerance`, and `residual_within_tolerance`.
5. Kept objective and residual categories explicit as `not-applicable` for the
   v0.18.0 slice.
6. Updated the aggregate `cargo xtask check` / `release-gate` path so the default
   conformance gate is enforced. The `extended` and `adversarial` placeholder
   suites remain honest `NOT-ENFORCED` reports until populated.
