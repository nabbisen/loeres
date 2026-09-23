//! Tests for the RFC 027 constrained projected first-order device kernel.

use super::{
    ConstrainedProjectedWorkspace, ConstrainedSolveConfig, solve_constrained_projected_first_order,
};
use crate::config::{DeviceSolveConfig, TimingMode};
use crate::problem::ProjectedFirstOrderProblem;
use crate::solve::{ProjectedFirstOrderWorkspace, solve_projected_first_order};
use crate::workspace::{DeviceWorkspace, DeviceWorkspaceDiagnostic};
use loeres::{BoxBounds, LinearInequalities, QuadraticObjective, SolveStatus, SolverError};

#[cfg(feature = "constant-iteration")]
use loeres::TerminationReason;
use loeres_backend_static::array::{FixedMatrix, FixedVector};
use loeres_backend_static::workspace::WorkspaceFootprint;

// ---------------------------------------------------------------------------
// A 2-variable, 1-constraint quadratic program: Q = I, c = -target, matching
// the RFC 006 fixture's separable-quadratic shape re-expressed as a
// QuadraticProgram (gradient Qx+c = x-target, identical to RFC006's q*(x-t)
// oracle for this Q=I case — the only shape where the two oracles coincide;
// RFC 027 §0.2.5 is explicit that they differ in general).
// ---------------------------------------------------------------------------

struct Qp2x1 {
    q: FixedMatrix<f64, 2, 2, 4>,
    c: FixedVector<f64, 2>,
    lo: FixedVector<f64, 2>,
    hi: FixedVector<f64, 2>,
    a: FixedMatrix<f64, 1, 2, 2>,
    b: FixedVector<f64, 1>,
}

impl QuadraticObjective<f64> for Qp2x1 {
    type Hessian = FixedMatrix<f64, 2, 2, 4>;
    type Linear = FixedVector<f64, 2>;
    fn hessian(&self) -> &Self::Hessian {
        &self.q
    }
    fn linear_term(&self) -> &Self::Linear {
        &self.c
    }
}
impl BoxBounds<f64> for Qp2x1 {
    type Bound = FixedVector<f64, 2>;
    fn lower_bounds(&self) -> &Self::Bound {
        &self.lo
    }
    fn upper_bounds(&self) -> &Self::Bound {
        &self.hi
    }
}
impl LinearInequalities<f64> for Qp2x1 {
    type Constraints = FixedMatrix<f64, 1, 2, 2>;
    type Rhs = FixedVector<f64, 1>;
    fn constraint_matrix(&self) -> &Self::Constraints {
        &self.a
    }
    fn constraint_rhs(&self) -> &Self::Rhs {
        &self.b
    }
}

fn identity_2x2() -> FixedMatrix<f64, 2, 2, 4> {
    FixedMatrix::from_row_major_array([1.0, 0.0, 0.0, 1.0])
}

fn box_only_config(max_iterations: u32, tolerance: f64) -> ConstrainedSolveConfig<f64> {
    ConstrainedSolveConfig {
        max_iterations,
        tolerance,
        timing_mode: TimingMode::EarlyExitAllowed,
        projection_max_sweeps: 200,
        projection_tolerance: 1e-12,
    }
}

fn workspace_2x1() -> ConstrainedProjectedWorkspace<f64, 2, 1> {
    ConstrainedProjectedWorkspace::new(
        FixedVector::from_array([0.0, 0.0]),
        FixedVector::from_array([0.0, 0.0]),
        FixedVector::from_array([0.0, 0.0]),
        FixedVector::from_array([0.0, 0.0]),
        FixedVector::from_array([0.0]),
        FixedVector::from_array([0.0]),
    )
}

// --- RFC 006 problem with the exact same target/box/alpha, for the identity
// comparison. Lives here rather than importing `solve::tests::fixtures`,
// which is private to that sibling module. ---

struct Rfc006Quadratic {
    target: FixedVector<f64, 2>,
    lo: FixedVector<f64, 2>,
    hi: FixedVector<f64, 2>,
    alpha: f64,
}

