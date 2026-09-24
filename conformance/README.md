# Loeres Conformance Corpus

This directory contains host-side conformance fixtures. Runtime crates do not
parse these files and do not depend on the fixture schema.

The enforced suite is `smoke/`, covering the shared projected-first-order box
quadratic path across `loeres-device` and `loeres-cluster`.

RFC 013 fixtures use `schema_version = 1` for baseline device/cluster parity.
RFC 017 fixtures use `schema_version = 2` for validation-cache conformance:
cache hit/miss, insufficient scope, stale/wrong evidence rejection,
current-iterate scan retention, hot-loop numerical-domain retention, and
reusable-cache insertion rejection cases.

The `trusted-cache-smoke` fixture group is a corpus-local review tag, not an RFC
011 target-profile group. `pfo-cache-match-001` covers the reusable-cache hit
path only; the provided-evidence happy path remains covered by RFC 015 unit
tests and is later-work conformance scope.

## Difficulty reporting (RFC 031)

Beside each fixture the runner prints, per solve path, the **outer iterations
executed against the configured cap** and, for the constrained kernels, the
**`projection_cap_hits`** and the terminal **`max_constraint_violation`**; each
suite ends with an aggregate (total iterations, the largest fraction of a cap
used, the projection cap-hit rate, the largest violation). This is **reporting
only**. A fixture passes or fails exactly as it did before, and no figure has a
threshold or a budget: measuring comes first.

## Extended and adversarial suites (RFC 031)

`cargo xtask conformance --suite extended` (and `--suite adversarial`) run
schema-3 fixtures above the smoke corpus. **They are reported, not enforced:**
`cargo xtask conformance` and `cargo xtask check` still run smoke only.

`extended/` holds the same geometry at larger sizes than smoke's `n <= 3`,
`m <= 3`: dimensions 4 to 12 and up to six halfspaces, on both kernels wherever
the device kernel has an instantiation for the shape, and on the cluster kernel
alone otherwise (the device kernel is instantiated per shape, so a cluster-only
fixture is how a runtime dimension the smoke corpus never uses is covered).
Outside smoke, `quadratic_diag` may be any positive diagonal.

**Every expected value comes from an independent exact reference, never from a
kernel.** Each fixture's comment states its derivation: an active-set enumeration
over the constraint rows **and** the box faces in `Q`'s metric, in exact rational
arithmetic. A test in `xtask` re-derives every value with a second implementation
on each `cargo test`. An infeasible fixture carries a Farkas certificate
(`infeasibility_certificate`) that proves the rows inconsistent without running a
kernel and bounds the violation from below; the same test checks it.

## Constrained kernels (RFC 027)

`schema_version = 3` fixtures (`problem_class =
"box_quadratic_linear_inequalities"`) run `m >= 1` polyhedra at dimension 2 and 3
with 1-3 halfspaces through **both** the constrained device kernel and the
constrained cluster kernel, against closed-form optima (each derived and
KKT-checked in the fixture's own comment). Every closed form is a Euclidean
projection, so `quadratic_diag` must be all `1.0`. `variant = "infeasible"`
fixtures assert `NotConverged` with `NoProgress` (`Converged` means feasible,
RFC 027 Amendment 5, and needs an uncapped final projection, RFC 033),
`projection_cap_hits > 0` and a violation that does **not shrink** when the sweep
cap is raised tenfold; they must not be reshaped to avoid
exactly-cancelling geometry (RFC 027 section 0.3.4). A `trusted-by-caller` fixture
is cluster-only and also checks that the record says finiteness was trusted.

An optional `[expected] infeasibility_evidence` asserts the kernels' heuristic hint
(RFC 034 Amendment 2) on every path. A fixture states it only where the geometry makes the
answer unambiguous: `true` for the exactly-cancelling infeasible systems, `false` for the
nearly-parallel and barely-feasible families; absent means unasserted. It is a hint, wrong in
both directions, never a status. The runner reports on how many constrained paths it was set.

A new `m0_identity` category runs every schema-1/2 solve fixture through the
constrained cluster kernel with `m = 0` and the fixtures' own oracle, and requires
the result to be identical to RFC 016's up to the sign of zero: numeric equality
plus a NaN check, never raw `to_bits()` (RFC 027 Amendment 4, section 0.4.1).
Under `TrustedByCaller` the error categories must also match; under
`ValidateAllInputs` an RFC 016 failure only requires the constrained kernel to
fail closed too, because it additionally scans `Q` and `c` (RFC 027 section 11.5).

A `feasibility_within_tolerance` category asserts that a feasible fixture's
returned point satisfies its own constraints.
