# RFC 013 - Design-Freeze Memo

**Artifact.** [`rfcs/done/013-conformance-corpus-and-numerical-parity.md`](../../done/013-conformance-corpus-and-numerical-parity.md)
**Decision pass.** v1, after RFC 011 v0.17.0 target-profile governance.
**Target release.** v0.18.0.
**Scope.** Freeze the first conformance corpus slice before implementation:
fixture format, smoke-suite scope, comparison categories, `cargo xtask
conformance` CLI behavior, and target-profile integration.

---

## Verdict

RFC 013 is the correct next design item. It is now unblocked by RFC 011:
profile names, enforcement classes, and `conformance_group` metadata exist in
`xtask/target-profiles.toml`.

The current proposed RFC is directionally right but stale. It still says the
first corpus may wait until RFC 006 chooses the baseline kernel. That is no
longer true: RFC 006 has shipped the device projected-first-order kernel, RFC
016 has shipped the dynamic cluster analog, and RFC 011 has supplied profile
metadata. RFC 013 should now define a small enforced smoke corpus for that
shared kernel family and keep extended/adversarial breadth staged.

## D1 - Current-repo scope

RFC 013 must be written against the repository as of v0.17.0:

| Area | Current state | RFC 013 implication |
|---|---|---|
| Device projected-first-order | Implemented in RFC 006 behind `loeres-device` + `owned-arrays` | First smoke fixtures can run a real device path. |
| Cluster projected-first-order | Implemented in RFC 016 over `DenseVector` | First smoke fixtures can run a real cluster path. |
| Target profiles | RFC 011 implemented | Fixtures must name `conformance_group` values from `xtask/target-profiles.toml`. |
| `xtask conformance` | Existing hook reports not-enforced when no corpus exists | RFC 013 promotes the smoke suite to enforced. |
| Extended/adversarial corpus | Not present | v0.18.0 may create directories/docs, but only smoke must be enforced. |

## D2 - v0.18.0 implementation slice

The first implementation should be intentionally narrow:

| Suite | v0.18.0 behavior | Rationale |
|---|---|---|
| `smoke` | Enforced by `cargo xtask conformance --suite smoke` and aggregate `cargo xtask check` | Turns the RFC 010 hook into a real gate. |
| `extended` | Directory/readme may exist; command may report `not-enforced` until fixtures are added | Avoids expensive or broad numerical claims in the first slice. |
| `adversarial` | Directory/readme may exist; at least one expected-failure smoke fixture should exist now | Failure behavior is important, but the broad adversarial suite can stage. |

The aggregate gate should run the smoke suite by default. `cargo xtask
conformance` with no arguments should be equivalent to `--suite smoke`.

## D3 - First fixture family

The v0.18.0 smoke corpus should focus on the only solver family implemented on
both sides:

| Field | Value |
|---|---|
| `problem_class` | `box_quadratic_diagonal` |
| `solver_family` | `projected_first_order` |
| device path | `loeres-device::solve_projected_first_order` over `FixedVector<f64, 2>` |
| cluster path | `loeres-cluster::solve_projected_first_order_dyn` over `DenseVector<f64>` |
| dimension | `2` only in v0.18.0 |
| scalar family | `float` / `f64` |
| target groups | `device-reference-smoke`, `cluster-reference-smoke` |

The fixture runner may dispatch on dimension and support only `dimension = 2`
for v0.18.0. Additional dimensions require adding explicit dispatch cases or a
different materialization strategy in a later patch.

The diagonal quadratic problem is:

```text
f(x) = 0.5 * Σ q_i * (x_i - c_i)^2
grad_i(x) = q_i * (x_i - c_i)
x_next_i = clamp(x_i - step_scale * grad_i(x), lower_i, upper_i)
```

This matches both existing kernels' public first-order oracle shape without
adding a modeling DSL.

## D4 - Required smoke fixtures

The first smoke suite should contain at least three fixtures:

| Fixture ID | Purpose | Expected category |
|---|---|---|
| `pfo-box-converged-001` | Well-conditioned 2D diagonal box quadratic where the unconstrained minimizer is inside bounds | `status_match`, `solution_within_tolerance` |
| `pfo-box-not-converged-001` | Same family but cap deliberately too low | `status_match` with `SolveStatus::NotConverged` / `TerminationReason::IterationCap` |
| `pfo-box-invalid-bound-001` | Lower bound greater than upper bound | `expected_failure_match` with `SolverError::InvalidInput` |

The failure fixture satisfies RFC 013's requirement that the corpus validate
failure behavior immediately, without waiting for the full adversarial suite.

## D5 - Fixture format

The first fixture format should be stable and host-only:

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
| `validation_state` | yes | v0.18.0 starts with `validate-all-inputs`. |
| `conformance_groups` | yes | Must include groups emitted by RFC 011 target profiles. |

Required sections:

| Section | Required | Notes |
|---|---:|---|
| `[config]` | yes | `max_iterations`, `tolerance`, `step_scale`. |
| `[problem]` | yes | `lower`, `upper`, `initial`, `quadratic_diag`, `center`; all length `dimension`. |
| `[expected]` | yes | Expected status/error/solution metadata. |
| `[tolerance]` | yes | Solution tolerance required; objective/residual may be `not-applicable`. |

The fixture parser lives in `xtask`; runtime crates must not parse fixture TOML
or depend on fixture schema types.

## D6 - Parser and dependency decision

