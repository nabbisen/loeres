//! `throughput` — RFC 037 §4.4 and S4: wall-time throughput, advisory, with its host.
//!
//! **Advisory, always. This command is never a gate.** Wall time depends on the host's
//! load, its frequency scaling, the toolchain and the process's placement, so a second run
//! on the same machine will differ. `cargo xtask check` and `cargo xtask release-gate` do
//! not run it and do not depend on any figure it prints. Its precedent is `size-budget`
//! ("advisory baseline reported") and RFC 031's difficulty reporting ("reported, not
//! enforced"). Do not add it to a gate list. Do not "fix" that omission.
//!
//! **Every figure carries its environment.** Each line names the host (architecture, OS,
//! logical threads) and the toolchain, so a figure cannot travel without the conditions it
//! was measured under. RFC 037 §5.5 relies on that: a wall-time figure in the book is
//! unchecked, and its host has to be visible next to it.
//!
//! **`std::time::Instant` only.** There is no statistical harness and no new dependency
//! (RFC 037 §4.5): the figures are medians of repeated runs, and the spread is printed so a
//! reader can see how much they move.
//!
//! The parallel figure needs the cluster crate built with `parallel-rayon`. xtask enables it
//! in its manifest. The command reads back `effective_execution()` and says which policy the
//! batch actually used, so a silent fallback to sequential cannot pass as a speedup.

use std::time::{Duration, Instant};

use loeres_cluster::{
    BatchExecutionPolicy, BatchItemOutcome, ClusterCancellationToken, ClusterConstrainedJob,
    ClusterJob, ClusterSolveConfig, ConstrainedProjectedConfig, solve_batch,
};

use super::bench::{Family, PROJECTION_TOLERANCE, Program, STEP_SCALE, measure};

/// Repeats for the single-solve figure. Enough for a median to settle.
const SINGLE_REPEATS: usize = 50;
/// Items in one batch. The same family member, so the work per item is constant.
const BATCH_ITEMS: usize = 64;
/// Repeats for each batch figure.
const BATCH_REPEATS: usize = 5;

/// The corpus point the throughput figures use: the one `bench-baseline` pins at its
/// centre, `n = 32, m = 16, off = 0.50`.
const FAMILY: Family = Family {
    n: 32,
    m: 16,
    off: 0.50,
};

/// The host and toolchain line that every figure is printed beside.
pub fn environment(threads: usize, toolchain: &str) -> String {
    format!(
        "host {} {} · {} logical threads · {}",
        std::env::consts::ARCH,
        std::env::consts::OS,
        threads,
        toolchain
    )
}

/// The median of a set of durations. Sorts a copy.
pub fn median(durations: &[Duration]) -> Option<Duration> {
    if durations.is_empty() {
        return None;
    }
    let mut sorted = durations.to_vec();
    sorted.sort();
    Some(sorted[sorted.len() / 2])
}

/// The spread of a set of durations, as `(min, max)`.
pub fn spread(durations: &[Duration]) -> Option<(Duration, Duration)> {
    Some((*durations.iter().min()?, *durations.iter().max()?))
}

/// Milliseconds with three decimals, for printing.
pub fn millis(d: Duration) -> String {
    format!("{:.3} ms", d.as_secs_f64() * 1e3)
}

/// The batch's effective policy, read back from the config. A `Parallel` request that the
/// build cannot honour falls back to `Sequential`, and that must be visible.
pub fn policy_name(policy: BatchExecutionPolicy) -> &'static str {
    match policy {
        BatchExecutionPolicy::Sequential => "sequential",
        BatchExecutionPolicy::Parallel => "parallel",
    }
}

/// The toolchain string, from `rustc -V`. Reported as unknown rather than guessed.
fn toolchain() -> String {
    std::process::Command::new("rustc")
        .arg("-V")
        .output()
        .ok()
        .filter(|out| out.status.success())
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_owned())
        .unwrap_or_else(|| "rustc version unknown".to_owned())
}

fn constrained_config() -> ConstrainedProjectedConfig<f64> {
    ConstrainedProjectedConfig {
        max_iterations: 20_000,
        tolerance: 1e-10,
        projection_max_sweeps: 500,
        projection_tolerance: PROJECTION_TOLERANCE,
    }
}

