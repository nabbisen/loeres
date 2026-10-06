//! `examples` — RFC 023 per-example build and dependency-isolation gate.
//!
//! The examples under `examples/` are deliberately **not** workspace members
//! (RFC 023 §11.1). As members they would share one lockfile and participate in
//! feature unification, so the device example could resolve a server-side crate
//! through a sibling and still compile cleanly — the isolation claim would be an
//! artifact of the build graph rather than a property of the example. Excluded,
//! each example's resolved graph is its own evidence, and this gate reads it.
//!
//! Two assertions per example: it builds under its declared feature set, and its
//! **resolved** dependency graph contains no forbidden crate. Resolved, not
//! declared: a manifest lists direct dependencies, while the resolve shows what
//! is actually reachable — transitively and through feature activation. The scan
//! covers the whole resolved set, not a short list of names already believed
//! absent.
//!
//! What this proves is **dependency reachability**. It does *not* prove
//! bare-metal buildability — that is `no-std`'s claim, against
//! `thumbv7em-none-eabihf` — and the two must not be conflated. An example is a
//! host program and its own `main` may use `std`.
//!
//! RFC 035 §3.4 adds two more: each example **runs** (compilation proves a
//! signature; running proves the program works), and every output block quoted
//! in the book is **checked**. A block is marked by an
//! `<!-- example-output: NAME -->` line immediately before its fence. The block
//! must be a contiguous run of that example's own stdout, byte-for-byte, and must
//! not elide anything with `...`. A block that cannot be checked must not be
//! presented as captured output.
//!
//! An absent example directory **fails**. A gate that silently passes when its
//! subject is missing proves nothing, which is the same fail-closed reasoning
//! RFC 022's coverage symmetry applies to a missing review.

use std::fs;
use std::path::Path;

use super::util::{cargo, cargo_stdout, collect_ext};

/// One example, its directory, and the crates its resolved graph must not carry.
struct Example {
    name: &'static str,
    dir: &'static str,
    /// Forbidden crate names. Empty for the cluster example: it is the
    /// server-side path, so no crate in the workspace is out of bounds for it.
    forbidden: &'static [&'static str],
}

/// The device forbidden set is external design §1.1's, verbatim.
const EXAMPLES: &[Example] = &[
    Example {
        name: "cluster-batch-solve",
        dir: "examples/cluster-batch-solve",
        forbidden: &[],
    },
    Example {
        name: "cluster-qp-constrained",
        dir: "examples/cluster-qp-constrained",
        forbidden: &[],
    },
    Example {
        name: "device-box-pfo",
        dir: "examples/device-box-pfo",
        forbidden: &[
            "loeres-cluster",
            "loeres-backend-std",
            "tokio",
            "rayon",
            "tracing",
        ],
    },
    Example {
        name: "cluster-capacity-dispatch",
        dir: "examples/cluster-capacity-dispatch",
        forbidden: &[],
    },
    Example {
        name: "device-mpc-step",
        dir: "examples/device-mpc-step",
        forbidden: &[
            "loeres-cluster",
            "loeres-backend-std",
            "tokio",
            "rayon",
            "tracing",
        ],
    },
];

pub fn run() -> bool {
    eprintln!("[examples] RFC 023 example build and dependency isolation");
    let mut ok = true;
    let mut outputs: Vec<(&str, Option<String>)> = Vec::new();

    for example in EXAMPLES {
        let missing = missing_inputs(example.dir, |path| Path::new(path).exists());
        if !missing.is_empty() {
            for path in missing {
                eprintln!(
                    "  MISSING EXAMPLE INPUT: {path} does not exist; an absent example fails rather than passing vacuously"
                );
            }
            ok = false;
            continue;
        }

        let manifest = format!("{}/Cargo.toml", example.dir);
        // `--locked` binds the example's own lockfile: a stale lockfile is a
        // failure here rather than a silent update, which keeps the excluded
        // crates' lockfiles current (RFC 023 §8).
        let build_ok = cargo(&["build", "--locked", "--manifest-path", &manifest]);
        // RFC 035 §3.4: an example runs, not only compiles. A run that does not
        // exit successfully is a finding; its stdout is kept for the book check.
        let output = if build_ok {
            cargo_stdout(&["run", "--locked", "--quiet", "--manifest-path", &manifest])
        } else {
            None
        };
        if build_ok && output.is_none() {
            eprintln!(
                "  EXAMPLE RUN: {} built but did not run to a successful exit",
                example.name
            );
            ok = false;
        }
        if let Some(text) = &output {
            eprintln!(
                "  {}: runs to a successful exit; {} line(s) of output",
                example.name,
                text.lines().count()
            );
        }
        outputs.push((example.name, output));
        let resolved = resolved_packages(&manifest);

        if let Some(ref names) = resolved {
            eprintln!(
                "  {}: {} resolved package(s): {}",
                example.name,
                names.len(),
                names.join(", ")
            );
        }

        for finding in example_findings(
            example.name,
            build_ok,
            resolved.as_deref(),
            example.forbidden,
        ) {
            eprintln!("  {finding}");
            ok = false;
        }

        if build_ok && example.forbidden.is_empty() {
            eprintln!("  {}: builds; no forbidden crate set applies", example.name);
        } else if build_ok {
            eprintln!(
                "  {}: builds; none of {} reaches the resolved graph",
                example.name,
                example.forbidden.join(", ")
            );
        }
    }

    for finding in captured_output_check(&outputs) {
        eprintln!("  {finding}");
        ok = false;
    }

    eprintln!("[examples] {}", if ok { "PASS" } else { "FAIL" });
    ok
}

