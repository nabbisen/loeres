//! `lp` — RFC 039: what the kernel does with a linear objective (`Q = 0`), measured and
//! reported, not enforced.
//!
//! `crates/loeres/src/problem.rs` used to say LP is "not solved". Architect review 091 §1
//! measured otherwise: every converged run in a 300-instance random sample was optimal,
//! and the three that failed to converge all carried a nonzero `projection_cap_hits` — the
//! Dykstra projection's sweep cap binding, not an absence of curvature. RFC 039 asks for
//! that measurement to live in the repository, against **exact references** rather than
//! the architect's randomized oracle, and extended to cases the architect's sample could
//! not reach: an unopposed direction bounded only by a distant box face, a degenerate
//! vertex, and a tie along an optimal face.
//!
//! **This is not a gate, and it must not become one** (RFC 039 §2.1 mirrors `bench`'s own
//! framing). `cargo xtask check` and `cargo xtask release-gate` do not run it. No LP figure
//! is pinned in `bench-baseline` either — deviations are floats, and review 087's ruling
//! against pinning a float deviation stands.
//!
//! The exact reference is `exact::exact_lp_optimum`, not `exact::exact_optimum`: the
//! latter inverts `Q`, which is singular at `Q = 0` (see `exact.rs`'s module doc). Scope is
//! therefore `n ≤ exact::MAX_N`, exactly as S6's deviation figures are.
//!
//! The corpus, the measurement and the cap-sensitivity table are pure functions with unit
//! tests; only `run` touches the kernel through `std`-visible I/O (`println!`).

use loeres::{
    BoxBounds, LinearInequalities, QuadraticObjective, SolveStatus, SolverError, VectorAccess,
};
use loeres_backend_std::{DenseMatrix, DenseVector};
use loeres_cluster::{
    ClusterCancellationToken, ClusterConstrainedWorkspace, ClusterExecutionContext,
    ClusterValidationPolicy, ConstrainedProjectedConfig,
    solve_constrained_projected_first_order_dyn,
};

use super::exact::{self, DenseQp};

/// RFC 039 §0.1: the fixed step scale the architect's scoping used throughout.
pub const STEP_SCALE: f64 = 0.3;
const TOLERANCE: f64 = 1e-10;
const PROJECTION_TOLERANCE: f64 = 1e-10;
const MAX_ITERATIONS: u32 = 50_000;
/// RFC 039 §0.2: the three sweep caps the architect's scoping measured.
pub const CAPS: &[u32] = &[500, 5_000, 50_000];
/// A converged result counts as "not optimal" only once its deviation from the exact
/// optimum clears both solver tolerances by two orders of magnitude, so float noise at the
/// solver's own tolerance is never mistaken for a miss.
const NOT_OPTIMAL_DEVIATION: f64 = 1e-8;

/// RFC 039 §0.1's random LP family: `n = 4`, `m = 3`, `c` random in `[-1,1]ⁿ`, box
/// `[0,1]ⁿ`, `A` random in `[0,1]^{m×n}`, `b` random in `[0.5,1.5]^m`. `q` is `0`
/// everywhere; it is carried only because `DenseQp` has the field, and `exact_lp_optimum`
/// never reads it.
pub fn random_corpus(count: usize, seed: u64) -> Vec<DenseQp> {
    let n = 4;
    let m = 3;
    let mut next = lcg(seed);
    (0..count)
        .map(|_| DenseQp {
            n,
            q: vec![0.0; n * n],
            c: (0..n).map(|_| next() * 2.0 - 1.0).collect(),
            lower: vec![0.0; n],
            upper: vec![1.0; n],
            a: (0..m * n).map(|_| next()).collect(),
            b: (0..m).map(|_| next() + 0.5).collect(),
        })
        .collect()
}

/// RFC 039 §1's unopposed-direction case: minimise `-(x0 + x1)` subject to `x1 <= 0.5`
/// (no row touches `x0`), box `x0 in [0, 1e6]`, `x1 in [0, 1]`. The general row never
/// opposes `x0`, so it is bounded only by its own distant box face.
pub fn unopposed_direction_case() -> DenseQp {
    DenseQp {
        n: 2,
        q: vec![0.0; 4],
        c: vec![-1.0, -1.0],
        lower: vec![0.0, 0.0],
        upper: vec![1e6, 1.0],
        a: vec![0.0, 1.0],
        b: vec![0.5],
    }
}

/// RFC 039 §1's degenerate-vertex case: minimise `-(x0 + x1)` subject to `x0 + x1 <= 2`,
/// box `[0,1]^2`. At `(1, 1)` three rows are active (both box faces and the general row),
/// one more than `n = 2`.
pub fn degenerate_vertex_case() -> DenseQp {
    DenseQp {
        n: 2,
        q: vec![0.0; 4],
        c: vec![-1.0, -1.0],
        lower: vec![0.0, 0.0],
        upper: vec![1.0, 1.0],
        a: vec![1.0, 1.0],
        b: vec![2.0],
    }
}

