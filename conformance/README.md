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

`extended/` and `adversarial/` are staged placeholders.

## Constrained kernels (RFC 027)

`schema_version = 3` fixtures (`problem_class =
"box_quadratic_linear_inequalities"`) run `m >= 1` polyhedra at dimension 2 and 3
with 1-3 halfspaces through **both** the constrained device kernel and the
constrained cluster kernel, against closed-form optima (each derived and
KKT-checked in the fixture's own comment). Every closed form is a Euclidean
projection, so `quadratic_diag` must be all `1.0`. `variant = "infeasible"`
fixtures assert `NotConverged` with `NoProgress` (`Converged` means feasible,
RFC 027 Amendment 5), `projection_cap_hits > 0` and a violation that does **not
shrink** when the sweep cap is raised tenfold; they must not be reshaped to avoid
exactly-cancelling geometry (RFC 027 section 0.3.4). A `trusted-by-caller` fixture
is cluster-only and also checks that the record says finiteness was trusted.

A new `m0_identity` category runs every schema-1/2 solve fixture through the
constrained cluster kernel with `m = 0` and the fixtures' own oracle, and requires
the result to be identical to RFC 016's up to the sign of zero: numeric equality
plus a NaN check, never raw `to_bits()` (RFC 027 Amendment 4, section 0.4.1).
Under `TrustedByCaller` the error categories must also match; under
`ValidateAllInputs` an RFC 016 failure only requires the constrained kernel to
fail closed too, because it additionally scans `Q` and `c` (RFC 027 section 11.5).

A `feasibility_within_tolerance` category asserts that a feasible fixture's
returned point satisfies its own constraints.
