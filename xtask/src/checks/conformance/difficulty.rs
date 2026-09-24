//! RFC 031 §3.3 difficulty reporting.
//!
//! The kernels already compute three numbers the corpus used to discard at its
//! boundary: outer iterations executed (against the configured cap),
//! `projection_cap_hits`, and the terminal `max_constraint_violation`. This module
//! carries them from each solve path to the printed output, per fixture and
//! aggregated per suite.
//!
//! **This is reporting, not a pass criterion.** Nothing here influences whether a
//! fixture passes, and there is deliberately no threshold or budget on any figure
//! (RFC 031 §4): a threshold on an unmeasured quantity is how a flaky gate is
//! built. A fixture that converges after one sweep and one that scrapes in at the
//! cap are now distinguishable in the output; whether either is acceptable is a
//! decision for the corpus's owner, made on evidence.

/// The cut-off below which a deviation is not counted as "deviating" in the
/// aggregate line. Reporting only.
const REPORTED_DEVIATION: f64 = 1e-6;

/// One solve path's difficulty figures for one fixture.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct PathDifficulty {
    pub(super) path: &'static str,
    /// Outer iterations executed.
    pub(super) iterations: u32,
    /// The configured outer-iteration cap, printed beside `iterations`.
    pub(super) iteration_cap: u32,
    /// Outer iterations whose projection hit its sweep cap. `None` for the
    /// box-only kernels, which have no inner projection.
    pub(super) cap_hits: Option<u32>,
    /// Terminal `max(0, maxᵢ(aᵢᵀx − bᵢ))`. `None` for the box-only kernels.
    pub(super) violation: Option<f64>,
    /// `maxⱼ |xⱼ − expectedⱼ|` against the fixture's exact expected solution
    /// (RFC 032 exit criterion 6: cap hits alone cannot judge a step rule, because
    /// the deviation is not monotone in the geometry). `None` where the fixture
    /// carries no expected solution.
    pub(super) deviation: Option<f64>,
}

impl PathDifficulty {
    pub(super) fn line(&self) -> String {
        let mut line = format!(
            "{}: outer iterations {} of {}",
            self.path, self.iterations, self.iteration_cap
        );
        if let Some(hits) = self.cap_hits {
            line.push_str(&format!("; projection_cap_hits {hits}"));
        }
        if let Some(violation) = self.violation {
            line.push_str(&format!("; max_constraint_violation {violation:e}"));
        }
        if let Some(deviation) = self.deviation {
            line.push_str(&format!("; deviation from the exact optimum {deviation:e}"));
        }
        line
    }
}

/// Difficulty aggregated over one suite's solve paths.
#[derive(Default, Debug)]
pub(super) struct DifficultySummary {
    /// Solve paths that reported figures (a fixture may run device and cluster).
    runs: u32,
    iterations_total: u64,
    /// The largest `iterations / iteration_cap` over all runs.
    max_iteration_fraction: f64,
    /// Runs of the constrained kernels, the only ones with an inner projection.
    constrained_runs: u32,
    /// Outer iterations of those runs: the denominator of the cap-hit rate.
    constrained_iterations: u64,
    cap_hits_total: u64,
    runs_with_cap_hit: u32,
    max_violation: f64,
    runs_with_violation: u32,
    /// Paths that had an exact expected solution to deviate from.
    deviation_runs: u32,
    max_deviation: f64,
    /// Paths whose deviation exceeds `1e-6`, the corpus's usual solution tolerance
    /// (a reporting cut-off, not a pass criterion).
    runs_deviating: u32,
}

impl DifficultySummary {
    pub(super) fn record(&mut self, paths: &[PathDifficulty]) {
        for path in paths {
            self.runs += 1;
            self.iterations_total += u64::from(path.iterations);
            if path.iteration_cap > 0 {
                let fraction = f64::from(path.iterations) / f64::from(path.iteration_cap);
                self.max_iteration_fraction = self.max_iteration_fraction.max(fraction);
            }
            if let Some(hits) = path.cap_hits {
                self.constrained_runs += 1;
                self.constrained_iterations += u64::from(path.iterations);
                self.cap_hits_total += u64::from(hits);
                if hits > 0 {
                    self.runs_with_cap_hit += 1;
                }
            }
            if let Some(violation) = path.violation {
                self.max_violation = self.max_violation.max(violation);
                if violation > 0.0 {
                    self.runs_with_violation += 1;
                }
            }
            if let Some(deviation) = path.deviation {
                self.deviation_runs += 1;
                self.max_deviation = self.max_deviation.max(deviation);
                if deviation > REPORTED_DEVIATION {
                    self.runs_deviating += 1;
                }
            }
        }
    }

