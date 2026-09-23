//! RFC 027 S4 conformance: the constrained projected first-order kernels.
//!
//! Two independent pieces, both run by the `smoke` suite:
//!
//! 1. **`m0_identity`** — every existing schema-1/2 fixture is also run through
//!    the constrained *cluster* kernel with `m = 0` and the **same problem
//!    oracle** (`qᵢ·(xᵢ − cᵢ)`, installed by overriding
//!    `QuadraticObjective::gradient_into`), and compared with RFC 016. The two
//!    must be identical **up to the sign of zero**: numeric equality plus a NaN
//!    check, never raw `to_bits()` (RFC 027 Amendment 4, §0.4.1). Fixtures are
//!    deliberately *not* re-expressed as a `Qx + c` `QuadraticProgram` for this
//!    comparison — that oracle differs from `q·(x − t)` in floating point
//!    (§0.2.5), so it would be tolerance-only.
//! 2. **Schema-3 fixtures** — `m ≥ 1` polyhedra at dimension 2 and 3 with 1–3
//!    halfspaces, checked against closed-form optima on the constrained device
//!    *and* cluster kernels, plus an infeasible polyhedron, an all-zero
//!    constraint row, a trust-skip case and a hot-loop NaN case.

use loeres::validation::{TrustToken, TrustedByCaller, ValidationScope};
use loeres::{
    BoxBounds, LinearInequalities, MatrixView, QuadraticObjective, SolveReport, SolveStatus,
    SolverError, TerminationReason, VectorAccess, VectorAccessMut, VectorView,
};
use loeres_backend_static::array::{FixedMatrix, FixedVector};
use loeres_backend_std::{DenseMatrix, DenseVector};
use loeres_cluster::{
    ClusterCancellationToken, ClusterConstrainedWorkspace, ClusterExecutionContext,
    ClusterProjectedFirstOrderWorkspace, ClusterValidationPolicy, ConstrainedProjectedConfig,
    ProjectedFirstOrderConfig, ProjectedFirstOrderFiniteEvidence,
    solve_constrained_projected_first_order_dyn, solve_projected_first_order_dyn,
};
use loeres_device::config::TimingMode;
use loeres_device::solve::{
    ConstrainedProjectedWorkspace, ConstrainedSolveConfig, solve_constrained_projected_first_order,
};
use serde::Deserialize;

use super::{CategoryResult, ClusterDiagonalProblem, ExecutionMode, Fixture, within_tolerance};

// ---------------------------------------------------------------------------
// 1. m = 0 identity against RFC 016, across every existing fixture
// ---------------------------------------------------------------------------

type Solved = (SolveReport, [f64; 2]);

/// A `m = 0` quadratic program whose first-order oracle is the fixtures' own
/// `qᵢ·(xᵢ − cᵢ)`. `Q` and `c` are still supplied honestly (so the finite scan
/// sees the same data) but are not what the gradient reads.
struct FixtureOracleProgram {
    q: DenseMatrix<f64>,
    c: DenseVector<f64>,
    lo: DenseVector<f64>,
    hi: DenseVector<f64>,
    a: MatrixView<'static, f64>,
    b: VectorView<'static, f64>,
    q_diag: [f64; 2],
    center: [f64; 2],
}

impl QuadraticObjective<f64> for FixtureOracleProgram {
    type Hessian = DenseMatrix<f64>;
    type Linear = DenseVector<f64>;
    fn hessian(&self) -> &Self::Hessian {
        &self.q
    }
    fn linear_term(&self) -> &Self::Linear {
        &self.c
    }
    fn gradient_into<X, G>(&self, x: &X, grad: &mut G) -> Result<(), SolverError>
    where
        X: VectorAccess<Scalar = f64>,
        G: VectorAccessMut<Scalar = f64>,
    {
        for i in 0..2 {
            grad.set(i, self.q_diag[i] * (x.get(i)? - self.center[i]))?;
        }
        Ok(())
    }
}

impl BoxBounds<f64> for FixtureOracleProgram {
    type Bound = DenseVector<f64>;
    fn lower_bounds(&self) -> &Self::Bound {
        &self.lo
    }
    fn upper_bounds(&self) -> &Self::Bound {
        &self.hi
    }
}

impl LinearInequalities<f64> for FixtureOracleProgram {
    type Constraints = MatrixView<'static, f64>;
    type Rhs = VectorView<'static, f64>;
    fn constraint_matrix(&self) -> &Self::Constraints {
        &self.a
    }
    fn constraint_rhs(&self) -> &Self::Rhs {
        &self.b
    }
}

fn policy_context(policy: ClusterValidationPolicy) -> ClusterExecutionContext {
    ClusterExecutionContext::new(ClusterCancellationToken::new(), 0, policy)
}

