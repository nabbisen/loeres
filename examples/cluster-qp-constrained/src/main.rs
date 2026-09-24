//! A constrained quadratic program on the server path.
//!
//! What this example demonstrates (RFC 027 Amendment 1):
//!
//! - implementing the `loeres::problem` contract — `QuadraticObjective`,
//!   `BoxBounds` and `LinearInequalities`, which together give a
//!   `QuadraticProgram` — over dynamic dense storage,
//! - solving it through the typed cluster entrypoint and reading the **terminal
//!   constraint violation beside the status**, for a slack, an active and an
//!   infeasible polyhedron,
//! - what the batch seam does and does not carry: status only.
//!
//! The problem is `minimize ½ xᵀQx + cᵀx` over `0 ≤ x ≤ 10` and `Ax ≤ b`, with
//! `Q = [[2, 0.5], [0.5, 1]]` and `c = (−4, −2)`. `Q` must be symmetric positive
//! semidefinite; that is a caller precondition the kernel does not verify.
//!
//! What this example does **not** show: LP (expressible with `Q = 0`, not solved
//! by this kernel), infeasibility *detection* (an infeasible polyhedron is only
//! reported as a non-converged status), or any rate claim. See RFC 027 §11.6.

use loeres::{
    BoxBounds, LinearInequalities, QuadraticObjective, QuadraticProgram, SolveStatus,
    TerminationReason, VectorAccess,
};
use loeres_backend_std::{DenseMatrix, DenseVector};
use loeres_cluster::{
    BatchItemOutcome, ClusterCancellationToken, ClusterConstrainedJob, ClusterConstrainedWorkspace,
    ClusterError, ClusterExecutionContext, ClusterJob, ClusterSolution, ClusterSolveConfig,
    ClusterValidationPolicy, ConstrainedProjectedConfig, solve_batch,
    solve_constrained_projected_first_order_dyn,
};

/// Step scale for the outer gradient step. The caller's responsibility, as in
/// RFC 006 and RFC 016: `0 < α < 2 / λ_max(Q)`, and `λ_max ≈ 2.28` here.
const STEP_SCALE: f64 = 0.4;

/// A dynamic QP with `Q = [[2, 0.5], [0.5, 1]]`, `c = (−4, −2)` and a box of
/// `[0, 10]²`; the halfspaces `Ax ≤ b` are the only thing that varies.
#[derive(Clone)]
struct Program {
    q: DenseMatrix<f64>,
    c: DenseVector<f64>,
    lower: DenseVector<f64>,
    upper: DenseVector<f64>,
    a: DenseMatrix<f64>,
    b: DenseVector<f64>,
}

impl Program {
    /// `rows` are the halfspaces, each `(a₀, a₁, bᵢ)` meaning `a₀x₀ + a₁x₁ ≤ bᵢ`.
    fn new(rows: &[(f64, f64, f64)]) -> Self {
        Self {
            q: matrix(2, 2, vec![2.0, 0.5, 0.5, 1.0]),
            c: dense(&[-4.0, -2.0]),
            lower: dense(&[0.0, 0.0]),
            upper: dense(&[10.0, 10.0]),
            a: matrix(
                rows.len(),
                2,
                rows.iter().flat_map(|&(a0, a1, _)| [a0, a1]).collect(),
            ),
            b: dense(&rows.iter().map(|&(_, _, b)| b).collect::<Vec<_>>()),
        }
    }
}

impl QuadraticObjective<f64> for Program {
    type Hessian = DenseMatrix<f64>;
    type Linear = DenseVector<f64>;
    fn hessian(&self) -> &Self::Hessian {
        &self.q
    }
    fn linear_term(&self) -> &Self::Linear {
        &self.c
    }
}

impl BoxBounds<f64> for Program {
    type Bound = DenseVector<f64>;
    fn lower_bounds(&self) -> &Self::Bound {
        &self.lower
    }
    fn upper_bounds(&self) -> &Self::Bound {
        &self.upper
    }
}

impl LinearInequalities<f64> for Program {
    type Constraints = DenseMatrix<f64>;
    type Rhs = DenseVector<f64>;
    fn constraint_matrix(&self) -> &Self::Constraints {
        &self.a
    }
    fn constraint_rhs(&self) -> &Self::Rhs {
        &self.b
    }
}

fn main() -> Result<(), ClusterError> {
    println!("typed entrypoint — status beside the terminal constraint violation:\n");
    for (label, program) in cases() {
        println!("{label:<20} {}", solve_typed(&program));
    }

    // The batch seam erases a constrained solve to `BatchItemOutcome`, which
    // carries the core report only. The status is truthful there (`Converged`
    // means feasible, stationary and exactly projected: RFC 027 Amendment 5,
    // RFC 029, RFC 033), but the magnitudes are not.
    println!("\nbatch seam — status only:\n");
    let mut labels = Vec::new();
    let mut jobs: Vec<Box<dyn ClusterJob<f64>>> = Vec::new();
    for (label, program) in cases() {
        labels.push(label);
        jobs.push(Box::new(ClusterConstrainedJob::new(
            program,
            STEP_SCALE,
            dense(&[0.0, 0.0]),
            config(),
        )));
    }
    let report = solve_batch(
        jobs,
        ClusterSolveConfig::default(),
        ClusterCancellationToken::new(),
    )?;
    for (label, outcome) in labels.iter().zip(&report.outcomes) {
        println!("{label:<20} {}", describe(outcome));
    }
    Ok(())
}

