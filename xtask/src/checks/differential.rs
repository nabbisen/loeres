//! `differential` — RFC 030 randomized-differential-test coverage gate.
//!
//! Every numerical solve kernel must carry a randomized differential test
//! against an independently constructed exact reference, or an explicit,
//! reasoned exemption. This gate asserts that the discipline is *present*:
//!
//! - **Coverage symmetry** (RFC 022's pattern). Every discovered kernel
//!   entrypoint has exactly one registry row, and every row names a discovered
//!   entrypoint. The two directions are reported separately because they have
//!   different causes: "in code, not in registry" is a new entrypoint nobody
//!   registered; "in registry, not in code" is a row that went stale.
//! - **The named test exists** in the named file, as a `#[test]` function.
//! - **Reasons are not blank**: an `Exempt` row needs a reason, and a
//!   `Differential` row needs the mutation it detects.
//!
//! It deliberately does **not** judge whether a reference is independent of the
//! kernel's own iteration, or whether the named mutation was really run. A gate
//! that claimed to decide that would be a gate nobody could trust; the registry's
//! `detects` field and architect review are the control (RFC 030 §8).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use super::util::collect_ext;

/// The two kernel crates whose `src/` the discovery scan covers.
const SCANNED_ROOTS: &[&str] = &["crates/loeres-device/src", "crates/loeres-cluster/src"];

/// One solve entrypoint and how it is covered.
struct Kernel {
    /// Repository-relative path of the module that defines the entrypoint.
    source: &'static str,
    /// The public entrypoint's name.
    entrypoint: &'static str,
    coverage: Coverage,
}