fn identity_policies() -> [(&'static str, ClusterValidationPolicy); 2] {
    let trust = TrustedByCaller::caller_assertion(
        ValidationScope::FINITE,
        TrustToken::new(27),
        Some("rfc027-s4-identity"),
    );
    [
        (
            "validate-all-inputs",
            ClusterValidationPolicy::ValidateAllInputs,
        ),
        (
            "trusted-by-caller",
            ClusterValidationPolicy::TrustedByCaller(trust),
        ),
    ]
}

fn run_rfc016(
    fixture: &Fixture,
    policy: ClusterValidationPolicy,
) -> Result<Result<Solved, SolverError>, String> {
    let problem = ClusterDiagonalProblem::new(fixture)?;
    let mut x = super::dense(&fixture.problem.initial)?;
    let mut workspace = ClusterProjectedFirstOrderWorkspace::new(2)
        .map_err(|e| format!("cluster workspace init failed: {e:?}"))?;
    let config = ProjectedFirstOrderConfig {
        max_iterations: fixture.config.max_iterations,
        tolerance: fixture.config.tolerance,
    };
    Ok(solve_projected_first_order_dyn(
        &problem,
        &mut x,
        &mut workspace,
        &config,
        &policy_context(policy),
    )
    .and_then(|record| Ok((record.report, pair(&x)?))))
}

fn run_constrained_m0(
    fixture: &Fixture,
    policy: ClusterValidationPolicy,
) -> Result<Result<Solved, SolverError>, String> {
    const EMPTY: &[f64] = &[];
    let q_diag = *super::array_ref(&fixture.problem.quadratic_diag)?;
    let center = *super::array_ref(&fixture.problem.center)?;
    let problem = FixtureOracleProgram {
        q: DenseMatrix::from_row_major_vec(2, 2, vec![q_diag[0], 0.0, 0.0, q_diag[1]])
            .map_err(|e| format!("Q init failed: {e:?}"))?,
        c: super::dense(&[-q_diag[0] * center[0], -q_diag[1] * center[1]])?,
        lo: super::dense(&fixture.problem.lower)?,
        hi: super::dense(&fixture.problem.upper)?,
        a: MatrixView::from_row_major(EMPTY, 0, 2).map_err(|e| format!("A init failed: {e:?}"))?,
        b: VectorView::from_slice(EMPTY),
        q_diag,
        center,
    };
    let mut x = super::dense(&fixture.problem.initial)?;
    let mut workspace = ClusterConstrainedWorkspace::new(2, 0)
        .map_err(|e| format!("constrained workspace init failed: {e:?}"))?;
    let config = ConstrainedProjectedConfig {
        max_iterations: fixture.config.max_iterations,
        tolerance: fixture.config.tolerance,
        projection_max_sweeps: 1,
        projection_tolerance: 1e-12,
    };
    Ok(solve_constrained_projected_first_order_dyn(
        &problem,
        fixture.config.step_scale,
        &mut x,
        &mut workspace,
        &config,
        &policy_context(policy),
    )
    .and_then(|record| Ok((record.report, pair(&x)?))))
}

fn pair(x: &DenseVector<f64>) -> Result<[f64; 2], SolverError> {
    Ok([x.get(0)?, x.get(1)?])
}

/// RFC 027 §0.4.1: numeric equality plus a NaN check. `+0.0` and `−0.0` are
/// identical; `NaN` is identical to nothing, itself included.
fn identical(a: f64, b: f64) -> bool {
    !a.is_nan() && !b.is_nan() && a == b
}

/// Compare one fixture, under one policy. `Ok(())` when the constrained
/// `m = 0` run is identical to RFC 016's under §0.4.1.
///
/// An `Err` from RFC 016 requires only that the constrained kernel also fail
/// closed, not that it fail with the same category: the constrained kernel
/// additionally scans `Q` and `c` (RFC 027 §11.5), so a fixture whose model data
/// is non-finite is rejected at the scan (`NonFiniteInput`) where RFC 016 only
/// meets it in the hot loop (`NumericalDomain`). Under `TrustedByCaller` the
/// scans are skipped on both sides and the categories must match exactly.
fn compare_identity(
    policy_name: &str,
    trusted: bool,
    reference: &Result<Solved, SolverError>,
    constrained: &Result<Solved, SolverError>,
) -> Result<(), String> {
    match (reference, constrained) {
        (Ok((ref_report, ref_x)), Ok((got_report, got_x))) => {
            if ref_report != got_report {
                return Err(format!(
                    "{policy_name}: report {got_report:?} differs from RFC 016's {ref_report:?}"
                ));
            }
            for i in 0..2 {
                if !identical(ref_x[i], got_x[i]) {
                    return Err(format!(
                        "{policy_name}: coordinate {i} is {:?}, RFC 016 has {:?}",
                        got_x[i], ref_x[i]
                    ));
                }
            }
            Ok(())
        }
        (Err(a), Err(b)) if trusted && a != b => Err(format!(
            "{policy_name}: constrained error {b:?} differs from RFC 016's {a:?} with no scans on either side"
        )),
        (Err(_), Err(_)) => Ok(()),
        (Ok(_), Err(e)) => Err(format!(
            "{policy_name}: RFC 016 solved but the constrained kernel failed with {e:?}"
        )),
        (Err(e), Ok(_)) => Err(format!(
            "{policy_name}: RFC 016 failed with {e:?} but the constrained kernel solved"
        )),
    }
}

/// The `m0_identity` category for one schema-1/2 fixture. Cache-insert fixtures
/// have no solve to compare.
pub(super) fn m0_identity(fixture: &Fixture) -> Result<CategoryResult, String> {
    if fixture.schema_version == 2 && fixture.execution_mode()? == ExecutionMode::CacheInsert {
        return Ok(CategoryResult::NotApplicable);
    }
    for (name, policy) in identity_policies() {
        let trusted = matches!(policy, ClusterValidationPolicy::TrustedByCaller(_));
        let reference = run_rfc016(fixture, policy)?;
        let constrained = run_constrained_m0(fixture, policy)?;
        if let Err(detail) = compare_identity(name, trusted, &reference, &constrained) {
            return Ok(CategoryResult::Fail(detail));
        }
    }
    Ok(CategoryResult::Pass)
}

// ---------------------------------------------------------------------------
// 2. Schema-3 fixtures
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Deserialize)]
pub(super) struct ConstrainedFixture {
    pub(super) schema_version: u32,
    pub(super) fixture_id: String,
    suite: String,
    problem_class: String,
    solver_family: String,
    dimension: usize,
    constraints: usize,
    scalar_family: String,
    validation_state: String,
    conformance_groups: Vec<String>,
    variant: String,
    config: ConstrainedConfig,
    problem: ConstrainedProblem,
    expected: ConstrainedExpected,
    tolerance: ConstrainedTolerance,
}

