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
        projection_max_sweeps: 5000,
        projection_tolerance: 1e-12,
    }
}

fn workspace_2x1() -> ConstrainedProjectedWorkspace<f64, 2, 1> {
    ConstrainedProjectedWorkspace::new(
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
    assert!(
        report.max_constraint_violation() <= 1e-12,
        "returned point violates its own constraints by {}",
        report.max_constraint_violation()
    );
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
    assert!(
        report.max_constraint_violation() <= 1e-12,
        "returned point violates its own constraints by {}",
        report.max_constraint_violation()
    );
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
    assert!(
        report.max_constraint_violation() <= 1e-12,
        "returned point violates its own constraints by {}",
        report.max_constraint_violation()
    );
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
// Infeasibility: never a `SolverError` (RFC 027 §11.6). Under RFC 027
// Amendment 3 (§0.3.4) an infeasible polyhedron needs no special handling: it
// runs to `projection_max_sweeps` and reports its true violation with
// `projection_cap_hits > 0`. An earlier version of this kernel instead settled
// this exact case at a stable, falsely-`Converged` point; that was the
// two-set/one-pass defect corrected by §0.3.1–§0.3.3, not a property of the
// geometry, and this test must not be reshaped to avoid exactly-cancelling
// constraints.
// ---------------------------------------------------------------------------

#[test]
fn an_infeasible_polyhedron_hits_the_cap_and_reports_its_true_violation() {
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

    assert!(report.projection_cap_hits() > 0);
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

/// RFC 027 §11.4 stands: `(3N + 2M)·size_of::<S>() + header`, where the header
/// is the `DiagnosticSnapshot` (16 bytes).
#[test]
fn footprint_is_three_n_plus_two_m_scalars_plus_the_header() {
    assert_eq!(
        core::mem::size_of::<ConstrainedProjectedWorkspace<f64, 2, 1>>(),
        (3 * 2 + 2) * 8 + 16
    );
    assert_eq!(
        core::mem::size_of::<ConstrainedProjectedWorkspace<f64, 3, 3>>(),
        (3 * 3 + 2 * 3) * 8 + 16
    );
    assert_eq!(
        core::mem::size_of::<ConstrainedProjectedWorkspace<f64, 2, 1>>(),
        80
    );
    assert_eq!(
        core::mem::size_of::<ConstrainedProjectedWorkspace<f64, 3, 3>>(),
        136
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
            projection_max_sweeps: 5000,
            projection_tolerance: 1e-12,
        };
        let report = solve_constrained_projected_first_order(&problem, 0.5, &mut x, &mut ws, &cfg)
            .expect("solve succeeds");
        assert_eq!(report.iterations_executed(), 50);
        assert_eq!(report.core().termination(), TerminationReason::IterationCap);
    }
}

// ---------------------------------------------------------------------------
// m >= 2 (RFC 027 Amendment 3, §0.3; review 054). With M = 1 a single Hildreth
// pass is exact and the two-set/one-pass defect cannot appear, which is why
// every test above passed against it. These use M = 2.
// ---------------------------------------------------------------------------

struct Qp2x2 {
    q: FixedMatrix<f64, 2, 2, 4>,
    c: FixedVector<f64, 2>,
    lo: FixedVector<f64, 2>,
    hi: FixedVector<f64, 2>,
    a: FixedMatrix<f64, 2, 2, 4>,
    b: FixedVector<f64, 2>,
}

impl QuadraticObjective<f64> for Qp2x2 {
    type Hessian = FixedMatrix<f64, 2, 2, 4>;
    type Linear = FixedVector<f64, 2>;
    fn hessian(&self) -> &Self::Hessian {
        &self.q
    }
    fn linear_term(&self) -> &Self::Linear {
        &self.c
    }
}
impl BoxBounds<f64> for Qp2x2 {
    type Bound = FixedVector<f64, 2>;
    fn lower_bounds(&self) -> &Self::Bound {
        &self.lo
    }
    fn upper_bounds(&self) -> &Self::Bound {
        &self.hi
    }
}
impl LinearInequalities<f64> for Qp2x2 {
    type Constraints = FixedMatrix<f64, 2, 2, 4>;
    type Rhs = FixedVector<f64, 2>;
    fn constraint_matrix(&self) -> &Self::Constraints {
        &self.a
    }
    fn constraint_rhs(&self) -> &Self::Rhs {
        &self.b
    }
}

/// `min ½‖x − t‖²` (`Q = I`, `c = −t`) over `[−10,10]²` and `Ax ≤ b`.
fn projection_program(a: [f64; 4], b: [f64; 2], t: [f64; 2]) -> Qp2x2 {
    Qp2x2 {
        q: identity_2x2(),
        c: FixedVector::from_array([-t[0], -t[1]]),
        lo: FixedVector::from_array([-10.0, -10.0]),
        hi: FixedVector::from_array([10.0, 10.0]),
        a: FixedMatrix::from_row_major_array(a),
        b: FixedVector::from_array(b),
    }
}

fn workspace_2x2() -> ConstrainedProjectedWorkspace<f64, 2, 2> {
    ConstrainedProjectedWorkspace::new(
        FixedVector::from_array([0.0, 0.0]),
        FixedVector::from_array([0.0, 0.0]),
        FixedVector::from_array([0.0, 0.0]),
        FixedVector::from_array([0.0, 0.0]),
        FixedVector::from_array([0.0, 0.0]),
    )
}

fn config_2x2(projection_tolerance: f64, sweeps: u32) -> ConstrainedSolveConfig<f64> {
    ConstrainedSolveConfig {
        max_iterations: 5000,
        tolerance: 1e-12,
        timing_mode: TimingMode::EarlyExitAllowed,
        projection_max_sweeps: sweeps,
        projection_tolerance,
    }
}

/// The review-054 regression case, exact `A`, `b`, `t` and expected value. The
/// defective two-set kernel returned `(5.345268, −0.168013)`, `Converged`,
/// `projection_cap_hits 0`, violation `7.170922`.
#[test]
fn review_054_regression_two_constraints_converge_to_the_exact_projection() {
    let problem = projection_program(
        [1.6, -1.3082, -0.0457, 0.9197],
        [1.6013, -0.3988],
        [5.3481, 4.7872],
    );
    let mut x = FixedVector::from_array([0.0_f64, 0.0]);
    let mut ws = workspace_2x2();
    let report = solve_constrained_projected_first_order(
        &problem,
        1.0,
        &mut x,
        &mut ws,
        &config_2x2(1e-14, 5000),
    )
    .expect("solve succeeds");

    assert_eq!(report.status(), SolveStatus::Converged);
    assert_eq!(report.projection_cap_hits(), 0);
    assert!(report.max_constraint_violation() <= 1e-14);
    assert!(
        (x.as_slice()[0] - 0.673643).abs() < 1e-6 && (x.as_slice()[1] + 0.400146).abs() < 1e-6,
        "x = {:?}",
        x.as_slice()
    );
}

/// Two constraints active at the optimum (a vertex). `min ½‖x − (3,1)‖²`
/// subject to `x₀ + x₁ ≤ 1.5` and `x₀ − x₁ ≤ 0.5`. Both active gives
/// `x₀ + x₁ = 1.5`, `x₀ − x₁ = 0.5`, i.e. `x = (1, 0.5)`. KKT: `∇f = x − t =
/// (−2, −0.5)`, and `∇f + λ₁(1,1) + λ₂(1,−1) = 0` gives `λ₁ + λ₂ = 2`,
/// `λ₁ − λ₂ = 0.5`, so `λ = (1.25, 0.75)`, both `≥ 0` with both constraints
/// active, so this is the optimum. The box never binds.
#[test]
fn a_vertex_with_two_active_constraints_is_found_exactly() {
    let problem = projection_program([1.0, 1.0, 1.0, -1.0], [1.5, 0.5], [3.0, 1.0]);
    let mut x = FixedVector::from_array([0.0_f64, 0.0]);
    let mut ws = workspace_2x2();
    let report = solve_constrained_projected_first_order(
        &problem,
        1.0,
        &mut x,
        &mut ws,
        &config_2x2(1e-13, 5000),
    )
    .expect("solve succeeds");

    assert_eq!(report.status(), SolveStatus::Converged);
    assert_eq!(report.projection_cap_hits(), 0);
    assert!(report.max_constraint_violation() <= 1e-13);
    assert!((x.as_slice()[0] - 1.0).abs() < 1e-9, "{:?}", x.as_slice());
    assert!((x.as_slice()[1] - 0.5).abs() < 1e-9, "{:?}", x.as_slice());
}

/// A constraint active early and inactive at the optimum. `min ½‖x‖²`
/// (`Q = I`, `c = 0`, optimum the origin) from `x₀ = (1, 1)` with a small
/// `step_scale = 0.3`, subject to `x₀ + x₁ ≤ 0.2` and `x₀ − x₁ ≤ 5`. The first
/// candidate `(0.7, 0.7)` violates the first row, so it is active and projects
/// to `(0.1, 0.1)`; later candidates `0.7·x` satisfy it, so it goes inactive
/// and the iterate descends to the origin, where `0 ≤ 0.2` strictly. KKT at
/// the origin: `∇f = 0`, all multipliers zero, both rows slack. The second row
/// never binds.
#[test]
fn a_constraint_active_early_and_inactive_at_the_optimum() {
    let problem = Qp2x2 {
        q: identity_2x2(),
        c: FixedVector::from_array([0.0, 0.0]),
        lo: FixedVector::from_array([-10.0, -10.0]),
        hi: FixedVector::from_array([10.0, 10.0]),
        a: FixedMatrix::from_row_major_array([1.0, 1.0, 1.0, -1.0]),
        b: FixedVector::from_array([0.2, 5.0]),
    };
    let mut x = FixedVector::from_array([1.0_f64, 1.0]);
    let mut ws = workspace_2x2();
    let report = solve_constrained_projected_first_order(
        &problem,
        0.3,
        &mut x,
        &mut ws,
        &config_2x2(1e-13, 5000),
    )
    .expect("solve succeeds");

    assert_eq!(report.status(), SolveStatus::Converged);
    assert!(report.max_constraint_violation() <= 1e-13);
    assert!(
        x.as_slice()[0].abs() < 1e-9 && x.as_slice()[1].abs() < 1e-9,
        "{:?}",
        x.as_slice()
    );
}

/// Randomized differential test against an exact reference (review-054 method:
/// M = 1 tests hide formulation errors). For `n = m = 2`, `Q = I`, the exact
/// solution is the Euclidean projection of `t` onto `{Ax ≤ b} ∩ box`, found
/// here by enumerating active sets (none, each single constraint, each pair)
/// and keeping the feasible KKT point with non-negative multipliers. Instances
/// are built feasible by construction (`b = A·x_feas + slack`). The defective
/// kernel returned a wrong projection on ~20% of instances like these.
#[test]
fn random_feasible_two_constraint_polytopes_match_the_exact_projection() {
    // Deterministic LCG; no dependency.
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut next = move || {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((state >> 11) as f64) / ((1u64 << 53) as f64)
    };

    let mut checked = 0;
    for _ in 0..300 {
        let a = [
            next() * 4.0 - 2.0,
            next() * 4.0 - 2.0,
            next() * 4.0 - 2.0,
            next() * 4.0 - 2.0,
        ];
        let n0 = (a[0] * a[0] + a[1] * a[1]).sqrt();
        let n1 = (a[2] * a[2] + a[3] * a[3]).sqrt();
        if n0 < 0.3 || n1 < 0.3 {
            continue;
        }
        let feas = [next() * 4.0 - 2.0, next() * 4.0 - 2.0];
        let b = [
            a[0] * feas[0] + a[1] * feas[1] + next() * 0.5,
            a[2] * feas[0] + a[3] * feas[1] + next() * 0.5,
        ];
        let t = [next() * 12.0 - 6.0, next() * 12.0 - 6.0];

        let expected = exact_projection(a, b, t);

        let problem = projection_program(a, b, t);
        let mut x = FixedVector::from_array([0.0_f64, 0.0]);
        let mut ws = workspace_2x2();
        let report = solve_constrained_projected_first_order(
            &problem,
            1.0,
            &mut x,
            &mut ws,
            &config_2x2(1e-13, 200_000),
        )
        .expect("solve succeeds");

        assert_eq!(report.projection_cap_hits(), 0, "a={a:?} b={b:?} t={t:?}");
        assert!(
            report.max_constraint_violation() <= 1e-12,
            "violates its own constraints: a={a:?} b={b:?} t={t:?}"
        );
        assert!(
            (x.as_slice()[0] - expected[0]).abs() < 1e-6
                && (x.as_slice()[1] - expected[1]).abs() < 1e-6,
            "a={a:?} b={b:?} t={t:?}: got {:?}, expected {expected:?}",
            x.as_slice()
        );
        checked += 1;
    }
    assert!(checked > 200, "too few usable instances: {checked}");
}

/// Exact Euclidean projection of `t` onto `{Ax ≤ b} ∩ [−10,10]²` for `2×2 A`,
/// by active-set enumeration over the 2 rows and the 4 box faces.
fn exact_projection(a: [f64; 4], b: [f64; 2], t: [f64; 2]) -> [f64; 2] {
    // Constraints g·x ≤ h.
    let g: [([f64; 2], f64); 6] = [
        ([a[0], a[1]], b[0]),
        ([a[2], a[3]], b[1]),
        ([1.0, 0.0], 10.0),
        ([-1.0, 0.0], 10.0),
        ([0.0, 1.0], 10.0),
        ([0.0, -1.0], 10.0),
    ];
    let feasible = |x: [f64; 2]| g.iter().all(|(n, h)| n[0] * x[0] + n[1] * x[1] <= h + 1e-9);

    if feasible(t) {
        return t;
    }
    for &(n, h) in &g {
        // Single active constraint: project onto its hyperplane.
        let nn = n[0] * n[0] + n[1] * n[1];
        let lam = (n[0] * t[0] + n[1] * t[1] - h) / nn;
        let x = [t[0] - lam * n[0], t[1] - lam * n[1]];
        if lam >= 0.0 && feasible(x) {
            return x;
        }
    }
    for i in 0..6 {
        for j in (i + 1)..6 {
            let (n1, h1) = g[i];
            let (n2, h2) = g[j];
            let det = n1[0] * n2[1] - n1[1] * n2[0];
            if det.abs() < 1e-12 {
                continue;
            }
            // Vertex: both active.
            let x = [
                (h1 * n2[1] - h2 * n1[1]) / det,
                (n1[0] * h2 - n2[0] * h1) / det,
            ];
            // Multipliers: t − x = λ1 n1 + λ2 n2.
            let d = [t[0] - x[0], t[1] - x[1]];
            let l1 = (d[0] * n2[1] - d[1] * n2[0]) / det;
            let l2 = (n1[0] * d[1] - n1[1] * d[0]) / det;
            if l1 >= -1e-12 && l2 >= -1e-12 && feasible(x) {
                return x;
            }
        }
    }
    unreachable!("a feasible instance always has an exact projection");
}
