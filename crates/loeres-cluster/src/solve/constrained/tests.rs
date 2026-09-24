//! Tests for the RFC 027 S3 constrained projected first-order cluster kernel.

use super::*;
use crate::model::{
    ClusterProjectedFirstOrderProblem, ClusterProjectedFirstOrderWorkspace,
    ProjectedFirstOrderConfig,
};
use crate::runtime::{ClusterCancellationToken, ClusterSolveConfig};
use crate::solve::{solve_batch, solve_projected_first_order_dyn};
use loeres::validation::{TrustToken, TrustedByCaller};
use loeres::{
    BoxBounds, LinearInequalities, MatrixView, QuadraticObjective, SolveStatus, TerminationReason,
    VectorView,
};
use loeres::{Dim2, DimensionKind};
use loeres_backend_std::{DenseMatrix, SparseIngestOptions, SparseMatrix};

fn dv(v: &[f64]) -> DenseVector<f64> {
    DenseVector::from_vec(v.to_vec()).unwrap()
}

fn ctx(policy: ClusterValidationPolicy) -> ClusterExecutionContext {
    ClusterExecutionContext::new(ClusterCancellationToken::new(), 0, policy)
}

fn scan() -> ClusterExecutionContext {
    ctx(ClusterValidationPolicy::ValidateAllInputs)
}

fn cfg(projection_tolerance: f64, sweeps: u32) -> ConstrainedProjectedConfig<f64> {
    ConstrainedProjectedConfig {
        max_iterations: 5000,
        tolerance: 1e-12,
        projection_max_sweeps: sweeps,
        projection_tolerance,
    }
}

/// A dynamic quadratic program; `C`/`R` are the constraint matrix and
/// right-hand-side storage, so one fixture serves dense `A`, CSR `A`, and the
/// canonical `m = 0` empty views.
struct Qp<C, R> {
    q: DenseMatrix<f64>,
    c: DenseVector<f64>,
    lo: DenseVector<f64>,
    hi: DenseVector<f64>,
    a: C,
    b: R,
}

impl<C, R> QuadraticObjective<f64> for Qp<C, R> {
    type Hessian = DenseMatrix<f64>;
    type Linear = DenseVector<f64>;
    fn hessian(&self) -> &Self::Hessian {
        &self.q
    }
    fn linear_term(&self) -> &Self::Linear {
        &self.c
    }
}
impl<C, R> BoxBounds<f64> for Qp<C, R> {
    type Bound = DenseVector<f64>;
    fn lower_bounds(&self) -> &Self::Bound {
        &self.lo
    }
    fn upper_bounds(&self) -> &Self::Bound {
        &self.hi
    }
}
impl<C: MatrixAccess<Scalar = f64>, R: VectorAccess<Scalar = f64>> LinearInequalities<f64>
    for Qp<C, R>
{
    type Constraints = C;
    type Rhs = R;
    fn constraint_matrix(&self) -> &C {
        &self.a
    }
    fn constraint_rhs(&self) -> &R {
        &self.b
    }
}

fn identity(n: usize) -> DenseMatrix<f64> {
    let mut data = vec![0.0; n * n];
    for i in 0..n {
        data[i * n + i] = 1.0;
    }
    DenseMatrix::from_row_major_vec(n, n, data).unwrap()
}

type Dense = Qp<DenseMatrix<f64>, DenseVector<f64>>;

/// `min ½‖x − t‖²` over `[−10, 10]ⁿ` and `Ax ≤ b` (`A` row-major, `m × n`).
fn projection(n: usize, a: &[f64], b: &[f64], t: &[f64]) -> Dense {
    Qp {
        q: identity(n),
        c: dv(&t.iter().map(|v| -v).collect::<Vec<_>>()),
        lo: dv(&vec![-10.0; n]),
        hi: dv(&vec![10.0; n]),
        a: DenseMatrix::from_row_major_vec(b.len(), n, a.to_vec()).unwrap(),
        b: dv(b),
    }
}

fn solve(
    problem: &Dense,
    step: f64,
    x0: &[f64],
    config: &ConstrainedProjectedConfig<f64>,
) -> (DenseVector<f64>, ConstrainedSolveRecord<f64>) {
    let shape = problem.shape().unwrap();
    let mut x = dv(x0);
    let mut ws = ClusterConstrainedWorkspace::new(shape.variables, shape.constraints).unwrap();
    let record = solve_constrained_projected_first_order_dyn(
        problem,
        step,
        &mut x,
        &mut ws,
        config,
        &scan(),
    )
    .unwrap();
    (x, record)
}

fn assert_feasible(record: &ConstrainedSolveRecord<f64>, tolerance: f64) {
    assert!(
        record.max_constraint_violation <= tolerance,
        "returned point violates its own constraints by {}",
        record.max_constraint_violation
    );
}

fn coords(x: &DenseVector<f64>) -> Vec<f64> {
    (0..x.len()).map(|i| x.get(i).unwrap()).collect()
}

// ---------------------------------------------------------------------------
// m >= 1 correctness (the RFC 027 Amendment 3 shapes)
// ---------------------------------------------------------------------------

/// The review-054 regression case — exact `A`, `b`, `t`, expected value.
#[test]
fn review_054_regression_two_constraints_converge_to_the_exact_projection() {
    let problem = projection(
        2,
        &[1.6, -1.3082, -0.0457, 0.9197],
        &[1.6013, -0.3988],
        &[5.3481, 4.7872],
    );
    let (x, record) = solve(&problem, 1.0, &[0.0, 0.0], &cfg(1e-14, 5000));
    assert_eq!(record.report.status(), SolveStatus::Converged);
    assert_eq!(record.projection_cap_hits, 0);
    assert_feasible(&record, 1e-14);
    let x = coords(&x);
    assert!(
        (x[0] - 0.673643).abs() < 1e-6 && (x[1] + 0.400146).abs() < 1e-6,
        "x = {x:?}"
    );
}

/// Vertex with two active constraints: `min ½‖x−(3,1)‖²`, `x₀+x₁ ≤ 1.5`,
/// `x₀−x₁ ≤ 0.5` → `(1, 0.5)`. KKT: `∇f = (−2,−0.5)`, `λ = (1.25, 0.75) ≥ 0`.
#[test]
fn a_vertex_with_two_active_constraints_is_found_exactly() {
    let problem = projection(2, &[1.0, 1.0, 1.0, -1.0], &[1.5, 0.5], &[3.0, 1.0]);
    let (x, record) = solve(&problem, 1.0, &[0.0, 0.0], &cfg(1e-13, 5000));
    assert_eq!(record.report.status(), SolveStatus::Converged);
    assert_eq!(record.projection_cap_hits, 0);
    assert_feasible(&record, 1e-13);
    let x = coords(&x);
    assert!(
        (x[0] - 1.0).abs() < 1e-9 && (x[1] - 0.5).abs() < 1e-9,
        "{x:?}"
    );
}

/// A constraint active early and slack at the optimum: `min ½‖x‖²` from
/// `(1,1)`, step `0.3`, `x₀+x₁ ≤ 0.2` (violated by the first candidate
/// `(0.7,0.7)`, so active), `x₀−x₁ ≤ 5`; optimum the origin, where both are slack.
#[test]
fn a_constraint_active_early_and_inactive_at_the_optimum() {
    let mut problem = projection(2, &[1.0, 1.0, 1.0, -1.0], &[0.2, 5.0], &[0.0, 0.0]);
    problem.c = dv(&[0.0, 0.0]);
    let (x, record) = solve(&problem, 0.3, &[1.0, 1.0], &cfg(1e-13, 5000));
    assert_eq!(record.report.status(), SolveStatus::Converged);
    assert_feasible(&record, 1e-13);
    let x = coords(&x);
    assert!(x[0].abs() < 1e-9 && x[1].abs() < 1e-9, "{x:?}");
}

/// One binding halfspace (`M = 1`): projection of `(2,2)` onto `x₀+x₁ ≤ 1` is
/// `(0.5, 0.5)` (KKT `λ = 1.5`).
#[test]
fn a_single_active_halfspace_matches_its_closed_form() {
    let problem = projection(2, &[1.0, 1.0], &[1.0], &[2.0, 2.0]);
    let (x, record) = solve(&problem, 0.4, &[0.0, 0.0], &cfg(1e-12, 5000));
    assert_eq!(record.report.status(), SolveStatus::Converged);
    assert_feasible(&record, 1e-12);
    let x = coords(&x);
    assert!(
        (x[0] - 0.5).abs() < 1e-6 && (x[1] - 0.5).abs() < 1e-6,
        "{x:?}"
    );
}