#[derive(Clone, Debug, Deserialize)]
struct ConstrainedConfig {
    max_iterations: u32,
    tolerance: f64,
    step_scale: f64,
    projection_max_sweeps: u32,
    projection_tolerance: f64,
}

#[derive(Clone, Debug, Deserialize)]
struct ConstrainedProblem {
    lower: Vec<f64>,
    upper: Vec<f64>,
    initial: Vec<f64>,
    quadratic_diag: Vec<f64>,
    center: Vec<f64>,
    /// Row-major `constraints × dimension`.
    constraint_matrix: Vec<f64>,
    constraint_rhs: Vec<f64>,
}

#[derive(Clone, Debug, Deserialize)]
struct ConstrainedExpected {
    status: String,
    termination: String,
    solution: Vec<f64>,
    error: String,
    /// Feasible fixtures: the returned point may violate its constraints by at
    /// most this much.
    #[serde(default)]
    violation_max: Option<f64>,
    /// Infeasible fixtures: the returned point must violate by at least this.
    #[serde(default)]
    violation_min: Option<f64>,
}

#[derive(Clone, Debug, Deserialize)]
struct ConstrainedTolerance {
    solution_abs: f64,
    solution_rel: f64,
}

impl ConstrainedFixture {
    #[cfg(test)]
    pub(super) fn expected_solution_for_test(&mut self, solution: Vec<f64>) {
        self.expected.solution = solution;
    }

    #[cfg(test)]
    pub(super) fn expected_status_for_test(&mut self, status: &str) {
        self.expected.status = status.to_owned();
    }

