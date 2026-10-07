//! How much work does a constrained solve take, as the problem grows and as it gets harder?
//!
//! A planner who builds a constrained model wants to know two things before committing to
//! it: how the solve's work grows with the number of variables, and how it grows when the
//! variables are more tightly coupled. This program answers both on a family of problems
//! whose difficulty is set by one number, and prints the work each solve did.
//!
//! The family is a tridiagonal objective with `off` on the off-diagonals, a box, and `m`
//! sliding-window constraints that each cap three neighbouring variables. Increasing `off`
//! makes the objective more tightly coupled, and that is the difficulty axis.
//!
//! Two facts the table shows, which a reader should keep in mind:
//!
//! - **Work does not grow with size, but it does grow with coupling.** Going from four
//!   variables to sixty-four barely changes the number of iterations. Raising `off` from
//!   `0.10` to `0.99` multiplies it roughly fortyfold.
//! - **A solve can finish and still be wrong.** The program accepts a result only when the
//!   status is `Converged`, the constraint violation is within `1e-10`, and no projection hit
//!   its sweep cap. Each row shows that decision, so a reader sees the rule applied, not only
//!   a count.
//!
//! The counts are reproducible on a given target. The program prints no timings: how long a
//! solve takes depends on the machine, and this example does not claim a speed.

use loeres::{BoxBounds, LinearInequalities, QuadraticObjective, SolveStatus};
use loeres_backend_std::{DenseMatrix, DenseVector};
use loeres_cluster::{
    ClusterCancellationToken, ClusterConstrainedWorkspace, ClusterExecutionContext,
    ClusterValidationPolicy, ConstrainedProjectedConfig,
    solve_constrained_projected_first_order_dyn,
};

/// The step scale, admissible across every coupling used here: `2/U` with `U = 2 + 2·off`
/// is at least `0.503` at `off = 0.99`.
const STEP_SCALE: f64 = 0.3;

/// A family member: `n` variables, `m` window constraints, and the off-diagonal `off`.
#[derive(Clone, Copy)]
struct Family {
    n: usize,
    m: usize,
    off: f64,
}

/// The dynamic QP for one family member.
#[derive(Clone)]
struct Problem {
    q: DenseMatrix<f64>,
    c: DenseVector<f64>,
    lower: DenseVector<f64>,
    upper: DenseVector<f64>,
    a: DenseMatrix<f64>,
    b: DenseVector<f64>,
}

impl Problem {
    fn new(family: Family) -> Result<Self, String> {
        let Family { n, m, off } = family;
        let mut q = vec![0.0; n * n];
        for i in 0..n {
            q[i * n + i] = 2.0;
            if i + 1 < n {
                q[i * n + i + 1] = off;
                q[(i + 1) * n + i] = off;
            }
        }
        // Row `r` caps the three variables `r`, `r+1`, `r+2` (modulo `n`) at a sum of one.
        let mut a = vec![0.0; m * n];
        for r in 0..m {
            for k in 0..3 {
                a[r * n + (r + k) % n] = 1.0;
            }
        }
        Ok(Self {
            q: matrix(n, n, q)?,
            c: dense(vec![-1.0; n])?,
            lower: dense(vec![0.0; n])?,
            upper: dense(vec![10.0; n])?,
            a: matrix(m, n, a)?,
            b: dense(vec![1.0; m])?,
        })
    }
}

fn matrix(rows: usize, cols: usize, data: Vec<f64>) -> Result<DenseMatrix<f64>, String> {
    DenseMatrix::from_row_major_vec(rows, cols, data)
        .map_err(|e| format!("not a valid dense container: {e:?}"))
}

fn dense(values: Vec<f64>) -> Result<DenseVector<f64>, String> {
    DenseVector::from_vec(values).map_err(|e| format!("not a valid dense container: {e:?}"))
}

impl QuadraticObjective<f64> for Problem {
    type Hessian = DenseMatrix<f64>;
    type Linear = DenseVector<f64>;
    fn hessian(&self) -> &Self::Hessian {
        &self.q
    }
    fn linear_term(&self) -> &Self::Linear {
        &self.c
    }
}

impl BoxBounds<f64> for Problem {
    type Bound = DenseVector<f64>;
    fn lower_bounds(&self) -> &Self::Bound {
        &self.lower
    }
    fn upper_bounds(&self) -> &Self::Bound {
        &self.upper
    }
}

impl LinearInequalities<f64> for Problem {
    type Constraints = DenseMatrix<f64>;
    type Rhs = DenseVector<f64>;
    fn constraint_matrix(&self) -> &Self::Constraints {
        &self.a
    }
    fn constraint_rhs(&self) -> &Self::Rhs {
        &self.b
    }
}

/// The work one solve did, and whether the result is accepted.
struct Row {
    family: Family,
    iterations: u32,
    cap_hits: u32,
    accepted: bool,
}

fn solve(family: Family) -> Result<Row, String> {
    let problem = Problem::new(family)?;
    let mut workspace = ClusterConstrainedWorkspace::new(family.n, family.m)
        .map_err(|e| format!("workspace rejected: {e:?}"))?;
    let config = ConstrainedProjectedConfig {
        max_iterations: 20_000,
        tolerance: 1e-10,
        projection_max_sweeps: 500,
        projection_tolerance: 1e-10,
    };
    let context = ClusterExecutionContext::new(
        ClusterCancellationToken::new(),
        0,
        ClusterValidationPolicy::ValidateAllInputs,
    );
    let mut x = dense(vec![0.0; family.n])?;
    let record = solve_constrained_projected_first_order_dyn(
        &problem,
        STEP_SCALE,
        &mut x,
        &mut workspace,
        &config,
        &context,
    )
    .map_err(|e| format!("solve failed: {e:?}"))?;
    // The rule: accept only a converged, feasible result whose every projection finished
    // within its cap. Any other outcome is reported, not used.
    let accepted = record.report.status() == SolveStatus::Converged
        && record.max_constraint_violation <= config.projection_tolerance
        && record.projection_cap_hits == 0;
    Ok(Row {
        family,
        iterations: record.report.iterations_executed(),
        cap_hits: record.projection_cap_hits,
        accepted,
    })
}

fn main() -> Result<(), String> {
    println!("Size, at moderate coupling (off = 0.50):");
    print_header();
    for n in [4, 16, 64] {
        print_row(solve(Family {
            n,
            m: n / 2,
            off: 0.50,
        })?);
    }

    println!();
    println!("Coupling, at fixed size (n = 32, m = 16):");
    print_header();
    for off in [0.10, 0.90, 0.99] {
        print_row(solve(Family { n: 32, m: 16, off })?);
    }

    println!();
    println!("A result is used only when accepted = yes: converged, feasible within 1e-10,");
    println!("and with no projection stopped by its sweep cap.");
    Ok(())
}

fn print_header() {
    println!(
        "{:>6} {:>6} {:>6} {:>12} {:>10} {:>10}",
        "n", "m", "off", "iterations", "cap hits", "accepted"
    );
}

fn print_row(row: Row) {
    println!(
        "{:>6} {:>6} {:>6.2} {:>12} {:>10} {:>10}",
        row.family.n,
        row.family.m,
        row.family.off,
        row.iterations,
        row.cap_hits,
        if row.accepted { "yes" } else { "no" },
    );
}
