//! `conformance` — RFC 013 corpus runner.

use std::fs;
use std::path::{Path, PathBuf};

use loeres::{ContiguousVectorAccess, SolveStatus, SolverError, TerminationReason};
use loeres_backend_static::array::FixedVector;
use loeres_backend_std::DenseVector;
use loeres_cluster::{
    ClusterCancellationToken, ClusterExecutionContext, ClusterProjectedFirstOrderProblem,
    ClusterProjectedFirstOrderWorkspace, ClusterValidationPolicy, ProjectedFirstOrderConfig,
    solve_projected_first_order_dyn,
};
use loeres_device::config::{DeviceSolveConfig, TimingMode};
use loeres_device::problem::ProjectedFirstOrderProblem;
use loeres_device::solve::{ProjectedFirstOrderWorkspace, solve_projected_first_order};
use serde::Deserialize;

const CORPUS: &str = "conformance";

pub fn run(args: &[String]) -> bool {
    let Some(suite) = parse_suite(args) else {
        usage();
        return false;
    };
    eprintln!("[conformance] suite: {}", suite.as_str());
    match suite {
        Suite::Smoke => run_smoke(),
        Suite::Extended | Suite::Adversarial => run_placeholder_suite(suite),
    }
}

fn parse_suite(args: &[String]) -> Option<Suite> {
    match args {
        [] => Some(Suite::Smoke),
        [flag, value] if flag == "--suite" => Suite::parse(value),
        _ => None,
    }
}

fn usage() {
    eprintln!("usage: cargo xtask conformance [--suite smoke|extended|adversarial]");
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum Suite {
    Smoke,
    Extended,
    Adversarial,
}

impl Suite {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "smoke" => Some(Self::Smoke),
            "extended" => Some(Self::Extended),
            "adversarial" => Some(Self::Adversarial),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Smoke => "smoke",
            Self::Extended => "extended",
            Self::Adversarial => "adversarial",
        }
    }
}

fn run_smoke() -> bool {
    let fixtures = match load_fixtures("smoke") {
        Ok(fixtures) => fixtures,
        Err(e) => {
            eprintln!("  ! {e}");
            eprintln!("[conformance] FAIL");
            return false;
        }
    };
    let mut summary = Summary::default();
    let mut ok = true;
    for fixture in &fixtures {
        eprintln!("  fixture: {}", fixture.fixture_id);
        match run_fixture(fixture) {
            Ok(result) => {
                ok &= result.fixture_passed();
                summary.record(&result);
            }
            Err(e) => {
                eprintln!("    FAIL: {e}");
                ok = false;
                summary.fixtures_total += 1;
                summary.fixtures_failed += 1;
            }
        }
    }
    summary.print();
    eprintln!("[conformance] {}", if ok { "PASS" } else { "FAIL" });
    ok
}

fn run_placeholder_suite(suite: Suite) -> bool {
    let path = corpus_root().join(suite.as_str());
    if !path.exists() {
        eprintln!("  not-enforced: {}/ does not exist yet", path.display());
        eprintln!("[conformance] NOT-ENFORCED");
        return true;
    }
    let fixtures = fixture_paths(&path);
    if fixtures.is_empty() {
        eprintln!(
            "  not-enforced: {}/ has no fixture TOML files",
            path.display()
        );
        eprintln!("[conformance] NOT-ENFORCED");
        return true;
    }
    eprintln!(
        "  fixtures are present, but {} suite execution is not implemented in v0.18.0",
        suite.as_str()
    );
    eprintln!("[conformance] FAIL");
    false
}

fn load_fixtures(suite: &str) -> Result<Vec<Fixture>, String> {
    let root = corpus_root().join(suite);
    let paths = fixture_paths(&root);
    if paths.is_empty() {
        return Err(format!("no fixture TOML files found in {}", root.display()));
    }
    let mut fixtures = Vec::new();
    for path in paths {
        let src = fs::read_to_string(&path)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        let fixture: Fixture =
            toml::from_str(&src).map_err(|e| format!("cannot parse {}: {e}", path.display()))?;
        fixture.validate(suite)?;
        fixtures.push(fixture);
    }
    let found = fixtures
        .iter()
        .map(|f| f.fixture_id.as_str())
        .collect::<Vec<_>>();
    for required in REQUIRED_SMOKE {
        if !found.contains(required) {
            return Err(format!("missing required smoke fixture `{required}`"));
        }
    }
    Ok(fixtures)
}