    pub(super) fn validate(&self, suite: &str) -> Result<(), String> {
        let id = &self.fixture_id;
        let expect = |ok: bool, what: &str| {
            if ok {
                Ok(())
            } else {
                Err(format!("{id}: {what}"))
            }
        };
        expect(self.schema_version == 3, "schema_version must be 3")?;
        expect(self.suite == suite, "suite mismatch")?;
        expect(
            self.problem_class == "box_quadratic_linear_inequalities",
            "problem_class must be box_quadratic_linear_inequalities",
        )?;
        expect(
            self.solver_family == "constrained_projected_first_order",
            "solver_family must be constrained_projected_first_order",
        )?;
        expect(self.scalar_family == "float", "scalar_family must be float")?;
        expect(
            matches!(self.dimension, 2 | 3),
            "dimension must be 2 or 3 (the device kernel is instantiated per shape)",
        )?;
        expect(
            (1..=3).contains(&self.constraints),
            "constraints must be 1..=3 (m = 0 is the RFC 006 entrypoint on device and is covered by m0_identity)",
        )?;
        expect(
            matches!(
                self.validation_state.as_str(),
                "validate-all-inputs" | "trusted-by-caller"
            ),
            "unknown validation_state",
        )?;
        expect(
            matches!(self.variant.as_str(), "solve" | "infeasible"),
            "variant must be solve or infeasible",
        )?;
        expect(
            self.conformance_groups
                .iter()
                .any(|g| g == "cluster-reference-smoke"),
            "conformance_groups must contain cluster-reference-smoke",
        )?;
        let (n, m) = (self.dimension, self.constraints);
        for (name, len, want) in [
            ("lower", self.problem.lower.len(), n),
            ("upper", self.problem.upper.len(), n),
            ("initial", self.problem.initial.len(), n),
            ("quadratic_diag", self.problem.quadratic_diag.len(), n),
            ("center", self.problem.center.len(), n),
            (
                "constraint_matrix",
                self.problem.constraint_matrix.len(),
                n * m,
            ),
            ("constraint_rhs", self.problem.constraint_rhs.len(), m),
        ] {
            expect(
                len == want,
                &format!("{name} has {len} values, expected {want}"),
            )?;
        }
        expect(
            self.problem.quadratic_diag.iter().all(|q| *q == 1.0),
            "quadratic_diag must be all 1.0: closed-form optima here are Euclidean projections, which holds only for Q = I",
        )?;
        expect(
            self.config.max_iterations > 0 && self.config.projection_max_sweeps > 0,
            "caps must be > 0",
        )?;
        expect(
            self.expected.solution.is_empty() || self.expected.solution.len() == n,
            "expected solution length must equal dimension",
        )?;
        Ok(())
    }

    fn has_device(&self) -> bool {
        self.conformance_groups
            .iter()
            .any(|g| g == "device-reference-smoke")
    }

    fn policy(&self) -> ClusterValidationPolicy {
        if self.validation_state == "trusted-by-caller" {
            ClusterValidationPolicy::TrustedByCaller(TrustedByCaller::caller_assertion(
                ValidationScope::FINITE,
                TrustToken::new(28),
                Some("rfc027-s4-fixture"),
            ))
        } else {
            ClusterValidationPolicy::ValidateAllInputs
        }
    }
}

/// One path's outcome.
#[derive(Clone, Debug)]
struct PathRun {
    result: Result<PathSolved, SolverError>,
}

#[derive(Clone, Debug)]
struct PathSolved {
    report: SolveReport,
    x: Vec<f64>,
    projection_cap_hits: u32,
    max_constraint_violation: f64,
    /// Cluster only: whether the record says finiteness was trusted.
    finite_trusted: Option<bool>,
}

// --- device -----------------------------------------------------------------

struct DeviceQp<const N: usize, const M: usize, const NN: usize, const MN: usize> {
    q: FixedMatrix<f64, N, N, NN>,
    c: FixedVector<f64, N>,
    lo: FixedVector<f64, N>,
    hi: FixedVector<f64, N>,
    a: FixedMatrix<f64, M, N, MN>,
    b: FixedVector<f64, M>,
}

impl<const N: usize, const M: usize, const NN: usize, const MN: usize> QuadraticObjective<f64>
    for DeviceQp<N, M, NN, MN>
{
    type Hessian = FixedMatrix<f64, N, N, NN>;
    type Linear = FixedVector<f64, N>;
    fn hessian(&self) -> &Self::Hessian {
        &self.q
    }
    fn linear_term(&self) -> &Self::Linear {
        &self.c
    }
}

impl<const N: usize, const M: usize, const NN: usize, const MN: usize> BoxBounds<f64>
    for DeviceQp<N, M, NN, MN>
{
    type Bound = FixedVector<f64, N>;
    fn lower_bounds(&self) -> &Self::Bound {
        &self.lo
    }
    fn upper_bounds(&self) -> &Self::Bound {
        &self.hi
    }
}

impl<const N: usize, const M: usize, const NN: usize, const MN: usize> LinearInequalities<f64>
    for DeviceQp<N, M, NN, MN>
{
    type Constraints = FixedMatrix<f64, M, N, MN>;
    type Rhs = FixedVector<f64, M>;
    fn constraint_matrix(&self) -> &Self::Constraints {
        &self.a
    }
    fn constraint_rhs(&self) -> &Self::Rhs {
        &self.b
    }
}

fn to_array<const K: usize>(values: &[f64]) -> Result<[f64; K], String> {
    <[f64; K]>::try_from(values).map_err(|_| format!("expected {K} values, found {}", values.len()))
}