impl ProjectedFirstOrderProblem<f64, 2> for Rfc006Quadratic {
    type Bounds = FixedVector<f64, 2>;
    fn validate_boundary(&self) -> Result<(), SolverError> {
        Ok(())
    }
    fn lower_bound(&self) -> &Self::Bounds {
        &self.lo
    }
    fn upper_bound(&self) -> &Self::Bounds {
        &self.hi
    }
    fn step_scale(&self) -> f64 {
        self.alpha
    }
    fn gradient_at(
        &self,
        x: &FixedVector<f64, 2>,
        grad: &mut FixedVector<f64, 2>,
    ) -> Result<(), SolverError> {
        for (g, (&xi, &ti)) in grad
            .as_mut_slice()
            .iter_mut()
            .zip(x.as_slice().iter().zip(self.target.as_slice()))
        {
            *g = xi - ti;
        }
        Ok(())
    }
    fn objective_at(&self, x: &FixedVector<f64, 2>) -> Result<f64, SolverError> {
        let mut acc = 0.0;
        for (&xi, &ti) in x.as_slice().iter().zip(self.target.as_slice()) {
            let d = xi - ti;
            acc += 0.5 * d * d;
        }
        Ok(acc)
    }
}

/// RFC 027 handoff S2, written first: with `m >= 1` constraints all inactive
/// at the optimum, the constrained kernel matches RFC 006's result within
/// tolerance. Bitwise equality is observed, never asserted (§0.2.5's warning
/// that the two oracles generally differ in floating point applies whenever
/// Q != I; here Q = I makes the oracles coincide, so bitwise equality is a
/// live possibility worth checking by hand rather than by assertion).
#[test]
fn m_at_least_one_all_inactive_matches_rfc006_within_tolerance() {
    let alpha = 0.5;

    let rfc006 = Rfc006Quadratic {
        target: FixedVector::from_array([0.5_f64, -0.5]),
        lo: FixedVector::from_array([-1.0_f64, -1.0]),
        hi: FixedVector::from_array([1.0_f64, 1.0]),
        alpha,
    };
    let mut x_rfc006 = FixedVector::from_array([0.0_f64, 0.0]);
    let mut ws_rfc006 = ProjectedFirstOrderWorkspace::new(FixedVector::from_array([0.0, 0.0]));
    let cfg_rfc006 = DeviceSolveConfig {
        max_iterations: 500,
        tolerance: 1e-12,
        timing_mode: TimingMode::EarlyExitAllowed,
    };
    solve_projected_first_order(&rfc006, &mut x_rfc006, &mut ws_rfc006, &cfg_rfc006)
        .expect("RFC 006 solve succeeds");

    // Same objective (Q=I, c=-target), same box; one linear inequality that is
    // strictly satisfied at the unconstrained optimum (0.5 + -0.5 = 0 << 10).
    let constrained = Qp2x1 {
        q: identity_2x2(),
        c: FixedVector::from_array([-0.5_f64, 0.5]),
        lo: FixedVector::from_array([-1.0_f64, -1.0]),
        hi: FixedVector::from_array([1.0_f64, 1.0]),
        a: FixedMatrix::from_row_major_array([1.0_f64, 1.0]),
        b: FixedVector::from_array([10.0_f64]),
    };
    let mut x_constrained = FixedVector::from_array([0.0_f64, 0.0]);
    let mut ws_constrained = workspace_2x1();
    let report = solve_constrained_projected_first_order(
        &constrained,
        alpha,
        &mut x_constrained,
        &mut ws_constrained,
        &box_only_config(500, 1e-12),
    )
    .expect("constrained solve succeeds");

    assert_eq!(report.status(), SolveStatus::Converged);
    assert_eq!(report.projection_cap_hits(), 0);
    assert!(report.max_constraint_violation() <= 0.0);

    for (a, b) in x_rfc006.as_slice().iter().zip(x_constrained.as_slice()) {
        assert!(
            (a - b).abs() < 1e-8,
            "RFC 006 x = {:?}, constrained x = {:?}",
            x_rfc006.as_slice(),
            x_constrained.as_slice()
        );
    }
}

