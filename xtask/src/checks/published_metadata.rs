//! `published-metadata` — RFC 036: the artifact a crates.io user receives is a
//! reviewed deliverable.
//!
//! The other gates reason about the repository. `link-audit` checks that links
//! resolve inside the checkout; `release-gate` checks the tagged tree. Neither
//! sees the package a user installs, so this gate reads the manifests and the
//! packaged files directly.
//!
//! S1 (this module's first assertion): every internal entry in
//! `[workspace.dependencies]` requires exactly the workspace version. A `"0"`
//! requirement admits every `0.x` release, so a published dependent can advertise
//! compatibility with a sibling it cannot compile against (RFC 036 §2.1). The
//! version is pinned here so a version bump that forgets one entry fails closed.

use std::fs;

/// The five crates that are published, in dependency order (RFC 036 §4.3).
const PUBLISHED_CRATES: &[&str] = &[
    "loeres",
    "loeres-backend-static",
    "loeres-backend-std",
    "loeres-device",
    "loeres-cluster",
];

pub fn run() -> bool {
    eprintln!("[published-metadata] RFC 036 published artifact metadata");
    let mut findings = Vec::new();
    match fs::read_to_string("Cargo.toml") {
        Ok(source) => findings.extend(manifest_findings(&source)),
        Err(error) => findings.push(format!("CARGO: cannot read Cargo.toml: {error}")),
    }
    for finding in &findings {
        eprintln!("  {finding}");
    }
    let ok = findings.is_empty();
    eprintln!("  checked {} published crate(s)", PUBLISHED_CRATES.len());
    eprintln!("[published-metadata] {}", if ok { "PASS" } else { "FAIL" });
    ok
}

/// Every finding for the workspace manifest. Pure, so the tests can feed it text.
fn manifest_findings(source: &str) -> Vec<String> {
    let document = match source.parse::<toml::Value>() {
        Ok(document) => document,
        Err(error) => return vec![format!("CARGO: cannot parse Cargo.toml: {error}")],
    };
    let Some(version) = document
        .get("workspace")
        .and_then(|workspace| workspace.get("package"))
        .and_then(|package| package.get("version"))
        .and_then(toml::Value::as_str)
    else {
        return vec!["VERSION: [workspace.package] has no string `version`".to_owned()];
    };
    internal_requirement_findings(&document, version)
}

/// Each internal workspace dependency must be a path entry for a published crate,
/// requiring exactly `version`. Fails closed on a missing entry, a missing or
/// mismatched `version`, and an internal crate the gate does not know about.
fn internal_requirement_findings(document: &toml::Value, version: &str) -> Vec<String> {
    let mut findings = Vec::new();
    let Some(dependencies) = document
        .get("workspace")
        .and_then(|workspace| workspace.get("dependencies"))
        .and_then(toml::Value::as_table)
    else {
        return vec!["DEPENDENCIES: [workspace.dependencies] is missing".to_owned()];
    };

    for name in PUBLISHED_CRATES {
        let Some(entry) = dependencies.get(*name).and_then(toml::Value::as_table) else {
            findings.push(format!(
                "INTERNAL REQUIREMENT: `{name}` has no entry in [workspace.dependencies]"
            ));
            continue;
        };
        match entry.get("version").and_then(toml::Value::as_str) {
            Some(required) if required == version => {}
            Some(required) => findings.push(format!(
                "INTERNAL REQUIREMENT: `{name}` requires \"{required}\"; [workspace.package] version is \"{version}\""
            )),
            None => findings.push(format!(
                "INTERNAL REQUIREMENT: `{name}` has no `version` key; it would publish with no requirement"
            )),
        }
    }

    for (name, entry) in dependencies {
        let internal = entry
            .get("path")
            .and_then(toml::Value::as_str)
            .is_some_and(|path| path.starts_with("crates/"));
        if internal && !PUBLISHED_CRATES.contains(&name.as_str()) {
            findings.push(format!(
                "INTERNAL REQUIREMENT: `{name}` is an internal crate the published-metadata gate does not list"
            ));
        }
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::{PUBLISHED_CRATES, manifest_findings};

    /// A manifest where every internal entry requires `version`.
    fn manifest(version: &str) -> String {
        let mut text =
            format!("[workspace.package]\nversion = \"{version}\"\n\n[workspace.dependencies]\n");
        for name in PUBLISHED_CRATES {
            text.push_str(&format!(
                "{name} = {{ version = \"{version}\", path = \"crates/{name}\" }}\n"
            ));
        }
        text.push_str("serde = { version = \"1\" }\n");
        text
    }

    #[test]
    fn matching_requirements_pass() {
        assert!(manifest_findings(&manifest("0.22.1")).is_empty());
    }

    #[test]
    fn a_wildcard_zero_requirement_is_refused_and_names_the_crate_and_both_values() {
        let text = manifest("0.22.1").replace(
            "loeres-cluster = { version = \"0.22.1\"",
            "loeres-cluster = { version = \"0\"",
        );
        let findings = manifest_findings(&text);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].contains("`loeres-cluster`"), "{findings:?}");
        assert!(findings[0].contains("\"0\""), "{findings:?}");
        assert!(findings[0].contains("\"0.22.1\""), "{findings:?}");
    }

    #[test]
    fn a_missing_version_key_fails_rather_than_publishing_unconstrained() {
        let text = manifest("0.22.1").replace(
            "loeres-device = { version = \"0.22.1\", ",
            "loeres-device = { ",
        );
        let findings = manifest_findings(&text);
        assert!(
            findings
                .iter()
                .any(|f| f.contains("`loeres-device`") && f.contains("no `version` key")),
            "{findings:?}"
        );
    }

    #[test]
    fn a_missing_entry_is_reported() {
        let text = manifest("0.22.1").replace(
            "loeres-backend-std = { version = \"0.22.1\", path = \"crates/loeres-backend-std\" }\n",
            "",
        );
        let findings = manifest_findings(&text);
        assert!(
            findings
                .iter()
                .any(|f| f.contains("`loeres-backend-std` has no entry")),
            "{findings:?}"
        );
    }

    #[test]
    fn an_unlisted_internal_crate_fails_closed() {
        let mut text = manifest("0.22.1");
        text.push_str("loeres-new = { version = \"0.22.1\", path = \"crates/loeres-new\" }\n");
        let findings = manifest_findings(&text);
        assert!(
            findings
                .iter()
                .any(|f| f.contains("`loeres-new`") && f.contains("does not list")),
            "{findings:?}"
        );
    }

    #[test]
    fn the_real_workspace_manifest_passes() {
        let source = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../Cargo.toml"))
            .expect("workspace manifest is readable");
        assert!(
            manifest_findings(&source).is_empty(),
            "{:?}",
            manifest_findings(&source)
        );
    }
}