/// One single-solve timing: the solve only, not the program construction.
fn time_one_solve() -> Result<Duration, String> {
    let program = Program::new(FAMILY).map_err(|e| format!("corpus rejected: {e:?}"))?;
    let started = Instant::now();
    let measured = measure_with(&program);
    let elapsed = started.elapsed();
    measured.map(|_| elapsed)
}

/// Solves the program once, through the same path `measure` uses. Separate from `measure`
/// so the timed region is the solve and nothing else.
fn measure_with(program: &Program) -> Result<(), String> {
    use loeres_cluster::{
        ClusterCancellationToken, ClusterConstrainedWorkspace, ClusterExecutionContext,
        ClusterValidationPolicy, solve_constrained_projected_first_order_dyn,
    };
    let mut workspace = ClusterConstrainedWorkspace::new(FAMILY.n, FAMILY.m)
        .map_err(|e| format!("workspace rejected: {e:?}"))?;
    let context = ClusterExecutionContext::new(
        ClusterCancellationToken::new(),
        0,
        ClusterValidationPolicy::ValidateAllInputs,
    );
    let mut x = loeres_backend_std::DenseVector::from_vec(vec![0.0; FAMILY.n])
        .map_err(|e| format!("start rejected: {e:?}"))?;
    solve_constrained_projected_first_order_dyn(
        program,
        STEP_SCALE,
        &mut x,
        &mut workspace,
        &constrained_config(),
        &context,
    )
    .map(|_| ())
    .map_err(|e| format!("solve failed: {e:?}"))
}

/// One batch of `BATCH_ITEMS` identical solves under the given policy. Returns the wall time
/// and the number of items that converged, so a batch that did not solve cannot pass as a
/// fast one.
fn time_one_batch(
    policy: BatchExecutionPolicy,
    threads: usize,
) -> Result<(Duration, usize), String> {
    let mut jobs: Vec<Box<dyn ClusterJob<f64>>> = Vec::with_capacity(BATCH_ITEMS);
    for _ in 0..BATCH_ITEMS {
        let program = Program::new(FAMILY).map_err(|e| format!("corpus rejected: {e:?}"))?;
        let initial = loeres_backend_std::DenseVector::from_vec(vec![0.0; FAMILY.n])
            .map_err(|e| format!("start rejected: {e:?}"))?;
        jobs.push(Box::new(ClusterConstrainedJob::new(
            program,
            STEP_SCALE,
            initial,
            constrained_config(),
        )));
    }
    let config = ClusterSolveConfig {
        max_parallelism: threads,
        execution_policy: policy,
        ..ClusterSolveConfig::default()
    };
    let started = Instant::now();
    let report = solve_batch(jobs, config, ClusterCancellationToken::new())
        .map_err(|e| format!("batch rejected: {e:?}"))?;
    let elapsed = started.elapsed();
    let converged = report
        .outcomes
        .iter()
        .filter(|outcome| matches!(outcome, BatchItemOutcome::Solved { report, .. } if report.status() == loeres::SolveStatus::Converged))
        .count();
    Ok((elapsed, converged))
}

