//! Tests for the baseline projected first-order kernel (RFC 006).

use super::{DeviceSolveReport, ProjectedFirstOrderWorkspace, solve_projected_first_order};
use crate::config::{DeviceSolveConfig, TimingMode};
use crate::problem::ProjectedFirstOrderProblem;
use crate::workspace::{DeviceWorkspaceDiagnostic, WorkspaceFor};
use loeres::{AsCoreReport, DiagnosticSnapshot, SolveStatus, SolverError, TerminationReason};
use loeres_backend_static::array::FixedVector;

/// `f(x) = 0.5 * sum (x_i - target_i)^2`, box-constrained. Gradient is
/// `x - target`; projected gradient descent converges to the projection of
/// `target` onto the box.
struct Quadratic<const N: usize> {
    target: FixedVector<f64, N>,
    lo: FixedVector<f64, N>,
    hi: FixedVector<f64, N>,
    alpha: f64,
}

impl<const N: usize> ProjectedFirstOrderProblem<f64, N> for Quadratic<N> {
    type Bounds = FixedVector<f64, N>;

    fn validate_boundary(&self) -> Result<(), SolverError> {
        for (l, h) in self.lo.as_slice().iter().zip(self.hi.as_slice()) {
            if !l.is_finite() || !h.is_finite() {
                return Err(SolverError::NonFiniteInput);
            }
            if *l > *h {
                return Err(SolverError::InvalidInput);
            }
        }
        Ok(())
    }

    fn lower_bound(&self) -> &FixedVector<f64, N> {
        &self.lo
    }

    fn upper_bound(&self) -> &FixedVector<f64, N> {
        &self.hi
    }

    fn step_scale(&self) -> f64 {
        self.alpha
    }

    fn gradient_at(
        &self,
        x: &FixedVector<f64, N>,
        grad: &mut FixedVector<f64, N>,
    ) -> Result<(), SolverError> {
        for ((g, &xi), &ti) in grad
            .as_mut_slice()
            .iter_mut()
            .zip(x.as_slice())
            .zip(self.target.as_slice())
        {
            *g = xi - ti;
        }
        Ok(())
    }

    fn objective_at(&self, x: &FixedVector<f64, N>) -> Result<f64, SolverError> {
        let mut acc = 0.0;
        for (&xi, &ti) in x.as_slice().iter().zip(self.target.as_slice()) {
            let d = xi - ti;
            acc += 0.5 * d * d;
        }
        Ok(acc)
    }
}

impl<const N: usize> WorkspaceFor<Quadratic<N>> for Quadratic<N> {
    type Workspace = ProjectedFirstOrderWorkspace<f64, N>;

    fn required_workspace_bytes() -> usize {
        core::mem::size_of::<ProjectedFirstOrderWorkspace<f64, N>>()
    }
}

fn quad2() -> Quadratic<2> {
    Quadratic {
        target: FixedVector::from_array([0.5, -0.5]),
        lo: FixedVector::from_array([-1.0, -1.0]),
        hi: FixedVector::from_array([1.0, 1.0]),
        alpha: 0.5,
    }
}

fn workspace<const N: usize>() -> ProjectedFirstOrderWorkspace<f64, N> {
    ProjectedFirstOrderWorkspace::new(FixedVector::from_array([0.0; N]))
}

fn config(max_iterations: u32, tolerance: f64, timing_mode: TimingMode) -> DeviceSolveConfig<f64> {
    DeviceSolveConfig {
        max_iterations,
        tolerance,
        timing_mode,
    }
}

#[test]
fn converges_early_within_box() {
    let problem = quad2();
    let mut x = FixedVector::from_array([0.0, 0.0]);
    let mut ws = workspace::<2>();
    let cfg = config(100, 1e-9, TimingMode::EarlyExitAllowed);

    let report = solve_projected_first_order(&problem, &mut x, &mut ws, &cfg).unwrap();

    assert_eq!(report.status(), SolveStatus::Converged);
    assert_eq!(
        report.core().termination(),
        TerminationReason::ConvergenceCriterion
    );
    assert!(report.iterations_executed() < 100);
    // converged to the (interior) target
    assert!((x.as_slice()[0] - 0.5).abs() < 1e-6);
    assert!((x.as_slice()[1] + 0.5).abs() < 1e-6);
}