    pub(super) fn print(&self) {
        eprintln!("  difficulty (reported, not enforced; RFC 031 §3.3):");
        eprintln!(
            "    solve paths reporting: {}; outer iterations: {} total, largest {:.1}% of its cap",
            self.runs,
            self.iterations_total,
            100.0 * self.max_iteration_fraction
        );
        if self.constrained_runs == 0 {
            eprintln!("    constrained paths: none");
            return;
        }
        eprintln!(
            "    constrained paths: {}; projection cap hits: {} in {} outer iterations ({:.2}%); paths with a cap hit: {} of {}",
            self.constrained_runs,
            self.cap_hits_total,
            self.constrained_iterations,
            self.cap_hit_rate_percent(),
            self.runs_with_cap_hit,
            self.constrained_runs
        );
        eprintln!(
            "    terminal max_constraint_violation: largest {:e}; paths with a positive violation: {} of {}",
            self.max_violation, self.runs_with_violation, self.constrained_runs
        );
        if self.deviation_runs > 0 {
            eprintln!(
                "    deviation from the exact optimum: largest {:e}; paths deviating by more than {REPORTED_DEVIATION:e}: {} of {}",
                self.max_deviation, self.runs_deviating, self.deviation_runs
            );
        }
    }

    /// Projection cap hits per hundred outer iterations, over the constrained
    /// paths. Zero when there were none.
    pub(super) fn cap_hit_rate_percent(&self) -> f64 {
        if self.constrained_iterations == 0 {
            0.0
        } else {
            100.0 * self.cap_hits_total as f64 / self.constrained_iterations as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path(
        iterations: u32,
        cap: u32,
        hits: Option<u32>,
        violation: Option<f64>,
    ) -> PathDifficulty {
        PathDifficulty {
            path: "cluster",
            iterations,
            iteration_cap: cap,
            cap_hits: hits,
            violation,
            deviation: None,
        }
    }

    #[test]
    fn a_line_shows_the_iterations_against_the_cap_and_the_constrained_figures() {
        assert_eq!(
            path(42, 5000, Some(42), Some(2.0)).line(),
            "cluster: outer iterations 42 of 5000; projection_cap_hits 42; max_constraint_violation 2e0"
        );
        assert_eq!(
            path(93, 100, None, None).line(),
            "cluster: outer iterations 93 of 100"
        );
    }

    #[test]
    fn the_aggregate_counts_only_constrained_paths_in_the_cap_hit_rate() {
        let mut summary = DifficultySummary::default();
        summary.record(&[
            path(100, 1000, None, None),
            path(10, 1000, Some(5), Some(0.0)),
            path(30, 1000, Some(0), Some(2.0)),
        ]);
        assert_eq!(summary.runs, 3);
        assert_eq!(summary.constrained_runs, 2);
        assert_eq!(summary.constrained_iterations, 40);
        assert_eq!(summary.cap_hits_total, 5);
        assert_eq!(summary.runs_with_cap_hit, 1);
        assert_eq!(summary.runs_with_violation, 1);
        assert_eq!(summary.max_violation, 2.0);
        assert!((summary.cap_hit_rate_percent() - 12.5).abs() < 1e-12);
        assert!((summary.max_iteration_fraction - 0.1).abs() < 1e-12);
    }

    #[test]
    fn the_aggregate_reports_the_largest_deviation_and_how_many_paths_exceed_the_cut_off() {
        let mut summary = DifficultySummary::default();
        let with = |deviation| {
            let mut p = path(2, 5000, Some(0), Some(0.0));
            p.deviation = deviation;
            p
        };
        summary.record(&[with(Some(1e-9)), with(Some(9e-4)), with(None)]);
        assert_eq!(summary.deviation_runs, 2);
        assert_eq!(summary.runs_deviating, 1);
        assert_eq!(summary.max_deviation, 9e-4);
        let mut p = path(2, 5000, Some(0), Some(0.0));
        p.deviation = Some(3e-6);
        assert!(p.line().ends_with("deviation from the exact optimum 3e-6"));
    }

    #[test]
    fn an_empty_summary_has_a_zero_rate_and_prints_without_dividing() {
        let summary = DifficultySummary::default();
        assert_eq!(summary.cap_hit_rate_percent(), 0.0);
    }
}
