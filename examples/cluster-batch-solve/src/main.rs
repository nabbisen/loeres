//! Batch projected first-order solve on the server path.
//!
//! What this example demonstrates (RFC 023 §11.3):
//!
//! - building several dynamic (runtime-dimensioned) box-constrained problems,
//! - erasing each into a `ClusterJob` and running them through `solve_batch`
//!   under `parallel-rayon`,
//! - reading **per-item outcomes**, including a **non-converged item handled as
//!   an `Ok` status** rather than a batch failure, and a genuinely invalid item
//!   arriving as `Failed` while its neighbours still succeed.
//!
//! The boundary this example does *not* cross: nothing here is available on the
//! edge path. `loeres-cluster` and `loeres-backend-std` are server-only, and the
//! device example is the other half of the pair.

use loeres::{ContiguousVectorAccess, SolveStatus, SolverError, VectorAccess, VectorAccessMut};
use loeres_backend_std::DenseVector;
use loeres_cluster::{
    BatchExecutionPolicy, BatchItemOutcome, ClusterCancellationToken, ClusterError, ClusterJob,
    ClusterProjectedFirstOrderJob, ClusterProjectedFirstOrderProblem, ClusterSolution,
    ClusterSolveConfig, ProjectedFirstOrderConfig, solve_batch,
};

/// Separable quadratic `f(x) = ½ Σ wᵢ (xᵢ − cᵢ)²` over a box, dimensioned at
/// runtime rather than in the type.
///
/// Gradient `wᵢ (xᵢ − cᵢ)`; box-constrained optimum `clamp(cᵢ, loᵢ, hiᵢ)`.
#[derive(Clone)]
struct BoxQuadratic {
    weights: Vec<f64>,
    centers: Vec<f64>,
    lower: DenseVector<f64>,
    upper: DenseVector<f64>,
    step_scale: f64,
}

impl ClusterProjectedFirstOrderProblem<f64> for BoxQuadratic {
    fn dimension(&self) -> usize {
        self.centers.len()
    }

    fn bounds(&self) -> (&DenseVector<f64>, &DenseVector<f64>) {
        (&self.lower, &self.upper)
    }

    fn gradient_at(
        &self,
        x: &DenseVector<f64>,
        grad: &mut DenseVector<f64>,
    ) -> Result<(), SolverError> {
        for i in 0..self.dimension() {
            let gi = self.weights[i] * (x.get(i)? - self.centers[i]);
            grad.set(i, gi)?;
        }
        Ok(())
    }

    fn step_scale(&self) -> f64 {
        self.step_scale
    }
}

fn main() -> Result<(), ClusterError> {
    let items = items();
    let labels: Vec<&str> = items.iter().map(|(label, _)| *label).collect();

    // Each job owns its problem, its starting iterate, and its numeric config.
    // `Box<dyn ClusterJob<f64>>` is the dispatch barrier: the monomorphized
    // kernel is erased once, here, so orchestration stays one code path.
    let jobs: Vec<Box<dyn ClusterJob<f64>>> = items
        .into_iter()
        .map(|(_, job)| Box::new(job) as Box<dyn ClusterJob<f64>>)
        .collect();

    let config = ClusterSolveConfig {
        max_parallelism: 4,
        execution_policy: BatchExecutionPolicy::Parallel,
        ..ClusterSolveConfig::default()
    };

    // A top-level `Err` here would mean an *orchestration* failure — invalid
    // global config, worker-pool init, runtime shutdown. Per-item solver
    // failures never surface here; they are carried in the outcomes.
    let report = solve_batch(jobs, config, ClusterCancellationToken::new())?;

    for (label, outcome) in labels.iter().zip(&report.outcomes) {
        println!("{label:<26} {}", describe(outcome));
    }

    let summary = report.summary;
    println!(
        "\n{} item(s): {} converged, {} not converged, {} failed, {} cancelled, {} panicked",
        summary.total(),
        summary.solved_converged,
        summary.solved_not_converged,
        summary.failed,
        summary.cancelled,
        summary.panicked,
    );

    Ok(())
}