/// Every marked output block in the book, checked against its example's run.
fn captured_output_check(outputs: &[(&str, Option<String>)]) -> Vec<String> {
    let mut docs = Vec::new();
    collect_ext(Path::new("docs/src"), "md", &mut docs);
    docs.sort();
    let mut blocks = Vec::new();
    let mut findings = Vec::new();
    for doc in docs {
        let label = doc.display().to_string();
        match fs::read_to_string(&doc) {
            Ok(source) => match captured_blocks(&source) {
                Ok(found) => blocks.extend(found.into_iter().map(|b| (label.clone(), b))),
                Err(error) => findings.push(format!("CAPTURED MALFORMED: {label}: {error}")),
            },
            Err(error) => findings.push(format!("CAPTURED UNREADABLE: {label}: {error}")),
        }
    }
    findings.extend(captured_output_findings(&blocks, outputs));
    findings
}

/// The output blocks the book quotes. Each must be marked once in the book, so a
/// quoted example cannot silently lose its check.
const CAPTURED_EXAMPLES: &[&str] = &[
    "device-mpc-step",
    "device-box-pfo",
    "cluster-batch-solve",
    "cluster-qp-constrained",
];

const CAPTURE_MARKER_PREFIX: &str = "<!-- example-output: ";
const CAPTURE_MARKER_SUFFIX: &str = " -->";

/// One fenced output block that a book page marks as captured from an example.
#[derive(Debug, PartialEq, Eq)]
struct CapturedBlock {
    example: String,
    /// 1-indexed line of the marker, for the finding text.
    line: usize,
    lines: Vec<String>,
}

/// Parses every `<!-- example-output: NAME -->` marker and the fenced block that
/// immediately follows it. A marker with no fence after it is an error, not a
/// silently skipped block.
fn captured_blocks(source: &str) -> Result<Vec<CapturedBlock>, String> {
    let mut blocks = Vec::new();
    let mut lines = source.lines().enumerate();
    while let Some((index, line)) = lines.next() {
        let trimmed = line.trim();
        let Some(name) = trimmed
            .strip_prefix(CAPTURE_MARKER_PREFIX)
            .and_then(|rest| rest.strip_suffix(CAPTURE_MARKER_SUFFIX))
        else {
            continue;
        };
        let marker_line = index + 1;
        let open = lines.next().map(|(_, l)| l.trim());
        if open != Some("```text") && open != Some("```") {
            return Err(format!(
                "line {marker_line}: marker `{}` is not followed by a fenced block",
                name.trim()
            ));
        }
        let mut body = Vec::new();
        let mut closed = false;
        for (_, l) in lines.by_ref() {
            if l.trim() == "```" {
                closed = true;
                break;
            }
            body.push(l.to_owned());
        }
        if !closed {
            return Err(format!("line {marker_line}: the fenced block never closes"));
        }
        blocks.push(CapturedBlock {
            example: name.trim().to_owned(),
            line: marker_line,
            lines: body,
        });
    }
    Ok(blocks)
}