fn corpus_root() -> PathBuf {
    let from_cwd = PathBuf::from(CORPUS);
    if from_cwd.exists() {
        return from_cwd;
    }
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask has a workspace parent")
        .join(CORPUS)
}

fn fixture_paths(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(_) => return out,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("toml") {
            out.push(path);
        }
    }
    out.sort();
    out
}

const REQUIRED_SMOKE: &[&str] = &[
    "pfo-box-converged-001",
    "pfo-box-not-converged-001",
    "pfo-box-invalid-bound-001",
];

#[derive(Clone, Debug, Deserialize)]
struct Fixture {
    schema_version: u32,
    fixture_id: String,
    suite: String,
    problem_class: String,
    solver_family: String,
    dimension: usize,
    scalar_family: String,
    validation_state: String,
    conformance_groups: Vec<String>,
    config: FixtureConfig,
    problem: FixtureProblem,
    expected: FixtureExpected,
    tolerance: FixtureTolerance,
}

impl Fixture {
    fn validate(&self, suite: &str) -> Result<(), String> {
        expect_eq(self.schema_version, 1, &self.fixture_id, "schema_version")?;
        expect_str(&self.suite, suite, &self.fixture_id, "suite")?;
        expect_str(
            &self.problem_class,
            "box_quadratic_diagonal",
            &self.fixture_id,
            "problem_class",
        )?;
        expect_str(
            &self.solver_family,
            "projected_first_order",
            &self.fixture_id,
            "solver_family",
        )?;
        expect_eq(self.dimension, 2, &self.fixture_id, "dimension")?;
        expect_str(
            &self.scalar_family,
            "float",
            &self.fixture_id,
            "scalar_family",
        )?;
        expect_str(
            &self.validation_state,
            "validate-all-inputs",
            &self.fixture_id,
            "validation_state",
        )?;
        for required in ["device-reference-smoke", "cluster-reference-smoke"] {
            if !self.conformance_groups.iter().any(|g| g == required) {
                return Err(format!(
                    "{}: conformance_groups must contain `{required}`",
                    self.fixture_id
                ));
            }
        }
        if self.config.max_iterations == 0 {
            return Err(format!("{}: max_iterations must be > 0", self.fixture_id));
        }
        if !self.config.tolerance.is_finite() || self.config.tolerance <= 0.0 {
            return Err(format!(
                "{}: tolerance must be finite and > 0",
                self.fixture_id
            ));
        }
        if !self.config.step_scale.is_finite() || self.config.step_scale <= 0.0 {
            return Err(format!(
                "{}: step_scale must be finite and > 0",
                self.fixture_id
            ));
        }
        for (name, values) in [
            ("lower", &self.problem.lower),
            ("upper", &self.problem.upper),
            ("initial", &self.problem.initial),
            ("quadratic_diag", &self.problem.quadratic_diag),
            ("center", &self.problem.center),
        ] {
            if values.len() != self.dimension {
                return Err(format!(
                    "{}: {name} length must equal dimension",
                    self.fixture_id
                ));
            }
            if !values.iter().all(|v| v.is_finite()) {
                return Err(format!("{}: {name} values must be finite", self.fixture_id));
            }
        }
        if self.expected.error == "none" && !self.problem.quadratic_diag.iter().all(|v| *v > 0.0) {
            return Err(format!(
                "{}: quadratic_diag values must be positive for non-error fixtures",
                self.fixture_id
            ));
        }
        if !self.expected.solution.is_empty() && self.expected.solution.len() != self.dimension {
            return Err(format!(
                "{}: expected solution length must equal dimension",
                self.fixture_id
            ));
        }
        if !self.tolerance.solution_abs.is_finite() || self.tolerance.solution_abs < 0.0 {
            return Err(format!(
                "{}: solution_abs must be finite and >= 0",
                self.fixture_id
            ));
        }
        if !self.tolerance.solution_rel.is_finite() || self.tolerance.solution_rel < 0.0 {
            return Err(format!(
                "{}: solution_rel must be finite and >= 0",
                self.fixture_id
            ));
        }
        if self.tolerance.objective_abs.as_deref() != Some("not-applicable") {
            return Err(format!(
                "{}: objective_abs must be \"not-applicable\" in v0.18.0",
                self.fixture_id
            ));
        }
        if self.tolerance.residual_abs.as_deref() != Some("not-applicable") {
            return Err(format!(
                "{}: residual_abs must be \"not-applicable\" in v0.18.0",
                self.fixture_id
            ));
        }
        Ok(())
    }
}

