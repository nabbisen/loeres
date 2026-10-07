//! `bench-baseline` — RFC 037 §4.3 and exit criterion 3: the counted work of the corpus
//! is pinned for the host target and asserted.
//!
//! `bench` reports counted work over both axes and is not a gate. A reported figure that
//! nobody asserts can drift without anyone noticing, so this gate pins the eleven unique
//! `(n, m, off)` points of the corpus, for both the cluster and the device path, and
//! fails when a measured count differs from its pin.
//!
//! **A changed count failing this gate is the intended behaviour, not a nuisance.** It
//! forces a conscious decision, exactly as RFC 022's citation registry and RFC 030's
//! differential registry do. The remedy is to re-measure the point, understand why the
//! count moved, and update its row in `PINNED` in the **same commit** as the change that
//! moved it. Do not update a pin to make the gate green without that understanding.
//!
//! **Host target only.** Counted work is reproducible per target, not across targets
//! (RFC 037 §4.3; `TERMS_OF_USE` determinism is target-scoped). The device *target* is
//! reported by `bench` and never asserted here. The footprint check below uses `size_of`
//! on the build host, which is the host target.
//!
//! **No wall time, ever.** This gate reads iteration counts, cap hits and footprint bytes.
//! Nothing in it depends on elapsed time.
//!
//! The pinned values were measured by the architect on `ddbf356` (RFC 037 §0.2) and
//! reproduced on the implementing tree; they are the eleven rows of that table.

use super::bench::{Family, Measured, corpus, measure, measure_device};

/// RFC 037 C1: the manifest of the example that carries a second copy of the corpus
/// family, so a drift between `bench` and the example fails here rather than silently.
const EXAMPLE_MANIFEST: &str = "examples/cluster-counted-work/Cargo.toml";

/// One pinned point of the corpus: the family member and its expected counted work.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pin {
    pub n: usize,
    pub m: usize,
    pub off: f64,
    /// Expected outer iterations. Identical on both paths at every pinned point.
    pub outer_iterations: u32,
    /// Expected projection cap hits. Zero at every pinned point.
    pub cap_hits: u32,
}

/// The eleven unique points: seven on the size axis at `off = 0.50`, and four more on the
/// conditioning axis at `n = 32, m = 16` (the `off = 0.50` point is shared, and is pinned
/// once). Adding or moving a point is a change to this table, made in the same commit as
/// the change to the corpus.
pub const PINNED: &[Pin] = &[
    Pin {
        n: 4,
        m: 2,
        off: 0.50,
        outer_iterations: 30,
        cap_hits: 0,
    },
    Pin {
        n: 8,
        m: 4,
        off: 0.50,
        outer_iterations: 43,
        cap_hits: 0,
    },
    Pin {
        n: 16,
        m: 8,
        off: 0.50,
        outer_iterations: 48,
        cap_hits: 0,
    },
    Pin {
        n: 32,
        m: 16,
        off: 0.50,
        outer_iterations: 48,
        cap_hits: 0,
    },
    Pin {
        n: 64,
        m: 32,
        off: 0.50,
        outer_iterations: 48,
        cap_hits: 0,
    },
    Pin {
        n: 128,
        m: 64,
        off: 0.50,
        outer_iterations: 48,
        cap_hits: 0,
    },
    Pin {
        n: 256,
        m: 128,
        off: 0.50,
        outer_iterations: 48,
        cap_hits: 0,
    },
    Pin {
        n: 32,
        m: 16,
        off: 0.10,
        outer_iterations: 25,
        cap_hits: 0,
    },
    Pin {
        n: 32,
        m: 16,
        off: 0.90,
        outer_iterations: 232,
        cap_hits: 0,
    },
    Pin {
        n: 32,
        m: 16,
        off: 0.97,
        outer_iterations: 578,
        cap_hits: 0,
    },
    Pin {
        n: 32,
        m: 16,
        off: 0.99,
        outer_iterations: 989,
        cap_hits: 0,
    },
];

pub fn run() -> bool {
    eprintln!("[bench-baseline] RFC 037 counted-work baseline, host target");
    let mut findings = baseline_findings(
        PINNED,
        &corpus().into_iter().map(|(_, f)| f).collect::<Vec<_>>(),
        measure,
        |family| {
            measure_device(family).map(|point| {
                (
                    point.measured,
                    point.footprint_bytes,
                    point.documented_bytes,
                )
            })
        },
    );
    findings.extend(example_output_findings());
    for finding in &findings {
        eprintln!("  {finding}");
    }
    let ok = findings.is_empty();
    eprintln!(
        "  checked {} pinned point(s) on both paths, the pinned set against the corpus, the device footprint against RFC 027, and the cluster-counted-work example's printed rows",
        PINNED.len()
    );
    eprintln!("[bench-baseline] {}", if ok { "PASS" } else { "FAIL" });
    ok
}

