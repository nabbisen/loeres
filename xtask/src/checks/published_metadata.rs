//! `published-metadata` — RFC 036: the artifact a crates.io user receives is a
//! reviewed deliverable.
//!
//! The other gates reason about the repository. `link-audit` checks that links
//! resolve inside the checkout; `release-gate` checks the tagged tree. Neither
//! sees the package a user installs, so this gate reads the manifests and the
//! packaged files directly.
//!
//! Assertions, all over the five published crates:
//!
//! - **Internal requirements** (RFC 036 §2.1): every internal entry in
//!   `[workspace.dependencies]` requires exactly the workspace version. A `"0"`
//!   requirement admits every `0.x` release, so a published dependent can advertise
//!   compatibility with a sibling it cannot compile against.
//! - **License text** (§2.2): each crate directory holds a `LICENSE` byte-identical
//!   to the workspace `LICENSE`. The copies are real files, not symlinks, because
//!   `release_gate/package.rs` rejects anything that is not a regular file.
//! - **Discovery** (§2.7): each crate declares 1–5 keywords of at most twenty
//!   characters, and 1–5 categories, every one a slug in `VALID_CATEGORIES`.
//!   crates.io rejects an unknown slug at publish time, which is too late to find
//!   out, so the list is held here.
//! - **Feature labels** (RFC 036 §3.2): every `doc(cfg(feature = "X"))` in a crate
//!   names a feature that crate declares in `[features]`. The `docsrs` build that
//!   would compile these labels needs nightly, and `doc-build` deliberately omits
//!   that flag, so this lexical check is the only guard against a typo or a
//!   removed feature.
//! - **Packaged READMEs** (§2.5): no `crates/<name>/README.md` carries a relative
//!   link target, because a relative target 404s on crates.io when the file it
//!   names is not in the tarball. The **root** `README.md` is deliberately exempt:
//!   it ships with no crate, and its relative links are correct on GitHub. Do not
//!   "fix" it to absolute URLs.

use std::fs;

/// The category slugs the crates use, each verified against the crates.io list.
/// A slug added to a manifest must be added here first.
const VALID_CATEGORIES: &[&str] = &[
    "science",
    "mathematics",
    "no-std",
    "embedded",
    "data-structures",
    "concurrency",
];

/// Keyword limits on crates.io.
const MAX_KEYWORDS: usize = 5;
const MAX_KEYWORD_LENGTH: usize = 20;

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
    let workspace_license = fs::read("LICENSE");
    for name in PUBLISHED_CRATES {
        let dir = format!("crates/{name}");
        match fs::read_to_string(format!("{dir}/Cargo.toml")) {
            Ok(source) => findings.extend(package_metadata_findings(name, &source)),
            Err(error) => findings.push(format!("PACKAGE: cannot read {dir}/Cargo.toml: {error}")),
        }
        findings.extend(license_findings(
            name,
            fs::read(format!("{dir}/LICENSE")),
            workspace_license
                .as_ref()
                .map_err(|e| e.to_string())
                .cloned(),
        ));
        findings.extend(doc_cfg_symmetry_findings(name, &dir));
        match fs::read_to_string(format!("{dir}/README.md")) {
            Ok(source) => findings.extend(readme_link_findings(name, &source)),
            Err(error) => findings.push(format!("README: cannot read {dir}/README.md: {error}")),
        }
    }
    for finding in &findings {
        eprintln!("  {finding}");
    }
    let ok = findings.is_empty();
    eprintln!(
        "  checked {} published crate(s): requirements, LICENSE, keywords/categories, README links",
        PUBLISHED_CRATES.len()
    );
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