/// A matrix that is deliberately *not* contiguous and offers no fast path: it
/// stores only its non-zeros as `(row, col, value)` and answers `get` with an
/// implicit zero, as a CSR matrix does. It predates the real `SparseMatrix`
/// test below and is kept because it isolates the property that matters — the
/// kernel sees only `MatrixAccess` — from `SparseMatrix`'s own behaviour.
struct Triplets {
    rows: usize,
    cols: usize,
    entries: Vec<(usize, usize, f64)>,
}

impl MatrixAccess for Triplets {
    type Scalar = f64;
    fn dims(&self) -> Dim2 {
        Dim2::new(self.rows, self.cols)
    }
    fn dimension_kind(&self) -> DimensionKind {
        DimensionKind::Dynamic
    }
    fn get(&self, row: usize, col: usize) -> Result<f64, SolverError> {
        if row >= self.rows || col >= self.cols {
            return Err(SolverError::DimensionMismatch { lhs: 0, rhs: 0 });
        }
        Ok(self
            .entries
            .iter()
            .find(|&&(r, c, _)| r == row && c == col)
            .map_or(0.0, |&(_, _, v)| v))
    }
}

/// The same regression polytope with `A` held as a real
/// `loeres-backend-std::SparseMatrix` (CSR), the integration evidence review 056
/// deferred from S3. `SparseMatrix::get` is an implicit-zero lookup, so this also
/// exercises a stored zero entry being absent from `A`.
#[test]
fn a_real_csr_sparse_matrix_gives_the_same_answer() {
    // A[1][0] is genuinely zero here, so it is not stored at all.
    let triplets = [(0, 0, 1.6), (0, 1, -1.3082), (1, 1, 0.9197)];
    let a = SparseMatrix::from_triplets(2, 2, &triplets, SparseIngestOptions::default()).unwrap();
    assert_eq!(a.nnz(), 3);
    let problem = Qp {
        q: identity(2),
        c: dv(&[-5.3481, -4.7872]),
        lo: dv(&[-10.0, -10.0]),
        hi: dv(&[10.0, 10.0]),
        a,
        b: dv(&[1.6013, -0.3988]),
    };
    let dense = projection(
        2,
        &[1.6, -1.3082, 0.0, 0.9197],
        &[1.6013, -0.3988],
        &[5.3481, 4.7872],
    );

    let mut x = dv(&[0.0, 0.0]);
    let mut ws = ClusterConstrainedWorkspace::new(2, 2).unwrap();
    let record = solve_constrained_projected_first_order_dyn(
        &problem,
        1.0,
        &mut x,
        &mut ws,
        &cfg(1e-14, 5000),
        &scan(),
    )
    .unwrap();
    let (x_dense, record_dense) = solve(&dense, 1.0, &[0.0, 0.0], &cfg(1e-14, 5000));

    assert_feasible(&record, 1e-14);
    assert_eq!(record.projection_cap_hits, 0);
    // CSR and dense storage of the same `A` must not change a single bit: the
    // kernel performs the same operations in the same order over either.
    assert_eq!(coords(&x), coords(&x_dense));
    assert_eq!(record.report, record_dense.report);
}

/// The same regression polytope with `A` held sparsely and *no* contiguous
/// fast path: the kernel reads it only through `MatrixAccess::get`.
#[test]
fn a_non_contiguous_constraint_matrix_gives_the_same_answer() {
    let problem = Qp {
        q: identity(2),
        c: dv(&[-5.3481, -4.7872]),
        lo: dv(&[-10.0, -10.0]),
        hi: dv(&[10.0, 10.0]),
        a: Triplets {
            rows: 2,
            cols: 2,
            entries: vec![
                (0, 0, 1.6),
                (0, 1, -1.3082),
                (1, 0, -0.0457),
                (1, 1, 0.9197),
            ],
        },
        b: dv(&[1.6013, -0.3988]),
    };
    let mut x = dv(&[0.0, 0.0]);
    let mut ws = ClusterConstrainedWorkspace::new(2, 2).unwrap();
    let record = solve_constrained_projected_first_order_dyn(
        &problem,
        1.0,
        &mut x,
        &mut ws,
        &cfg(1e-14, 5000),
        &scan(),
    )
    .unwrap();
    assert_feasible(&record, 1e-14);
    let x = coords(&x);
    assert!(
        (x[0] - 0.673643).abs() < 1e-6 && (x[1] + 0.400146).abs() < 1e-6,
        "{x:?}"
    );
}

/// An infeasible polyhedron (`x ≤ −1` and `x ≥ 1`) needs no special handling:
/// it runs to the cap and reports its true violation (RFC 027 §0.3.4).
#[test]
fn an_infeasible_polyhedron_hits_the_cap_and_reports_its_true_violation() {
    let problem = projection(1, &[1.0, -1.0], &[-1.0, -1.0], &[0.0]);
    let mut x = dv(&[0.0]);
    let mut ws = ClusterConstrainedWorkspace::new(1, 2).unwrap();
    let record = solve_constrained_projected_first_order_dyn(
        &problem,
        0.5,
        &mut x,
        &mut ws,
        &cfg(1e-12, 200),
        &scan(),
    )
    .unwrap();
    assert!(record.projection_cap_hits > 0);
    assert!(record.max_constraint_violation > 0.5);
}

/// RFC 027 Amendment 5 (§0.5.1): `Converged` means feasible. The capped
/// projection map has a fixed point even when the polyhedron is empty, so the
/// outer step stops moving; that is `NotConverged`/`NoProgress`, never
/// `Converged`, and the violation and cap hits stay reported unchanged. RFC 034
/// Amendment 2 adds the heuristic `infeasibility_evidence` field beside them; the
/// status is not `Infeasible` because no such status exists.
#[test]
fn an_infeasible_polyhedron_is_not_converged_and_sets_the_evidence_field() {
    let problem = projection(1, &[1.0, -1.0], &[-1.0, -1.0], &[0.0]);
    let mut x = dv(&[0.0]);
    let mut ws = ClusterConstrainedWorkspace::new(1, 2).unwrap();
    let record = solve_constrained_projected_first_order_dyn(
        &problem,
        0.5,
        &mut x,
        &mut ws,
        &cfg(1e-12, 200),
        &scan(),
    )
    .unwrap();
    assert_eq!(record.report.status(), SolveStatus::NotConverged);
    assert_eq!(record.report.termination(), TerminationReason::NoProgress);
    assert!(record.infeasibility_evidence);
    assert!(record.projection_cap_hits > 0);
    assert!((record.max_constraint_violation - 2.0).abs() < 1e-9);
}

#[test]
fn a_feasible_control_still_reports_converged() {
    // x <= 1 and -x <= 1: the same shape, feasible, optimum x = 0.
    let problem = projection(1, &[1.0, -1.0], &[1.0, 1.0], &[0.0]);
    let mut x = dv(&[0.0]);
    let mut ws = ClusterConstrainedWorkspace::new(1, 2).unwrap();
    let record = solve_constrained_projected_first_order_dyn(
        &problem,
        0.5,
        &mut x,
        &mut ws,
        &cfg(1e-12, 200),
        &scan(),
    )
    .unwrap();
    assert_eq!(record.report.status(), SolveStatus::Converged);
    assert_eq!(
        record.report.termination(),
        TerminationReason::ConvergenceCriterion
    );
    assert_eq!(record.projection_cap_hits, 0);
    assert_eq!(record.max_constraint_violation, 0.0);
}

// ---------------------------------------------------------------------------
// RFC 034 Amendment 2: `infeasibility_evidence` is a heuristic field, set only
// when the final projection capped, the cap is at least 64 sweeps, max|λ| at the
// final sweep is at least 1.9 × at the midpoint, the terminal violation is not
// shrinking, AND it exceeds `projection_tolerance`. It is wrong in both
// directions; these tests pin the deterministic cases and measure the rates.
// ---------------------------------------------------------------------------