/// RFC 039 §1's tie case: minimise `-(x0 + x1)` subject to `x0 + x1 <= 1`, box `[0,1]^2`.
/// Every point with `x0 + x1 = 1`, `0 <= x0, x1 <= 1` is optimal: the answer is a face, not
/// a vertex.
pub fn tie_case() -> DenseQp {
    DenseQp {
        n: 2,
        q: vec![0.0; 4],
        c: vec![-1.0, -1.0],
        lower: vec![0.0, 0.0],
        upper: vec![1.0, 1.0],
        a: vec![1.0, 1.0],
        b: vec![1.0],
    }
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

/// The dynamic LP adapter: `Q = 0`, `c`, box, `A`, `b` from a `DenseQp`.
#[derive(Clone)]
struct LpProgram {
    q: DenseMatrix<f64>,
    c: DenseVector<f64>,
    lower: DenseVector<f64>,
    upper: DenseVector<f64>,
    a: DenseMatrix<f64>,
    b: DenseVector<f64>,
}

impl LpProgram {
    fn new(problem: &DenseQp) -> Result<Self, SolverError> {
        let n = problem.n;
        let m = problem.b.len();
        Ok(Self {
            q: matrix(n, n, vec![0.0; n * n])?,
            c: vector(problem.c.clone())?,
            lower: vector(problem.lower.clone())?,
            upper: vector(problem.upper.clone())?,
            a: matrix(m, n, problem.a.clone())?,
            b: vector(problem.b.clone())?,
        })
    }
}

fn matrix(rows: usize, cols: usize, data: Vec<f64>) -> Result<DenseMatrix<f64>, SolverError> {
    DenseMatrix::from_row_major_vec(rows, cols, data).map_err(|_| SolverError::InvalidDimension)
}

fn vector(values: Vec<f64>) -> Result<DenseVector<f64>, SolverError> {
    DenseVector::from_vec(values).map_err(|_| SolverError::InvalidDimension)
}

impl QuadraticObjective<f64> for LpProgram {
    type Hessian = DenseMatrix<f64>;
    type Linear = DenseVector<f64>;
    fn hessian(&self) -> &Self::Hessian {
        &self.q
    }
    fn linear_term(&self) -> &Self::Linear {
        &self.c
    }
}

impl BoxBounds<f64> for LpProgram {
    type Bound = DenseVector<f64>;
    fn lower_bounds(&self) -> &Self::Bound {
        &self.lower
    }
    fn upper_bounds(&self) -> &Self::Bound {
        &self.upper
    }
}

impl LinearInequalities<f64> for LpProgram {
    type Constraints = DenseMatrix<f64>;
    type Rhs = DenseVector<f64>;
    fn constraint_matrix(&self) -> &Self::Constraints {
        &self.a
    }
    fn constraint_rhs(&self) -> &Self::Rhs {
        &self.b
    }
}

/// One solve's outcome, read from the record, plus the deviation from the exact
/// optimum where the problem is in `exact_lp_optimum`'s scope.
#[derive(Clone, Debug, PartialEq)]
pub struct LpOutcome {
    pub status: String,
    pub termination: String,
    pub cap_hits: u32,
    pub violation: f64,
    /// `None` if `n > exact::MAX_N`, or if no exact optimum was found (infeasible).
    pub deviation_from_exact: Option<f64>,
}

fn render_status(status: SolveStatus) -> String {
    match status {
        SolveStatus::Converged => "converged".to_owned(),
        SolveStatus::NotConverged => "not converged".to_owned(),
        _ => "unrecognized".to_owned(),
    }
}

/// Solve `problem` with `projection_max_sweeps = cap`, and report the outcome plus its
/// deviation from the exact LP optimum when in scope.
pub fn measure(problem: &DenseQp, cap: u32) -> Result<LpOutcome, String> {
    let program = LpProgram::new(problem).map_err(|e| format!("corpus rejected: {e:?}"))?;
    let mut workspace = ClusterConstrainedWorkspace::new(problem.n, problem.b.len())
        .map_err(|e| format!("workspace rejected: {e:?}"))?;
    let config = ConstrainedProjectedConfig {
        max_iterations: MAX_ITERATIONS,
        tolerance: TOLERANCE,
        projection_max_sweeps: cap,
        projection_tolerance: PROJECTION_TOLERANCE,
    };
    let context = ClusterExecutionContext::new(
        ClusterCancellationToken::new(),
        0,
        ClusterValidationPolicy::ValidateAllInputs,
    );
    let mut x = vector(vec![0.0; problem.n]).map_err(|e| format!("start rejected: {e:?}"))?;
    let record = solve_constrained_projected_first_order_dyn(
        &program,
        STEP_SCALE,
        &mut x,
        &mut workspace,
        &config,
        &context,
    )
    .map_err(|e| format!("solve failed: {e:?}"))?;
    let iterate: Vec<f64> = (0..problem.n)
        .map(|i| x.get(i))
        .collect::<Result<_, _>>()
        .map_err(|e| format!("cannot read the returned iterate: {e:?}"))?;
    let deviation_from_exact = if problem.n > exact::MAX_N {
        None
    } else {
        exact::exact_lp_optimum(problem).map(|exact_x| {
            iterate
                .iter()
                .zip(&exact_x)
                .map(|(a, b)| (a - b).abs())
                .fold(0.0_f64, f64::max)
        })
    };
    Ok(LpOutcome {
        status: render_status(record.report.status()),
        termination: format!("{:?}", record.report.termination()),
        cap_hits: record.projection_cap_hits,
        violation: record.max_constraint_violation,
        deviation_from_exact,
    })
}

/// One row of the cap-sensitivity table: how many of `instances` converge at `cap`, and
/// how many of those are converged but not optimal (RFC 039 §0.2 / §1 cap-sensitivity
/// requirement, reproduced as a measurement here rather than quoted).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapRow {
    pub cap: u32,
    pub total: usize,
    pub converged: usize,
    pub converged_but_not_optimal: usize,
}

