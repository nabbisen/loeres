# Terms of Engineering Use

This document states the engineering limitations of Loeres for integrators
evaluating it in real systems. It complements — and does not replace or modify —
the warranty disclaimer and limitation of liability in the
[Apache License 2.0](LICENSE), sections 7 and 8. It is engineering guidance, not
legal advice.

Loeres describes itself as suitable for edge and control-adjacent domains,
including robotics, model-predictive control, industrial controllers, and
medical IoT. Those domains carry qualification obligations. This document
records precisely what the project does and does not establish, so that an
integrator can tell the difference.

## No safety certification

Loeres holds **no** safety certification and has not been assessed against any
functional-safety or medical-device standard, including but not limited to
IEC 61508, ISO 26262, IEC 62304, EN 50128, and DO-178C.

No artifact in this repository — no test result, verification gate, release
evidence bundle, or conformance fixture — constitutes evidence of compliance
with any such standard. Producing certification evidence is the integrator's
responsibility and requires work this project has not performed.

## Panic-averse is not panic-free

The project engineers against panics and mechanically checks for them: the
`no_std` production crates are scanned for `unwrap`, `expect`, `panic!`,
`todo!`, and `unimplemented!`, the core forbids `unsafe`, and public solve paths
use fallible access rather than indexing.

These are **panic-averse evidence, not a proof of panic freedom.** The project
states this deliberately and repeatedly (RFC 010, RFC 011, requirements
PANIC-004, ADR-014). No entrypoint in Loeres is documented as formally proven
panic-free, and no such claim may be inferred from a passing gate.

## Determinism is target-scoped

Bounded iteration counts, caller-owned workspaces, and fixed memory footprints
are real properties of the device path. **Bit-for-bit numerical reproducibility
across targets is not.**

Target evidence is classified as mandatory, advisory-installed, or
documented-only (RFC 011). A profile appearing in the manifest does not mean it
was executed. Passing the mandatory profiles does not establish universal
cross-target numerical identity, worst-case execution time, or panic freedom.
Any timing or reproducibility claim an integrator needs must be re-established
on that integrator's target, toolchain, and compiler settings.

Constant-iteration mode stabilizes iteration count. It is **not** cryptographic
constant-time execution and must not be relied on for side-channel resistance.

## Narrow numerical scope

The implemented solver breadth is **one** projected-first-order family, on both
the device and cluster paths, in two forms: over a box, and over a box together
with linear inequalities `Ax <= b` (a quadratic program, RFC 027). `loeres::problem`
defines a storage-agnostic quadratic-program contract. No SOCP contract exists,
and no LP, SOCP, interior-point, or ADMM solver ships. The limits of the
constrained (quadratic-program) kernels are:

- **LP is expressible, not solved.** `Q = 0` is a legal input, but projected
  gradient on a linear objective has no curvature to converge against.
- **Infeasibility is not detected.** An infeasible polyhedron is reported as
  `NotConverged` with `NoProgress` and a positive `max_constraint_violation`
  that does not shrink as the projection cap is raised; it is never an error.
  `Converged` means feasible within `projection_tolerance`.
- **The projection is inexact by design.** It converges linearly at a rate set
  by the angles between constraint normals, and nearly parallel constraints can
  make the inner cap bind routinely. A cap hit is not an error: read
  `projection_cap_hits` and `max_constraint_violation` on the typed entrypoint.
- **The step size is bounded, not chosen for you.** For symmetric positive
  semidefinite `Q`, `curvature_bounds()` returns `U = max_i sum_j |Q_ij|`
  (`U >= lambda_max`) and `L = max_i Q_ii` (`L <= lambda_max`). A step
  `step_scale < 2 / U` is **provably convergent**; a step `step_scale >= 2 / L` is
  **provably divergent** and both constrained kernels reject it as
  `InvalidInput`; the band `2 / U <= step_scale < 2 / L` is **accepted and no
  claim is made** (it holds steps that converge and steps that do not).
  `suggested_step_scale()` returns `1 / U`: always safe, never optimal, and
  **nothing calls it on your behalf**. Both bounds are meaningless unless `Q` is
  symmetric positive semidefinite.
- **The rate is stated in form, not as a number.** With an exact projection, `Q`
  positive **definite** and `step_scale` in `(0, 2 / U)`, the iteration contracts
  linearly by `max(|1 - a*lambda_min|, |1 - a*lambda_max|)` (`a` the step). The
  library computes neither `lambda_min` nor `lambda_max`, so it gives no numeric
  rate; for a merely semidefinite `Q` no rate is claimed.
- **`Q` must be symmetric positive semidefinite.** That is a documented caller
  precondition; it is not verified, because verifying it needs a factorization.
- **Device and cluster agree within tolerance, not bitwise** (RFC 013).

Conformance evidence is a **bounded smoke corpus** — small, fixed dimension.
It is not broad numerical parity, adversarial-input coverage, large-N
validation, ill-conditioning characterization, or throughput evidence.

## Server-side boundaries

Observability is metadata-only and redacted by default; it reduces disclosure
risk but does **not** by itself establish multi-tenant isolation. No
multi-tenant stress or isolation evidence exists.

The gateway surface is a safe mock only. No native or FFI adapter ships, and
enabling the `ffi-gateway` feature does not provide one or discharge the
memory-ownership, licensing, thread-safety, and failure-containment obligations
a real adapter would require.

Validation evidence caching is process-local. It is neither persistent nor
distributed, and it never skips per-call current-iterate or hot-loop
numerical-domain checks.

Worker-panic containment applies only when the host process uses
`panic = "unwind"`. Under `panic = "abort"`, a panic aborts the process and no
containment is promised.

## Pre-1.0 and unpublished

Loeres is pre-1.0. The public API carries no stability guarantee, and breaking
changes may land in any `0.x` release. A `1.0` release requires explicit
project-owner sign-off and has not occurred.

The version of record is the repository release tag. Repository state alone does
not establish registry publication.

## Integrator responsibility

If you deploy Loeres in a system where failure can cause harm, you are
responsible for hazard analysis, requirements qualification, target-specific
verification, timing characterization, numerical validation against your own
problem set, and any certification evidence your domain requires.

Use the project's own scope statements as a starting point for that work, not as
a substitute for it. Where this document and any other project document
disagree, the more conservative reading governs, and the discrepancy should be
reported as a defect.