fn solve_status(
    problem: &Dense,
    step: f64,
    sweeps: u32,
    x0: &[f64],
) -> (SolveStatus, ConstrainedSolveRecord<f64>) {
    let mut x = dv(x0);
    let mut ws =
        ClusterConstrainedWorkspace::new(x0.len(), problem.shape().unwrap().constraints).unwrap();
    let config = ConstrainedProjectedConfig {
        max_iterations: 5000,
        tolerance: 1e-10,
        projection_max_sweeps: sweeps,
        projection_tolerance: 1e-10,
    };
    let record = solve_constrained_projected_first_order_dyn(
        problem,
        step,
        &mut x,
        &mut ws,
        &config,
        &scan(),
    )
    .unwrap();
    (record.report.status(), record)
}

/// The guard for the false-positive shape RFC 034 found: a feasible problem whose
/// rows are never active, so every multiplier is identically zero. A ratio
/// `λ(final)/λ(midpoint)` reads `0/0` there. The projection is capped (one sweep
/// cannot clamp a far target into the box) and the multipliers are trivially
/// "not decreasing", so the other conditions hold — only the positive-violation
/// condition keeps the evidence field false.
#[test]
fn a_feasible_problem_whose_rows_are_never_active_sets_no_evidence() {
    // x0 + x1 <= 1e9 and -x0 <= 1e9 never bind; the box [-10, 10] does.
    let problem = projection(2, &[1.0, 1.0, -1.0, 0.0], &[1e9, 1e9], &[50.0, 50.0]);
    let (status, record) = solve_status(&problem, 1.0, 1, &[0.0, 0.0]);
    assert!(record.projection_cap_hits > 0, "the projection must cap");
    assert_eq!(record.max_constraint_violation, 0.0);
    assert_eq!(status, SolveStatus::NotConverged);
    assert!(!record.infeasibility_evidence);
}

/// The criterion that matters: a nearly-parallel FEASIBLE problem whose projection
/// caps is `NotConverged` and sets no evidence (RFC 031's family, ε = 0.001).
#[test]
fn a_nearly_parallel_feasible_projection_that_caps_sets_no_evidence() {
    let eps = 0.001;
    let problem = projection(
        2,
        &[1.0, 0.0, 1.0, eps],
        &[1.0, 1.0 + eps],
        &[3.0, 1.0 + eps],
    );
    let (status, record) = solve_status(&problem, 1.0, 100_000, &[0.0, 0.0]);
    assert!(record.projection_cap_hits > 0, "the projection must cap");
    assert_eq!(status, SolveStatus::NotConverged);
    assert!(!record.infeasibility_evidence);
}

fn lcg(seed: u64) -> impl FnMut() -> f64 {
    let mut state = seed;
    move || {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((state >> 11) as f64) / ((1u64 << 53) as f64)
    }
}

/// A feasible polytope found by the randomized measurement below: three rows
/// nearly parallel to one another and a fourth nearly antiparallel, so the
/// feasible region is a thin sliver and Dykstra converges slowly. At a cap of 10000
/// sweeps the projection is still capped with a positive violation, but the
/// multipliers have plateaued (final/midpoint ratio about 1.01), so the divergence
/// condition is what keeps it `NotConverged`. (At caps of 100 to 1000 the same
/// problem set the original three-condition rule's status; RFC 034 C1.)
fn thin_sliver() -> Dense {
    projection(
        2,
        &[
            -1.2577257973197087,
            1.4451877576296583,
            -1.258018400284745,
            1.4445350472678065,
            -0.6286158630589771,
            -1.9820704555474795,
            -1.288998551079334,
            1.426784101205777,
            1.3178755616905868,
            -1.427315144595859,
        ],
        &[
            3.9702890534542297,
            3.513513360778795,
            -0.9810486113686183,
            3.7615599338942487,
            -3.2660715103109026,
        ],
        &[4.111709162799526, 4.75894809479691],
    )
}

#[test]
fn a_slow_feasible_projection_whose_multipliers_have_plateaued_sets_no_evidence() {
    let (status, record) = solve_status(&thin_sliver(), 1.0, 10_000, &[0.0, 0.0]);
    assert!(record.projection_cap_hits > 0, "the projection must cap");
    assert!(
        record.max_constraint_violation > 1e-10,
        "still infeasible at the cap"
    );
    assert_eq!(status, SolveStatus::NotConverged);
    assert!(!record.infeasibility_evidence);
}

/// `(n, A row-major, b, target)`.
type RandomInstance = (usize, Vec<f64>, Vec<f64>, Vec<f64>);

/// A random feasible polytope and target: rows in `[-2, 2]`, about 40% nearly
/// parallel to the first and 10% nearly *anti*parallel to it (the thin slivers
/// RFC 034 Amendment 1 is about), feasible by construction (`b = A·x_f + slack`),
/// with a tiny slack on half of the sliver rows so the region really is thin.
fn random_feasible(next: &mut impl FnMut() -> f64, thin: bool) -> Option<RandomInstance> {
    let n = 2 + (next() * 2.0) as usize;
    let m = 2 + (next() * 4.0) as usize;
    let mut a: Vec<f64> = (0..m * n).map(|_| next() * 4.0 - 2.0).collect();
    let mut sliver = vec![false; m];
    for i in 1..m {
        let shape = next();
        if shape < 0.5 {
            let eps = 10f64.powf(-1.0 - next() * 3.0);
            let sign = if shape < 0.4 { 1.0 } else { -1.0 };
            for j in 0..n {
                a[i * n + j] = sign * a[j] + eps * (next() - 0.5);
            }
            sliver[i] = true;
        }
    }
    if (0..m).any(|i| (0..n).map(|j| a[i * n + j].powi(2)).sum::<f64>() < 1e-3) {
        return None;
    }
    let feasible_point: Vec<f64> = (0..n).map(|_| next() * 4.0 - 2.0).collect();
    let b: Vec<f64> = (0..m)
        .map(|i| {
            let slack = if thin && sliver[i] && next() < 0.5 {
                10f64.powf(-1.0 - next() * 3.0)
            } else {
                next() * 0.5
            };
            (0..n)
                .map(|j| a[i * n + j] * feasible_point[j])
                .sum::<f64>()
                + slack
        })
        .collect();
    let t: Vec<f64> = (0..n).map(|_| next() * 12.0 - 6.0).collect();
    Some((n, a, b, t))
}

/// The sweep caps the false-positive tests span: from 10 (below the 64-sweep
/// precondition) through the boundary to 3000.
const FEASIBLE_CAPS: [u32; 9] = [10, 30, 63, 64, 100, 300, 1000, 3000, 10_000];

/// Random FEASIBLE polytopes at every cap in [`FEASIBLE_CAPS`]: `(trials,
/// Infeasible reports)`, every instance at every cap.
fn feasible_false_positives(seed: u64, instances: usize, thin: bool) -> (u32, u32) {
    let mut next = lcg(seed);
    let (mut runs, mut false_positives) = (0, 0);
    for _ in 0..instances {
        let Some((n, a, b, t)) = random_feasible(&mut next, thin) else {
            continue;
        };
        let problem = projection(n, &a, &b, &t);
        for &cap in &FEASIBLE_CAPS {
            let (_, record) = solve_status(&problem, 1.0, cap, &vec![0.0; n]);
            runs += 1;
            false_positives += u32::from(record.infeasibility_evidence);
        }
    }
    (runs, false_positives)
}

/// Amendment 2 §0.2.6: the false-positive **rate** of `infeasibility_evidence` on
/// feasible problems is bounded, not zero. **Ordinary near-(anti)parallel shapes**
/// (40% nearly parallel and 10% nearly antiparallel rows, caps 10 to 10000):
/// measured `4 / 134,973 ≈ 3e-5` over six seeds; this guard allows `3e-4` (ten
/// times the measured rate), so it fails on a regression to the original
/// three-condition rule, which set the evidence on 1–4% of the capped runs.
#[test]
fn the_evidence_false_positive_rate_on_ordinary_feasible_polytopes_is_below_3e_minus_4() {
    let (runs, false_positives) = feasible_false_positives(0x9E37_79B9_7F4A_7C15, 2500, false);
    assert!(runs >= 20_000, "only {runs} trials were usable");
    assert!(
        u64::from(false_positives) * 10_000 < u64::from(runs) * 3,
        "{false_positives} false positives in {runs} trials"
    );
}