/// Every `doc(cfg(feature = "X"))` under `dir/src` must name a feature in the
/// crate's `[features]` table.
fn doc_cfg_symmetry_findings(name: &str, dir: &str) -> Vec<String> {
    let manifest = match fs::read_to_string(format!("{dir}/Cargo.toml")) {
        Ok(source) => source,
        Err(error) => {
            return vec![format!(
                "FEATURES: `{name}`: cannot read its manifest: {error}"
            )];
        }
    };
    let declared = match declared_features(&manifest) {
        Ok(features) => features,
        Err(error) => return vec![format!("FEATURES: `{name}`: {error}")],
    };
    let mut files = Vec::new();
    super::util::collect_ext(
        std::path::Path::new(&format!("{dir}/src")),
        "rs",
        &mut files,
    );
    let mut findings = Vec::new();
    for path in files {
        let Ok(source) = fs::read_to_string(&path) else {
            findings.push(format!(
                "FEATURES: `{name}`: cannot read {}",
                path.display()
            ));
            continue;
        };
        for feature in doc_cfg_features(&source) {
            if !declared.contains(&feature) {
                findings.push(format!(
                    "FEATURE LABEL: `{name}` labels an item `doc(cfg(feature = \"{feature}\"))` in {}, but [features] declares no `{feature}`",
                    path.display()
                ));
            }
        }
    }
    findings
}

/// The keys of a manifest's `[features]` table.
fn declared_features(manifest: &str) -> Result<Vec<String>, String> {
    let document = manifest
        .parse::<toml::Value>()
        .map_err(|error| format!("cannot parse its manifest: {error}"))?;
    let table = document
        .get("features")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| "has no [features] table".to_owned())?;
    Ok(table.keys().cloned().collect())
}

/// Every feature named in a `doc(cfg(feature = "…"))` attribute in `source`.
fn doc_cfg_features(source: &str) -> Vec<String> {
    let mut features = Vec::new();
    let mut rest = source;
    while let Some(index) = rest.find("doc(cfg(feature = \"") {
        let after = &rest[index + "doc(cfg(feature = \"".len()..];
        if let Some(end) = after.find('"') {
            features.push(after[..end].to_owned());
        }
        rest = after;
    }
    features
}

/// The `[package]` discovery metadata of one crate.
fn package_metadata_findings(name: &str, source: &str) -> Vec<String> {
    let document = match source.parse::<toml::Value>() {
        Ok(document) => document,
        Err(error) => {
            return vec![format!(
                "PACKAGE: `{name}`: cannot parse its manifest: {error}"
            )];
        }
    };
    let Some(package) = document.get("package").and_then(toml::Value::as_table) else {
        return vec![format!("PACKAGE: `{name}` has no [package] table")];
    };
    let mut findings = Vec::new();

    match string_array(package.get("keywords")) {
        None => findings.push(format!("KEYWORDS: `{name}` declares no `keywords` array")),
        Some(keywords) => {
            if keywords.is_empty() || keywords.len() > MAX_KEYWORDS {
                findings.push(format!(
                    "KEYWORDS: `{name}` declares {} keyword(s); crates.io allows 1 to {MAX_KEYWORDS}",
                    keywords.len()
                ));
            }
            for keyword in keywords {
                if keyword.is_empty() || keyword.chars().count() > MAX_KEYWORD_LENGTH {
                    findings.push(format!(
                        "KEYWORDS: `{name}` keyword \"{keyword}\" is not 1 to {MAX_KEYWORD_LENGTH} characters"
                    ));
                }
            }
        }
    }

    match string_array(package.get("categories")) {
        None => findings.push(format!(
            "CATEGORIES: `{name}` declares no `categories` array"
        )),
        Some(categories) => {
            if categories.is_empty() || categories.len() > MAX_KEYWORDS {
                findings.push(format!(
                    "CATEGORIES: `{name}` declares {} categor(ies); crates.io allows 1 to {MAX_KEYWORDS}",
                    categories.len()
                ));
            }
            for category in categories {
                if !VALID_CATEGORIES.contains(&category.as_str()) {
                    findings.push(format!(
                        "CATEGORIES: `{name}` category \"{category}\" is not in VALID_CATEGORIES"
                    ));
                }
            }
        }
    }
    findings
}