/// Findings for the captured blocks: each must name a registered example, be
/// marked at least once, be free of elision, and be a contiguous run of that
/// example's own stdout.
fn captured_output_findings(
    blocks: &[(String, CapturedBlock)],
    outputs: &[(&str, Option<String>)],
) -> Vec<String> {
    let mut findings = Vec::new();
    for required in CAPTURED_EXAMPLES {
        if !blocks.iter().any(|(_, b)| b.example == *required) {
            findings.push(format!(
                "CAPTURED MISSING: no marked output block in the book for {required}"
            ));
        }
    }
    for (doc, block) in blocks {
        let at = format!("{doc}:{}", block.line);
        let Some((_, output)) = outputs.iter().find(|(name, _)| *name == block.example) else {
            findings.push(format!(
                "CAPTURED UNKNOWN: {at} marks `{}`, which is not a registered example",
                block.example
            ));
            continue;
        };
        if block.lines.is_empty() {
            findings.push(format!("CAPTURED EMPTY: {at} marks an empty block"));
            continue;
        }
        if block.lines.iter().any(|l| l.contains("...")) {
            findings.push(format!(
                "CAPTURED ELIDED: {at} contains `...`; quote the complete lines or describe the output in prose"
            ));
            continue;
        }
        let Some(output) = output else {
            findings.push(format!(
                "CAPTURED UNCHECKABLE: {at} marks `{}`, which did not run",
                block.example
            ));
            continue;
        };
        let produced: Vec<&str> = output.lines().collect();
        if !contains_run(&produced, &block.lines) {
            findings.push(format!(
                "CAPTURED MISMATCH: {at} is not a contiguous run of the output of `{}`",
                block.example
            ));
        }
    }
    findings
}

/// Whether `needle` appears as a contiguous run of lines in `haystack`.
fn contains_run(haystack: &[&str], needle: &[String]) -> bool {
    !needle.is_empty()
        && haystack.len() >= needle.len()
        && haystack
            .windows(needle.len())
            .any(|window| window.iter().zip(needle).all(|(a, b)| *a == b))
}

/// The inputs an example must have before it can be checked at all.
fn missing_inputs<F>(dir: &str, mut exists: F) -> Vec<String>
where
    F: FnMut(&str) -> bool,
{
    [
        dir.to_owned(),
        format!("{dir}/Cargo.toml"),
        format!("{dir}/Cargo.lock"),
        format!("{dir}/src/main.rs"),
    ]
    .into_iter()
    .filter(|path| !exists(path))
    .collect()
}

/// Every finding for one example: a failed build, an unreadable resolve, or a
/// forbidden crate reachable from it.
fn example_findings(
    name: &str,
    build_ok: bool,
    resolved: Option<&[String]>,
    forbidden: &[&str],
) -> Vec<String> {
    let mut findings = Vec::new();
    if !build_ok {
        findings.push(format!(
            "EXAMPLE BUILD: {name} failed to build under its declared feature set"
        ));
    }
    match resolved {
        None => findings.push(format!(
            "EXAMPLE RESOLVE: cannot read the resolved dependency graph for {name}; \
             its lockfile may be stale (the scan runs `--locked`)"
        )),
        Some(names) => {
            for crate_name in forbidden_present(names, forbidden) {
                findings.push(format!(
                    "EXAMPLE ISOLATION: {name} reaches forbidden crate `{crate_name}` in its resolved graph"
                ));
            }
        }
    }
    findings
}

/// The resolved package set for one example, read from its own lockfile.
///
/// `--edges normal,build,dev` is wider than the runtime graph on purpose: a
/// forbidden crate arriving as a build-script or dev dependency is still
/// reachable from the example's resolve, and a check that ignored those kinds
/// would be narrower than the claim it supports.
fn resolved_packages(manifest: &str) -> Option<Vec<String>> {
    let tree = cargo_stdout(&[
        "tree",
        "--locked",
        "--manifest-path",
        manifest,
        "--edges",
        "normal,build,dev",
        "--prefix",
        "none",
    ])?;
    Some(resolved_package_names(&tree))
}

/// Package names from `cargo tree --prefix none` output, deduplicated.
///
/// Each line is `name vX.Y.Z` for a registry crate or `name vX.Y.Z (/path)` for
/// a path crate, with `(*)` marking a repeat of an already-printed subtree.
fn resolved_package_names(tree: &str) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for line in tree.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Some(name) = line.split_whitespace().next() else {
            continue;
        };
        if !names.iter().any(|existing| existing == name) {
            names.push(name.to_owned());
        }
    }
    names.sort();
    names
}