fn expect_eq<T: Eq + std::fmt::Display>(
    actual: T,
    expected: T,
    fixture: &str,
    field: &str,
) -> Result<(), String> {
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "{fixture}: expected {field} = {expected}, found {actual}"
        ))
    }
}

fn expect_str(actual: &str, expected: &str, fixture: &str, field: &str) -> Result<(), String> {
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "{fixture}: expected {field} = `{expected}`, found `{actual}`"
        ))
    }
}

#[derive(Clone, Debug, Deserialize)]
struct FixtureConfig {
    max_iterations: u32,
    tolerance: f64,
    step_scale: f64,
}

#[derive(Clone, Debug, Deserialize)]
struct FixtureProblem {
    lower: Vec<f64>,
    upper: Vec<f64>,
    initial: Vec<f64>,
    quadratic_diag: Vec<f64>,
    center: Vec<f64>,
}

#[derive(Clone, Debug, Deserialize)]
struct FixtureExpected {
    status: String,
    termination: String,
    solution: Vec<f64>,
    error: String,
}

#[derive(Clone, Debug, Deserialize)]
struct FixtureTolerance {
    solution_abs: f64,
    solution_rel: f64,
    objective_abs: Option<String>,
    residual_abs: Option<String>,
}

fn run_fixture(fixture: &Fixture) -> Result<FixtureResult, String> {
    let device = run_device(fixture)?;
    let cluster = run_cluster(fixture)?;
    let mut result = FixtureResult::new(fixture.fixture_id.clone());
    result.status_match = compare_status(fixture, &device, &cluster);
    result.solution_within_tolerance = compare_solution(fixture, &device, &cluster);
    result.expected_failure_match = compare_failure(fixture, &device, &cluster);
    result.objective_within_tolerance = CategoryResult::NotApplicable;
    result.residual_within_tolerance = CategoryResult::NotApplicable;
    result.print();
    Ok(result)
}

#[derive(Debug)]
struct RunOutcome {
    report: Option<(SolveStatus, TerminationReason)>,
    solution: Option<[f64; 2]>,
    error: Option<SolverError>,
}

fn run_device(fixture: &Fixture) -> Result<RunOutcome, String> {
    let problem = DeviceDiagonalProblem::new(fixture)?;
    let mut x = fixed(&fixture.problem.initial)?;
    let mut workspace = ProjectedFirstOrderWorkspace::new(FixedVector::from_array([0.0, 0.0]));
    let config = DeviceSolveConfig {
        max_iterations: fixture.config.max_iterations,
        tolerance: fixture.config.tolerance,
        timing_mode: TimingMode::EarlyExitAllowed,
    };
    Ok(
        match solve_projected_first_order(&problem, &mut x, &mut workspace, &config) {
            Ok(report) => RunOutcome {
                report: Some((report.status(), report.core().termination())),
                solution: Some(*array_ref(x.as_slice())?),
                error: None,
            },
            Err(error) => RunOutcome {
                report: None,
                solution: None,
                error: Some(error),
            },
        },
    )
}

fn run_cluster(fixture: &Fixture) -> Result<RunOutcome, String> {
    let problem = ClusterDiagonalProblem::new(fixture)?;
    let mut x = dense(&fixture.problem.initial)?;
    let mut workspace = ClusterProjectedFirstOrderWorkspace::new(2)
        .map_err(|e| format!("cluster workspace init failed: {e:?}"))?;
    let config = ProjectedFirstOrderConfig {
        max_iterations: fixture.config.max_iterations,
        tolerance: fixture.config.tolerance,
    };
    let ctx = ClusterExecutionContext::new(
        ClusterCancellationToken::new(),
        0,
        ClusterValidationPolicy::ValidateAllInputs,
    );
    Ok(
        match solve_projected_first_order_dyn(&problem, &mut x, &mut workspace, &config, &ctx) {
            Ok(record) => RunOutcome {
                report: Some((record.report.status(), record.report.termination())),
                solution: Some(*array_ref(
                    x.as_contiguous()
                        .ok_or("cluster solution is not contiguous")?,
                )?),
                error: None,
            },
            Err(error) => RunOutcome {
                report: None,
                solution: None,
                error: Some(error),
            },
        },
    )
}

fn fixed(values: &[f64]) -> Result<FixedVector<f64, 2>, String> {
    Ok(FixedVector::from_array(*array_ref(values)?))
}