fn string_array(value: Option<&toml::Value>) -> Option<Vec<String>> {
    value?
        .as_array()?
        .iter()
        .map(|item| item.as_str().map(str::to_owned))
        .collect()
}

/// The crate's `LICENSE` must exist and match the workspace `LICENSE` byte for byte.
fn license_findings(
    name: &str,
    crate_license: std::io::Result<Vec<u8>>,
    workspace_license: Result<Vec<u8>, String>,
) -> Vec<String> {
    let workspace = match workspace_license {
        Ok(bytes) => bytes,
        Err(error) => {
            return vec![format!(
                "LICENSE: cannot read the workspace LICENSE: {error}"
            )];
        }
    };
    match crate_license {
        Err(error) => vec![format!("LICENSE: `{name}` has no LICENSE file: {error}")],
        Ok(bytes) if bytes != workspace => vec![format!(
            "LICENSE: `{name}`'s LICENSE differs from the workspace LICENSE ({} bytes against {})",
            bytes.len(),
            workspace.len()
        )],
        Ok(_) => Vec::new(),
    }
}

/// Every relative link target in a packaged README. Inline `](target)` and
/// reference definitions `[label]: target` are both checked.
fn readme_link_findings(name: &str, source: &str) -> Vec<String> {
    let mut findings = Vec::new();
    for target in link_targets(source) {
        if is_relative(&target) {
            findings.push(format!(
                "README LINK: `{name}`'s README links `{target}`, a relative path that will 404 on crates.io"
            ));
        }
    }
    findings
}

fn link_targets(source: &str) -> Vec<String> {
    let mut targets = Vec::new();
    let mut rest = source;
    while let Some(index) = rest.find("](") {
        let after = &rest[index + 2..];
        if let Some(end) = after.find(')') {
            let target = after[..end].split_whitespace().next().unwrap_or("");
            targets.push(target.to_owned());
        }
        rest = after;
    }
    for line in source.lines() {
        let trimmed = line.trim_start();
        let definition = trimmed
            .strip_prefix('[')
            .and_then(|stripped| stripped.split_once("]:"))
            .and_then(|(_, after)| after.split_whitespace().next());
        if let Some(target) = definition {
            targets.push(target.to_owned());
        }
    }
    targets
}