#[test]
fn projects_onto_box_when_target_outside() {
    // target outside the box -> converges to the clamped projection
    let problem = Quadratic {
        target: FixedVector::from_array([5.0, -5.0]),
        lo: FixedVector::from_array([-1.0, -1.0]),
        hi: FixedVector::from_array([1.0, 1.0]),
        alpha: 0.5,
    };
    let mut x = FixedVector::from_array([0.0, 0.0]);
    let mut ws = workspace::<2>();
    let cfg = config(200, 1e-9, TimingMode::EarlyExitAllowed);

    let report = solve_projected_first_order(&problem, &mut x, &mut ws, &cfg).unwrap();

    assert_eq!(report.status(), SolveStatus::Converged);
    assert!((x.as_slice()[0] - 1.0).abs() < 1e-6);
    assert!((x.as_slice()[1] + 1.0).abs() < 1e-6);
}

#[test]
fn non_convergence_at_cap_is_ok_not_error() {
    let problem = quad2();
    let mut x = FixedVector::from_array([0.0, 0.0]);
    let mut ws = workspace::<2>();
    let cfg = config(1, 1e-9, TimingMode::EarlyExitAllowed);

    let report = solve_projected_first_order(&problem, &mut x, &mut ws, &cfg).unwrap();

    assert_eq!(report.status(), SolveStatus::NotConverged);
    assert_eq!(report.core().termination(), TerminationReason::IterationCap);
    assert_eq!(report.iterations_executed(), 1);
}

#[test]
fn inverted_bounds_rejected() {
    let problem = Quadratic {
        target: FixedVector::from_array([0.0, 0.0]),
        lo: FixedVector::from_array([1.0, 1.0]),
        hi: FixedVector::from_array([-1.0, -1.0]),
        alpha: 0.5,
    };
    let mut x = FixedVector::from_array([0.0, 0.0]);
    let mut ws = workspace::<2>();
    let cfg = config(10, 1e-9, TimingMode::EarlyExitAllowed);

    let err = solve_projected_first_order(&problem, &mut x, &mut ws, &cfg).unwrap_err();
    assert_eq!(err, SolverError::InvalidInput);
}

#[test]
fn non_finite_tolerance_rejected() {
    let problem = quad2();
    let mut x = FixedVector::from_array([0.0, 0.0]);
    let mut ws = workspace::<2>();
    let cfg = config(10, f64::NAN, TimingMode::EarlyExitAllowed);

    let err = solve_projected_first_order(&problem, &mut x, &mut ws, &cfg).unwrap_err();
    assert_eq!(err, SolverError::NonFiniteInput);
}

/// `N = 3` work vectors but `Bounds` of length 2 — `validate_boundary` is
/// deliberately lax so the kernel's own defensive dimension check fires.
struct Mismatch {
    lo: FixedVector<f64, 2>,
    hi: FixedVector<f64, 2>,
}

impl ProjectedFirstOrderProblem<f64, 3> for Mismatch {
    type Bounds = FixedVector<f64, 2>;

    fn validate_boundary(&self) -> Result<(), SolverError> {
        Ok(())
    }
    fn lower_bound(&self) -> &FixedVector<f64, 2> {
        &self.lo
    }
    fn upper_bound(&self) -> &FixedVector<f64, 2> {
        &self.hi
    }
    fn step_scale(&self) -> f64 {
        0.5
    }
    fn gradient_at(
        &self,
        _x: &FixedVector<f64, 3>,
        grad: &mut FixedVector<f64, 3>,
    ) -> Result<(), SolverError> {
        for g in grad.as_mut_slice().iter_mut() {
            *g = 0.0;
        }
        Ok(())
    }
    fn objective_at(&self, _x: &FixedVector<f64, 3>) -> Result<f64, SolverError> {
        Ok(0.0)
    }
}

#[test]
fn dimension_mismatch_detected() {
    let problem = Mismatch {
        lo: FixedVector::from_array([-1.0, -1.0]),
        hi: FixedVector::from_array([1.0, 1.0]),
    };
    let mut x = FixedVector::from_array([0.0, 0.0, 0.0]);
    let mut ws = workspace::<3>();
    let cfg = config(10, 1e-9, TimingMode::EarlyExitAllowed);

    let err = solve_projected_first_order(&problem, &mut x, &mut ws, &cfg).unwrap_err();
    assert_eq!(err, SolverError::DimensionMismatch { lhs: 3, rhs: 2 });
}