fn dense(values: &[f64]) -> Result<DenseVector<f64>, String> {
    DenseVector::from_vec(values.to_vec()).map_err(|e| format!("dense vector failed: {e:?}"))
}

fn array_ref(values: &[f64]) -> Result<&[f64; 2], String> {
    values
        .try_into()
        .map_err(|_| format!("expected 2 values, found {}", values.len()))
}

struct DeviceDiagonalProblem {
    lower: FixedVector<f64, 2>,
    upper: FixedVector<f64, 2>,
    q: [f64; 2],
    center: [f64; 2],
    step_scale: f64,
}

impl DeviceDiagonalProblem {
    fn new(fixture: &Fixture) -> Result<Self, String> {
        Ok(Self {
            lower: fixed(&fixture.problem.lower)?,
            upper: fixed(&fixture.problem.upper)?,
            q: *array_ref(&fixture.problem.quadratic_diag)?,
            center: *array_ref(&fixture.problem.center)?,
            step_scale: fixture.config.step_scale,
        })
    }
}

impl ProjectedFirstOrderProblem<f64, 2> for DeviceDiagonalProblem {
    type Bounds = FixedVector<f64, 2>;

    fn validate_boundary(&self) -> Result<(), SolverError> {
        for (&lo, &hi) in self.lower.as_slice().iter().zip(self.upper.as_slice()) {
            if !lo.is_finite() || !hi.is_finite() {
                return Err(SolverError::NonFiniteInput);
            }
            if lo > hi {
                return Err(SolverError::InvalidInput);
            }
        }
        Ok(())
    }

    fn lower_bound(&self) -> &Self::Bounds {
        &self.lower
    }

    fn upper_bound(&self) -> &Self::Bounds {
        &self.upper
    }

    fn step_scale(&self) -> f64 {
        self.step_scale
    }

    fn gradient_at(
        &self,
        x: &FixedVector<f64, 2>,
        grad: &mut FixedVector<f64, 2>,
    ) -> Result<(), SolverError> {
        for i in 0..2 {
            grad.as_mut_slice()[i] = self.q[i] * (x.as_slice()[i] - self.center[i]);
        }
        Ok(())
    }

    fn objective_at(&self, x: &FixedVector<f64, 2>) -> Result<f64, SolverError> {
        Ok(objective(x.as_slice(), &self.q, &self.center))
    }
}

#[derive(Clone)]
struct ClusterDiagonalProblem {
    lower: DenseVector<f64>,
    upper: DenseVector<f64>,
    q: [f64; 2],
    center: [f64; 2],
    step_scale: f64,
}

impl ClusterDiagonalProblem {
    fn new(fixture: &Fixture) -> Result<Self, String> {
        Ok(Self {
            lower: dense(&fixture.problem.lower)?,
            upper: dense(&fixture.problem.upper)?,
            q: *array_ref(&fixture.problem.quadratic_diag)?,
            center: *array_ref(&fixture.problem.center)?,
            step_scale: fixture.config.step_scale,
        })
    }
}

impl ClusterProjectedFirstOrderProblem<f64> for ClusterDiagonalProblem {
    fn dimension(&self) -> usize {
        2
    }

    fn bounds(&self) -> (&DenseVector<f64>, &DenseVector<f64>) {
        (&self.lower, &self.upper)
    }

    fn gradient_at(
        &self,
        x: &DenseVector<f64>,
        grad: &mut DenseVector<f64>,
    ) -> Result<(), SolverError> {
        let x = x
            .as_contiguous()
            .ok_or(SolverError::InternalInvariantViolation)?;
        let grad = loeres::ContiguousVectorAccessMut::as_contiguous_mut(grad)
            .ok_or(SolverError::InternalInvariantViolation)?;
        for i in 0..2 {
            grad[i] = self.q[i] * (x[i] - self.center[i]);
        }
        Ok(())
    }

    fn step_scale(&self) -> f64 {
        self.step_scale
    }
}

fn objective(x: &[f64], q: &[f64; 2], center: &[f64; 2]) -> f64 {
    let mut total = 0.0;
    for i in 0..2 {
        let diff = x[i] - center[i];
        total += 0.5 * q[i] * diff * diff;
    }
    total
}