/// **Thin slivers**: the same rows with a tiny slack on half of the near-(anti)
/// parallel ones, so the feasible region is a wedge of angle about `1e-3` whose
/// Dykstra convergence time is `10^6`–`10^7` sweeps. Measured `30 / 134,964 ≈ 2e-4`
/// over six seeds (25 distinct polytopes, caps 64 to 10000); this guard allows
/// `1e-3` (five times the measured rate). These are feasible problems the field is
/// set on because no signal inside the cap separates them from an infeasible one;
/// see the documentation of `infeasibility_evidence`.
#[test]
fn the_evidence_false_positive_rate_on_thin_slivers_is_below_1e_minus_3() {
    let (runs, false_positives) = feasible_false_positives(0x1234_5678_9ABC_DEF1, 2500, true);
    assert!(runs >= 20_000, "only {runs} trials were usable");
    assert!(
        u64::from(false_positives) * 1000 < u64::from(runs),
        "{false_positives} false positives in {runs} trials"
    );
}

/// Amendment 1 condition 2: a cap below 64 sweeps never sets the evidence, even on
/// a system that is infeasible by a wide margin (two antipodal rows), while at 64
/// the same system sets it. The status is `NotConverged` either way.
#[test]
fn a_cap_below_sixty_four_sweeps_never_sets_the_evidence() {
    let problem = projection(1, &[1.0, -1.0], &[-1.0, -1.0], &[0.0]);
    let (below, below_record) = solve_status(&problem, 0.5, 63, &[0.0]);
    let (at, at_record) = solve_status(&problem, 0.5, 64, &[0.0]);
    assert_eq!(below, SolveStatus::NotConverged);
    assert_eq!(at, SolveStatus::NotConverged);
    assert!(!below_record.infeasibility_evidence);
    assert!(at_record.infeasibility_evidence);
}

/// The feasible thin sliver the original rule flagged at caps 100 to 1000 (ratio
/// 1.55 to 1.70, violation shrinking): now no evidence.
#[test]
fn the_sliver_the_original_rule_misreported_sets_no_evidence() {
    for cap in [64, 100, 300, 1000] {
        let (status, record) = solve_status(&thin_sliver(), 1.0, cap, &[0.0, 0.0]);
        assert!(record.projection_cap_hits > 0);
        assert_eq!(status, SolveStatus::NotConverged, "cap {cap}");
        assert!(!record.infeasibility_evidence, "cap {cap}");
    }
}

/// RFC 034 exit criterion 5, measurement: the false-positive count by sweep cap on
/// feasible polytopes (every instance at every cap), and the detection rate on
/// infeasible ones (built with a Farkas certificate) by cap and by margin decade —
/// **printed, not asserted to be anything but sane**, since detection is one-sided.
/// Asserts that an infeasible polytope is never `Converged` (Amendment 5) and that
/// the field is set at all; the false-positive rate guards are above.
#[test]
fn infeasibility_detection_is_measured_by_sweep_cap() {
    let mut next = lcg(0x1234_5678_9ABC_DEF1);
    let mut thin_instances = 0;
    let mut capped = [0u32; FEASIBLE_CAPS.len()];
    let mut thin_false_positives = [0u32; FEASIBLE_CAPS.len()];
    for _ in 0..2500 {
        let Some((n, a, b, t)) = random_feasible(&mut next, true) else {
            continue;
        };
        thin_instances += 1;
        let problem = projection(n, &a, &b, &t);
        for (k, &cap) in FEASIBLE_CAPS.iter().enumerate() {
            let (_, record) = solve_status(&problem, 1.0, cap, &vec![0.0; n]);
            capped[k] += u32::from(record.projection_cap_hits > 0);
            thin_false_positives[k] += u32::from(record.infeasibility_evidence);
        }
    }
    let (ordinary_trials, ordinary_false_positives) =
        feasible_false_positives(0x1234_5678_9ABC_DEF1, 2500, false);

    const DETECTION_CAPS: [u32; 4] = [100, 300, 1000, 3000];
    let mut infeasible_runs = 0;
    let mut detected = [0u32; DETECTION_CAPS.len()];
    // (runs, detected per cap) per decade of the infeasibility margin.
    let mut by_margin = [(0u32, [0u32; DETECTION_CAPS.len()]); 4];
    let mut converged = 0;
    for _ in 0..600 {
        let n = 2 + (next() * 2.0) as usize;
        let k = 2 + (next() * 2.0) as usize;
        let weights: Vec<f64> = (0..k).map(|_| 0.5 + next() * 1.5).collect();
        let mut rows: Vec<Vec<f64>> = (0..k)
            .map(|_| (0..n).map(|_| next() * 4.0 - 2.0).collect())
            .collect();
        let rhs: Vec<f64> = (0..k).map(|_| next() * 2.0 - 1.0).collect();
        let last = weights[0];
        // The last row closes the cycle: Σ wᵢaᵢ + w_last·a_last = 0 and the offsets
        // sum to a NEGATIVE margin, so λ = (w, w_last) is a Farkas certificate.
        let margin = 10f64.powf(-next() * 4.0); // 1e-4 .. 1
        let closing: Vec<f64> = (0..n)
            .map(|j| -(0..k).map(|i| weights[i] * rows[i][j]).sum::<f64>() / last)
            .collect();
        if closing.iter().map(|v| v * v).sum::<f64>() < 1e-3 {
            continue;
        }
        let weighted: f64 = (0..k).map(|i| weights[i] * rhs[i]).sum();
        rows.push(closing);
        let mut rhs = rhs;
        rhs.push((-weighted - margin) / last);
        let a: Vec<f64> = rows.iter().flatten().copied().collect();
        let t: Vec<f64> = (0..n).map(|_| next() * 4.0 - 2.0).collect();
        let problem = projection(n, &a, &rhs, &t);
        infeasible_runs += 1;
        let decade = ((-margin.log10()) as usize).min(3);
        by_margin[decade].0 += 1;
        for (slot, &cap) in DETECTION_CAPS.iter().enumerate() {
            let (status, record) = solve_status(&problem, 1.0, cap, &vec![0.0; n]);
            let found = u32::from(record.infeasibility_evidence);
            detected[slot] += found;
            by_margin[decade].1[slot] += found;
            converged += u32::from(status == SolveStatus::Converged);
        }
    }

    println!(
        "RFC034 MEASURED (cluster) feasible, ordinary near-(anti)parallel shapes: {ordinary_false_positives} set the evidence field in {ordinary_trials} instance-cap trials (caps {FEASIBLE_CAPS:?})"
    );
    println!(
        "RFC034 MEASURED (cluster) feasible thin slivers: {thin_instances} instances, each at every cap"
    );
    for (k, cap) in FEASIBLE_CAPS.iter().enumerate() {
        println!(
            "RFC034 MEASURED (cluster) feasible thin slivers, cap {cap}: capped {}, set the evidence field {}",
            capped[k], thin_false_positives[k]
        );
    }
    for (slot, cap) in DETECTION_CAPS.iter().enumerate() {
        println!(
            "RFC034 MEASURED (cluster) infeasible, cap {cap}: {} of {infeasible_runs} detected ({:.1}%)",
            detected[slot],
            100.0 * f64::from(detected[slot]) / f64::from(infeasible_runs)
        );
    }
    for (decade, (runs, found)) in by_margin.iter().enumerate() {
        println!(
            "RFC034 MEASURED (cluster) infeasible, margin 1e-{decade}..1e-{}: {runs} runs, detected at caps {DETECTION_CAPS:?}: {found:?}",
            decade + 1
        );
    }
    assert_eq!(
        converged, 0,
        "an infeasible polytope was reported Converged"
    );
    assert!(detected.iter().any(|&d| d > 0), "the rule never fired");
}

// ---------------------------------------------------------------------------
// RFC 034 Amendment 1: the decision logic in isolation, at each condition's
// boundary. These are unit tests of the pure functions the kernels call, so a
// mutation of any single condition is caught here whatever the geometry.
// ---------------------------------------------------------------------------

mod amendment_1 {
    use super::super::{
        Projection, Snapshots, SolveReport, has_infeasibility_evidence, infeasibility_evidence_of,
        stationary_report,
    };

    fn snap(
        mid_multiplier: f64,
        final_multiplier: f64,
        mid_violation: f64,
        final_violation: f64,
    ) -> Snapshots<f64> {
        Snapshots {
            midpoint_multiplier: mid_multiplier,
            midpoint_violation: mid_violation,
            final_multiplier,
            final_violation,
        }
    }