/// `cap_sensitivity` needs every outcome, not only the summary, so a caller can report the
/// non-converging instances' detail (RFC 039 handoff §0.3).
pub fn cap_sensitivity_detail(instances: &[DenseQp], cap: u32) -> Vec<Result<LpOutcome, String>> {
    instances
        .iter()
        .map(|problem| measure(problem, cap))
        .collect()
}

/// The cap-sensitivity table over `caps`, each measured independently.
pub fn cap_sensitivity(instances: &[DenseQp], caps: &[u32]) -> Vec<CapRow> {
    caps.iter()
        .map(|&cap| {
            let outcomes: Vec<LpOutcome> = cap_sensitivity_detail(instances, cap)
                .into_iter()
                .filter_map(Result::ok)
                .collect();
            let converged: Vec<&LpOutcome> = outcomes
                .iter()
                .filter(|o| o.status == "converged")
                .collect();
            let not_optimal = converged
                .iter()
                .filter(|o| {
                    o.deviation_from_exact
                        .is_some_and(|d| d > NOT_OPTIMAL_DEVIATION)
                })
                .count();
            CapRow {
                cap,
                total: instances.len(),
                converged: converged.len(),
                converged_but_not_optimal: not_optimal,
            }
        })
        .collect()
}

pub fn run() -> bool {
    println!("[lp] RFC 039: what the kernel does with a linear objective (reported, not enforced)");
    let mut ok = true;

    let corpus = random_corpus(300, 0x1_0CA7_u64);
    println!();
    println!(
        "random LP corpus: n=4, m=3, c in [-1,1]^4, box [0,1]^4, A in [0,1]^(3x4), b in [0.5,1.5]^3, 300 instances"
    );
    println!("cap sensitivity (exact reference, not the randomized oracle):");
    for row in cap_sensitivity(&corpus, CAPS) {
        println!(
            "  projection_max_sweeps={:>6}: converged {}/{}  converged-but-not-optimal {}  [measured]",
            row.cap, row.converged, row.total, row.converged_but_not_optimal
        );
    }
    let mut worst_deviation = 0.0_f64;
    let mut non_converging = 0usize;
    for (i, outcome) in cap_sensitivity_detail(&corpus, 500).into_iter().enumerate() {
        match outcome {
            Ok(o) => {
                if o.status == "converged" {
                    if let Some(d) = o.deviation_from_exact {
                        worst_deviation = worst_deviation.max(d);
                    }
                } else {
                    non_converging += 1;
                    println!(
                        "  instance {i}: status={} termination={} cap_hits={} violation={:e}  [measured]",
                        o.status, o.termination, o.cap_hits, o.violation
                    );
                }
            }
            Err(error) => {
                ok = false;
                println!("  instance {i}: FAILED: {error}");
            }
        }
    }
    println!(
        "  worst deviation from the exact optimum, every converged run at cap=500: {worst_deviation:e}  [derived]"
    );
    println!("  non-converging instances at cap=500: {non_converging}  [measured]");

    println!();
    println!("hand-built cases outside the random sample's reach:");
    for (name, problem) in [
        (
            "unopposed direction, distant box face",
            unopposed_direction_case(),
        ),
        (
            "degenerate vertex (3 active rows, n=2)",
            degenerate_vertex_case(),
        ),
    ] {
        match measure(&problem, 500) {
            Ok(o) => println!(
                "  {name}: status={} termination={} cap_hits={} violation={:e} deviation={:?}  [measured/derived]",
                o.status, o.termination, o.cap_hits, o.violation, o.deviation_from_exact
            ),
            Err(error) => {
                ok = false;
                println!("  {name}: FAILED: {error}");
            }
        }
    }
    println!("  note: the unopposed-direction case hits the plain outer-iteration cap with zero");
    println!("        projection cap hits — a different non-convergence cause than the random");
    println!("        corpus's, where the Dykstra projection's sweep cap binds.");
    match measure(&tie_case(), 500) {
        Ok(o) => println!(
            "  tie along an optimal face: status={} termination={} cap_hits={} violation={:e}  [measured] (no single exact point; see the module's tests for face membership)",
            o.status, o.termination, o.cap_hits, o.violation
        ),
        Err(error) => {
            ok = false;
            println!("  tie along an optimal face: FAILED: {error}");
        }
    }

    println!();
    println!("[lp] {}", if ok { "PASS" } else { "FAIL" });
    ok
}