/// RFC 037 C1: runs `cluster-counted-work`, parses its printed rows, and checks each one
/// against the same pinned table. Closes the loop pinned table -> `bench` -> example: a
/// drift in either copy of the family now fails here, not only in a reader's eye.
fn example_output_findings() -> Vec<String> {
    let Some(output) = super::util::cargo_stdout(&[
        "run",
        "--locked",
        "--quiet",
        "--manifest-path",
        EXAMPLE_MANIFEST,
    ]) else {
        return vec!["BASELINE: cluster-counted-work did not run to a successful exit".to_owned()];
    };
    let rows = parse_example_rows(&output);
    if rows.is_empty() {
        return vec![
            "BASELINE: cluster-counted-work printed no parseable row; its output format may have changed"
                .to_owned(),
        ];
    }
    example_row_findings(&rows, PINNED)
}

/// One row of the example's table: `n`, `m`, `off`, the printed iteration count and cap
/// hits. Lines that do not parse as six whitespace-separated columns (the header, blank
/// lines, the closing prose) are skipped rather than treated as a malformed row.
fn parse_example_rows(output: &str) -> Vec<(Family, u32, u32)> {
    let mut rows = Vec::new();
    for line in output.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        let [n, m, off, iterations, cap_hits, _accepted] = fields.as_slice() else {
            continue;
        };
        let (Ok(n), Ok(m), Ok(off), Ok(iterations), Ok(cap_hits)) = (
            n.parse::<usize>(),
            m.parse::<usize>(),
            off.parse::<f64>(),
            iterations.parse::<u32>(),
            cap_hits.parse::<u32>(),
        ) else {
            continue;
        };
        rows.push((Family { n, m, off }, iterations, cap_hits));
    }
    rows
}

/// Pure core: each parsed row must match its pinned row's iteration count and cap hits.
/// A row the pinned table does not contain is itself a finding, fail-closed.
fn example_row_findings(rows: &[(Family, u32, u32)], pins: &[Pin]) -> Vec<String> {
    let mut findings = Vec::new();
    for (family, iterations, cap_hits) in rows {
        let Some(pin) = pins.iter().find(|p| same_point(p, *family)) else {
            findings.push(format!(
                "BASELINE: cluster-counted-work prints n={}, m={}, off={:.2}, which has no pinned row",
                family.n, family.m, family.off
            ));
            continue;
        };
        if *iterations != pin.outer_iterations {
            findings.push(format!(
                "BASELINE: n={}, m={}, off={:.2} (cluster-counted-work example): outer iterations expected {}, measured {}",
                family.n, family.m, family.off, pin.outer_iterations, iterations
            ));
        }
        if *cap_hits != pin.cap_hits {
            findings.push(format!(
                "BASELINE: n={}, m={}, off={:.2} (cluster-counted-work example): projection cap hits expected {}, measured {}",
                family.n, family.m, family.off, pin.cap_hits, cap_hits
            ));
        }
    }
    findings
}

/// Every finding for the baseline. Pure, with the measurements passed in, so the tests
/// can feed it fixed figures and so the gate reads no other state.
pub fn baseline_findings(
    pins: &[Pin],
    corpus: &[Family],
    cluster: impl Fn(Family) -> Result<Measured, String>,
    device: impl Fn(Family) -> Result<(Measured, usize, usize), String>,
) -> Vec<String> {
    let mut findings = Vec::new();

    // Fail closed on the pinned set against the corpus, in both directions: a corpus point
    // with no pin is unasserted, and a pin the corpus no longer produces is stale.
    for family in corpus {
        if !pins.iter().any(|pin| same_point(pin, *family)) {
            findings.push(format!(
                "BASELINE: corpus point n={}, m={}, off={:.2} has no pinned row",
                family.n, family.m, family.off
            ));
        }
    }
    for pin in pins {
        let family = Family {
            n: pin.n,
            m: pin.m,
            off: pin.off,
        };
        if !corpus.iter().any(|f| same_point(pin, *f)) {
            findings.push(format!(
                "BASELINE: pinned row n={}, m={}, off={:.2} is not a point of the corpus",
                family.n, family.m, family.off
            ));
        }
    }

    for pin in pins {
        let family = Family {
            n: pin.n,
            m: pin.m,
            off: pin.off,
        };
        let row = format!("n={}, m={}, off={:.2}", family.n, family.m, family.off);
        match cluster(family) {
            Ok(measured) => {
                compare(&mut findings, &row, "cluster", pin, &measured);
            }
            Err(error) => findings.push(format!("BASELINE: {row} (cluster): {error}")),
        }
        match device(family) {
            Ok((measured, footprint, documented)) => {
                compare(&mut findings, &row, "device", pin, &measured);
                if footprint != documented {
                    findings.push(format!(
                        "BASELINE: {row} (device): workspace footprint expected {documented} bytes (RFC 027's (3N + 2M)·8 + 16), measured {footprint}"
                    ));
                }
            }
            Err(error) => findings.push(format!("BASELINE: {row} (device): {error}")),
        }
    }
    findings
}