fn run_device_shape<const N: usize, const M: usize, const NN: usize, const MN: usize>(
    f: &ConstrainedFixture,
    projection_max_sweeps: u32,
) -> Result<PathRun, String> {
    let p = &f.problem;
    let mut q = [0.0; NN];
    for (i, d) in p.quadratic_diag.iter().enumerate() {
        q[i * N + i] = *d;
    }
    let center = to_array::<N>(&p.center)?;
    let mut c = [0.0; N];
    for ((slot, q), t) in c.iter_mut().zip(&p.quadratic_diag).zip(&center) {
        *slot = -q * t;
    }
    let problem = DeviceQp::<N, M, NN, MN> {
        q: FixedMatrix::from_row_major_array(q),
        c: FixedVector::from_array(c),
        lo: FixedVector::from_array(to_array::<N>(&p.lower)?),
        hi: FixedVector::from_array(to_array::<N>(&p.upper)?),
        a: FixedMatrix::from_row_major_array(to_array::<MN>(&p.constraint_matrix)?),
        b: FixedVector::from_array(to_array::<M>(&p.constraint_rhs)?),
    };
    let mut x = FixedVector::from_array(to_array::<N>(&p.initial)?);
    let mut workspace = ConstrainedProjectedWorkspace::<f64, N, M>::new(
        FixedVector::from_array([0.0; N]),
        FixedVector::from_array([0.0; N]),
        FixedVector::from_array([0.0; N]),
        FixedVector::from_array([0.0; M]),
        FixedVector::from_array([0.0; M]),
    );
    let config = ConstrainedSolveConfig {
        max_iterations: f.config.max_iterations,
        tolerance: f.config.tolerance,
        timing_mode: TimingMode::EarlyExitAllowed,
        projection_max_sweeps,
        projection_tolerance: f.config.projection_tolerance,
    };
    Ok(PathRun {
        result: solve_constrained_projected_first_order(
            &problem,
            f.config.step_scale,
            &mut x,
            &mut workspace,
            &config,
        )
        .map(|report| PathSolved {
            report: report.core(),
            x: x.as_slice().to_vec(),
            projection_cap_hits: report.projection_cap_hits(),
            max_constraint_violation: report.max_constraint_violation(),
            finite_trusted: None,
        }),
    })
}

fn run_device(f: &ConstrainedFixture, sweeps: u32) -> Result<PathRun, String> {
    match (f.dimension, f.constraints) {
        (2, 1) => run_device_shape::<2, 1, 4, 2>(f, sweeps),
        (2, 2) => run_device_shape::<2, 2, 4, 4>(f, sweeps),
        (2, 3) => run_device_shape::<2, 3, 4, 6>(f, sweeps),
        (3, 1) => run_device_shape::<3, 1, 9, 3>(f, sweeps),
        (3, 2) => run_device_shape::<3, 2, 9, 6>(f, sweeps),
        (3, 3) => run_device_shape::<3, 3, 9, 9>(f, sweeps),
        (n, m) => Err(format!(
            "no device instantiation for dimension {n}, constraints {m}"
        )),
    }
}

// --- cluster ----------------------------------------------------------------

struct ClusterQp {
    q: DenseMatrix<f64>,
    c: DenseVector<f64>,
    lo: DenseVector<f64>,
    hi: DenseVector<f64>,
    a: DenseMatrix<f64>,
    b: DenseVector<f64>,
}

impl QuadraticObjective<f64> for ClusterQp {
    type Hessian = DenseMatrix<f64>;
    type Linear = DenseVector<f64>;
    fn hessian(&self) -> &Self::Hessian {
        &self.q
    }
    fn linear_term(&self) -> &Self::Linear {
        &self.c
    }
}

impl BoxBounds<f64> for ClusterQp {
    type Bound = DenseVector<f64>;
    fn lower_bounds(&self) -> &Self::Bound {
        &self.lo
    }
    fn upper_bounds(&self) -> &Self::Bound {
        &self.hi
    }
}

impl LinearInequalities<f64> for ClusterQp {
    type Constraints = DenseMatrix<f64>;
    type Rhs = DenseVector<f64>;
    fn constraint_matrix(&self) -> &Self::Constraints {
        &self.a
    }
    fn constraint_rhs(&self) -> &Self::Rhs {
        &self.b
    }
}

