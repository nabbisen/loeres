//! Capacity-limited dispatch of three production units against a demand floor.
//!
//! A grid operator, a logistics planner or a plant scheduler has several units
//! that can each supply up to a fixed capacity, at a cost that rises with output,
//! and must together meet a demand. The question is how much each unit should
//! produce: the cheapest allocation that still meets demand, if demand can be met
//! at all. The solve runs on a server-side worker, where it is one of many
//! dispatch problems in a batch.
//!
//! The reduction from the operational problem to the programme is the part a
//! reader cannot reconstruct, so it is stated here:
//!
//! - **Cost.** Unit `i` producing `xᵢ` costs `½·qᵢ·xᵢ² + cᵢ·xᵢ`, with
//!   `q = (0.5, 1.0, 2.0)` and `c = (1.0, 2.0, 0.5)`. The marginal cost rises with
//!   output, so the cheapest plan spreads load across units rather than loading
//!   one.
//! - **Capacity.** `0 ≤ xᵢ ≤ capᵢ` with `cap = (6, 6, 6)`. That is the box.
//! - **Demand floor.** `x₀ + x₁ + x₂ ≥ D` is written in the solver's form
//!   `Ax ≤ b` as `−x₀ − x₁ − x₂ ≤ −D`. That single row is the only linear
//!   constraint.
//! - **Programme.** `minimize ½xᵀQx + cᵀx` over the box and the row, with
//!   `Q = diag(q)`. `Q` is positive definite, so the programme has at most one
//!   minimiser.
//!
//! The step is `0 < α < 2/λmax(Q)`; `λmax = 2` here, so `α = 0.4` is inside.
//!
//! Three demands are run. For `D = 10` and `D = 16` demand can be met, and the
//! result is the cheapest dispatch. For `D = 20` it cannot: capacity totals 18, so
//! at least two units of demand go unmet whatever the dispatch. The solve then
//! reports `not converged`, and the example does **not** present its setpoints as
//! an optimum. What it reports for that case is the shortfall, which no dispatch
//! can avoid.
//!
//! For each case the output prints the status, the setpoint and utilisation of
//! every unit, and the delivered total against the demand. A dispatcher acts on
//! the setpoints and the shortfall, not on the status word alone.

use loeres::{
    BoxBounds, LinearInequalities, QuadraticObjective, QuadraticProgram, SolveStatus, SolverError,
    TerminationReason, VectorAccess,
};
use loeres_backend_std::{DenseMatrix, DenseVector};
use loeres_cluster::{
    ClusterCancellationToken, ClusterConstrainedWorkspace, ClusterExecutionContext,
    ClusterValidationPolicy, ConstrainedProjectedConfig,
    solve_constrained_projected_first_order_dyn,
};

/// Step scale, `0 < α < 2/λmax(Q)` with `λmax(Q) = 2`.
const STEP_SCALE: f64 = 0.4;

/// Capacity of each unit; the upper bound of its box.
const CAPACITY: [f64; 3] = [6.0, 6.0, 6.0];

/// The dispatch programme for one demand. `Q` and `c` are fixed; only the demand
/// row changes between cases.
#[derive(Clone)]
struct Dispatch {
    q: DenseMatrix<f64>,
    c: DenseVector<f64>,
    lower: DenseVector<f64>,
    upper: DenseVector<f64>,
    a: DenseMatrix<f64>,
    b: DenseVector<f64>,
}

impl Dispatch {
    fn new(demand: f64) -> Self {
        Self {
            q: diagonal(&[0.5, 1.0, 2.0]),
            c: dense(&[1.0, 2.0, 0.5]),
            lower: dense(&[0.0; 3]),
            upper: dense(&CAPACITY),
            // `−Σxᵢ ≤ −D`: the demand floor in the solver's `Ax ≤ b` form.
            a: matrix(1, 3, vec![-1.0, -1.0, -1.0]),
            b: dense(&[-demand]),
        }
    }
}

impl QuadraticObjective<f64> for Dispatch {
    type Hessian = DenseMatrix<f64>;
    type Linear = DenseVector<f64>;
    fn hessian(&self) -> &Self::Hessian {
        &self.q
    }
    fn linear_term(&self) -> &Self::Linear {
        &self.c
    }
}

impl BoxBounds<f64> for Dispatch {
    type Bound = DenseVector<f64>;
    fn lower_bounds(&self) -> &Self::Bound {
        &self.lower
    }
    fn upper_bounds(&self) -> &Self::Bound {
        &self.upper
    }
}

impl LinearInequalities<f64> for Dispatch {
    type Constraints = DenseMatrix<f64>;
    type Rhs = DenseVector<f64>;
    fn constraint_matrix(&self) -> &Self::Constraints {
        &self.a
    }
    fn constraint_rhs(&self) -> &Self::Rhs {
        &self.b
    }
}

fn main() -> Result<(), SolverError> {
    for demand in [10.0, 16.0, 20.0] {
        dispatch(demand)?;
    }
    Ok(())
}

/// Solves one demand and acts on the result: prints the setpoints and the
/// delivered total, and the shortfall when demand cannot be met.
fn dispatch(demand: f64) -> Result<(), SolverError> {
    let program = Dispatch::new(demand);
    let shape = program.shape()?;
    let mut x = dense(&[0.0; 3]);
    let mut workspace = ClusterConstrainedWorkspace::new(shape.variables, shape.constraints)?;
    let config = ConstrainedProjectedConfig {
        max_iterations: 5_000,
        tolerance: 1e-10,
        projection_max_sweeps: 500,
        projection_tolerance: 1e-10,
    };
    let context = ClusterExecutionContext::new(
        ClusterCancellationToken::new(),
        0,
        ClusterValidationPolicy::ValidateAllInputs,
    );
    let record = solve_constrained_projected_first_order_dyn(
        &program,
        STEP_SCALE,
        &mut x,
        &mut workspace,
        &config,
        &context,
    )?;

    let verdict = match (record.report.status(), record.report.termination()) {
        (SolveStatus::Converged, _) => "converged".to_owned(),
        (SolveStatus::NotConverged, TerminationReason::NoProgress) => {
            "not converged (no progress)".to_owned()
        }
        (SolveStatus::NotConverged, _) => "not converged".to_owned(),
        // `#[non_exhaustive]` downstream: name a future variant, never panic.
        _ => "unrecognized status".to_owned(),
    };
    println!(
        "demand {demand:.1}: {verdict} in {} iteration(s)",
        record.report.iterations_executed()
    );

    let mut delivered = 0.0;
    for (unit, &cap) in CAPACITY.iter().enumerate() {
        let produced = x.get(unit)?;
        println!(
            "  unit {}: set to {produced:.3} of {cap:.1} ({:.0}% utilised)",
            unit + 1,
            100.0 * produced / cap
        );
        delivered += produced;
    }
    println!("  delivered {delivered:.3} of demanded {demand:.3}");
    // A tolerance, not `<`: the iterate can sit a round-off below the floor.
    if demand - delivered > 1e-6 {
        println!(
            "  shortfall {:.3} units: no dispatch within capacity can avoid it",
            demand - delivered
        );
    }
    println!();
    Ok(())
}

fn diagonal(values: &[f64]) -> DenseMatrix<f64> {
    let n = values.len();
    let mut data = vec![0.0; n * n];
    for (i, &value) in values.iter().enumerate() {
        data[i * n + i] = value;
    }
    matrix(n, n, data)
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