fn compare_status(fixture: &Fixture, device: &RunOutcome, cluster: &RunOutcome) -> CategoryResult {
    if fixture.expected.error != "none" {
        return CategoryResult::NotApplicable;
    }
    let expected_status = match fixture.expected.status.as_str() {
        "converged" => SolveStatus::Converged,
        "not-converged" => SolveStatus::NotConverged,
        other => return CategoryResult::Fail(format!("unknown expected status `{other}`")),
    };
    let expected_term = match fixture.expected.termination.as_str() {
        "convergence-criterion" => TerminationReason::ConvergenceCriterion,
        "iteration-cap" => TerminationReason::IterationCap,
        other => return CategoryResult::Fail(format!("unknown expected termination `{other}`")),
    };
    match (device.report, cluster.report) {
        (Some(d), Some(c)) if d == c && d == (expected_status, expected_term) => {
            CategoryResult::Pass
        }
        (Some(d), Some(c)) => CategoryResult::Fail(format!(
            "status mismatch: device={d:?}, cluster={c:?}, expected=({expected_status:?}, {expected_term:?})"
        )),
        _ => CategoryResult::Fail(format!(
            "expected reports, got device_error={:?}, cluster_error={:?}",
            device.error, cluster.error
        )),
    }
}

fn compare_solution(
    fixture: &Fixture,
    device: &RunOutcome,
    cluster: &RunOutcome,
) -> CategoryResult {
    if fixture.expected.status != "converged" {
        return CategoryResult::NotApplicable;
    }
    let Ok(expected) = array_ref(&fixture.expected.solution) else {
        return CategoryResult::Fail("expected solution must have dimension 2".to_owned());
    };
    let (Some(device), Some(cluster)) = (device.solution, cluster.solution) else {
        return CategoryResult::Fail("missing solution from one or both paths".to_owned());
    };
    let abs = fixture.tolerance.solution_abs;
    let rel = fixture.tolerance.solution_rel;
    for i in 0..2 {
        if !within_tolerance(device[i], expected[i], abs, rel) {
            return CategoryResult::Fail(format!(
                "device solution[{i}]={} expected={} abs={abs} rel={rel}",
                device[i], expected[i]
            ));
        }
        if !within_tolerance(cluster[i], expected[i], abs, rel) {
            return CategoryResult::Fail(format!(
                "cluster solution[{i}]={} expected={} abs={abs} rel={rel}",
                cluster[i], expected[i]
            ));
        }
        if !within_tolerance(device[i], cluster[i], abs, rel) {
            return CategoryResult::Fail(format!(
                "cross-path solution[{i}] device={} cluster={} abs={abs} rel={rel}",
                device[i], cluster[i]
            ));
        }
    }
    CategoryResult::Pass
}

fn within_tolerance(actual: f64, expected: f64, abs: f64, rel: f64) -> bool {
    let diff = (actual - expected).abs();
    diff <= abs.max(rel * expected.abs())
}

fn compare_failure(fixture: &Fixture, device: &RunOutcome, cluster: &RunOutcome) -> CategoryResult {
    if fixture.expected.error == "none" {
        return CategoryResult::NotApplicable;
    }
    let expected = match fixture.expected.error.as_str() {
        "invalid-input" => SolverError::InvalidInput,
        other => return CategoryResult::Fail(format!("unknown expected error `{other}`")),
    };
    match (device.error, cluster.error) {
        (Some(d), Some(c)) if d == expected && c == expected => CategoryResult::Pass,
        (Some(d), Some(c)) => CategoryResult::Fail(format!(
            "error mismatch: device={d:?}, cluster={c:?}, expected={expected:?}"
        )),
        _ => CategoryResult::Fail(format!(
            "expected errors, got device_report={:?}, cluster_report={:?}",
            device.report, cluster.report
        )),
    }
}

#[derive(Clone, Debug)]
enum CategoryResult {
    Pass,
    Fail(String),
    NotApplicable,
}

impl CategoryResult {
    fn passed(&self) -> bool {
        matches!(self, Self::Pass | Self::NotApplicable)
    }

    fn label(&self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Fail(_) => "FAIL",
            Self::NotApplicable => "not-applicable",
        }
    }

    fn detail(&self) -> Option<&str> {
        match self {
            Self::Fail(detail) => Some(detail),
            _ => None,
        }
    }
}

#[derive(Debug)]
struct FixtureResult {
    fixture_id: String,
    status_match: CategoryResult,
    solution_within_tolerance: CategoryResult,
    expected_failure_match: CategoryResult,
    objective_within_tolerance: CategoryResult,
    residual_within_tolerance: CategoryResult,
}