    #[test]
    fn the_cap_must_be_at_least_sixty_four_sweeps() {
        let diverging = snap(10.0, 20.0, 1.0, 1.0);
        assert!(!has_infeasibility_evidence(63, diverging));
        assert!(has_infeasibility_evidence(64, diverging));
    }

    /// The factor is 1.9, not the superseded 1.5 and not 2: a final multiplier of
    /// 1.89 times the midpoint is not divergence, 1.9 times is, and exactly 2 is.
    #[test]
    fn the_multiplier_factor_is_one_point_nine() {
        assert!(!has_infeasibility_evidence(
            100,
            snap(100.0, 189.0, 1.0, 1.0)
        ));
        assert!(!has_infeasibility_evidence(
            100,
            snap(100.0, 150.0, 1.0, 1.0)
        ));
        assert!(has_infeasibility_evidence(
            100,
            snap(100.0, 190.0, 1.0, 1.0)
        ));
        assert!(has_infeasibility_evidence(
            100,
            snap(100.0, 200.0, 1.0, 1.0)
        ));
    }

    /// The violation must not be shrinking: 0.99 times the midpoint violation or
    /// more passes, 0.98 does not, and a growing violation passes.
    #[test]
    fn the_violation_must_not_be_shrinking() {
        assert!(has_infeasibility_evidence(100, snap(10.0, 20.0, 1.0, 0.99)));
        assert!(!has_infeasibility_evidence(
            100,
            snap(10.0, 20.0, 1.0, 0.98)
        ));
        assert!(has_infeasibility_evidence(100, snap(10.0, 20.0, 1.0, 1.5)));
    }

    /// Multipliers identically zero (no row ever active): `0 ≥ 1.9 × 0` holds, so
    /// the multiplier and shrink conditions alone would say "evidence". It is the
    /// fifth condition — the violation exceeds the tolerance — that keeps such a
    /// feasible problem from setting the field.
    #[test]
    fn zero_multipliers_read_as_evidence_and_only_the_violation_condition_stops_them() {
        assert!(has_infeasibility_evidence(100, snap(0.0, 0.0, 0.0, 0.0)));
        let capped = Projection {
            capped: true,
            infeasibility_evidence: true,
        };
        // Feasible (violation within tolerance), capped, "evidence": no field.
        assert!(!infeasibility_evidence_of(true, capped));
        // Infeasible (violation over tolerance): the same evidence now counts.
        assert!(infeasibility_evidence_of(false, capped));
    }

    #[test]
    fn the_field_needs_every_condition() {
        let p = |capped, infeasibility_evidence| Projection {
            capped,
            infeasibility_evidence,
        };
        // the projection did not cap: no field, however the multipliers looked
        assert!(!infeasibility_evidence_of(false, p(false, true)));
        // capped, no divergence evidence: no field
        assert!(!infeasibility_evidence_of(false, p(true, false)));
        // feasible: no field
        assert!(!infeasibility_evidence_of(true, p(true, true)));
        // infeasible, capped, evidence: the field
        assert!(infeasibility_evidence_of(false, p(true, true)));
    }

    /// The status never depends on the evidence and is never `Infeasible` (there is
    /// no such status): feasible and exact converges, everything else at a stationary
    /// step is `NotConverged` with `NoProgress`.
    #[test]
    fn the_status_at_a_stationary_step_does_not_depend_on_the_evidence() {
        let converged = SolveReport::converged_early(5);
        let stalled = SolveReport::not_converged_stalled(5);
        let p = |capped, infeasibility_evidence| Projection {
            capped,
            infeasibility_evidence,
        };
        assert_eq!(
            stationary_report(true, p(false, false), 5, converged),
            converged
        );
        assert_eq!(
            stationary_report(true, p(true, true), 5, converged),
            stalled
        );
        assert_eq!(
            stationary_report(false, p(true, true), 5, converged),
            stalled
        );
        assert_eq!(
            stationary_report(false, p(true, false), 5, converged),
            stalled
        );
        assert_eq!(
            stationary_report(false, p(false, false), 5, converged),
            stalled
        );
    }
}

// ---------------------------------------------------------------------------
// RFC 032: a step at or above 2/L is rejected; the band [2/U, 2/L) is not.
//
// Q = [[4, 1], [1, 3]]: L = 4, U = 5, λ_max = (7 + √5)/2 = 4.618. A step below
// 2/U = 0.4 provably converges, one at or above 2/L = 0.5 provably diverges, and
// the band [0.4, 0.5) is indeterminate: it holds steps that converge (0.42) and
// steps that do not (0.45 > 2/λ_max = 0.433).
// ---------------------------------------------------------------------------

fn band_program(q: [f64; 4]) -> Dense {
    Qp {
        q: DenseMatrix::from_row_major_vec(2, 2, q.to_vec()).unwrap(),
        c: dv(&[-1.0, -1.0]),
        lo: dv(&[-10.0, -10.0]),
        hi: dv(&[10.0, 10.0]),
        a: DenseMatrix::from_row_major_vec(1, 2, vec![1.0, 0.0]).unwrap(),
        b: dv(&[1e9]),
    }
}

fn solve_with_step(
    problem: &Dense,
    step: f64,
    max_iterations: u32,
    policy: ClusterValidationPolicy,
) -> Result<ConstrainedSolveRecord<f64>, SolverError> {
    let mut x = dv(&[0.0, 0.0]);
    let mut ws = ClusterConstrainedWorkspace::new(2, 1).unwrap();
    let config = ConstrainedProjectedConfig {
        max_iterations,
        tolerance: 1e-10,
        projection_max_sweeps: 5000,
        projection_tolerance: 1e-12,
    };
    solve_constrained_projected_first_order_dyn(
        problem,
        step,
        &mut x,
        &mut ws,
        &config,
        &ctx(policy),
    )
}

const BAND: [f64; 4] = [4.0, 1.0, 1.0, 3.0];

#[test]
fn a_step_at_exactly_two_over_l_is_rejected() {
    let problem = band_program(BAND);
    for step in [0.5, 0.75] {
        assert_eq!(
            solve_with_step(
                &problem,
                step,
                100,
                ClusterValidationPolicy::ValidateAllInputs
            )
            .map(|_| ()),
            Err(SolverError::InvalidInput),
            "step {step}"
        );
    }
}

/// Structural, so it holds under `TrustedByCaller` as well: trust skips the
/// finite scans, not the step rule.
#[test]
fn the_step_rule_is_not_skippable_under_trust() {
    let trust = TrustedByCaller::caller_assertion(
        loeres::validation::ValidationScope::FINITE,
        TrustToken::new(32),
        Some("rfc032"),
    );
    assert_eq!(
        solve_with_step(
            &band_program(BAND),
            0.5,
            100,
            ClusterValidationPolicy::TrustedByCaller(trust)
        )
        .map(|_| ()),
        Err(SolverError::InvalidInput)
    );
}

#[test]
fn a_step_just_below_two_over_u_is_accepted_and_converges() {
    let record = solve_with_step(
        &band_program(BAND),
        0.399,
        5000,
        ClusterValidationPolicy::ValidateAllInputs,
    )
    .unwrap();
    assert_eq!(record.report.status(), SolveStatus::Converged);
    assert_eq!(record.max_constraint_violation, 0.0);
}

/// The regression guard against over-rejection: the indeterminate band is
/// accepted.
#[test]
fn the_indeterminate_band_is_accepted_without_a_claim() {
    let problem = band_program(BAND);
    let usable = solve_with_step(
        &problem,
        0.42,
        5000,
        ClusterValidationPolicy::ValidateAllInputs,
    )
    .unwrap();
    assert_eq!(usable.report.status(), SolveStatus::Converged);
    let unusable = solve_with_step(
        &problem,
        0.45,
        200,
        ClusterValidationPolicy::ValidateAllInputs,
    )
    .unwrap();
    assert_eq!(unusable.report.status(), SolveStatus::NotConverged);
}

#[test]
fn the_suggested_step_is_accepted_and_converges() {
    let problem = band_program(BAND);
    let step = problem.suggested_step_scale().unwrap();
    assert_eq!(step, 0.2);
    let record = solve_with_step(
        &problem,
        step,
        5000,
        ClusterValidationPolicy::ValidateAllInputs,
    )
    .unwrap();
    assert_eq!(record.report.status(), SolveStatus::Converged);
}

