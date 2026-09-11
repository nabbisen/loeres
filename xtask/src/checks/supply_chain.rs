//! `supply-chain` — RFC 026 dependency advisory, license, ban, and source gate.
//!
//! Runs `cargo deny --all-features check` over the whole workspace: advisories
//! (RustSec), licenses (allow-list), bans (duplicate versions, wildcards), and
//! sources (crates.io only). Policy lives in the tracked `deny.toml`, not here.
//!
//! Enforced, and enforced *hard*: if `cargo-deny` is not on PATH the gate
//! reports `unavailable` **and fails**. Unlike RFC 022's maintainer-held review
//! corpus, which is legitimately absent from a clean extraction, a missing tool
//! is an environment defect (RFC 026 §11) — the difference is whether the
//! absence is expected by design.
//!
//! Second assertion: each edge crate, built `--no-default-features`, must have
//! an empty external dependency set. `cargo deny check bans` is run against the
//! edge crate's own manifest, and its dependency graph is asserted to contain
//! only path-local workspace crates. This doubles as an independent zero-bleed
//! witness: `zero-bleed` enumerates forbidden *internal* edges, while this says
//! nothing registry-sourced reaches an edge crate at all.

use std::process::Command;

use super::util::cargo_stdout;

/// Crates that must reach zero external dependencies with default features off.
const EDGE_CRATES: &[(&str, &str)] = &[
    ("loeres", "crates/loeres/Cargo.toml"),
    (
        "loeres-backend-static",
        "crates/loeres-backend-static/Cargo.toml",
    ),
    ("loeres-device", "crates/loeres-device/Cargo.toml"),
];

pub fn run() -> bool {
    eprintln!("[supply-chain] RFC 026 advisories, licenses, bans, sources");

    let version = match tool_gate(cargo_deny_version()) {
        Ok(version) => version,
        Err(reason) => {
            eprintln!("  TOOL: {reason}");
            eprintln!("[supply-chain] FAIL (tool unavailable)");
            return false;
        }
    };
    eprintln!("  tool: {version}");

    let mut ok = true;

    eprintln!("  $ cargo deny --all-features check");
    if !deny(&["--all-features", "check"]) {
        eprintln!("  WORKSPACE: `cargo deny --all-features check` failed; see its output above");
        ok = false;
    }

    for (krate, manifest) in EDGE_CRATES {
        eprintln!("  $ cargo deny --manifest-path {manifest} --no-default-features check bans");
        if !deny(&[
            "--manifest-path",
            manifest,
            "--no-default-features",
            "check",
            "bans",
        ]) {
            eprintln!("  EDGE BANS: {krate} failed the bans check with default features off");
            ok = false;
        }
        match external_dependencies(krate) {
            Some(external) if external.is_empty() => {
                eprintln!("  {krate}: zero external dependencies (--no-default-features)");
            }
            Some(external) => {
                for dependency in external {
                    eprintln!(
                        "  EDGE DEPENDENCY: {krate} pulls external crate `{dependency}` with default features off"
                    );
                }
                ok = false;
            }
            None => {
                eprintln!("  EDGE DEPENDENCY: cannot read the dependency tree for {krate}");
                ok = false;
            }
        }
    }

    eprintln!("[supply-chain] {}", if ok { "PASS" } else { "FAIL" });
    ok
}

fn cargo_deny_version() -> Option<String> {
    let output = Command::new("cargo")
        .args(["deny", "--version"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let version = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if version.is_empty() {
        None
    } else {
        Some(version)
    }
}

fn deny(args: &[&str]) -> bool {
    Command::new("cargo")
        .arg("deny")
        .args(args)
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

/// External (registry-sourced) crates in one crate's `--no-default-features`
/// normal dependency graph. A workspace member prints its path in parentheses;
/// a registry crate does not, which is what separates the two.
fn external_dependencies(krate: &str) -> Option<Vec<String>> {
    let tree = cargo_stdout(&[
        "tree",
        "-p",
        krate,
        "--no-default-features",
        "--edges",
        "normal",
        "--prefix",
        "none",
    ])?;
    Some(external_from_tree(&tree))
}

fn external_from_tree(tree: &str) -> Vec<String> {
    let mut external = Vec::new();
    for line in tree.lines() {
        let line = line.trim();
        if line.is_empty() || line.ends_with("(*)") {
            continue;
        }
        if is_path_local(line) {
            continue;
        }
        let name = line.split_whitespace().next().unwrap_or(line).to_owned();
        if !external.contains(&name) {
            external.push(name);
        }
    }
    external
}

/// `cargo tree --prefix none` prints a workspace member as
/// `name v0.0.0 (/abs/path)`, and a registry crate as `name v0.0.0`.
fn is_path_local(line: &str) -> bool {
    line.rsplit_once(" (")
        .is_some_and(|(_, tail)| tail.starts_with('/') && tail.ends_with(')'))
}

/// Turn an observed tool probe into a gate decision. A failed probe is
/// `unavailable`, and unavailable **fails** — it never degrades to an advisory
/// result the way `size-budget` does, because an absent tool is an environment
/// defect rather than a legitimate absence (RFC 026 §11).
fn tool_gate(version: Option<String>) -> Result<String, String> {
    version.ok_or_else(|| {
        "`cargo deny` is not runnable — unavailable, and unavailable fails this gate. \
         Install it with `cargo install cargo-deny --locked`; a missing tool is an \
         environment defect, not a legitimate absence like RFC 022's maintainer-held corpus"
            .to_owned()
    })
}

#[cfg(test)]
mod tests {
    use super::{external_from_tree, is_path_local, tool_gate};

    #[test]
    fn an_unavailable_tool_fails_rather_than_being_skipped() {
        let reason = tool_gate(None).expect_err("an absent tool must not be accepted");
        assert!(reason.contains("unavailable fails this gate"));
        assert_eq!(
            tool_gate(Some("cargo-deny 0.20.2".to_owned())).unwrap(),
            "cargo-deny 0.20.2"
        );
    }

    #[test]
    fn workspace_members_are_recognized_as_path_local() {
        assert!(is_path_local(
            "loeres v0.20.3 (/home/user/loeres/crates/loeres)"
        ));
        assert!(!is_path_local("rayon v1.12.0"));
        assert!(!is_path_local("serde v1.0.228 (proc-macro)"));
    }

    #[test]
    fn an_edge_tree_of_only_workspace_members_has_no_external_dependencies() {
        let tree = "loeres-device v0.20.3 (/repo/crates/loeres-device)\n\
                    loeres v0.20.3 (/repo/crates/loeres)\n\
                    loeres-backend-static v0.20.3 (/repo/crates/loeres-backend-static)\n\
                    loeres v0.20.3 (/repo/crates/loeres) (*)\n";
        assert!(external_from_tree(tree).is_empty());
    }

    #[test]
    fn a_registry_dependency_in_an_edge_tree_is_reported_once() {
        let tree = "loeres-device v0.20.3 (/repo/crates/loeres-device)\n\
                    loeres v0.20.3 (/repo/crates/loeres)\n\
                    libm v0.2.15\n\
                    heapless v0.8.0\n\
                    libm v0.2.15 (*)\n";
        assert_eq!(
            external_from_tree(tree),
            vec!["libm".to_owned(), "heapless".to_owned()]
        );
    }
}