impl FixtureResult {
    fn new(fixture_id: String) -> Self {
        Self {
            fixture_id,
            status_match: CategoryResult::NotApplicable,
            solution_within_tolerance: CategoryResult::NotApplicable,
            expected_failure_match: CategoryResult::NotApplicable,
            objective_within_tolerance: CategoryResult::NotApplicable,
            residual_within_tolerance: CategoryResult::NotApplicable,
        }
    }

    fn fixture_passed(&self) -> bool {
        self.status_match.passed()
            && self.solution_within_tolerance.passed()
            && self.expected_failure_match.passed()
            && self.objective_within_tolerance.passed()
            && self.residual_within_tolerance.passed()
    }

    fn print(&self) {
        eprintln!(
            "    fixture result for {}: {}",
            self.fixture_id,
            if self.fixture_passed() {
                "pass"
            } else {
                "FAIL"
            }
        );
        for (name, result) in [
            ("status_match", &self.status_match),
            ("solution_within_tolerance", &self.solution_within_tolerance),
            ("expected_failure_match", &self.expected_failure_match),
            (
                "objective_within_tolerance",
                &self.objective_within_tolerance,
            ),
            ("residual_within_tolerance", &self.residual_within_tolerance),
        ] {
            eprintln!("    {name}: {}", result.label());
            if let Some(detail) = result.detail() {
                eprintln!("      {detail}");
            }
        }
    }
}

#[derive(Default)]
struct Summary {
    fixtures_total: u32,
    fixtures_passed: u32,
    fixtures_failed: u32,
    status_match: CategorySummary,
    solution_within_tolerance: CategorySummary,
    expected_failure_match: CategorySummary,
    objective_within_tolerance: CategorySummary,
    residual_within_tolerance: CategorySummary,
}

impl Summary {
    fn record(&mut self, result: &FixtureResult) {
        self.fixtures_total += 1;
        if result.fixture_passed() {
            self.fixtures_passed += 1;
        } else {
            self.fixtures_failed += 1;
        }
        self.status_match.record(&result.status_match);
        self.solution_within_tolerance
            .record(&result.solution_within_tolerance);
        self.expected_failure_match
            .record(&result.expected_failure_match);
        self.objective_within_tolerance
            .record(&result.objective_within_tolerance);
        self.residual_within_tolerance
            .record(&result.residual_within_tolerance);
    }

    fn print(&self) {
        eprintln!(
            "  fixtures: {} total / {} passed / {} failed",
            self.fixtures_total, self.fixtures_passed, self.fixtures_failed
        );
        self.status_match.print("status_match");
        self.solution_within_tolerance
            .print("solution_within_tolerance");
        self.expected_failure_match.print("expected_failure_match");
        self.objective_within_tolerance
            .print("objective_within_tolerance");
        self.residual_within_tolerance
            .print("residual_within_tolerance");
    }
}

#[derive(Default)]
struct CategorySummary {
    passed: u32,
    failed: u32,
    not_applicable: u32,
}

impl CategorySummary {
    fn record(&mut self, result: &CategoryResult) {
        match result {
            CategoryResult::Pass => self.passed += 1,
            CategoryResult::Fail(_) => self.failed += 1,
            CategoryResult::NotApplicable => self.not_applicable += 1,
        }
    }

    fn print(&self, name: &str) {
        eprintln!(
            "  {name}: {} passed / {} failed / {} not-applicable",
            self.passed, self.failed, self.not_applicable
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke_fixtures_parse_and_validate() {
        let fixtures = load_fixtures("smoke").unwrap();
        assert_eq!(fixtures.len(), 3);
    }

    #[test]
    fn converged_fixture_passes() {
        let fixtures = load_fixtures("smoke").unwrap();
        let fixture = fixtures
            .iter()
            .find(|f| f.fixture_id == "pfo-box-converged-001")
            .unwrap();
        assert!(run_fixture(fixture).unwrap().fixture_passed());
    }

    #[test]
    fn invalid_bound_fixture_compares_structured_error() {
        let fixtures = load_fixtures("smoke").unwrap();
        let fixture = fixtures
            .iter()
            .find(|f| f.fixture_id == "pfo-box-invalid-bound-001")
            .unwrap();
        let result = run_fixture(fixture).unwrap();
        assert!(matches!(
            result.expected_failure_match,
            CategoryResult::Pass
        ));
    }
}