#[test]
fn a_zero_q_is_never_rejected_by_the_step_rule() {
    let mut problem = band_program([0.0; 4]);
    problem.c = dv(&[0.0, 0.0]);
    let record = solve_with_step(
        &problem,
        1e6,
        100,
        ClusterValidationPolicy::ValidateAllInputs,
    )
    .unwrap();
    assert_eq!(record.report.status(), SolveStatus::Converged);
}

// ---------------------------------------------------------------------------
// RFC 033: `Converged` requires the FINAL outer iteration's projection to have
// returned without hitting `projection_max_sweeps`.
//
// Rows a1 = (1, 0), a2 = (1, 0.2), b = (1, 1.2), target (3, 1.2), step 0.5: the
// exact optimum is the vertex (1, 1), both rows active. Started far away
// (−60, 60) the first candidates are far from the polyhedron, so early
// projections are the hard ones; the sweep cap decides which of them bind.
// ---------------------------------------------------------------------------

fn nearly_parallel(sweeps: u32) -> (DenseVector<f64>, ConstrainedSolveRecord<f64>) {
    let problem = projection(2, &[1.0, 0.0, 1.0, 0.2], &[1.0, 1.2], &[3.0, 1.2]);
    let mut x = dv(&[-60.0, 60.0]);
    let mut ws = ClusterConstrainedWorkspace::new(2, 2).unwrap();
    let config = ConstrainedProjectedConfig {
        max_iterations: 5000,
        tolerance: 1e-12,
        projection_max_sweeps: sweeps,
        projection_tolerance: 1e-10,
    };
    let record = solve_constrained_projected_first_order_dyn(
        &problem,
        0.5,
        &mut x,
        &mut ws,
        &config,
        &scan(),
    )
    .unwrap();
    (x, record)
}

/// The final projection capped: the point is feasible and the outer step is
/// stationary, and it is still not claimed to be the projection.
#[test]
fn a_capped_final_projection_is_not_converged() {
    let (_, record) = nearly_parallel(400);
    assert_eq!(record.report.status(), SolveStatus::NotConverged);
    assert_eq!(record.report.termination(), TerminationReason::NoProgress);
    assert!(record.projection_cap_hits > 0);
    // Amendment 5's gate does not fire: the point is feasible. This is the
    // third leg of the claim, not the first.
    assert!(record.max_constraint_violation <= 1e-10);
}

/// The regression guard against over-firing: an EARLY iteration capped, later
/// ones converged cleanly, so the final projection was exact and the solve is
/// still `Converged`. Gating on `projection_cap_hits > 0` would downgrade it.
#[test]
fn an_early_capped_projection_does_not_stop_a_clean_final_one_being_converged() {
    let (x, record) = nearly_parallel(500);
    assert!(record.projection_cap_hits >= 1, "no early iteration capped");
    assert_eq!(record.report.status(), SolveStatus::Converged);
    assert_eq!(
        record.report.termination(),
        TerminationReason::ConvergenceCriterion
    );
    let x = coords(&x);
    assert!(
        (x[0] - 1.0).abs() < 1e-6 && (x[1] - 1.0).abs() < 1e-6,
        "{x:?}"
    );
}

#[test]
fn an_uncapped_solve_is_unchanged() {
    let (_, record) = nearly_parallel(100_000);
    assert_eq!(record.projection_cap_hits, 0);
    assert_eq!(record.report.status(), SolveStatus::Converged);
}

// ---------------------------------------------------------------------------
// Randomized differential tests against an exact active-set reference. A
// per-sweep multiplier reset (violating §0.3.2) is caught by these and by none
// of the deterministic cases above (review 055).
// ---------------------------------------------------------------------------

/// Exact Euclidean projection of `t` onto `{g·x ≤ h}` by active-set
/// enumeration: for each subset `S` of the constraints (ascending size), solve
/// the KKT system `x = t − Σ λᵢgᵢ`, `gᵢ·x = hᵢ` (a Gram system) and accept the
/// first feasible point with non-negative multipliers.
fn exact_projection(rows: &[(Vec<f64>, f64)], t: &[f64]) -> Vec<f64> {
    let n = t.len();
    let feasible = |x: &[f64]| rows.iter().all(|(g, h)| dot(g, x) <= h + 1e-9);
    if feasible(t) {
        return t.to_vec();
    }
    for size in 1..=n {
        for subset in subsets(rows.len(), size) {
            let gram: Vec<Vec<f64>> = subset
                .iter()
                .map(|&i| {
                    subset
                        .iter()
                        .map(|&j| dot(&rows[i].0, &rows[j].0))
                        .collect()
                })
                .collect();
            let rhs: Vec<f64> = subset
                .iter()
                .map(|&i| dot(&rows[i].0, t) - rows[i].1)
                .collect();
            let Some(lambda) = solve_linear(gram, rhs) else {
                continue;
            };
            if lambda.iter().any(|&l| l < -1e-12) {
                continue;
            }
            let mut x = t.to_vec();
            for (&i, &l) in subset.iter().zip(&lambda) {
                for (xk, gk) in x.iter_mut().zip(&rows[i].0) {
                    *xk -= l * gk;
                }
            }
            if feasible(&x) {
                return x;
            }
        }
    }
    panic!("a feasible instance always has an exact projection");
}

fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

fn subsets(n: usize, size: usize) -> Vec<Vec<usize>> {
    let mut out = Vec::new();
    let mut current = Vec::new();
    fn go(start: usize, n: usize, size: usize, cur: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
        if cur.len() == size {
            out.push(cur.clone());
            return;
        }
        for i in start..n {
            cur.push(i);
            go(i + 1, n, size, cur, out);
            cur.pop();
        }
    }
    go(0, n, size, &mut current, &mut out);
    out
}

