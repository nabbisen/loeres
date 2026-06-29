//! Tests for the baseline projected first-order kernel (RFC 006).

use super::{DeviceSolveReport, ProjectedFirstOrderWorkspace, solve_projected_first_order};
use crate::config::TimingMode;
use crate::problem::ProjectedFirstOrderProblem;
use crate::workspace::{DeviceWorkspaceDiagnostic, WorkspaceFor};
use loeres::{AsCoreReport, DiagnosticSnapshot, SolveStatus, SolverError, TerminationReason};
use loeres_backend_static::array::FixedVector;

mod fixtures;
use fixtures::*;

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

// --- v0.10.1 fail-safe validation (B2 / B3 / M4) ---

#[test]
fn zero_step_scale_rejected() {
    let problem = quad2_with_alpha(0.0);
    let mut x = FixedVector::from_array([0.0, 0.0]);
    let mut ws = workspace::<2>();
    let cfg = config(100, 1e-9, TimingMode::EarlyExitAllowed);
    let err = solve_projected_first_order(&problem, &mut x, &mut ws, &cfg).unwrap_err();
    assert_eq!(err, SolverError::InvalidInput);
}

#[test]
fn negative_step_scale_rejected() {
    let problem = quad2_with_alpha(-0.5);
    let mut x = FixedVector::from_array([0.0, 0.0]);
    let mut ws = workspace::<2>();
    let cfg = config(100, 1e-9, TimingMode::EarlyExitAllowed);
    let err = solve_projected_first_order(&problem, &mut x, &mut ws, &cfg).unwrap_err();
    assert_eq!(err, SolverError::InvalidInput);
}

#[test]
fn nan_step_scale_rejected() {
    let problem = quad2_with_alpha(f64::NAN);
    let mut x = FixedVector::from_array([0.0, 0.0]);
    let mut ws = workspace::<2>();
    let cfg = config(100, 1e-9, TimingMode::EarlyExitAllowed);
    let err = solve_projected_first_order(&problem, &mut x, &mut ws, &cfg).unwrap_err();
    assert_eq!(err, SolverError::NonFiniteInput);
}

#[test]
fn infinite_step_scale_rejected() {
    let problem = quad2_with_alpha(f64::INFINITY);
    let mut x = FixedVector::from_array([0.0, 0.0]);
    let mut ws = workspace::<2>();
    let cfg = config(100, 1e-9, TimingMode::EarlyExitAllowed);
    let err = solve_projected_first_order(&problem, &mut x, &mut ws, &cfg).unwrap_err();
    assert_eq!(err, SolverError::NonFiniteInput);
}

#[test]
fn non_finite_initial_x_rejected() {
    let problem = quad2();
    let mut x = FixedVector::from_array([f64::NAN, 0.0]);
    let mut ws = workspace::<2>();
    let cfg = config(100, 1e-9, TimingMode::EarlyExitAllowed);
    let err = solve_projected_first_order(&problem, &mut x, &mut ws, &cfg).unwrap_err();
    assert_eq!(err, SolverError::NonFiniteInput);
}

#[test]
fn non_finite_gradient_output_rejected() {
    let problem = NanGradient {
        lo: FixedVector::from_array([-1.0, -1.0]),
        hi: FixedVector::from_array([1.0, 1.0]),
    };
    let mut x = FixedVector::from_array([0.0, 0.0]);
    let mut ws = workspace::<2>();
    let cfg = config(100, 1e-9, TimingMode::EarlyExitAllowed);
    let err = solve_projected_first_order(&problem, &mut x, &mut ws, &cfg).unwrap_err();
    assert_eq!(err, SolverError::NonFiniteInput);
}

#[test]
fn non_finite_bound_rejected_by_kernel() {
    let problem = LaxNanBounds {
        lo: FixedVector::from_array([f64::NAN, -1.0]),
        hi: FixedVector::from_array([1.0, 1.0]),
    };
    let mut x = FixedVector::from_array([0.0, 0.0]);
    let mut ws = workspace::<2>();
    let cfg = config(100, 1e-9, TimingMode::EarlyExitAllowed);
    let err = solve_projected_first_order(&problem, &mut x, &mut ws, &cfg).unwrap_err();
    assert_eq!(err, SolverError::NonFiniteInput);
}