enum Coverage {
    /// A randomized differential test meeting RFC 030 §4.2.
    Differential {
        /// Repository-relative path of the file containing the test.
        tests: &'static str,
        /// The `#[test]` function's name.
        test: &'static str,
        /// A mutation the author verified this test detects.
        detects: &'static str,
    },
    /// Not a numerical kernel. The reason is mandatory and non-empty.
    Exempt { reason: &'static str },
}

const KERNELS: &[Kernel] = &[
    Kernel {
        source: "crates/loeres-device/src/solve.rs",
        entrypoint: "solve_projected_first_order",
        coverage: Coverage::Differential {
            tests: "crates/loeres-device/src/solve/tests.rs",
            test: "random_separable_quadratics_match_the_exact_box_minimiser",
            detects: "per-coordinate bound indexing (clamp using coordinate 0's bounds)",
        },
    },
    Kernel {
        source: "crates/loeres-device/src/solve/constrained.rs",
        entrypoint: "solve_constrained_projected_first_order",
        coverage: Coverage::Differential {
            tests: "crates/loeres-device/src/solve/constrained/tests.rs",
            test: "random_feasible_two_constraint_polytopes_match_the_exact_projection",
            detects: "per-sweep multiplier reset (RFC 027 §0.3.2)",
        },
    },
    Kernel {
        source: "crates/loeres-cluster/src/solve/projected_first_order.rs",
        entrypoint: "solve_projected_first_order_dyn",
        coverage: Coverage::Exempt {
            reason: "retrofit pending, RFC 030 S2/S3",
        },
    },
    Kernel {
        source: "crates/loeres-cluster/src/solve/constrained.rs",
        entrypoint: "solve_constrained_projected_first_order_dyn",
        coverage: Coverage::Differential {
            tests: "crates/loeres-cluster/src/solve/constrained/tests.rs",
            test: "random_feasible_polytopes_n3_m3_match_the_exact_projection",
            detects: "per-sweep multiplier reset (RFC 027 §0.3.2)",
        },
    },
    Kernel {
        source: "crates/loeres-cluster/src/solve/projected_first_order.rs",
        entrypoint: "solve_projected_first_order_dyn_cached",
        coverage: Coverage::Exempt {
            reason: "RFC 017 caching wrapper; delegates the numerical work to \
                     `solve_projected_first_order_dyn`",
        },
    },
    Kernel {
        source: "crates/loeres-cluster/src/solve.rs",
        entrypoint: "solve_batch",
        coverage: Coverage::Exempt {
            reason: "batch orchestration; performs no numerical iteration",
        },
    },
    Kernel {
        source: "crates/loeres-cluster/src/solve.rs",
        entrypoint: "solve_batch_async",
        coverage: Coverage::Exempt {
            reason: "batch orchestration (Tokio offload of `solve_batch`); performs no \
                     numerical iteration",
        },
    },
    Kernel {
        source: "crates/loeres-cluster/src/observe.rs",
        entrypoint: "solve_batch_observed",
        coverage: Coverage::Exempt {
            reason: "batch orchestration; performs no numerical iteration",
        },
    },
];

pub fn run() -> bool {
    eprintln!("[differential] RFC 030 randomized differential-test coverage");
    let discovered = discover_tree();
    let mut ok = true;

    match &discovered {
        Ok(_) => {}
        Err(error) => {
            eprintln!("  DISCOVERY: {error}");
            ok = false;
        }
    }
    let discovered = discovered.unwrap_or_default();

    let findings = findings(KERNELS, &discovered, |path| {
        std::fs::read_to_string(path).ok()
    });
    for finding in &findings {
        eprintln!("  {finding}");
    }
    ok &= findings.is_empty();

    for kernel in KERNELS {
        eprintln!("  {}", summary_line(kernel));
    }

    eprintln!("[differential] {}", if ok { "PASS" } else { "FAIL" });
    ok
}

fn summary_line(kernel: &Kernel) -> String {
    match &kernel.coverage {
        Coverage::Differential {
            tests,
            test,
            detects,
        } => format!(
            "{}: differential ({tests}::{test}); detects {detects}",
            kernel.entrypoint
        ),
        Coverage::Exempt { reason } => format!("{}: exempt ({reason})", kernel.entrypoint),
    }
}

/// A discovered entrypoint: `(repository-relative source path, name)`.
type Entrypoint = (String, String);

/// Every solve entrypoint in the scanned crates, read from disk.
fn discover_tree() -> Result<BTreeSet<Entrypoint>, String> {
    let mut discovered = BTreeSet::new();
    for root in SCANNED_ROOTS {
        if !Path::new(root).is_dir() {
            return Err(format!(
                "{root} does not exist; an absent kernel crate fails rather than passing vacuously"
            ));
        }
        let mut files: Vec<PathBuf> = Vec::new();
        collect_ext(Path::new(root), "rs", &mut files);
        files.sort();
        for file in files {
            let path = file.to_string_lossy().replace('\\', "/");
            if is_test_file(&path) {
                continue;
            }
            let source = std::fs::read_to_string(&file)
                .map_err(|error| format!("cannot read {path}: {error}"))?;
            for name in discover_entrypoints(&source) {
                discovered.insert((path.clone(), name));
            }
        }
    }
    Ok(discovered)
}

/// Test modules define helpers named `solve_*`; they are not entrypoints.
fn is_test_file(path: &str) -> bool {
    path.ends_with("/tests.rs") || path.contains("/tests/")
}

/// `pub fn solve_*` (or `pub async fn solve_*`) at module scope.
///
/// Module scope means column 0, or column 4 inside a `mod` block — the RFC 006
/// device entrypoint lives in `mod owned`, and a scan that misses it is wrong.
/// A method inside an `impl` block is not an entrypoint. The scan relies on
/// rustfmt layout (block openers and closers at column 0), which the tree's
/// `cargo fmt --check` gate already guarantees.
fn discover_entrypoints(source: &str) -> Vec<String> {
    #[derive(PartialEq)]
    enum Block {
        None,
        Module,
        Other,
    }
    let mut block = Block::None;
    let mut names = Vec::new();
    for line in source.lines() {
        let indent = line.len() - line.trim_start().len();
        if indent == 0 {
            let opener = strip_visibility(line);
            if line.starts_with('}') {
                block = Block::None;
            } else if opener.starts_with("mod ") && line.trim_end().ends_with('{') {
                block = Block::Module;
            } else if line.trim_end().ends_with('{') && !line.trim_start().starts_with("pub fn") {
                block = Block::Other;
            }
        }
        let in_scope = indent == 0 || (indent == 4 && block == Block::Module);
        if !in_scope {
            continue;
        }
        let rest = line.trim_start();
        let rest = rest
            .strip_prefix("pub fn ")
            .or_else(|| rest.strip_prefix("pub async fn "));
        let Some(rest) = rest.filter(|rest| rest.starts_with("solve_")) else {
            continue;
        };
        let name: String = rest
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        names.push(name);
    }
    names
}

fn strip_visibility(line: &str) -> &str {
    let line = line.trim_start();
    let line = line.strip_prefix("pub(crate) ").unwrap_or(line);
    line.strip_prefix("pub ").unwrap_or(line)
}

/// Every problem with the registry against what was discovered. Empty is a pass.
fn findings<F>(kernels: &[Kernel], discovered: &BTreeSet<Entrypoint>, read: F) -> Vec<String>
where
    F: Fn(&str) -> Option<String>,
{
    let mut findings = Vec::new();

    let mut registered: BTreeSet<Entrypoint> = BTreeSet::new();
    for kernel in kernels {
        let key = (kernel.source.to_owned(), kernel.entrypoint.to_owned());
        if !registered.insert(key) {
            findings.push(format!(
                "DUPLICATE ROW: {} ({}) has more than one registry row",
                kernel.entrypoint, kernel.source
            ));
        }
    }

    for (source, name) in discovered.difference(&registered) {
        findings.push(format!(
            "IN CODE, NOT IN REGISTRY: {name} ({source}) has no row; add a Differential or a reasoned Exempt"
        ));
    }
    for (source, name) in registered.difference(discovered) {
        findings.push(format!(
            "IN REGISTRY, NOT IN CODE: {name} ({source}) is not a discovered entrypoint; remove or correct the row"
        ));
    }

    for kernel in kernels {
        match &kernel.coverage {
            Coverage::Exempt { reason } => {
                if reason.trim().is_empty() {
                    findings.push(format!(
                        "EMPTY REASON: {} is exempt without a reason",
                        kernel.entrypoint
                    ));
                }
            }
            Coverage::Differential {
                tests,
                test,
                detects,
            } => {
                if detects.trim().is_empty() {
                    findings.push(format!(
                        "EMPTY DETECTS: {} names no mutation its test detects",
                        kernel.entrypoint
                    ));
                }
                match read(tests) {
                    None => findings.push(format!(
                        "MISSING TEST FILE: {tests} (for {}) cannot be read",
                        kernel.entrypoint
                    )),
                    Some(contents) if !defines_test(&contents, test) => {
                        findings.push(format!(
                            "MISSING TEST: {tests} defines no `#[test] fn {test}(` (for {})",
                            kernel.entrypoint
                        ));
                    }
                    Some(_) => {}
                }
            }
        }
    }

    findings
}

/// Whether `source` defines `fn <test>(` carrying a `#[test]` attribute.
///
/// A same-named helper without the attribute does not count: the registry claims
/// a *test*, and a function nobody runs would satisfy the gate vacuously.
fn defines_test(source: &str, test: &str) -> bool {
    let needle = format!("fn {test}(");
    let lines: Vec<&str> = source.lines().collect();
    lines.iter().enumerate().any(|(i, line)| {
        let trimmed = line.trim_start();
        let declares = trimmed.starts_with(&needle)
            || trimmed
                .strip_prefix("pub ")
                .is_some_and(|rest| rest.starts_with(&needle));
        declares
            && lines[..i]
                .iter()
                .rev()
                .map(|l| l.trim())
                .take_while(|l| l.starts_with("#[") || l.starts_with("//"))
                .any(|l| l == "#[test]")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(entries: &[(&str, &str)]) -> BTreeSet<Entrypoint> {
        entries
            .iter()
            .map(|(s, n)| ((*s).to_owned(), (*n).to_owned()))
            .collect()
    }

    fn exempt(source: &'static str, entrypoint: &'static str, reason: &'static str) -> Kernel {
        Kernel {
            source,
            entrypoint,
            coverage: Coverage::Exempt { reason },
        }
    }

    fn differential(
        source: &'static str,
        entrypoint: &'static str,
        test: &'static str,
        detects: &'static str,
    ) -> Kernel {
        Kernel {
            source,
            entrypoint,
            coverage: Coverage::Differential {
                tests: "t.rs",
                test,
                detects,
            },
        }
    }

    fn no_files(_: &str) -> Option<String> {
        None
    }

    #[test]
    fn discovery_finds_a_module_scope_function_and_one_inside_a_mod_block() {
        let source = "\
pub fn solve_top<P>(p: P) {}

#[cfg(feature = \"owned-arrays\")]
mod owned {
    use x;

    pub fn solve_inside<P>(p: P) {}
    fn solve_private() {}
}

pub async fn solve_async() {}
";
        assert_eq!(
            discover_entrypoints(source),
            vec!["solve_top", "solve_inside", "solve_async"]
        );
    }

    #[test]
    fn discovery_ignores_methods_inside_an_impl_block_and_non_solve_names() {
        let source = "\
pub struct Job;

impl Job {
    pub fn solve_method(&self) {}
}

pub fn other_function() {}
fn solve_private() {}
";
        assert!(discover_entrypoints(source).is_empty());
    }

    #[test]
    fn discovery_resumes_after_an_impl_block_closes() {
        let source = "\
impl Job {
    pub fn solve_method(&self) {}
}

pub fn solve_after() {}
";
        assert_eq!(discover_entrypoints(source), vec!["solve_after"]);
    }

    #[test]
    fn test_modules_are_not_scanned() {
        assert!(is_test_file("crates/x/src/solve/tests.rs"));
        assert!(is_test_file("crates/x/src/solve/tests/fixtures.rs"));
        assert!(!is_test_file("crates/x/src/solve.rs"));
    }

    #[test]
    fn a_fully_registered_tree_has_no_findings() {
        let kernels = [exempt("a.rs", "solve_a", "orchestration")];
        assert!(findings(&kernels, &set(&[("a.rs", "solve_a")]), no_files).is_empty());
    }

    #[test]
    fn an_entrypoint_in_code_but_not_in_the_registry_is_reported_on_its_own() {
        let kernels = [exempt("a.rs", "solve_a", "orchestration")];
        let found = findings(
            &kernels,
            &set(&[("a.rs", "solve_a"), ("b.rs", "solve_new")]),
            no_files,
        );
        assert_eq!(found.len(), 1);
        assert!(
            found[0].starts_with("IN CODE, NOT IN REGISTRY"),
            "{found:?}"
        );
        assert!(found[0].contains("solve_new"));
    }

    #[test]
    fn a_row_naming_no_real_entrypoint_is_reported_on_its_own() {
        let kernels = [
            exempt("a.rs", "solve_a", "orchestration"),
            exempt("b.rs", "solve_gone", "orchestration"),
        ];
        let found = findings(&kernels, &set(&[("a.rs", "solve_a")]), no_files);
        assert_eq!(found.len(), 1);
        assert!(
            found[0].starts_with("IN REGISTRY, NOT IN CODE"),
            "{found:?}"
        );
        assert!(found[0].contains("solve_gone"));
    }

    #[test]
    fn a_duplicate_row_is_reported() {
        let kernels = [
            exempt("a.rs", "solve_a", "one"),
            exempt("a.rs", "solve_a", "two"),
        ];
        let found = findings(&kernels, &set(&[("a.rs", "solve_a")]), no_files);
        assert!(
            found.iter().any(|f| f.starts_with("DUPLICATE ROW")),
            "{found:?}"
        );
    }

    #[test]
    fn an_exemption_without_a_reason_fails() {
        for reason in ["", "   "] {
            let kernels = [exempt("a.rs", "solve_a", reason)];
            let found = findings(&kernels, &set(&[("a.rs", "solve_a")]), no_files);
            assert_eq!(found.len(), 1, "{found:?}");
            assert!(found[0].starts_with("EMPTY REASON"));
        }
    }

    #[test]
    fn a_differential_row_without_a_detected_mutation_fails() {
        let kernels = [differential("a.rs", "solve_a", "t_fn", " ")];
        let found = findings(&kernels, &set(&[("a.rs", "solve_a")]), |_| {
            Some("#[test]\nfn t_fn() {}".to_owned())
        });
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].starts_with("EMPTY DETECTS"));
    }

