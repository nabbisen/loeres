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
//! An absent example directory **fails**. A gate that silently passes when its
//! subject is missing proves nothing, which is the same fail-closed reasoning
//! RFC 022's coverage symmetry applies to a missing review.

use std::path::Path;

use super::util::{cargo, cargo_stdout};

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
];

pub fn run() -> bool {
    eprintln!("[examples] RFC 023 example build and dependency isolation");
    let mut ok = true;

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

    eprintln!("[examples] {}", if ok { "PASS" } else { "FAIL" });
    ok
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
}