#[test]
fn box_constraint_active_matches_rfc006_projection() {
    // Target outside the box on both axes, inequality still inactive.
    let alpha = 0.5;

    let rfc006 = Rfc006Quadratic {
        target: FixedVector::from_array([5.0_f64, -5.0]),
        lo: FixedVector::from_array([-1.0_f64, -1.0]),
        hi: FixedVector::from_array([1.0_f64, 1.0]),
        alpha,
    };
    let mut x_rfc006 = FixedVector::from_array([0.0_f64, 0.0]);
    let mut ws_rfc006 = ProjectedFirstOrderWorkspace::new(FixedVector::from_array([0.0, 0.0]));
    let cfg = DeviceSolveConfig {
        max_iterations: 500,
        tolerance: 1e-12,
        timing_mode: TimingMode::EarlyExitAllowed,
    };
    solve_projected_first_order(&rfc006, &mut x_rfc006, &mut ws_rfc006, &cfg)
        .expect("RFC 006 solve succeeds");
    // The box must be active for this to be the comparison it claims to be.
    assert_eq!(x_rfc006.as_slice(), &[1.0, -1.0]);

    let constrained = Qp2x1 {
        q: identity_2x2(),
        c: FixedVector::from_array([-5.0_f64, 5.0]),
        lo: FixedVector::from_array([-1.0_f64, -1.0]),
        hi: FixedVector::from_array([1.0_f64, 1.0]),
        a: FixedMatrix::from_row_major_array([1.0_f64, 1.0]),
        b: FixedVector::from_array([10.0_f64]),
    };
    let mut x_constrained = FixedVector::from_array([0.0_f64, 0.0]);
    let mut ws_constrained = workspace_2x1();
    let report = solve_constrained_projected_first_order(
        &constrained,
        alpha,
        &mut x_constrained,
        &mut ws_constrained,
        &box_only_config(500, 1e-12),
    )
    .expect("constrained solve succeeds");

    assert_eq!(report.status(), SolveStatus::Converged);
    for (a, b) in x_rfc006.as_slice().iter().zip(x_constrained.as_slice()) {
        assert!((a - b).abs() < 1e-8);
    }
}