fn same_point(pin: &Pin, family: Family) -> bool {
    pin.n == family.n && pin.m == family.m && (pin.off - family.off).abs() < 1e-12
}

/// Names the row, the path, the expected value and the measured one, in the style
/// `published-metadata` uses.
fn compare(findings: &mut Vec<String>, row: &str, path: &str, pin: &Pin, measured: &Measured) {
    if measured.outer_iterations != pin.outer_iterations {
        findings.push(format!(
            "BASELINE: {row} ({path}): outer iterations expected {}, measured {}",
            pin.outer_iterations, measured.outer_iterations
        ));
    }
    if measured.cap_hits != pin.cap_hits {
        findings.push(format!(
            "BASELINE: {row} ({path}): projection cap hits expected {}, measured {}",
            pin.cap_hits, measured.cap_hits
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::{PINNED, Pin, baseline_findings};
    use crate::checks::bench::{Family, Measured, corpus};

    fn measured(outer_iterations: u32, cap_hits: u32) -> Measured {
        Measured {
            outer_iterations,
            status: "converged".to_owned(),
            termination: "Converged".to_owned(),
            cap_hits,
            violation: 0.0,
            infeasibility_evidence: false,
        }
    }

    fn corpus_points() -> Vec<Family> {
        corpus().into_iter().map(|(_, f)| f).collect()
    }

    /// The pinned value for a family, read back from `PINNED`, so a passing fake matches
    /// the table exactly.
    fn pinned_for(family: Family) -> Pin {
        *PINNED
            .iter()
            .find(|p| p.n == family.n && p.m == family.m && (p.off - family.off).abs() < 1e-12)
            .expect("corpus point is pinned")
    }

    #[test]
    fn the_pins_cover_the_corpus_exactly_and_there_are_eleven_of_them() {
        assert_eq!(PINNED.len(), 11);
        assert!(
            baseline_findings(
                PINNED,
                &corpus_points(),
                |f| Ok(measured(pinned_for(f).outer_iterations, 0)),
                |f| { Ok((measured(pinned_for(f).outer_iterations, 0), 0, 0)) }
            )
            .iter()
            .all(|finding| !finding.contains("no pinned row") && !finding.contains("not a point"))
        );
    }

    #[test]
    fn a_matching_measurement_passes_on_both_paths_when_footprint_matches_the_formula() {
        let findings = baseline_findings(
            PINNED,
            &corpus_points(),
            |f| {
                Ok(measured(
                    pinned_for(f).outer_iterations,
                    pinned_for(f).cap_hits,
                ))
            },
            |f| {
                let pin = pinned_for(f);
                let footprint = (3 * f.n + 2 * f.m) * 8 + 16;
                Ok((
                    measured(pin.outer_iterations, pin.cap_hits),
                    footprint,
                    footprint,
                ))
            },
        );
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn a_changed_count_names_the_row_the_path_the_expected_and_the_measured_value() {
        let target = Family {
            n: 32,
            m: 16,
            off: 0.50,
        };
        let findings = baseline_findings(
            PINNED,
            &corpus_points(),
            |f| {
                let pin = pinned_for(f);
                let iters = if f == target {
                    pin.outer_iterations + 1
                } else {
                    pin.outer_iterations
                };
                Ok(measured(iters, pin.cap_hits))
            },
            |f| {
                let pin = pinned_for(f);
                Ok((measured(pin.outer_iterations, pin.cap_hits), 0, 0))
            },
        );
        let expected = findings
            .iter()
            .find(|f| f.contains("n=32, m=16, off=0.50") && f.contains("(cluster)"))
            .expect("the moved row is named");
        assert!(expected.contains("expected 48, measured 49"), "{expected}");
    }

    #[test]
    fn a_footprint_that_departs_from_the_documented_formula_is_a_finding() {
        let findings = baseline_findings(
            PINNED,
            &corpus_points(),
            |f| Ok(measured(pinned_for(f).outer_iterations, 0)),
            |f| {
                let pin = pinned_for(f);
                Ok((measured(pin.outer_iterations, pin.cap_hits), 999, 1000))
            },
        );
        assert!(
            findings
                .iter()
                .any(|f| f.contains("workspace footprint expected 1000 bytes")
                    && f.contains("measured 999")),
            "{findings:?}"
        );
    }

    #[test]
    fn a_corpus_point_without_a_pin_fails_closed() {
        let mut points = corpus_points();
        points.push(Family {
            n: 2,
            m: 1,
            off: 0.5,
        });
        let findings = baseline_findings(
            PINNED,
            &points,
            |f| Ok(measured(pinned_for_or_zero(f), 0)),
            |f| Ok((measured(pinned_for_or_zero(f), 0), 0, 0)),
        );
        assert!(
            findings
                .iter()
                .any(|f| f.contains("n=2, m=1") && f.contains("no pinned row")),
            "{findings:?}"
        );
    }

    #[test]
    fn a_pin_the_corpus_no_longer_produces_is_stale() {
        let pins = [Pin {
            n: 3,
            m: 1,
            off: 0.5,
            outer_iterations: 1,
            cap_hits: 0,
        }];
        let findings = baseline_findings(
            &pins,
            &[],
            |_| Ok(measured(1, 0)),
            |_| Ok((measured(1, 0), 0, 0)),
        );
        assert!(
            findings
                .iter()
                .any(|f| f.contains("is not a point of the corpus")),
            "{findings:?}"
        );
    }

    #[test]
    fn a_cap_hit_change_is_named_separately_from_an_iteration_change() {
        let target = Family {
            n: 16,
            m: 8,
            off: 0.50,
        };
        let findings = baseline_findings(
            PINNED,
            &corpus_points(),
            |f| {
                let pin = pinned_for(f);
                let caps = if f == target { 2 } else { pin.cap_hits };
                Ok(measured(pin.outer_iterations, caps))
            },
            |f| {
                let pin = pinned_for(f);
                Ok((measured(pin.outer_iterations, pin.cap_hits), 0, 0))
            },
        );
        assert!(
            findings
                .iter()
                .any(|f| f.contains("projection cap hits expected 0, measured 2")),
            "{findings:?}"
        );
    }

    fn pinned_for_or_zero(family: Family) -> u32 {
        PINNED
            .iter()
            .find(|p| p.n == family.n && p.m == family.m && (p.off - family.off).abs() < 1e-12)
            .map_or(0, |p| p.outer_iterations)
    }

    #[test]
    fn parse_example_rows_reads_the_data_rows_and_skips_the_header_and_prose() {
        let output = "     n      m    off   iterations   cap hits   accepted\n\
                       \x20    4      2   0.50           30          0        yes\n\
                       \x20   32     16   0.99          989          0        yes\n\
                       \n\
                       A result is used only when accepted = yes.\n";
        let rows = super::parse_example_rows(output);
        assert_eq!(
            rows,
            vec![
                (
                    super::Family {
                        n: 4,
                        m: 2,
                        off: 0.50
                    },
                    30,
                    0
                ),
                (
                    super::Family {
                        n: 32,
                        m: 16,
                        off: 0.99
                    },
                    989,
                    0
                ),
            ]
        );
    }

    #[test]
    fn a_row_matching_its_pin_passes() {
        let rows = vec![(
            super::Family {
                n: 4,
                m: 2,
                off: 0.50,
            },
            30,
            0,
        )];
        assert!(super::example_row_findings(&rows, PINNED).is_empty());
    }

    #[test]
    fn a_row_disagreeing_with_its_pin_names_the_row_and_both_values() {
        let rows = vec![(
            super::Family {
                n: 4,
                m: 2,
                off: 0.50,
            },
            31,
            0,
        )];
        let findings = super::example_row_findings(&rows, PINNED);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(
            findings[0].contains("cluster-counted-work example"),
            "{findings:?}"
        );
        assert!(
            findings[0].contains("expected 30, measured 31"),
            "{findings:?}"
        );
    }

    #[test]
    fn a_row_with_no_pinned_point_fails_closed() {
        let rows = vec![(
            super::Family {
                n: 2,
                m: 1,
                off: 0.5,
            },
            1,
            0,
        )];
        let findings = super::example_row_findings(&rows, PINNED);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].contains("no pinned row"), "{findings:?}");
    }
}