#[test]
fn workspace_reused_after_error() {
    let mut ws = workspace::<2>();
    let mut x = FixedVector::from_array([0.0, 0.0]);
    let cfg = config(100, 1e-9, TimingMode::EarlyExitAllowed);

    // first solve errors on inverted bounds
    let bad = Quadratic {
        target: FixedVector::from_array([0.0, 0.0]),
        lo: FixedVector::from_array([1.0, 1.0]),
        hi: FixedVector::from_array([-1.0, -1.0]),
        alpha: 0.5,
    };
    assert!(solve_projected_first_order(&bad, &mut x, &mut ws, &cfg).is_err());

    // same workspace reused for a clean solve
    let good = quad2();
    let mut x2 = FixedVector::from_array([0.0, 0.0]);
    let report = solve_projected_first_order(&good, &mut x2, &mut ws, &cfg).unwrap();
    assert_eq!(report.status(), SolveStatus::Converged);
}

#[test]
fn workspace_reused_after_non_convergence() {
    let problem = quad2();
    let mut ws = workspace::<2>();

    let mut x1 = FixedVector::from_array([0.0, 0.0]);
    let capped = config(1, 1e-9, TimingMode::EarlyExitAllowed);
    let r1 = solve_projected_first_order(&problem, &mut x1, &mut ws, &capped).unwrap();
    assert_eq!(r1.status(), SolveStatus::NotConverged);

    let mut x2 = FixedVector::from_array([0.0, 0.0]);
    let generous = config(100, 1e-9, TimingMode::EarlyExitAllowed);
    let r2 = solve_projected_first_order(&problem, &mut x2, &mut ws, &generous).unwrap();
    assert_eq!(r2.status(), SolveStatus::Converged);
}

#[test]
fn workspace_for_sizing_matches_workspace() {
    let bytes = <Quadratic<2> as WorkspaceFor<Quadratic<2>>>::required_workspace_bytes();
    assert_eq!(
        bytes,
        core::mem::size_of::<ProjectedFirstOrderWorkspace<f64, 2>>()
    );
}

#[test]
fn objective_is_reporting_only_but_available() {
    let problem = quad2();
    let x = FixedVector::from_array([0.0, 0.0]);
    // 0.5 * (0.5^2 + 0.5^2) = 0.25
    let value = problem.objective_at(&x).unwrap();
    assert!((value - 0.25).abs() < 1e-12);
}

#[test]
fn workspace_diagnostic_is_empty_in_baseline() {
    let ws = workspace::<2>();
    assert_eq!(ws.diagnostic(), DiagnosticSnapshot::EMPTY);
}

#[test]
fn device_report_derives_core_report() {
    let report = DeviceSolveReport::from_core(loeres::SolveReport::converged_early(3));
    assert_eq!(report.as_core_report(), report.core());
    assert_eq!(report.as_core_report().iterations_executed(), 3);
}

#[cfg(feature = "constant-iteration")]
#[test]
fn constant_iteration_runs_full_count_when_converged() {
    let problem = quad2();
    let mut x = FixedVector::from_array([0.0, 0.0]);
    let mut ws = workspace::<2>();
    let cfg = config(50, 1e-9, TimingMode::ConstantIteration);

    let report = solve_projected_first_order(&problem, &mut x, &mut ws, &cfg).unwrap();

    // converged, but ran to the cap: status Converged, termination IterationCap,
    // and the full iteration count was executed.
    assert_eq!(report.status(), SolveStatus::Converged);
    assert_eq!(report.core().termination(), TerminationReason::IterationCap);
    assert_eq!(report.iterations_executed(), 50);
}

#[cfg(feature = "constant-iteration")]
#[test]
fn constant_iteration_reports_non_convergence_at_cap() {
    // tolerance unreachable in a single allowed-then-capped run
    let problem = quad2();
    let mut x = FixedVector::from_array([0.0, 0.0]);
    let mut ws = workspace::<2>();
    let cfg = config(2, 0.0, TimingMode::ConstantIteration);

    let report = solve_projected_first_order(&problem, &mut x, &mut ws, &cfg).unwrap();

    assert_eq!(report.status(), SolveStatus::NotConverged);
    assert_eq!(report.core().termination(), TerminationReason::IterationCap);
    assert_eq!(report.iterations_executed(), 2);
}