/// A halfspace that genuinely binds at the optimum: minimize toward `(2,2)`
/// inside a box that would otherwise allow it, cut by `x0 + x1 <= 1`. The
/// unconstrained minimizer of `½‖x−(2,2)‖²` subject to that one halfspace is
/// exactly its Euclidean projection: `(2,2) − max(0,(2+2−1)/2)·(1,1) =
/// (0.5, 0.5)` — confirmed independently by KKT: at `(0.5,0.5)`,
/// `∇f = (−1.5,−1.5)` and the constraint gradient is `(1,1)`, so
/// `∇f + λ·(1,1) = 0` at `λ = 1.5 ≥ 0`, with the constraint exactly active.
/// The box never binds.
#[test]
fn active_linear_constraint_converges_near_the_closed_form_optimum() {
    let problem = Qp2x1 {
        q: identity_2x2(),
        c: FixedVector::from_array([-2.0_f64, -2.0]),
        lo: FixedVector::from_array([-10.0_f64, -10.0]),
        hi: FixedVector::from_array([10.0_f64, 10.0]),
        a: FixedMatrix::from_row_major_array([1.0_f64, 1.0]),
        b: FixedVector::from_array([1.0_f64]),
    };
    let mut x = FixedVector::from_array([0.0_f64, 0.0]);
    let mut ws = workspace_2x1();
    let report = solve_constrained_projected_first_order(
        &problem,
        0.4,
        &mut x,
        &mut ws,
        &box_only_config(2000, 1e-12),
    )
    .expect("solve succeeds");

    assert_eq!(report.status(), SolveStatus::Converged);
    assert!((x.as_slice()[0] - 0.5).abs() < 1e-4, "{:?}", x.as_slice());
    assert!((x.as_slice()[1] - 0.5).abs() < 1e-4, "{:?}", x.as_slice());
    assert!(
        report.max_constraint_violation() <= 1e-9,
        "{}",
        report.max_constraint_violation()
    );
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn valid_problem() -> Qp2x1 {
    Qp2x1 {
        q: identity_2x2(),
        c: FixedVector::from_array([-0.5_f64, 0.5]),
        lo: FixedVector::from_array([-1.0_f64, -1.0]),
        hi: FixedVector::from_array([1.0_f64, 1.0]),
        a: FixedMatrix::from_row_major_array([1.0_f64, 1.0]),
        b: FixedVector::from_array([10.0_f64]),
    }
}

#[test]
fn an_all_zero_constraint_row_is_invalid_input() {
    let mut problem = valid_problem();
    problem.a = FixedMatrix::from_row_major_array([0.0_f64, 0.0]);
    let mut x = FixedVector::from_array([0.0_f64, 0.0]);
    let mut ws = workspace_2x1();
    assert_eq!(
        solve_constrained_projected_first_order(
            &problem,
            0.5,
            &mut x,
            &mut ws,
            &box_only_config(100, 1e-9),
        ),
        Err(SolverError::InvalidInput)
    );
}

#[test]
fn inverted_box_bounds_are_invalid_input() {
    let mut problem = valid_problem();
    problem.lo = FixedVector::from_array([1.0_f64, -1.0]);
    problem.hi = FixedVector::from_array([-1.0_f64, 1.0]);
    let mut x = FixedVector::from_array([0.0_f64, 0.0]);
    let mut ws = workspace_2x1();
    assert_eq!(
        solve_constrained_projected_first_order(
            &problem,
            0.5,
            &mut x,
            &mut ws,
            &box_only_config(100, 1e-9),
        ),
        Err(SolverError::InvalidInput)
    );
}

#[test]
fn non_finite_box_bound_is_non_finite_input() {
    let mut problem = valid_problem();
    problem.hi = FixedVector::from_array([f64::NAN, 1.0]);
    let mut x = FixedVector::from_array([0.0_f64, 0.0]);
    let mut ws = workspace_2x1();
    assert_eq!(
        solve_constrained_projected_first_order(
            &problem,
            0.5,
            &mut x,
            &mut ws,
            &box_only_config(100, 1e-9),
        ),
        Err(SolverError::NonFiniteInput)
    );
}

#[test]
fn non_finite_step_scale_is_non_finite_input() {
    let problem = valid_problem();
    let mut x = FixedVector::from_array([0.0_f64, 0.0]);
    let mut ws = workspace_2x1();
    assert_eq!(
        solve_constrained_projected_first_order(
            &problem,
            f64::NAN,
            &mut x,
            &mut ws,
            &box_only_config(100, 1e-9),
        ),
        Err(SolverError::NonFiniteInput)
    );
}

#[test]
fn non_positive_step_scale_is_invalid_input() {
    let problem = valid_problem();
    let mut x = FixedVector::from_array([0.0_f64, 0.0]);
    let mut ws = workspace_2x1();
    for alpha in [0.0_f64, -0.5] {
        assert_eq!(
            solve_constrained_projected_first_order(
                &problem,
                alpha,
                &mut x,
                &mut ws,
                &box_only_config(100, 1e-9),
            ),
            Err(SolverError::InvalidInput),
            "alpha = {alpha}"
        );
    }
}

#[test]
fn zero_max_iterations_is_invalid_input() {
    let problem = valid_problem();
    let mut x = FixedVector::from_array([0.0_f64, 0.0]);
    let mut ws = workspace_2x1();
    assert_eq!(
        solve_constrained_projected_first_order(
            &problem,
            0.5,
            &mut x,
            &mut ws,
            &box_only_config(0, 1e-9),
        ),
        Err(SolverError::InvalidInput)
    );
}

#[test]
fn zero_projection_max_sweeps_is_invalid_input() {
    let problem = valid_problem();
    let mut x = FixedVector::from_array([0.0_f64, 0.0]);
    let mut ws = workspace_2x1();
    let mut cfg = box_only_config(100, 1e-9);
    cfg.projection_max_sweeps = 0;
    assert_eq!(
        solve_constrained_projected_first_order(&problem, 0.5, &mut x, &mut ws, &cfg),
        Err(SolverError::InvalidInput)
    );
}

#[test]
fn non_finite_initial_iterate_is_non_finite_input() {
    let problem = valid_problem();
    let mut x = FixedVector::from_array([f64::NAN, 0.0]);
    let mut ws = workspace_2x1();
    assert_eq!(
        solve_constrained_projected_first_order(
            &problem,
            0.5,
            &mut x,
            &mut ws,
            &box_only_config(100, 1e-9),
        ),
        Err(SolverError::NonFiniteInput)
    );
}

// ---------------------------------------------------------------------------
// Infeasibility: never a `SolverError` (RFC 027 §11.6). The full
// "non-shrinking violation across an extended run" fixture belongs to S4's
// conformance corpus; this is the device-level sanity check.
//
// **Finding, not asserted on `SolveStatus` alone.** For this specific
// infeasible pair — two exactly-opposing unit-norm halfspaces in one
// dimension — the *primal* iterate stabilizes sweep-over-sweep even though
// the dual multipliers grow without bound underneath it (traced by hand:
// `x` settles to `1` forever from the second sweep on, while `λ` climbs
// `(1,2) → (2,4) → (3,6) → …`). The outer and inner convergence checks are
// both defined on the primal iterate's change, per RFC 027 §11.3, so they
// see zero change and report `Converged` — on an infeasible point. This is a
// property of this degenerate geometry (the two corrections cancel exactly
// every sweep), not a general guarantee that infeasibility always destabilizes
// the primal iterate. `max_constraint_violation()`, RFC 027's other honest
// field, is unaffected and reports the true violation regardless: this test
// asserts on it, not on `SolveStatus`. Flagged for the architect (review
// request §9/§11) — S4's own infeasibility fixture should use a polyhedron
// that does not have this cancellation property (e.g. three or more
// non-antiparallel conflicting halfspaces) to also exercise `NotConverged`.
// ---------------------------------------------------------------------------

#[test]
fn an_infeasible_polyhedron_reports_a_positive_constraint_violation() {
    // x0 <= -1 and x0 >= 1 (via -x0 <= -1) cannot both hold: infeasible.
    struct Infeasible {
        q: FixedMatrix<f64, 1, 1, 1>,
        c: FixedVector<f64, 1>,
        lo: FixedVector<f64, 1>,
        hi: FixedVector<f64, 1>,
        a: FixedMatrix<f64, 2, 1, 2>,
        b: FixedVector<f64, 2>,
    }
    impl QuadraticObjective<f64> for Infeasible {
        type Hessian = FixedMatrix<f64, 1, 1, 1>;
        type Linear = FixedVector<f64, 1>;
        fn hessian(&self) -> &Self::Hessian {
            &self.q
        }
        fn linear_term(&self) -> &Self::Linear {
            &self.c
        }
    }
    impl BoxBounds<f64> for Infeasible {
        type Bound = FixedVector<f64, 1>;
        fn lower_bounds(&self) -> &Self::Bound {
            &self.lo
        }
        fn upper_bounds(&self) -> &Self::Bound {
            &self.hi
        }
    }
    impl LinearInequalities<f64> for Infeasible {
        type Constraints = FixedMatrix<f64, 2, 1, 2>;
        type Rhs = FixedVector<f64, 2>;
        fn constraint_matrix(&self) -> &Self::Constraints {
            &self.a
        }
        fn constraint_rhs(&self) -> &Self::Rhs {
            &self.b
        }
    }

    let problem = Infeasible {
        q: FixedMatrix::from_row_major_array([1.0]),
        c: FixedVector::from_array([0.0_f64]),
        lo: FixedVector::from_array([-100.0_f64]),
        hi: FixedVector::from_array([100.0_f64]),
        a: FixedMatrix::from_row_major_array([1.0_f64, -1.0]),
        b: FixedVector::from_array([-1.0_f64, -1.0]),
    };
    let mut x = FixedVector::from_array([0.0_f64]);
    let mut ws = ConstrainedProjectedWorkspace::new(
        FixedVector::from_array([0.0]),
        FixedVector::from_array([0.0]),
        FixedVector::from_array([0.0]),
        FixedVector::from_array([0.0]),
        FixedVector::from_array([0.0, 0.0]),
        FixedVector::from_array([0.0, 0.0]),
    );
    let report = solve_constrained_projected_first_order(
        &problem,
        0.5,
        &mut x,
        &mut ws,
        &box_only_config(200, 1e-12),
    )
    .expect("infeasibility is a status, not an error");

    // Not asserted on `SolveStatus` — see the finding above.
    assert!(report.max_constraint_violation() > 0.0);
    assert!((report.max_constraint_violation() - 2.0).abs() < 1e-9);
}

// ---------------------------------------------------------------------------
// Workspace
// ---------------------------------------------------------------------------

#[test]
fn footprint_is_size_of_self() {
    assert_eq!(
        <ConstrainedProjectedWorkspace<f64, 4, 3> as WorkspaceFootprint>::footprint_bytes(),
        core::mem::size_of::<ConstrainedProjectedWorkspace<f64, 4, 3>>()
    );
}

#[test]
fn reset_for_entry_clears_the_diagnostic() {
    let mut ws = workspace_2x1();
    ws.reset_for_entry();
    assert_eq!(ws.diagnostic(), loeres::DiagnosticSnapshot::EMPTY);
}

#[test]
#[cfg(feature = "constant-iteration")]
fn constant_iteration_runs_the_full_cap_when_the_feature_is_enabled() {
    {
        let problem = valid_problem();
        let mut x = FixedVector::from_array([0.0_f64, 0.0]);
        let mut ws = workspace_2x1();
        let cfg = ConstrainedSolveConfig {
            max_iterations: 50,
            tolerance: 1e-12,
            timing_mode: TimingMode::ConstantIteration,
            projection_max_sweeps: 200,
            projection_tolerance: 1e-12,
        };
        let report = solve_constrained_projected_first_order(&problem, 0.5, &mut x, &mut ws, &cfg)
            .expect("solve succeeds");
        assert_eq!(report.iterations_executed(), 50);
        assert_eq!(report.core().termination(), TerminationReason::IterationCap);
    }
}