/// Gaussian elimination with partial pivoting; `None` if singular.
fn solve_linear(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Option<Vec<f64>> {
    let n = b.len();
    for col in 0..n {
        let pivot = (col..n).max_by(|&i, &j| a[i][col].abs().total_cmp(&a[j][col].abs()))?;
        if a[pivot][col].abs() < 1e-12 {
            return None;
        }
        a.swap(col, pivot);
        b.swap(col, pivot);
        for row in (col + 1)..n {
            let f = a[row][col] / a[col][col];
            let pivot_row = a[col].clone();
            for (target, source) in a[row].iter_mut().zip(&pivot_row).skip(col) {
                *target -= f * source;
            }
            let v = b[col];
            b[row] -= f * v;
        }
    }
    let mut x = vec![0.0; n];
    for row in (0..n).rev() {
        let s: f64 = ((row + 1)..n).map(|k| a[row][k] * x[k]).sum();
        x[row] = (b[row] - s) / a[row][row];
    }
    Some(x)
}

fn differential(n: usize, m: usize, instances: usize, seed: u64) {
    let mut state = seed;
    let mut next = move || {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((state >> 11) as f64) / ((1u64 << 53) as f64)
    };
    let mut checked = 0;
    for _ in 0..instances {
        let a: Vec<f64> = (0..m * n).map(|_| next() * 4.0 - 2.0).collect();
        if (0..m).any(|i| dot(&a[i * n..(i + 1) * n], &a[i * n..(i + 1) * n]) < 0.09) {
            continue;
        }
        let feas: Vec<f64> = (0..n).map(|_| next() * 4.0 - 2.0).collect();
        let b: Vec<f64> = (0..m)
            .map(|i| dot(&a[i * n..(i + 1) * n], &feas) + next() * 0.5)
            .collect();
        let t: Vec<f64> = (0..n).map(|_| next() * 12.0 - 6.0).collect();

        let mut rows: Vec<(Vec<f64>, f64)> = (0..m)
            .map(|i| (a[i * n..(i + 1) * n].to_vec(), b[i]))
            .collect();
        for k in 0..n {
            let mut up = vec![0.0; n];
            up[k] = 1.0;
            let mut down = vec![0.0; n];
            down[k] = -1.0;
            rows.push((up, 10.0));
            rows.push((down, 10.0));
        }
        let expected = exact_projection(&rows, &t);

        let problem = projection(n, &a, &b, &t);
        let (x, record) = solve(&problem, 1.0, &vec![0.0; n], &cfg(1e-13, 200_000));
        let x = coords(&x);
        assert_eq!(record.projection_cap_hits, 0, "a={a:?} b={b:?} t={t:?}");
        assert_feasible(&record, 1e-12);
        for (got, want) in x.iter().zip(&expected) {
            assert!(
                (got - want).abs() < 1e-6,
                "a={a:?} b={b:?} t={t:?}: got {x:?}, expected {expected:?}"
            );
        }
        checked += 1;
    }
    assert!(
        checked > instances * 2 / 3,
        "too few usable instances: {checked}"
    );
}

#[test]
fn random_feasible_polytopes_n2_m2_match_the_exact_projection() {
    differential(2, 2, 300, 0x9E37_79B9_7F4A_7C15);
}

/// `M = 3` is a shape no `M ≤ 2` test exercises.
#[test]
fn random_feasible_polytopes_n3_m3_match_the_exact_projection() {
    differential(3, 3, 120, 0xD1B5_4A32_D192_ED03);
}

// ---------------------------------------------------------------------------
// m = 0
// ---------------------------------------------------------------------------

type Unconstrained = Qp<MatrixView<'static, f64>, VectorView<'static, f64>>;

fn unconstrained(t: &[f64], lo: f64, hi: f64) -> Unconstrained {
    let n = t.len();
    const EMPTY: &[f64] = &[];
    Qp {
        q: identity(n),
        c: dv(&t.iter().map(|v| -v).collect::<Vec<_>>()),
        lo: dv(&vec![lo; n]),
        hi: dv(&vec![hi; n]),
        a: MatrixView::from_row_major(EMPTY, 0, n).unwrap(),
        b: VectorView::from_slice(EMPTY),
    }
}

/// The RFC 016 oracle `q·(x − t)` with `q = 1`.
struct Rfc016 {
    target: Vec<f64>,
    lo: DenseVector<f64>,
    hi: DenseVector<f64>,
    alpha: f64,
}

impl ClusterProjectedFirstOrderProblem<f64> for Rfc016 {
    fn dimension(&self) -> usize {
        self.target.len()
    }
    fn bounds(&self) -> (&DenseVector<f64>, &DenseVector<f64>) {
        (&self.lo, &self.hi)
    }
    fn gradient_at(
        &self,
        x: &DenseVector<f64>,
        grad: &mut DenseVector<f64>,
    ) -> Result<(), SolverError> {
        for (i, t) in self.target.iter().enumerate() {
            grad.set(i, x.get(i)? - t)?;
        }
        Ok(())
    }
    fn step_scale(&self) -> f64 {
        self.alpha
    }
}

/// RFC 027 Amendment 4, §0.4.1: identity is numeric equality plus a NaN check,
/// never raw `to_bits()`. `+0.0` and `−0.0` satisfy it (they differ only in the
/// sign of a zero, which no numerical property sees); `NaN` never does.
fn assert_identical(actual: f64, expected: f64, context: impl Fn() -> String) {
    assert!(
        !actual.is_nan() && !expected.is_nan() && actual == expected,
        "{}: {actual:?} is not identical to {expected:?}",
        context()
    );
}

struct Fixture {
    target: Vec<f64>,
    lo: f64,
    hi: f64,
    alpha: f64,
    start: Vec<f64>,
    max_iterations: u32,
}

fn fixture(
    target: &[f64],
    lo: f64,
    hi: f64,
    alpha: f64,
    start: &[f64],
    max_iterations: u32,
) -> Fixture {
    Fixture {
        target: target.to_vec(),
        lo,
        hi,
        alpha,
        start: start.to_vec(),
        max_iterations,
    }
}

/// `m = 0` performs exactly the RFC 016 step: with `Q = I` the two oracles
/// (`x − t` and `−t + x`) agree bit for bit, so the results are bit-identical
/// — across several fixtures, not one (handoff §6).
#[test]
fn m_zero_is_identical_to_rfc_016_across_fixtures_up_to_the_sign_of_zero() {
    let fixtures = [
        fixture(&[0.5, -0.5], -1.0, 1.0, 0.5, &[0.0, 0.0], 500),
        fixture(&[5.0, -5.0], -1.0, 1.0, 0.5, &[0.0, 0.0], 500),
        fixture(&[0.1, 0.2, 0.3], -1.0, 1.0, 0.3, &[0.9, -0.9, 0.0], 400),
        fixture(&[3.0], -2.0, 2.0, 0.1, &[-2.0], 7), // stops at the iteration cap
        fixture(&[0.7, 0.7], -1.0, 1.0, 0.99, &[1.0, -1.0], 500),
        // Review 056 F1's reproducer: a zero target coordinate with a −0.0
        // iterate. The two solvers agree in value but can differ in the sign of
        // a zero, which raw `to_bits()` would report as a difference.
        fixture(&[0.0, 3.0], -1.0, 1.0, 0.5, &[-0.0, 0.5], 500),
    ];
    for Fixture {
        target,
        lo,
        hi,
        alpha,
        start,
        max_iterations,
    } in fixtures
    {
        let (target, start) = (target.as_slice(), start.as_slice());
        let n = target.len();

        let reference = Rfc016 {
            target: target.to_vec(),
            lo: dv(&vec![lo; n]),
            hi: dv(&vec![hi; n]),
            alpha,
        };
        let mut x_ref = dv(start);
        let mut ws_ref = ClusterProjectedFirstOrderWorkspace::new(n).unwrap();
        let record_ref = solve_projected_first_order_dyn(
            &reference,
            &mut x_ref,
            &mut ws_ref,
            &ProjectedFirstOrderConfig {
                max_iterations,
                tolerance: 1e-12,
            },
            &scan(),
        )
        .unwrap();

        let problem = unconstrained(target, lo, hi);
        let mut x = dv(start);
        let mut ws = ClusterConstrainedWorkspace::new(n, 0).unwrap();
        let record = solve_constrained_projected_first_order_dyn(
            &problem,
            alpha,
            &mut x,
            &mut ws,
            &ConstrainedProjectedConfig {
                max_iterations,
                tolerance: 1e-12,
                projection_max_sweeps: 1,
                projection_tolerance: 1e-12,
            },
            &scan(),
        )
        .unwrap();

        assert_eq!(record.report, record_ref.report, "target {target:?}");
        assert_eq!(record.projection_cap_hits, 0);
        assert_eq!(record.max_constraint_violation, 0.0);
        for i in 0..n {
            assert_identical(x.get(i).unwrap(), x_ref.get(i).unwrap(), || {
                format!("target {target:?}, coordinate {i}")
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Validation (RFC 012 / 016 discipline)
// ---------------------------------------------------------------------------

fn valid() -> Dense {
    projection(2, &[1.0, 1.0], &[10.0], &[0.5, -0.5])
}

fn run(
    problem: &Dense,
    step: f64,
    x0: &[f64],
    policy: ClusterValidationPolicy,
) -> Result<ConstrainedSolveRecord<f64>, SolverError> {
    let shape = problem.shape()?;
    let mut x = dv(x0);
    let mut ws = ClusterConstrainedWorkspace::new(shape.variables, shape.constraints)?;
    solve_constrained_projected_first_order_dyn(
        problem,
        step,
        &mut x,
        &mut ws,
        &cfg(1e-12, 1000),
        &ctx(policy),
    )
}

#[test]
fn an_all_zero_constraint_row_is_invalid_input() {
    let problem = projection(2, &[0.0, 0.0], &[1.0], &[0.0, 0.0]);
    assert_eq!(
        run(
            &problem,
            0.5,
            &[0.0, 0.0],
            ClusterValidationPolicy::ValidateAllInputs
        )
        .unwrap_err(),
        SolverError::InvalidInput
    );
}

#[test]
fn an_overflowing_row_norm_is_overflow() {
    let problem = projection(2, &[1e200, 1e200], &[1.0], &[0.0, 0.0]);
    assert_eq!(
        run(
            &problem,
            0.5,
            &[0.0, 0.0],
            ClusterValidationPolicy::ValidateAllInputs
        )
        .unwrap_err(),
        SolverError::Overflow
    );
}

#[test]
fn inverted_finite_bounds_are_invalid_input() {
    let mut problem = valid();
    problem.lo = dv(&[1.0, -1.0]);
    problem.hi = dv(&[-1.0, 1.0]);
    assert_eq!(
        run(
            &problem,
            0.5,
            &[0.0, 0.0],
            ClusterValidationPolicy::ValidateAllInputs
        )
        .unwrap_err(),
        SolverError::InvalidInput
    );
}

#[test]
fn step_scale_config_and_dimension_are_validated_structurally() {
    let problem = valid();
    let policy = ClusterValidationPolicy::ValidateAllInputs;
    assert_eq!(
        run(&problem, f64::NAN, &[0.0, 0.0], policy).unwrap_err(),
        SolverError::NonFiniteInput
    );
    for alpha in [0.0, -1.0] {
        assert_eq!(
            run(&problem, alpha, &[0.0, 0.0], policy).unwrap_err(),
            SolverError::InvalidInput
        );
    }
    assert!(matches!(
        run(&problem, 0.5, &[0.0, 0.0, 0.0], policy).unwrap_err(),
        SolverError::DimensionMismatch { .. }
    ));
    let mut bad = cfg(1e-12, 100);
    bad.projection_max_sweeps = 0;
    assert_eq!(bad.validate(), Err(SolverError::InvalidInput));
    bad = cfg(1e-12, 100);
    bad.projection_tolerance = f64::INFINITY;
    assert_eq!(bad.validate(), Err(SolverError::NonFiniteInput));
    bad = cfg(1e-12, 100);
    bad.tolerance = 0.0;
    assert_eq!(bad.validate(), Err(SolverError::InvalidInput));
    assert_eq!(
        ClusterConstrainedWorkspace::<f64>::new(0, 1).unwrap_err(),
        SolverError::InvalidDimension
    );
}

#[test]
fn a_workspace_of_the_wrong_size_is_a_dimension_mismatch() {
    let problem = valid();
    let mut x = dv(&[0.0, 0.0]);
    let mut ws = ClusterConstrainedWorkspace::new(2, 2).unwrap();
    let err = solve_constrained_projected_first_order_dyn(
        &problem,
        0.5,
        &mut x,
        &mut ws,
        &cfg(1e-12, 100),
        &scan(),
    )
    .unwrap_err();
    assert!(matches!(err, SolverError::DimensionMismatch { .. }));
}

/// Finite scans cover `Q, c, A, b, lo, hi, x₀` and are skippable under trust.
#[test]
fn finite_scans_cover_every_input_and_are_skippable_under_trust() {
    let scanning = ClusterValidationPolicy::ValidateAllInputs;
    type Mutation = (&'static str, fn(&mut Dense));
    let mutations: [Mutation; 5] = [
        ("c", |p| p.c = dv(&[f64::NAN, 0.0])),
        ("q", |p| {
            p.q = DenseMatrix::from_row_major_vec(2, 2, vec![1.0, f64::NAN, 0.0, 1.0]).unwrap()
        }),
        ("hi", |p| p.hi = dv(&[f64::INFINITY, 1.0])),
        ("a", |p| {
            p.a = DenseMatrix::from_row_major_vec(1, 2, vec![1.0, f64::NAN]).unwrap()
        }),
        ("b", |p| p.b = dv(&[f64::NAN])),
    ];
    for (name, mutate) in mutations {
        let mut problem = valid();
        mutate(&mut problem);
        let err = run(&problem, 0.5, &[0.0, 0.0], scanning).unwrap_err();
        assert!(
            matches!(
                err,
                SolverError::NonFiniteInput | SolverError::NumericalDomain
            ),
            "{name}: {err:?}"
        );
    }
    // x0 scan
    assert_eq!(
        run(&valid(), 0.5, &[f64::NAN, 0.0], scanning).unwrap_err(),
        SolverError::NonFiniteInput
    );

    // Under trust the pre-loop scan is skipped and the record says so; a NaN
    // in `c` then reaches the hot loop and is NumericalDomain, never `Solved`
    // over NaN (never skippable).
    let trust = TrustedByCaller::caller_assertion(ValidationScope::ALL, TrustToken::new(1), None);
    let trusted = ClusterValidationPolicy::TrustedByCaller(trust);
    let record = run(&valid(), 0.5, &[0.0, 0.0], trusted).unwrap();
    assert!(matches!(
        record.finite,
        ProjectedFirstOrderFiniteEvidence::Trusted(_)
    ));
    assert_eq!(record.checked_scope, ValidationScope::PROBLEM_CONFIG);
    let record = run(&valid(), 0.5, &[0.0, 0.0], scanning).unwrap();
    assert!(matches!(
        record.finite,
        ProjectedFirstOrderFiniteEvidence::Scanned
    ));
    assert!(record.checked_scope.contains(ValidationScope::FINITE));

    let mut poisoned = valid();
    poisoned.c = dv(&[f64::NAN, 0.0]);
    assert_eq!(
        run(&poisoned, 0.5, &[0.0, 0.0], trusted).unwrap_err(),
        SolverError::NumericalDomain
    );
}

/// Structural checks are never skippable under trust: a zero row is rejected
/// even when the caller has vouched for finiteness.
#[test]
fn structural_checks_run_under_trust() {
    let trust = TrustedByCaller::caller_assertion(ValidationScope::ALL, TrustToken::new(2), None);
    let problem = projection(2, &[0.0, 0.0], &[1.0], &[0.0, 0.0]);
    assert_eq!(
        run(
            &problem,
            0.5,
            &[0.0, 0.0],
            ClusterValidationPolicy::TrustedByCaller(trust)
        )
        .unwrap_err(),
        SolverError::InvalidInput
    );
}

#[test]
fn cancellation_is_observed() {
    let token = ClusterCancellationToken::new();
    token.cancel();
    let problem = valid();
    let mut x = dv(&[0.0, 0.0]);
    let mut ws = ClusterConstrainedWorkspace::new(2, 1).unwrap();
    let err = solve_constrained_projected_first_order_dyn(
        &problem,
        0.5,
        &mut x,
        &mut ws,
        &cfg(1e-12, 100),
        &ClusterExecutionContext::new(token, 0, ClusterValidationPolicy::ValidateAllInputs),
    )
    .unwrap_err();
    assert_eq!(err, SolverError::Cancelled);
}

#[test]
fn a_workspace_is_reusable_across_solves() {
    let problem = projection(
        2,
        &[1.6, -1.3082, -0.0457, 0.9197],
        &[1.6013, -0.3988],
        &[5.3481, 4.7872],
    );
    let mut ws = ClusterConstrainedWorkspace::new(2, 2).unwrap();
    let mut results = Vec::new();
    for _ in 0..2 {
        let mut x = dv(&[0.0, 0.0]);
        solve_constrained_projected_first_order_dyn(
            &problem,
            1.0,
            &mut x,
            &mut ws,
            &cfg(1e-14, 5000),
            &scan(),
        )
        .unwrap();
        results.push(coords(&x));
    }
    assert_eq!(results[0], results[1]);
}

// ---------------------------------------------------------------------------
// The ClusterJob seam
// ---------------------------------------------------------------------------

#[test]
fn the_job_runs_through_solve_batch_beside_the_rfc_016_adapter() {
    let problem = projection(
        2,
        &[1.6, -1.3082, -0.0457, 0.9197],
        &[1.6013, -0.3988],
        &[5.3481, 4.7872],
    );
    let job = ClusterConstrainedJob::new(problem, 1.0, dv(&[0.0, 0.0]), cfg(1e-14, 5000));
    let jobs: Vec<Box<dyn ClusterJob<f64>>> = vec![Box::new(job)];
    let report = solve_batch(
        jobs,
        ClusterSolveConfig::default(),
        ClusterCancellationToken::new(),
    )
    .unwrap();
    assert_eq!(report.summary.solved_converged, 1);
    match &report.outcomes[0] {
        BatchItemOutcome::Solved {
            solution: ClusterSolution::DenseVector(x),
            ..
        } => {
            let x = coords(x);
            assert!((x[0] - 0.673643).abs() < 1e-6 && (x[1] + 0.400146).abs() < 1e-6);
        }
        other => panic!("expected a solved outcome, got {other:?}"),
    }
}

#[test]
fn a_job_with_an_invalid_problem_fails_per_item() {
    let bad = projection(2, &[0.0, 0.0], &[1.0], &[0.0, 0.0]);
    let job = ClusterConstrainedJob::new(bad, 0.5, dv(&[0.0, 0.0]), cfg(1e-12, 100));
    let jobs: Vec<Box<dyn ClusterJob<f64>>> = vec![Box::new(job)];
    let report = solve_batch(
        jobs,
        ClusterSolveConfig::default(),
        ClusterCancellationToken::new(),
    )
    .unwrap();
    assert_eq!(report.summary.failed, 1);
}