/// A target that is neither absolute (a scheme such as `https:`) nor a fragment
/// within the same page. Everything else resolves only inside the checkout.
fn is_relative(target: &str) -> bool {
    if target.is_empty() || target.starts_with('#') {
        return false;
    }
    let has_scheme = target.split_once(':').is_some_and(|(scheme, _)| {
        !scheme.is_empty()
            && scheme
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.')
    });
    !has_scheme && !target.starts_with('/')
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

    fn package(keywords: &str, categories: &str) -> String {
        format!("[package]\nname = \"x\"\nkeywords = [{keywords}]\ncategories = [{categories}]\n")
    }

    #[test]
    fn valid_discovery_metadata_passes() {
        let text = package("\"optimization\", \"solver\"", "\"science\"");
        assert!(super::package_metadata_findings("x", &text).is_empty());
    }

    #[test]
    fn an_unknown_category_slug_is_refused_by_name() {
        let text = package("\"solver\"", "\"science::math\"");
        let findings = super::package_metadata_findings("x", &text);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].contains("science::math"), "{findings:?}");
    }

    #[test]
    fn too_many_or_too_long_keywords_are_refused() {
        let six = package("\"a\", \"b\", \"c\", \"d\", \"e\", \"f\"", "\"science\"");
        assert!(
            super::package_metadata_findings("x", &six)
                .iter()
                .any(|f| f.contains("6 keyword"))
        );
        let long = package(&format!("\"{}\"", "k".repeat(21)), "\"science\"");
        assert!(
            super::package_metadata_findings("x", &long)
                .iter()
                .any(|f| f.contains("1 to 20"))
        );
    }

    #[test]
    fn an_empty_categories_list_is_refused() {
        let text = package("\"solver\"", "");
        assert!(
            super::package_metadata_findings("x", &text)
                .iter()
                .any(|f| f.contains("0 categor"))
        );
    }

    #[test]
    fn a_license_byte_for_byte_copy_passes_and_a_single_byte_difference_fails() {
        let workspace = b"Apache License\n".to_vec();
        assert!(
            super::license_findings("x", Ok(workspace.clone()), Ok(workspace.clone())).is_empty()
        );
        let mut changed = workspace.clone();
        changed[0] = b'x';
        let findings = super::license_findings("x", Ok(changed), Ok(workspace));
        assert!(
            findings.iter().any(|f| f.contains("differs")),
            "{findings:?}"
        );
    }

    #[test]
    fn a_missing_license_file_is_reported() {
        let missing = Err(std::io::Error::from(std::io::ErrorKind::NotFound));
        let findings = super::license_findings("x", missing, Ok(b"A".to_vec()));
        assert!(
            findings.iter().any(|f| f.contains("has no LICENSE file")),
            "{findings:?}"
        );
    }

    #[test]
    fn a_relative_readme_link_is_refused_and_absolute_and_fragment_links_pass() {
        let relative = "See the [README](../../README.md).";
        assert_eq!(super::readme_link_findings("x", relative).len(), 1);
        let reference = "[index]: ../../rfcs/README.md\n";
        assert_eq!(super::readme_link_findings("x", reference).len(), 1);
        let absolute =
            "[a](https://github.com/nabbisen/loeres) and [b](#top) and [c](mailto:x@y.z)";
        assert!(super::readme_link_findings("x", absolute).is_empty());
    }

    #[test]
    fn a_doc_cfg_naming_an_undeclared_feature_is_refused() {
        let source = "#[cfg_attr(docsrs, doc(cfg(feature = \"parallel-rayon\")))]\npub fn f() {}\n\
                      #[cfg_attr(docsrs, doc(cfg(feature = \"dense\")))]\n";
        assert_eq!(
            super::doc_cfg_features(source),
            vec!["parallel-rayon", "dense"]
        );
        let manifest = "[features]\ndense = []\ndefault = []\n";
        let declared = super::declared_features(manifest).expect("parses");
        let unknown: Vec<_> = super::doc_cfg_features(source)
            .into_iter()
            .filter(|f| !declared.contains(f))
            .collect();
        assert_eq!(unknown, vec!["parallel-rayon".to_owned()]);
    }

    #[test]
    fn the_real_published_crates_pass_every_assertion() {
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/..");
        let workspace_license =
            std::fs::read(format!("{root}/LICENSE")).expect("workspace LICENSE");
        for name in PUBLISHED_CRATES {
            let dir = format!("{root}/crates/{name}");
            let manifest = std::fs::read_to_string(format!("{dir}/Cargo.toml")).expect("manifest");
            assert!(
                super::package_metadata_findings(name, &manifest).is_empty(),
                "{name}: {:?}",
                super::package_metadata_findings(name, &manifest)
            );
            let license = std::fs::read(format!("{dir}/LICENSE")).expect("crate LICENSE");
            assert!(
                super::license_findings(name, Ok(license), Ok(workspace_license.clone()))
                    .is_empty()
            );
            let readme = std::fs::read_to_string(format!("{dir}/README.md")).expect("crate README");
            assert!(
                super::readme_link_findings(name, &readme).is_empty(),
                "{name}"
            );
            let dir = format!("{root}/crates/{name}");
            assert!(
                super::doc_cfg_symmetry_findings(name, &dir).is_empty(),
                "{name}: {:?}",
                super::doc_cfg_symmetry_findings(name, &dir)
            );
        }
    }
}