    #[test]
    fn a_missing_test_file_and_a_missing_test_are_distinct_failures() {
        let kernels = [differential("a.rs", "solve_a", "t_fn", "a mutation")];
        let discovered = set(&[("a.rs", "solve_a")]);

        let no_file = findings(&kernels, &discovered, no_files);
        assert!(no_file[0].starts_with("MISSING TEST FILE"), "{no_file:?}");

        let no_test = findings(&kernels, &discovered, |_| Some("fn other() {}".to_owned()));
        assert!(no_test[0].starts_with("MISSING TEST:"), "{no_test:?}");
    }

    #[test]
    fn a_same_named_function_without_the_test_attribute_does_not_count() {
        assert!(defines_test("#[test]\nfn t_fn() {}", "t_fn"));
        assert!(defines_test("#[test]\n#[cfg(x)]\nfn t_fn() {}", "t_fn"));
        assert!(!defines_test("fn t_fn() {}", "t_fn"));
        assert!(!defines_test(
            "#[test]\nfn other() {}\nfn t_fn() {}",
            "t_fn"
        ));
        // A prefix of a longer name is not the test.
        assert!(!defines_test("#[test]\nfn t_fn_longer() {}", "t_fn"));
    }

    /// Against the real tree, not a fixture: a registry checked only against
    /// invented sources would prove the parser rather than the repository.
    #[test]
    fn the_real_registry_matches_the_real_tree() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let discovered = {
            let mut found = BTreeSet::new();
            for scanned in SCANNED_ROOTS {
                let mut files = Vec::new();
                collect_ext(&root.join(scanned), "rs", &mut files);
                for file in files {
                    let relative = file
                        .strip_prefix(&root)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/");
                    if is_test_file(&relative) {
                        continue;
                    }
                    for name in discover_entrypoints(&std::fs::read_to_string(&file).unwrap()) {
                        found.insert((relative.clone(), name));
                    }
                }
            }
            found
        };
        assert!(
            discovered
                .iter()
                .any(|(_, name)| name == "solve_projected_first_order"),
            "the RFC 006 entrypoint (inside `mod owned`) was missed: {discovered:?}"
        );
        let problems = findings(KERNELS, &discovered, |path| {
            std::fs::read_to_string(root.join(path)).ok()
        });
        assert!(problems.is_empty(), "{problems:#?}");
    }
}
