use super::*;

#[test]
fn job_runs_converging_problem_through_solve_batch() {
    let p = Quadratic {
        weights: vec![1.0, 1.0],
        centers: vec![3.0, -2.0],
        lo: dv(&[-10.0, -10.0]),
        hi: dv(&[10.0, 10.0]),
        alpha: 0.5,
    };
    let job = ClusterProjectedFirstOrderJob::new(p, dv(&[0.0, 0.0]), cfg(1000, 1e-9));
    let jobs: Vec<Box<dyn ClusterJob<f64>>> = vec![Box::new(job)];
    let report = solve_batch(
        jobs,
        ClusterSolveConfig::default(),
        ClusterCancellationToken::new(),
    )
    .unwrap();
    assert_eq!(report.summary.solved_converged, 1);
    assert_eq!(report.summary.failed, 0);
}

#[test]
fn job_maps_cancellation_to_cancelled_outcome() {
    let cancel = ClusterCancellationToken::new();
    cancel.cancel();
    let p = Quadratic {
        weights: vec![1.0],
        centers: vec![1.0],
        lo: dv(&[-1.0]),
        hi: dv(&[1.0]),
        alpha: 0.5,
    };
    let job = ClusterProjectedFirstOrderJob::new(p, dv(&[0.0]), cfg(10, 1e-9));
    let jobs: Vec<Box<dyn ClusterJob<f64>>> = vec![Box::new(job)];
    let report = solve_batch(jobs, ClusterSolveConfig::default(), cancel).unwrap();
    assert_eq!(report.summary.cancelled, 1);
}