pub fn run() -> bool {
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
    let toolchain = toolchain();
    let env = environment(threads, &toolchain);
    println!("[throughput] RFC 037 S4: wall time, advisory. Not reproducible; never gated.");
    println!("  every figure below was measured on: {env}");
    println!(
        "  family: n={}, m={}, off={:.2}; the same member throughout",
        FAMILY.n, FAMILY.m, FAMILY.off
    );
    println!();

    // Single solve.
    let mut ok = true;
    let mut solves = Vec::with_capacity(SINGLE_REPEATS);
    for _ in 0..SINGLE_REPEATS {
        match time_one_solve() {
            Ok(d) => solves.push(d),
            Err(error) => {
                println!("  single solve FAILED: {error}");
                ok = false;
                break;
            }
        }
    }
    if let (Some(med), Some((lo, hi))) = (median(&solves), spread(&solves)) {
        let iterations = measure(FAMILY)
            .map(|m| m.outer_iterations)
            .unwrap_or(0)
            .max(1);
        let per_iteration = med / iterations;
        println!("single solve, {SINGLE_REPEATS} repeats  [wall; {env}]");
        println!(
            "  median {}   range {} to {}",
            millis(med),
            millis(lo),
            millis(hi)
        );
        println!(
            "  per outer iteration {}  [derived: median ÷ {iterations} outer iterations]",
            millis(per_iteration)
        );
    }
    println!();

    // Batch, sequential and parallel.
    let seq = repeat_batch(BatchExecutionPolicy::Sequential, threads);
    let par = repeat_batch(BatchExecutionPolicy::Parallel, threads);
    for (label, result) in [("sequential", &seq), ("parallel", &par)] {
        match result {
            Ok(summary) => {
                println!("batch of {BATCH_ITEMS}, {label}, {BATCH_REPEATS} repeats  [wall; {env}]");
                println!(
                    "  policy used: {}   median {}   range {} to {}   converged {}/{}",
                    summary.policy,
                    millis(summary.median),
                    millis(summary.spread.0),
                    millis(summary.spread.1),
                    summary.converged,
                    BATCH_ITEMS
                );
            }
            Err(error) => {
                ok = false;
                println!("batch ({label}) FAILED: {error}");
            }
        }
    }
    if let (Ok(seq), Ok(par)) = (&seq, &par) {
        let speedup = seq.median.as_secs_f64() / par.median.as_secs_f64();
        println!();
        println!(
            "parallel speedup over sequential: {speedup:.2}×  [derived: ratio of two wall-time medians; {env}]"
        );
        println!(
            "batch throughput, parallel: {:.0} solves/s  [derived: {BATCH_ITEMS} ÷ parallel median; {env}]",
            BATCH_ITEMS as f64 / par.median.as_secs_f64()
        );
    }
    println!();
    println!(
        "Wall time is not reproducible: a re-run on this host will differ, and a run on another host will differ more."
    );
    println!("No figure above is a gate, and no gate depends on one (RFC 037 §4.4).");
    ok
}

/// One batch policy measured over `BATCH_REPEATS` runs.
pub struct BatchSummary {
    pub median: Duration,
    pub spread: (Duration, Duration),
    pub converged: usize,
    pub policy: &'static str,
}

/// The median batch time over `BATCH_REPEATS` runs, with its spread, the converged count of
/// the last run, and the policy the config resolves to.
fn repeat_batch(policy: BatchExecutionPolicy, threads: usize) -> Result<BatchSummary, String> {
    let mut times = Vec::with_capacity(BATCH_REPEATS);
    let mut converged = 0;
    for _ in 0..BATCH_REPEATS {
        let (elapsed, count) = time_one_batch(policy, threads)?;
        times.push(elapsed);
        converged = count;
    }
    let effective = policy_name(
        ClusterSolveConfig {
            execution_policy: policy,
            ..ClusterSolveConfig::default()
        }
        .effective_execution(),
    );
    Ok(BatchSummary {
        median: median(&times).ok_or("no batch repeats")?,
        spread: spread(&times).ok_or("no batch repeats")?,
        converged,
        policy: effective,
    })
}

#[cfg(test)]
mod tests {
    use super::{median, millis, policy_name, spread};
    use loeres_cluster::BatchExecutionPolicy;
    use std::time::Duration;

    #[test]
    fn the_median_is_the_middle_value_regardless_of_input_order() {
        let ds = [
            Duration::from_millis(5),
            Duration::from_millis(1),
            Duration::from_millis(3),
        ];
        assert_eq!(median(&ds), Some(Duration::from_millis(3)));
        assert_eq!(median(&[]), None);
    }

    #[test]
    fn the_spread_is_the_min_and_the_max() {
        let ds = [
            Duration::from_millis(5),
            Duration::from_millis(1),
            Duration::from_millis(3),
        ];
        assert_eq!(
            spread(&ds),
            Some((Duration::from_millis(1), Duration::from_millis(5)))
        );
    }

    #[test]
    fn a_millisecond_figure_prints_with_three_decimals() {
        assert_eq!(millis(Duration::from_micros(1500)), "1.500 ms");
    }

    #[test]
    fn the_policy_name_reports_what_the_batch_actually_used() {
        assert_eq!(policy_name(BatchExecutionPolicy::Sequential), "sequential");
        assert_eq!(policy_name(BatchExecutionPolicy::Parallel), "parallel");
    }
}
