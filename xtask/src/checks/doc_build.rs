//! `doc-build` — RFC 036 §3.3: the API reference builds with no rustdoc diagnostic.
//!
//! Runs `cargo doc --workspace --all-features --no-deps` under `RUSTDOCFLAGS=-D
//! warnings`. Before this gate no tracked command built the API reference, so the
//! five rustdoc diagnostics RFC 036 §2.4 records were invisible to every gate.
//!
//! **Deliberately without `--cfg docsrs`.** That path enables the
//! `cfg_attr(docsrs, doc(cfg(…)))` feature labels, but it needs nightly
//! (`feature(doc_cfg)`), which this project does not pin; the stable release
//! channel rejects the `#![feature]` attribute outright. So this gate does not
//! compile the labels. The lexical `doc(cfg)` symmetry assertion in
//! `published-metadata` covers the failure a nightly-only build would otherwise
//! catch: a label naming a feature that does not exist.

use std::process::Command;

pub fn run() -> bool {
    eprintln!("[doc-build] RFC 036 API reference under -D warnings (no --cfg docsrs)");
    let output = Command::new(env!("CARGO"))
        .env("RUSTDOCFLAGS", "-D warnings")
        .args(["doc", "--workspace", "--all-features", "--no-deps"])
        .output();
    let ok = match output {
        Ok(output) if output.status.success() => true,
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            for line in stderr.lines().filter(|line| line.starts_with("error")) {
                eprintln!("  {line}");
            }
            let failing = failing_crates(&stderr);
            if failing.is_empty() {
                eprintln!("  cargo doc failed; no crate name was reported");
            } else {
                eprintln!("  failing crate(s): {}", failing.join(", "));
            }
            false
        }
        Err(error) => {
            eprintln!("  cannot run cargo doc: {error}");
            false
        }
    };
    eprintln!("[doc-build] {}", if ok { "PASS" } else { "FAIL" });
    ok
}

/// The crate names rustdoc reports as `could not document …`, in order, once each.
fn failing_crates(stderr: &str) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for line in stderr.lines() {
        let Some((_, after)) = line.split_once("could not document `") else {
            continue;
        };
        let Some((name, _)) = after.split_once('`') else {
            continue;
        };
        if !names.iter().any(|known| known == name) {
            names.push(name.to_owned());
        }
    }
    names
}

#[cfg(test)]
mod tests {
    use super::failing_crates;

    #[test]
    fn failing_crate_names_are_reported_once_each_in_order() {
        let stderr = "error: could not document `loeres-cluster`\n\
                      error: could not document `loeres-backend-static`\n\
                      error: could not document `loeres-cluster`\n";
        assert_eq!(
            failing_crates(stderr),
            vec![
                "loeres-cluster".to_owned(),
                "loeres-backend-static".to_owned()
            ]
        );
    }

    #[test]
    fn a_failure_without_a_crate_name_reports_none() {
        assert!(failing_crates("error: something else\n").is_empty());
    }
}