/// Render one per-item outcome.
///
/// The shape worth noticing: `Solved` does **not** mean converged. A solve that
/// hit its iteration cap is `Solved` with a `NotConverged` report and a usable
/// last iterate — the caller decides what to do about it. Only a fail-safe
/// solver error is `Failed`.
fn describe(outcome: &BatchItemOutcome<f64>) -> String {
    match outcome {
        BatchItemOutcome::Solved { solution, report } => {
            let verdict = match report.status() {
                SolveStatus::Converged => "converged",
                SolveStatus::NotConverged => "not converged (cap reached)",
                // `#[non_exhaustive]` downstream: a future variant is something
                // to name, never a reason to panic.
                _ => "unrecognized status",
            };
            match solution {
                ClusterSolution::DenseVector(v) => format!(
                    "{verdict} in {} iteration(s); x = {}",
                    report.iterations_executed(),
                    render(v),
                ),
                // Sparse and dense-matrix variants are reserved; a producer for
                // them does not ship yet.
                _ => format!("{verdict}; solution shape not recognized by this example"),
            }
        }
        BatchItemOutcome::Failed { error } => format!("failed: {error:?}"),
        BatchItemOutcome::Cancelled => "cancelled".to_owned(),
        BatchItemOutcome::Panicked => "worker panic contained at the item boundary".to_owned(),
    }
}

/// Four labelled batch items: two that converge, one that reaches its cap, and
/// one whose bounds are inverted.
fn items() -> Vec<(
    &'static str,
    ClusterProjectedFirstOrderJob<BoxQuadratic, f64>,
)> {
    vec![
        (
            "interior optimum:",
            job(
                BoxQuadratic {
                    weights: vec![1.0, 2.0, 0.5],
                    centers: vec![1.5, -2.0, 4.0],
                    lower: dense(&[-10.0, -10.0, -10.0]),
                    upper: dense(&[10.0, 10.0, 10.0]),
                    step_scale: 0.4,
                },
                &[0.0, 0.0, 0.0],
                2_000,
            ),
        ),
        (
            "bounds active:",
            job(
                BoxQuadratic {
                    weights: vec![1.0, 1.0],
                    centers: vec![5.0, -5.0],
                    lower: dense(&[-1.0, -1.0]),
                    upper: dense(&[1.0, 1.0]),
                    step_scale: 0.5,
                },
                &[0.0, 0.0],
                2_000,
            ),
        ),
        (
            // A cap far below what this problem needs. `Solved` + `NotConverged`.
            "cap reached:",
            job(
                BoxQuadratic {
                    weights: vec![1.0, 1.0],
                    centers: vec![9.0, 9.0],
                    lower: dense(&[-10.0, -10.0]),
                    upper: dense(&[10.0, 10.0]),
                    step_scale: 0.01,
                },
                &[0.0, 0.0],
                5,
            ),
        ),
        (
            // `lower > upper`: rejected before the loop, as a per-item `Failed`.
            // The batch as a whole still returns `Ok`.
            "inverted bounds:",
            job(
                BoxQuadratic {
                    weights: vec![1.0],
                    centers: vec![0.0],
                    lower: dense(&[1.0]),
                    upper: dense(&[-1.0]),
                    step_scale: 0.5,
                },
                &[0.0],
                100,
            ),
        ),
    ]
}

fn job(
    problem: BoxQuadratic,
    initial: &[f64],
    max_iterations: u32,
) -> ClusterProjectedFirstOrderJob<BoxQuadratic, f64> {
    ClusterProjectedFirstOrderJob::new(
        problem,
        dense(initial),
        ProjectedFirstOrderConfig {
            max_iterations,
            tolerance: 1e-9,
        },
    )
}

/// Render a dense solution through the contiguous fast path when storage offers
/// one, falling back to per-element fallible access otherwise — the same
/// two-path shape the solvers use internally (RFC 002). No indexing either way.
fn render(v: &DenseVector<f64>) -> String {
    if let Some(slice) = v.as_contiguous() {
        return format!("{slice:?}");
    }
    let mut parts = Vec::with_capacity(v.len());
    for i in 0..v.len() {
        match v.get(i) {
            Ok(value) => parts.push(format!("{value}")),
            Err(e) => parts.push(format!("<{e:?}>")),
        }
    }
    format!("[{}]", parts.join(", "))
}

/// `DenseVector::from_vec` rejects nothing for a non-empty `Vec<f64>`, but it is
/// fallible in general; an example should not hide that behind `unwrap`.
fn dense(values: &[f64]) -> DenseVector<f64> {
    match DenseVector::from_vec(values.to_vec()) {
        Ok(v) => v,
        Err(e) => panic!("example vector literal is not a valid dense vector: {e:?}"),
    }
}