/// Forbidden crates actually present in a resolved set. Compares the whole set
/// against the whole forbidden list — never a spot check of names already
/// believed absent (RFC 023 §7).
fn forbidden_present(resolved: &[String], forbidden: &[&str]) -> Vec<String> {
    forbidden
        .iter()
        .filter(|name| resolved.iter().any(|resolved| resolved == *name))
        .map(|name| (*name).to_owned())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        EXAMPLES, example_findings, forbidden_present, missing_inputs, resolved_package_names,
        resolved_packages,
    };

    const DEVICE_FORBIDDEN: &[&str] = &[
        "loeres-cluster",
        "loeres-backend-std",
        "tokio",
        "rayon",
        "tracing",
    ];

    fn names(values: &[&str]) -> Vec<String> {
        values.iter().map(|v| (*v).to_owned()).collect()
    }

    #[test]
    fn resolved_names_drop_versions_paths_and_repeat_markers() {
        let tree = "device-box-pfo v0.0.0 (/repo/examples/device-box-pfo)\n\
                    loeres-device v0.20.3 (/repo/crates/loeres-device)\n\
                    loeres v0.20.3 (/repo/crates/loeres)\n\
                    loeres-backend-static v0.20.3 (/repo/crates/loeres-backend-static)\n\
                    loeres v0.20.3 (/repo/crates/loeres) (*)\n\
                    \n";
        assert_eq!(
            resolved_package_names(tree),
            names(&[
                "device-box-pfo",
                "loeres",
                "loeres-backend-static",
                "loeres-device"
            ])
        );
    }

    #[test]
    fn an_example_gaining_a_forbidden_dependency_fails_the_gate() {
        // The device graph with `rayon` in it — what a `parallel-rayon`
        // activation leaking through a sibling would look like.
        let resolved = names(&["device-box-pfo", "loeres", "loeres-device", "rayon"]);
        assert_eq!(
            forbidden_present(&resolved, DEVICE_FORBIDDEN),
            vec!["rayon"]
        );

        let findings = example_findings("device-box-pfo", true, Some(&resolved), DEVICE_FORBIDDEN);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].contains("EXAMPLE ISOLATION"), "{findings:?}");
        assert!(findings[0].contains("rayon"), "{findings:?}");
    }

    #[test]
    fn every_forbidden_crate_is_reported_not_just_the_first() {
        let resolved = names(&["device-box-pfo", "loeres-backend-std", "rayon", "tokio"]);
        assert_eq!(
            forbidden_present(&resolved, DEVICE_FORBIDDEN),
            names(&["loeres-backend-std", "tokio", "rayon"])
        );
    }

    #[test]
    fn an_example_that_fails_to_build_fails_the_gate() {
        let resolved = names(&["device-box-pfo", "loeres", "loeres-device"]);
        let findings = example_findings("device-box-pfo", false, Some(&resolved), DEVICE_FORBIDDEN);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].contains("EXAMPLE BUILD"), "{findings:?}");
    }

    #[test]
    fn an_unreadable_resolve_fails_rather_than_passing_with_no_forbidden_crate_found() {
        let findings = example_findings("device-box-pfo", true, None, DEVICE_FORBIDDEN);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].contains("EXAMPLE RESOLVE"), "{findings:?}");
    }

    #[test]
    fn an_absent_example_directory_fails_rather_than_passing_vacuously() {
        let missing = missing_inputs("examples/device-box-pfo", |_| false);
        assert_eq!(missing.len(), 4);
        assert!(missing.iter().any(|p| p.ends_with("Cargo.lock")));

        let present = missing_inputs("examples/device-box-pfo", |_| true);
        assert!(present.is_empty());

        // A directory that exists but has lost its lockfile is still a failure:
        // without it there is no resolve to read.
        let no_lockfile = missing_inputs("examples/device-box-pfo", |path| {
            !path.ends_with("Cargo.lock")
        });
        assert_eq!(no_lockfile, vec!["examples/device-box-pfo/Cargo.lock"]);
    }

    /// Against the real example, not a fixture: RFC 023 §16.6 requires the
    /// device example to demonstrably reach no server crate, and a fixture-only
    /// assertion would prove the parser rather than the graph.
    #[test]
    fn the_real_device_example_resolves_to_no_forbidden_crate() {
        let device = EXAMPLES
            .iter()
            .find(|example| example.name == "device-box-pfo")
            .expect("the device example is registered");
        let manifest = format!(
            "{}/../{}/Cargo.toml",
            env!("CARGO_MANIFEST_DIR"),
            device.dir
        );
        let resolved =
            resolved_packages(&manifest).expect("the device example resolves against its lockfile");
        assert!(
            resolved.iter().any(|name| name == "device-box-pfo"),
            "resolved set does not contain the example itself: {resolved:?}"
        );
        assert!(
            forbidden_present(&resolved, device.forbidden).is_empty(),
            "device example reaches a forbidden crate: {resolved:?}"
        );
    }

    fn block(example: &str, lines: &[&str]) -> (String, super::CapturedBlock) {
        (
            "docs/src/example.md".to_owned(),
            super::CapturedBlock {
                example: example.to_owned(),
                line: 7,
                lines: lines.iter().map(|l| (*l).to_owned()).collect(),
            },
        )
    }

    /// One block per registered example, each matching its output, so a test can
    /// vary a single block and see exactly one finding.
    fn all_required(output: &str) -> Vec<(&'static str, Option<String>)> {
        super::CAPTURED_EXAMPLES
            .iter()
            .map(|name| (*name, Some(output.to_owned())))
            .collect()
    }

    fn covering_blocks(lines: &[&str]) -> Vec<(String, super::CapturedBlock)> {
        super::CAPTURED_EXAMPLES
            .iter()
            .map(|name| block(name, lines))
            .collect()
    }

    #[test]
    fn captured_blocks_parse_the_marker_and_its_fence_only() {
        let source = "text\n<!-- example-output: device-box-pfo -->\n```text\na\nb\n```\nafter\n\
                      ```text\nunmarked\n```\n";
        let blocks = super::captured_blocks(source).expect("well formed");
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].example, "device-box-pfo");
        assert_eq!(blocks[0].line, 2);
        assert_eq!(blocks[0].lines, vec!["a".to_owned(), "b".to_owned()]);
    }

    #[test]
    fn a_marker_without_a_closed_fence_is_an_error_not_a_skipped_block() {
        let no_fence = "<!-- example-output: device-box-pfo -->\nprose\n";
        assert!(super::captured_blocks(no_fence).is_err());
        let unclosed = "<!-- example-output: device-box-pfo -->\n```text\na\n";
        assert!(super::captured_blocks(unclosed).is_err());
    }

    #[test]
    fn a_contiguous_run_matches_and_a_reordered_one_does_not() {
        let haystack = ["a", "b", "c", "d"];
        let run = |lines: &[&str]| lines.iter().map(|l| (*l).to_owned()).collect::<Vec<_>>();
        assert!(super::contains_run(&haystack, &run(&["b", "c"])));
        assert!(!super::contains_run(&haystack, &run(&["c", "b"])));
        assert!(!super::contains_run(&haystack, &run(&[])));
    }

    #[test]
    fn a_block_equal_to_the_run_passes_and_a_one_digit_change_fails() {
        let output =
            "interior optimum: converged in 93 iteration(s)\nbounds active: converged in 28\n";
        let right = covering_blocks(&["bounds active: converged in 28"]);
        assert!(super::captured_output_findings(&right, &all_required(output)).is_empty());

        let changed = covering_blocks(&["bounds active: converged in 29"]);
        let findings = super::captured_output_findings(&changed, &all_required(output));
        assert_eq!(findings.len(), 4, "{findings:?}");
        assert!(
            findings.iter().all(|f| f.starts_with("CAPTURED MISMATCH")),
            "{findings:?}"
        );
    }

    #[test]
    fn an_elided_block_is_refused_even_when_its_visible_lines_match() {
        let output = "line one\nline two\n";
        let elided = covering_blocks(&["line one", "..."]);
        let findings = super::captured_output_findings(&elided, &all_required(output));
        assert!(
            findings.iter().all(|f| f.starts_with("CAPTURED ELIDED")),
            "{findings:?}"
        );
    }

    #[test]
    fn a_required_example_with_no_marked_block_fails_rather_than_passing_unchecked() {
        let output = "x\n";
        let blocks = vec![block("device-box-pfo", &["x"])];
        let findings = super::captured_output_findings(&blocks, &all_required(output));
        assert!(
            findings.iter().any(|f| f.starts_with("CAPTURED MISSING")),
            "{findings:?}"
        );
    }

    #[test]
    fn an_example_that_did_not_run_leaves_its_block_uncheckable_and_failing() {
        let blocks = covering_blocks(&["x"]);
        let outputs: Vec<(&str, Option<String>)> = super::CAPTURED_EXAMPLES
            .iter()
            .map(|name| (*name, None))
            .collect();
        let findings = super::captured_output_findings(&blocks, &outputs);
        assert!(
            findings
                .iter()
                .all(|f| f.starts_with("CAPTURED UNCHECKABLE")),
            "{findings:?}"
        );
    }

    #[test]
    fn a_block_naming_an_unregistered_example_is_reported() {
        let blocks = vec![block("no-such-example", &["x"])];
        let findings = super::captured_output_findings(&blocks, &all_required("x\n"));
        assert!(
            findings.iter().any(|f| f.starts_with("CAPTURED UNKNOWN")),
            "{findings:?}"
        );
    }

    #[test]
    fn a_marker_written_as_inline_code_in_prose_is_not_a_block() {
        let prose = "An exact block is marked by `<!-- example-output: device-box-pfo -->` above its fence.\n";
        assert_eq!(
            super::captured_blocks(prose).expect("well formed"),
            Vec::new()
        );
    }
}