fn run_cluster(f: &ConstrainedFixture, sweeps: u32) -> Result<PathRun, String> {
    let p = &f.problem;
    let (n, m) = (f.dimension, f.constraints);
    let mut q = vec![0.0; n * n];
    for (i, d) in p.quadratic_diag.iter().enumerate() {
        q[i * n + i] = *d;
    }
    let c: Vec<f64> = p
        .quadratic_diag
        .iter()
        .zip(&p.center)
        .map(|(q, t)| -q * t)
        .collect();
    let dm = |rows: usize, cols: usize, data: Vec<f64>| {
        DenseMatrix::from_row_major_vec(rows, cols, data).map_err(|e| format!("{e:?}"))
    };
    let problem = ClusterQp {
        q: dm(n, n, q)?,
        c: super::dense(&c)?,
        lo: super::dense(&p.lower)?,
        hi: super::dense(&p.upper)?,
        a: dm(m, n, p.constraint_matrix.clone())?,
        b: super::dense(&p.constraint_rhs)?,
    };
    let mut x = super::dense(&p.initial)?;
    let mut workspace = ClusterConstrainedWorkspace::new(n, m)
        .map_err(|e| format!("constrained workspace init failed: {e:?}"))?;
    let config = ConstrainedProjectedConfig {
        max_iterations: f.config.max_iterations,
        tolerance: f.config.tolerance,
        projection_max_sweeps: sweeps,
        projection_tolerance: f.config.projection_tolerance,
    };
    let result = solve_constrained_projected_first_order_dyn(
        &problem,
        f.config.step_scale,
        &mut x,
        &mut workspace,
        &config,
        &policy_context(f.policy()),
    )
    .and_then(|record| {
        let mut coords = Vec::with_capacity(n);
        for i in 0..n {
            coords.push(x.get(i)?);
        }
        Ok(PathSolved {
            report: record.report,
            x: coords,
            projection_cap_hits: record.projection_cap_hits,
            max_constraint_violation: record.max_constraint_violation,
            finite_trusted: Some(matches!(
                record.finite,
                ProjectedFirstOrderFiniteEvidence::Trusted(_)
            )),
        })
    });
    Ok(PathRun { result })
}

// --- comparison ---------------------------------------------------------------

/// Everything a schema-3 fixture reports, in the runner's category vocabulary.
pub(super) struct ConstrainedResult {
    pub(super) status_match: CategoryResult,
    pub(super) solution_within_tolerance: CategoryResult,
    pub(super) expected_failure_match: CategoryResult,
    pub(super) feasibility_within_tolerance: CategoryResult,
}

pub(super) fn run_constrained_fixture(f: &ConstrainedFixture) -> Result<ConstrainedResult, String> {
    let sweeps = f.config.projection_max_sweeps;
    let mut paths: Vec<(&'static str, PathRun)> = Vec::new();
    if f.has_device() {
        paths.push(("device", run_device(f, sweeps)?));
    }
    paths.push(("cluster", run_cluster(f, sweeps)?));

    let mut result = ConstrainedResult {
        status_match: CategoryResult::NotApplicable,
        solution_within_tolerance: CategoryResult::NotApplicable,
        expected_failure_match: CategoryResult::NotApplicable,
        feasibility_within_tolerance: CategoryResult::NotApplicable,
    };

    if f.expected.error != "none" {
        result.expected_failure_match = compare_failure(f, &paths);
        return Ok(result);
    }
    result.status_match = compare_status(f, &paths);
    if !f.expected.solution.is_empty() {
        result.solution_within_tolerance = compare_solution(f, &paths);
    }
    result.feasibility_within_tolerance = compare_feasibility(f, &paths, sweeps)?;
    Ok(result)
}

fn solved<'a>(name: &str, run: &'a PathRun) -> Result<&'a PathSolved, String> {
    run.result
        .as_ref()
        .map_err(|e| format!("{name} failed with {e:?}"))
}