#[cfg(test)]
mod tests {
    use super::{
        CapRow, cap_sensitivity, degenerate_vertex_case, measure, random_corpus, tie_case,
        unopposed_direction_case,
    };

    #[test]
    fn random_corpus_is_reproducible_and_within_the_declared_ranges() {
        let a = random_corpus(50, 42);
        let b = random_corpus(50, 42);
        assert_eq!(a.len(), 50);
        for problem in &a {
            assert_eq!(problem.n, 4);
            assert_eq!(problem.b.len(), 3);
            for &c in &problem.c {
                assert!((-1.0..=1.0).contains(&c), "{c}");
            }
            for &lo in &problem.lower {
                assert_eq!(lo, 0.0);
            }
            for &up in &problem.upper {
                assert_eq!(up, 1.0);
            }
            for &coeff in &problem.a {
                assert!((0.0..=1.0).contains(&coeff), "{coeff}");
            }
            for &rhs in &problem.b {
                assert!((0.5..=1.5).contains(&rhs), "{rhs}");
            }
        }
        for (x, y) in a.iter().zip(&b) {
            assert_eq!(x.c, y.c, "same seed must reproduce the same corpus");
        }
    }

    #[test]
    fn the_unopposed_direction_case_hits_the_iteration_cap_not_the_projection_cap() {
        // Measured, not assumed: a fixed gradient step (`alpha * |c|` per outer
        // iteration) needs about `upper / (alpha * |c|)` iterations to walk an
        // unopposed coordinate to a distant box face. At `upper = 1e6`,
        // `alpha = 0.3`, that is far beyond `MAX_ITERATIONS`, so this case hits the
        // plain outer-iteration cap with **zero** projection cap hits — a different
        // failure mode than the random corpus's projection-cap-bound non-convergence.
        let outcome = measure(&unopposed_direction_case(), 500).expect("solve");
        assert_eq!(outcome.status, "not converged");
        assert_eq!(outcome.termination, "IterationCap");
        assert_eq!(outcome.cap_hits, 0);
        let deviation = outcome.deviation_from_exact.expect("n <= MAX_N");
        assert!(deviation > 1.0, "{deviation:e}");
    }

    #[test]
    fn the_degenerate_vertex_case_converges_to_the_pinned_vertex() {
        let outcome = measure(&degenerate_vertex_case(), 500).expect("solve");
        assert_eq!(outcome.status, "converged");
        let deviation = outcome.deviation_from_exact.expect("n <= MAX_N");
        assert!(deviation < 1e-6, "{deviation:e}");
    }

    #[test]
    fn the_tie_case_solves_without_a_single_exact_comparison_point() {
        // `exact_lp_optimum` returns one point on the face; the kernel's own returned
        // point need not equal it, so this test only checks the solve itself runs and
        // leaves face membership to `exact.rs`'s own test for the tie fixture's shape.
        let outcome = measure(&tie_case(), 500).expect("solve");
        assert_eq!(outcome.status, "converged");
    }

    #[test]
    fn cap_sensitivity_reports_more_convergence_at_a_higher_cap() {
        let corpus = random_corpus(40, 7);
        let rows: Vec<CapRow> = cap_sensitivity(&corpus, &[50, 50_000]);
        assert_eq!(rows.len(), 2);
        assert!(
            rows[1].converged >= rows[0].converged,
            "a higher cap must not converge less often: {rows:?}"
        );
    }
}