/// Three polyhedra over the same objective and box.
///
/// - **slack:** `x₀ + x₁ ≤ 5` does not bind; the optimum is the unconstrained
///   `Q⁻¹(4, 2) = (12/7, 8/7)`.
/// - **active:** `x₀ + x₁ ≤ 2` binds; the KKT point is `(1.5, 0.5)` with
///   multiplier `0.75`.
/// - **infeasible:** `x₀ ≤ −1` cannot hold together with `x₀ ≥ 0`.
fn cases() -> Vec<(&'static str, Program)> {
    vec![
        ("slack halfspace:", Program::new(&[(1.0, 1.0, 5.0)])),
        ("active halfspace:", Program::new(&[(1.0, 1.0, 2.0)])),
        ("infeasible:", Program::new(&[(1.0, 0.0, -1.0)])),
    ]
}

/// The outer and inner caps and tolerances are all runtime configuration, and
/// the sweep cap has no default: it must suit `projection_tolerance`.
fn config() -> ConstrainedProjectedConfig<f64> {
    ConstrainedProjectedConfig {
        max_iterations: 5_000,
        tolerance: 1e-10,
        projection_max_sweeps: 500,
        projection_tolerance: 1e-10,
    }
}

fn solve_typed(program: &Program) -> String {
    let shape = match program.shape() {
        Ok(shape) => shape,
        Err(e) => return format!("rejected: {e:?}"),
    };
    let mut x = dense(&[0.0, 0.0]);
    let mut workspace = match ClusterConstrainedWorkspace::new(shape.variables, shape.constraints)
    {
        Ok(workspace) => workspace,
        Err(e) => return format!("workspace rejected: {e:?}"),
    };
    let context = ClusterExecutionContext::new(
        ClusterCancellationToken::new(),
        0,
        ClusterValidationPolicy::ValidateAllInputs,
    );
    match solve_constrained_projected_first_order_dyn(
        program,
        STEP_SCALE,
        &mut x,
        &mut workspace,
        &config(),
        &context,
    ) {
        Ok(record) => format!(
            "{} in {} iteration(s); x = {}; violation = {:.3e}; projection cap hits = {}",
            verdict(record.report.status(), record.report.termination()),
            record.report.iterations_executed(),
            render(&x),
            record.max_constraint_violation,
            record.projection_cap_hits,
        ),
        Err(e) => format!("failed: {e:?}"),
    }
}

/// `Converged` means feasible within `projection_tolerance` (Amendment 5),
/// stationary, and produced by a projection that did not hit its cap (RFC 033), so
/// a non-converged constrained solve with `NoProgress` is an infeasible
/// polyhedron or a capped projection, not merely an unfinished one.
fn verdict(status: SolveStatus, termination: TerminationReason) -> String {
    match (status, termination) {
        (SolveStatus::Converged, _) => "converged".to_owned(),
        (SolveStatus::NotConverged, TerminationReason::NoProgress) => {
            "not converged (no progress: stationary but not feasible)".to_owned()
        }
        (SolveStatus::NotConverged, _) => "not converged".to_owned(),
        // `#[non_exhaustive]` downstream: name a future variant, never panic.
        _ => "unrecognized status".to_owned(),
    }
}

fn describe(outcome: &BatchItemOutcome<f64>) -> String {
    match outcome {
        BatchItemOutcome::Solved { solution, report } => {
            let solution = match solution {
                ClusterSolution::DenseVector(x) => render(x),
                _ => "solution shape not recognized by this example".to_owned(),
            };
            format!(
                "{} in {} iteration(s); x = {solution}",
                verdict(report.status(), report.termination()),
                report.iterations_executed(),
            )
        }
        BatchItemOutcome::Failed { error } => format!("failed: {error:?}"),
        BatchItemOutcome::Cancelled => "cancelled".to_owned(),
        BatchItemOutcome::Panicked => "worker panic contained at the item boundary".to_owned(),
    }
}

/// Per-element fallible access, no indexing.
fn render(v: &DenseVector<f64>) -> String {
    let mut parts = Vec::with_capacity(v.len());
    for i in 0..v.len() {
        match v.get(i) {
            Ok(value) => parts.push(format!("{value:.6}")),
            Err(e) => parts.push(format!("<{e:?}>")),
        }
    }
    format!("[{}]", parts.join(", "))
}

/// `DenseVector::from_vec` is fallible in general; an example should not hide
/// that behind `unwrap`.
fn dense(values: &[f64]) -> DenseVector<f64> {
    match DenseVector::from_vec(values.to_vec()) {
        Ok(v) => v,
        Err(e) => panic!("example vector literal is not a valid dense vector: {e:?}"),
    }
}

fn matrix(rows: usize, cols: usize, data: Vec<f64>) -> DenseMatrix<f64> {
    match DenseMatrix::from_row_major_vec(rows, cols, data) {
        Ok(m) => m,
        Err(e) => panic!("example matrix literal is not a valid dense matrix: {e:?}"),
    }
}