fn compare_status(f: &ConstrainedFixture, paths: &[(&'static str, PathRun)]) -> CategoryResult {
    // Only an infeasible fixture may leave the status unasserted, and it is
    // held to the feasibility category instead (cap hit, violation, no shrink).
    if f.expected.status == "not-asserted" {
        return if f.variant == "infeasible" {
            CategoryResult::NotApplicable
        } else {
            CategoryResult::Fail(
                "status may only be not-asserted for an infeasible fixture".to_owned(),
            )
        };
    }
    let status = match f.expected.status.as_str() {
        "converged" => SolveStatus::Converged,
        "not-converged" => SolveStatus::NotConverged,
        other => return CategoryResult::Fail(format!("unknown expected status `{other}`")),
    };
    let termination = match f.expected.termination.as_str() {
        "convergence-criterion" => TerminationReason::ConvergenceCriterion,
        "iteration-cap" => TerminationReason::IterationCap,
        other => return CategoryResult::Fail(format!("unknown expected termination `{other}`")),
    };
    for (name, run) in paths {
        match solved(name, run) {
            Ok(s) if s.report.status() == status && s.report.termination() == termination => {}
            Ok(s) => {
                return CategoryResult::Fail(format!(
                    "{name} reported ({:?}, {:?}), expected ({status:?}, {termination:?})",
                    s.report.status(),
                    s.report.termination()
                ));
            }
            Err(e) => return CategoryResult::Fail(e),
        }
    }
    CategoryResult::Pass
}

fn compare_solution(f: &ConstrainedFixture, paths: &[(&'static str, PathRun)]) -> CategoryResult {
    let (abs, rel) = (f.tolerance.solution_abs, f.tolerance.solution_rel);
    let mut reference: Option<(&str, &Vec<f64>)> = None;
    for (name, run) in paths {
        let s = match solved(name, run) {
            Ok(s) => s,
            Err(e) => return CategoryResult::Fail(e),
        };
        for (i, (got, want)) in s.x.iter().zip(&f.expected.solution).enumerate() {
            if !within_tolerance(*got, *want, abs, rel) {
                return CategoryResult::Fail(format!(
                    "{name} solution[{i}]={got} expected={want} abs={abs} rel={rel}"
                ));
            }
        }
        if let Some((ref_name, ref_x)) = reference {
            for (i, (a, b)) in ref_x.iter().zip(&s.x).enumerate() {
                if !within_tolerance(*b, *a, abs, rel) {
                    return CategoryResult::Fail(format!(
                        "cross-path solution[{i}] {ref_name}={a} {name}={b} abs={abs} rel={rel}"
                    ));
                }
            }
        } else {
            reference = Some((name, &s.x));
        }
    }
    CategoryResult::Pass
}

fn compare_failure(f: &ConstrainedFixture, paths: &[(&'static str, PathRun)]) -> CategoryResult {
    let expected = match f.expected.error.as_str() {
        "invalid-input" => SolverError::InvalidInput,
        "non-finite-input" => SolverError::NonFiniteInput,
        "numerical-domain" => SolverError::NumericalDomain,
        "overflow" => SolverError::Overflow,
        other => return CategoryResult::Fail(format!("unknown expected error `{other}`")),
    };
    for (name, run) in paths {
        match &run.result {
            Err(e) if *e == expected => {}
            Err(e) => {
                return CategoryResult::Fail(format!("{name} error={e:?}, expected={expected:?}"));
            }
            Ok(s) => {
                return CategoryResult::Fail(format!(
                    "expected {name} error {expected:?}, got report {:?}",
                    s.report
                ));
            }
        }
    }
    CategoryResult::Pass
}

/// The feasibility category.
///
/// * Feasible fixtures (`violation_max`): the returned point violates its own
///   constraints by at most that much, no projection hit its cap, and — for a
///   trusted fixture — the record says finiteness was trusted.
/// * Infeasible fixtures (`violation_min`): the projection hits its cap
///   (`projection_cap_hits > 0`, RFC 027 §0.3.4) and the violation is **not
///   shrinking**: rerunning at ten times the sweep cap must not reduce it, and
///   it must stay at least `violation_min`. A polyhedron with no feasible point
///   cannot have a shrinking violation; one that did would be converging on a
///   point it should not be able to reach.
fn compare_feasibility(
    f: &ConstrainedFixture,
    paths: &[(&'static str, PathRun)],
    sweeps: u32,
) -> Result<CategoryResult, String> {
    if let Some(max) = f.expected.violation_max {
        for (name, run) in paths {
            let s = solved(name, run)?;
            if s.max_constraint_violation > max {
                return Ok(CategoryResult::Fail(format!(
                    "{name} violation {} exceeds {max}",
                    s.max_constraint_violation
                )));
            }
            if s.projection_cap_hits != 0 {
                return Ok(CategoryResult::Fail(format!(
                    "{name} hit the projection cap {} time(s) on a feasible fixture",
                    s.projection_cap_hits
                )));
            }
            if f.validation_state == "trusted-by-caller"
                && *name == "cluster"
                && s.finite_trusted != Some(true)
            {
                return Ok(CategoryResult::Fail(
                    "cluster record does not say finiteness was trusted".to_owned(),
                ));
            }
        }
    }
    if let Some(min) = f.expected.violation_min {
        let longer = sweeps.saturating_mul(10);
        for (name, run) in paths {
            let short = solved(name, run)?;
            let long_run = if *name == "device" {
                run_device(f, longer)?
            } else {
                run_cluster(f, longer)?
            };
            let long = solved(name, &long_run)?;
            if short.projection_cap_hits == 0 || long.projection_cap_hits == 0 {
                return Ok(CategoryResult::Fail(format!(
                    "{name}: an infeasible polyhedron must hit the projection cap (short {}, long {})",
                    short.projection_cap_hits, long.projection_cap_hits
                )));
            }
            if short.max_constraint_violation < min || long.max_constraint_violation < min {
                return Ok(CategoryResult::Fail(format!(
                    "{name}: violation {} / {} is below the required {min}",
                    short.max_constraint_violation, long.max_constraint_violation
                )));
            }
            if long.max_constraint_violation < short.max_constraint_violation - 1e-9 {
                return Ok(CategoryResult::Fail(format!(
                    "{name}: violation shrank from {} to {} with ten times the sweeps",
                    short.max_constraint_violation, long.max_constraint_violation
                )));
            }
        }
    }
    Ok(CategoryResult::Pass)
}

// ---------------------------------------------------------------------------
// Host property test: projection output satisfies the constraints
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_from(
        n: usize,
        m: usize,
        a: Vec<f64>,
        b: Vec<f64>,
        t: Vec<f64>,
        projection_tolerance: f64,
    ) -> ConstrainedFixture {
        ConstrainedFixture {
            schema_version: 3,
            fixture_id: "property".to_owned(),
            suite: "smoke".to_owned(),
            problem_class: "box_quadratic_linear_inequalities".to_owned(),
            solver_family: "constrained_projected_first_order".to_owned(),
            dimension: n,
            constraints: m,
            scalar_family: "float".to_owned(),
            validation_state: "validate-all-inputs".to_owned(),
            conformance_groups: vec![
                "device-reference-smoke".to_owned(),
                "cluster-reference-smoke".to_owned(),
            ],
            variant: "solve".to_owned(),
            config: ConstrainedConfig {
                max_iterations: 5000,
                tolerance: 1e-12,
                step_scale: 1.0,
                projection_max_sweeps: 500_000,
                projection_tolerance,
            },
            problem: ConstrainedProblem {
                lower: vec![-10.0; n],
                upper: vec![10.0; n],
                initial: vec![0.0; n],
                quadratic_diag: vec![1.0; n],
                center: t,
                constraint_matrix: a,
                constraint_rhs: b,
            },
            expected: ConstrainedExpected {
                status: "converged".to_owned(),
                termination: "convergence-criterion".to_owned(),
                solution: Vec::new(),
                error: "none".to_owned(),
                violation_max: None,
                violation_min: None,
            },
            tolerance: ConstrainedTolerance {
                solution_abs: 1e-6,
                solution_rel: 1e-6,
            },
        }
    }

    /// RFC 027 S4 host property test: on random **feasible** polyhedra, the
    /// returned point satisfies every constraint within `projection_tolerance`
    /// and the box exactly, on the device and cluster kernels, at every
    /// instantiated shape.
    #[test]
    fn random_feasible_polyhedra_are_satisfied_within_the_projection_tolerance() {
        let mut state: u64 = 0xA24B_AED4_963E_E407;
        let mut next = move || {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((state >> 11) as f64) / ((1u64 << 53) as f64)
        };
        let tolerance = 1e-11;
        let mut checked = 0;
        for &(n, m) in &[(2, 1), (2, 2), (2, 3), (3, 1), (3, 2), (3, 3)] {
            for _ in 0..25 {
                let a: Vec<f64> = (0..n * m).map(|_| next() * 4.0 - 2.0).collect();
                if (0..m).any(|i| a[i * n..(i + 1) * n].iter().map(|v| v * v).sum::<f64>() < 0.09) {
                    continue;
                }
                let feasible: Vec<f64> = (0..n).map(|_| next() * 4.0 - 2.0).collect();
                let b: Vec<f64> = (0..m)
                    .map(|i| {
                        a[i * n..(i + 1) * n]
                            .iter()
                            .zip(&feasible)
                            .map(|(x, y)| x * y)
                            .sum::<f64>()
                            + next() * 0.5
                    })
                    .collect();
                let t: Vec<f64> = (0..n).map(|_| next() * 12.0 - 6.0).collect();
                let f = fixture_from(n, m, a, b, t, tolerance);
                for (name, run) in [
                    (
                        "device",
                        run_device(&f, f.config.projection_max_sweeps).unwrap(),
                    ),
                    (
                        "cluster",
                        run_cluster(&f, f.config.projection_max_sweeps).unwrap(),
                    ),
                ] {
                    let s = run
                        .result
                        .unwrap_or_else(|e| panic!("{name} n={n} m={m}: {e:?}"));
                    assert!(
                        s.max_constraint_violation <= tolerance,
                        "{name} n={n} m={m}: violation {} exceeds {tolerance}",
                        s.max_constraint_violation
                    );
                    assert_eq!(s.projection_cap_hits, 0, "{name} n={n} m={m}");
                    assert!(
                        s.x.iter().all(|v| (-10.0..=10.0).contains(v)),
                        "{name} n={n} m={m}: left the box: {:?}",
                        s.x
                    );
                    checked += 1;
                }
            }
        }
        assert!(checked > 200, "too few usable instances: {checked}");
    }

    #[test]
    fn identity_is_numeric_equality_plus_a_nan_check() {
        assert!(identical(0.0, -0.0));
        assert!(identical(-0.0, 0.0));
        assert!(identical(1.5, 1.5));
        assert!(!identical(1.5, 1.5000000000000002));
        assert!(!identical(f64::NAN, f64::NAN));
        assert!(!identical(f64::NAN, 0.0));
        // Raw bits do see the difference the rule deliberately ignores.
        assert_ne!(0.0_f64.to_bits(), (-0.0_f64).to_bits());
    }
}