Use a host-only TOML parser in `xtask` for v0.18.0. The fixture files are
release-gate input, and a bespoke partial TOML parser would add avoidable
correctness and maintenance risk for arrays, floats, sentinels such as
`"not-applicable"`, and future schema extensions.

`xtask` may depend on `toml` / `serde` or equivalent host-only parsing crates.
No runtime crate may depend on fixture schema types or parser crates.

## D7 - Comparison categories

`cargo xtask conformance` must report per-fixture category results. For v0.18.0:

| Category | v0.18.0 status |
|---|---|
| `status_match` | Enforced for all fixtures. |
| `solution_within_tolerance` | Enforced for converged fixtures with an expected solution. |
| `expected_failure_match` | Enforced for error fixtures. |
| `objective_within_tolerance` | Report `not-applicable` because shipped reports do not carry objective values. |
| `residual_within_tolerance` | Report `not-applicable` because the projected-first-order contract has no residual output. |
| `not_comparable` | Allowed only when explicitly declared; not needed for the first three smoke fixtures. |

The command output must not collapse these into a single pass/fail line. It
must name the category that failed.

## D8 - CLI behavior

RFC 013 should freeze the v0.18.0 command contract:

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
| `--suite smoke` | enforced, aggregate-gate path |
| `--suite extended` | if fixtures exist, run them; if absent, report not-enforced/advisory |
| `--suite adversarial` | if fixtures exist, run them; if absent, report not-enforced/advisory |
| unknown args/suite | fail with usage |

The aggregate `cargo xtask check` should invoke the default smoke suite.

## D9 - Runner integration shape

The runner may link to workspace crates from `xtask`:

| Dependency direction | Allowed? | Rationale |
|---|---|---|
| `xtask` -> `loeres`, `loeres-backend-static`, `loeres-device`, `loeres-backend-std`, `loeres-cluster` | yes | Host-only verification crate consuming public APIs. |
| Runtime crates -> `xtask` | no | Would violate the tooling boundary. |
| Device crates -> fixture parser | no | Would pull host parsing into edge crates. |

If `xtask` adds path dependencies for the runner, it must keep them host-only and
default to explicit crate features required for tests, such as
`loeres-device/owned-arrays` and `loeres-backend-static/owned-arrays`.

The runner should materialize device fixture data into `FixedVector<f64, 2>` in
host memory and call the real device kernel. It should materialize cluster data
into `DenseVector<f64>` and call the real cluster typed entrypoint. It should
not generate Rust source files in v0.18.0.

## D10 - Output summary

The command should end with a summary similar to:

```text
[conformance] suite: smoke
  fixtures: 3 total / 3 passed / 0 failed
  status_match: 2 passed / 0 failed / 1 not-applicable
  solution_within_tolerance: 1 passed / 0 failed / 2 not-applicable
  expected_failure_match: 1 passed / 0 failed / 2 not-applicable
  objective_within_tolerance: 0 passed / 0 failed / 3 not-applicable
  residual_within_tolerance: 0 passed / 0 failed / 3 not-applicable
[conformance] PASS
```

The release-gate summary can still show `conformance: pass`, because once RFC
013 lands the smoke suite is enforced. It must no longer say
`not-enforced hook ready` for the default aggregate path.

## D11 - Required RFC patch before implementation

Before implementation starts, patch RFC 013 to:

1. replace stale "until RFC 006 chooses the baseline kernel" language with the
   v0.17.0 repo state;
2. make the v0.18.0 smoke suite enforced and default;
3. freeze the `box_quadratic_diagonal` fixture format and dimension-2 scope;
4. list the three required smoke fixtures;
5. classify objective/residual as `not-applicable` for v0.18.0;
6. define CLI behavior for `--suite`;
7. explicitly allow host-only TOML parsing in `xtask`;
8. state that `xtask` may link workspace crates host-side but runtime crates may
   not parse fixtures;
9. state that trusted-input fixtures are deferred until RFC 015;
10. update acceptance gates to distinguish enforced smoke, optional extended, and
   optional adversarial behavior.

## D12 - Implementation checklist after RFC patch

After the RFC patch is reviewed:

- Add `conformance/README.md`.
- Add `conformance/smoke/*.toml` for the three required smoke fixtures.
- Replace `xtask/src/checks/conformance.rs` hook logic with fixture loading,
  parser validation, device/cluster runner, category comparison, and suite CLI.
- Add parser/runner tests in `xtask`.
- Update `xtask/src/main.rs` to pass trailing args into conformance or otherwise
  support `--suite`.
- Update `release_gate` summary classification so conformance is an enforced
  pass after RFC 013.
- Move RFC 013 to `done/` only after `cargo xtask conformance --suite smoke`,
  `cargo xtask check`, full workspace tests, and clean-copy aggregate checks
  pass.

## Review decisions

The v0.1 design-freeze review settled the open questions:

1. Use a host-only TOML parser in `xtask`; do not hand-roll fixture TOML parsing.
2. The v0.18.0 smoke suite uses `ValidateAllInputs` only. Trusted-input fixtures
   wait for RFC 015-owned trust/caching semantics.
3. Create `extended/` and `adversarial/` as README-backed placeholders in
   v0.18.0.

## Gates for this memo

This memo is a design-freeze artifact only. It does not move RFC 013 out of
`proposed/` and does not implement the corpus. The next step is an RFC 013 text
patch following D11, then review before implementation.
