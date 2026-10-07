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
//! - **Packaged source set** (RFC 036 C1.1): `cargo package -p <crate> --list
//!   --offline --allow-dirty` lists the package's files, and the gate compares its
//!   `src/` files with the `src/` files git tracks under `crates/<crate>/src`, in both
//!   directions. The package must also contain `LICENSE` and `README.md`. The proof
//!   rests on that comparison, not on cargo's cleanliness check. `--allow-dirty` is
//!   passed so the listing is available whatever the working tree state is. It is also
//!   what makes the packaged-but-untracked direction reachable: without it, cargo
//!   refuses to list a tree with uncommitted changes, and an untracked file never
//!   appears. A package that lists a file git does not track fails the gate, because
//!   that file would be published permanently. The tarball check is not used:
//!   `cargo package` of a dependent cannot resolve its unpublished siblings, with or
//!   without `[patch.crates-io]`.
//!
//!   The assertion requires the git working tree to be the tree under test. When git's
//!   toplevel is a different repository, the assertion is reported as not applicable,
//!   in one line: that is the clean extraction, which lives inside this repository. When
//!   git itself is unusable (absent, broken, or not a repository), the assertion is a
//!   **failure**. The two are kept apart because they look alike: both leave the
//!   packaged-source check unrun, and only the first is a legitimate state. Collapsing
//!   them lets a broken git silently remove the check. The guard deliberately uses `git rev-parse --show-toplevel` and not
//!   `--is-inside-work-tree`. The latter is `true` inside that in-repository extraction,
//!   where git silently answers from the outer repository and `ls-files` returns an
//!   empty, plausible listing. A failing `ls-files`, or an empty tracked `src/` set,
//!   at a matching toplevel is still a failure.
//! - **Reserved features** (RFC 036 C2.1): every declared feature with no
//!   `cfg(feature = …)` site in `src/` is listed in `RESERVED_INERT_FEATURES`, and
//!   every listed feature is still inert. A new feature cannot be silently inert,
//!   and a reserved one that goes live fails until it is de-registered.
//! - **README feature tables** (RFC 036 C7.2): each packaged README has a
//!   `## Features` section that mentions every declared feature except `default` as
//!   inline code, and gives each reserved feature's line the phrase
//!   `reserved; no effect yet`. The gate does not parse the table or check the wording
//!   of live features: that would be brittle and would fail on ordinary editing.
//!   The phrase rule applies to **every** line in the section naming a reserved
//!   feature, not only its table row. A sentence such as "must never be a default
//!   feature" that names one therefore fails here, deliberately: a reader skimming a
//!   single line about a reserved feature must not be left thinking it does
//!   something. Carry the phrase on that line too, or put the sentence outside
//!   `## Features`.
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

/// Features declared in `[features]` that gate no code, by RFC 009's reservation
/// posture (see `docs/specs/loeres-external-design-v1.md`). Exactly these pairs must
/// be inert: a registered feature that acquires a `cfg` site fails the gate, and an
/// unregistered inert feature fails it too.
const RESERVED_INERT_FEATURES: &[(&str, &str)] = &[
    ("loeres", "libm"),
    ("loeres", "fixed-point-hooks"),
    ("loeres-backend-static", "static-views"),
    ("loeres-backend-static", "diagnostic-snapshot"),
    ("loeres-backend-std", "serde"),
    ("loeres-backend-std", "parallel-rayon"),
    ("loeres-backend-std", "adapter-ndarray"),
    ("loeres-backend-std", "adapter-nalgebra"),
    ("loeres-backend-std", "native-linalg"),
    ("loeres-device", "diagnostic-snapshot"),
    ("loeres-device", "panic-gate"),
    ("loeres-cluster", "observability-tracing"),
    ("loeres-cluster", "observability-metrics"),
    ("loeres-cluster", "serde"),
    ("loeres-cluster", "ffi-gateway"),
];

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
    let applicability = tree_under_test();
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
        findings.extend(inert_feature_findings(name, &dir));
        if matches!(applicability, Applicability::Applicable) {
            findings.extend(packaged_source_findings(name));
        }
        findings.extend(features_section_findings(name, &dir));
        match fs::read_to_string(format!("{dir}/README.md")) {
            Ok(source) => findings.extend(readme_link_findings(name, &source)),
            Err(error) => findings.push(format!("README: cannot read {dir}/README.md: {error}")),
        }
    }
    match &applicability {
        Applicability::Applicable => {}
        Applicability::OtherRepository(reason) => {
            eprintln!("  packaged source set: not applicable here: {reason}");
        }
        Applicability::GitUnusable(reason) => {
            findings.push(format!("{GIT_UNUSABLE}: {reason}"));
        }
    }
    for finding in &findings {
        eprintln!("  {finding}");
    }
    let ok = findings.is_empty();
    eprintln!(
        "  checked {} published crate(s): requirements, LICENSE, keywords/categories, README links, feature labels, reserved features, README feature tables{}",
        PUBLISHED_CRATES.len(),
        if matches!(applicability, Applicability::Applicable) {
            ", packaged source"
        } else {
            ""
        }
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

/// Both directions of the reserved-feature symmetry for one crate (C2.1).
fn inert_feature_findings(name: &str, dir: &str) -> Vec<String> {
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
    let sources = match read_rust_sources(&format!("{dir}/src")) {
        Ok(sources) => sources,
        Err(error) => return vec![format!("FEATURES: `{name}`: {error}")],
    };
    inert_feature_check(name, &declared, &sources, RESERVED_INERT_FEATURES)
}

/// Pure core of `inert_feature_findings`, so the tests can feed it text.
fn inert_feature_check(
    name: &str,
    declared: &[String],
    sources: &[String],
    registry: &[(&str, &str)],
) -> Vec<String> {
    let mut findings = Vec::new();
    let registered = |feature: &str| registry.iter().any(|(c, f)| *c == name && *f == feature);
    let mut declared_real: Vec<&String> = declared
        .iter()
        .filter(|f| f.as_str() != "default")
        .collect();
    declared_real.sort();
    for feature in &declared_real {
        let sites: usize = sources
            .iter()
            .map(|source| feature_sites(source, feature))
            .sum();
        match (sites == 0, registered(feature)) {
            (true, false) => findings.push(format!(
                "INERT FEATURE: `{name}` declares `{feature}` with no cfg site and it is not in RESERVED_INERT_FEATURES; gate code on it or register it as reserved"
            )),
            (false, true) => findings.push(format!(
                "RESERVED FEATURE IS LIVE: `{name}`'s `{feature}` now has {sites} cfg site(s); remove it from RESERVED_INERT_FEATURES"
            )),
            _ => {}
        }
    }
    for (crate_name, feature) in registry {
        if *crate_name == name && !declared_real.iter().any(|f| f.as_str() == *feature) {
            findings.push(format!(
                "RESERVED FEATURE: RESERVED_INERT_FEATURES names `{name}`'s `{feature}`, which [features] does not declare"
            ));
        }
    }
    findings
}

/// The `cfg(feature = "X")` and `cfg!(feature = "X")` sites for `feature` in one
/// source file. A `doc(cfg(…))` attribute is a rustdoc label, not a gate, and is
/// excluded.
fn feature_sites(source: &str, feature: &str) -> usize {
    let mut count = 0;
    for needle in [
        format!("cfg(feature = \"{feature}\")"),
        format!("cfg!(feature = \"{feature}\")"),
    ] {
        let mut from = 0;
        while let Some(index) = source[from..].find(&needle) {
            let at = from + index;
            if !source[..at].ends_with("doc(") {
                count += 1;
            }
            from = at + needle.len();
        }
    }
    count
}

fn read_rust_sources(src_dir: &str) -> Result<Vec<String>, String> {
    let mut files = Vec::new();
    super::util::collect_ext(std::path::Path::new(src_dir), "rs", &mut files);
    files
        .iter()
        .map(|path| {
            fs::read_to_string(path)
                .map_err(|error| format!("cannot read {}: {error}", path.display()))
        })
        .collect()
}

/// C7.2: the packaged README's `## Features` section names every declared feature,
/// and marks each reserved feature with the phrase `reserved; no effect yet`.
fn features_section_findings(name: &str, dir: &str) -> Vec<String> {
    let manifest = match fs::read_to_string(format!("{dir}/Cargo.toml")) {
        Ok(source) => source,
        Err(error) => {
            return vec![format!(
                "FEATURES SECTION: `{name}`: cannot read its manifest: {error}"
            )];
        }
    };
    let declared = match declared_features(&manifest) {
        Ok(features) => features,
        Err(error) => return vec![format!("FEATURES SECTION: `{name}`: {error}")],
    };
    let readme = match fs::read_to_string(format!("{dir}/README.md")) {
        Ok(source) => source,
        Err(error) => {
            return vec![format!(
                "FEATURES SECTION: `{name}`: cannot read its README: {error}"
            )];
        }
    };
    features_section_check(name, &readme, &declared, RESERVED_INERT_FEATURES)
}

/// Pure core of `features_section_findings`.
fn features_section_check(
    name: &str,
    readme: &str,
    declared: &[String],
    registry: &[(&str, &str)],
) -> Vec<String> {
    let Some(section) = features_section(readme) else {
        return vec![format!(
            "FEATURES SECTION: `{name}`'s README has no `## Features` section"
        )];
    };
    let mut findings = Vec::new();
    for feature in declared.iter().filter(|f| f.as_str() != "default") {
        if !section.contains(&format!("`{feature}`")) {
            findings.push(format!(
                "FEATURES SECTION: `{name}`'s `## Features` does not mention `{feature}` as inline code"
            ));
        }
    }
    for (crate_name, feature) in registry {
        if *crate_name != name {
            continue;
        }
        let needle = format!("`{feature}`");
        for line in section.lines().filter(|line| line.contains(&needle)) {
            if !line.contains("reserved; no effect yet") {
                findings.push(format!(
                    "FEATURES SECTION: `{name}`'s `{feature}` is reserved, but its line in `## Features` lacks `reserved; no effect yet`"
                ));
            }
        }
    }
    findings
}

/// The lines under `## Features`, up to the next level-two heading.
fn features_section(readme: &str) -> Option<String> {
    let mut lines = readme.lines();
    lines.find(|line| line.trim() == "## Features")?;
    let mut section = String::new();
    for line in lines {
        if line.starts_with("## ") {
            break;
        }
        section.push_str(line);
        section.push('\n');
    }
    Some(section)
}

/// Why the packaged-source assertion does or does not run (C8, C9).
#[derive(Debug, PartialEq, Eq)]
enum Applicability {
    /// git's repository is the working tree under test: the assertion runs.
    Applicable,
    /// git works and names a different repository: the checkout sits inside another
    /// repository. Not applicable; reported in one line, with no finding.
    OtherRepository(String),
    /// git cannot say what the working tree is: git is absent, broken, or the
    /// directory is not a repository, or git named a toplevel that does not resolve.
    /// A failure, because the assertion cannot be made and must not silently vanish.
    GitUnusable(String),
}

/// Prefix of the finding for an unusable git. The reason is appended, so the finding
/// is self-contained like every other one this gate emits; a separate context line
/// would be separated from it once another check also reports.
const GIT_UNUSABLE: &str = "PACKAGED SOURCE: git is not usable here, so the tracked set cannot be read, and git is required to compare a package with what it tracks";

/// C8, C9: the packaged-source assertion compares a package with git's history, so it
/// applies only when git's repository is the working tree under test.
///
/// Two situations are kept apart, because they look alike from outside.
///
/// - **Another repository** is a legitimate state, and is not applicable. `release-gate`
///   extracts the archive under `.git-exclude/tmp/`, inside this repository. git there
///   resolves against the outer repository and answers with a plausible, empty listing.
///   That archive has no history to be consistent with, and the check runs at the
///   repository root in the source-tree suite of the same run.
/// - **An unusable git** is a failure. No legitimate environment here has no git:
///   `release-gate` needs git to build its archive, and `xtask` is not published, so no
///   consumer runs this gate from unpacked sources. If git is absent or broken, this
///   assertion has nothing to compare with, so it must say so rather than pass.
///
/// The guard is the toplevel, not `git rev-parse --is-inside-work-tree`, which is `true`
/// inside the in-repository extraction and so would not tell the two apart.
fn tree_under_test() -> Applicability {
    let Ok(cwd) = std::env::current_dir().and_then(|dir| dir.canonicalize()) else {
        return Applicability::GitUnusable("the working directory cannot be resolved".to_owned());
    };
    let toplevel = super::util::command_stdout("git", &["rev-parse", "--show-toplevel"]);
    applicability(&cwd, toplevel.as_deref().map(str::trim))
}

/// Pure core of `tree_under_test`. `toplevel` is git's answer, or `None` if git failed.
fn applicability(cwd: &std::path::Path, toplevel: Option<&str>) -> Applicability {
    let Some(toplevel) = toplevel else {
        return Applicability::GitUnusable(
            "git did not report a repository for this directory (git is absent, broken, or this is not a repository)".to_owned(),
        );
    };
    let Ok(top) = std::path::Path::new(toplevel).canonicalize() else {
        return Applicability::GitUnusable(format!(
            "git reported the toplevel `{toplevel}`, which does not resolve"
        ));
    };
    if top == cwd {
        Applicability::Applicable
    } else {
        Applicability::OtherRepository(format!(
            "git's repository is `{}`, not this working tree, so the checkout sits inside another repository",
            top.display()
        ))
    }
}

/// C1.1: the packaged `src/` set equals the tracked `src/` set, and the package
/// carries `LICENSE` and `README.md`. Hermetic: `--offline`. `--allow-dirty` is passed
/// so the listing does not depend on the working tree being clean; the comparison
/// against `git ls-files` is what carries the proof.
fn packaged_source_findings(name: &str) -> Vec<String> {
    let output = std::process::Command::new(env!("CARGO"))
        .args([
            "package",
            "-p",
            name,
            "--list",
            "--offline",
            "--allow-dirty",
        ])
        .output();
    let listing = match output {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).into_owned()
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let reason = first_error_line(&stderr).unwrap_or("no error line reported");
            return vec![format!(
                "PACKAGED SOURCE: `cargo package -p {name} --list --offline --allow-dirty` refused: {reason}"
            )];
        }
        Err(error) => {
            return vec![format!(
                "PACKAGED SOURCE: cannot run `cargo package -p {name} --list`: {error}"
            )];
        }
    };
    let Some(tracked) =
        super::util::command_stdout("git", &["ls-files", &format!("crates/{name}/src")])
    else {
        return vec![format!(
            "PACKAGED SOURCE: `git ls-files` failed for `{name}`"
        )];
    };
    let listed: Vec<&str> = listing.lines().map(str::trim).collect();
    let tracked: Vec<&str> = tracked
        .lines()
        .filter_map(|line| line.strip_prefix(&format!("crates/{name}/")))
        .collect();
    packaged_source_check(name, &listed, &tracked)
}

/// The first line cargo reports as an error, without its `error: ` prefix.
fn first_error_line(stderr: &str) -> Option<&str> {
    stderr
        .lines()
        .find_map(|line| line.strip_prefix("error: "))
        .map(str::trim)
}

/// Pure core of `packaged_source_findings`.
fn packaged_source_check(name: &str, listed: &[&str], tracked: &[&str]) -> Vec<String> {
    use std::collections::BTreeSet;
    let mut findings = Vec::new();
    let packaged_src: BTreeSet<&str> = listed
        .iter()
        .copied()
        .filter(|path| path.starts_with("src/"))
        .collect();
    let tracked_src: BTreeSet<&str> = tracked
        .iter()
        .copied()
        .filter(|path| path.starts_with("src/"))
        .collect();
    if tracked_src.is_empty() {
        findings.push(format!(
            "PACKAGED SOURCE: git tracks no `src/` file for `{name}`, so there is nothing to compare the package with"
        ));
    }
    for path in packaged_src.difference(&tracked_src) {
        findings.push(format!(
            "PACKAGED SOURCE: `{name}` packages `{path}`, which git does not track under src/"
        ));
    }
    for path in tracked_src.difference(&packaged_src) {
        findings.push(format!(
            "PACKAGED SOURCE: `{name}` tracks `{path}` under src/, but the package omits it"
        ));
    }
    for required in ["LICENSE", "README.md"] {
        if !listed.contains(&required) {
            findings.push(format!(
                "PACKAGED SOURCE: `{name}`'s package omits `{required}`"
            ));
        }
    }
    findings
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

    #[test]
    fn a_feature_gating_code_is_not_inert_and_doc_labels_do_not_count() {
        let source = "#[cfg(feature = \"dense\")]\nfn a() {}\n\
                      #[cfg_attr(docsrs, doc(cfg(feature = \"sparse\")))]\n\
                      if cfg!(feature = \"serde\") {}\n";
        assert_eq!(super::feature_sites(source, "dense"), 1);
        assert_eq!(super::feature_sites(source, "sparse"), 0);
        assert_eq!(super::feature_sites(source, "serde"), 1);
        assert_eq!(super::feature_sites(source, "libm"), 0);
    }

    fn feature_list(names: &[&str]) -> Vec<String> {
        names.iter().map(|n| (*n).to_owned()).collect()
    }

    #[test]
    fn an_unregistered_inert_feature_fails_and_a_registered_one_passes() {
        let registry = [("x", "reserved-one")];
        let declared = feature_list(&["default", "reserved-one", "live"]);
        let sources = vec!["#[cfg(feature = \"live\")] fn a() {}".to_owned()];
        assert!(super::inert_feature_check("x", &declared, &sources, &registry).is_empty());

        let unregistered = feature_list(&["default", "reserved-one", "live", "new-inert"]);
        let findings = super::inert_feature_check("x", &unregistered, &sources, &registry);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(
            findings[0].contains("`new-inert`") && findings[0].contains("INERT FEATURE"),
            "{findings:?}"
        );
    }

    #[test]
    fn a_registered_feature_that_gains_a_gate_fails_until_deregistered() {
        let registry = [("x", "went-live")];
        let declared = feature_list(&["went-live"]);
        let sources = vec!["#[cfg(feature = \"went-live\")] fn a() {}".to_owned()];
        let findings = super::inert_feature_check("x", &declared, &sources, &registry);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(
            findings[0].contains("RESERVED FEATURE IS LIVE"),
            "{findings:?}"
        );
    }

    #[test]
    fn a_registry_entry_for_an_undeclared_feature_fails() {
        let registry = [("x", "gone")];
        let findings = super::inert_feature_check("x", &feature_list(&[]), &[], &registry);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].contains("`gone`"), "{findings:?}");
    }

    #[test]
    fn the_registry_matches_the_real_tree_in_both_directions() {
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/..");
        for name in PUBLISHED_CRATES {
            let dir = format!("{root}/crates/{name}");
            let manifest = std::fs::read_to_string(format!("{dir}/Cargo.toml")).expect("manifest");
            let declared = super::declared_features(&manifest).expect("features");
            let sources = super::read_rust_sources(&format!("{dir}/src")).expect("sources");
            assert!(
                super::inert_feature_check(
                    name,
                    &declared,
                    &sources,
                    super::RESERVED_INERT_FEATURES
                )
                .is_empty(),
                "{name}"
            );
        }
    }

    #[test]
    fn a_packaged_file_the_tree_does_not_track_is_named() {
        let listed = ["src/lib.rs", "src/extra.rs", "LICENSE", "README.md"];
        let tracked = ["src/lib.rs"];
        let findings = super::packaged_source_check("x", &listed, &tracked);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].contains("`src/extra.rs`"), "{findings:?}");
    }

    #[test]
    fn a_tracked_file_the_package_omits_is_named() {
        let listed = ["src/lib.rs", "LICENSE", "README.md"];
        let tracked = ["src/lib.rs", "src/missing.rs"];
        let findings = super::packaged_source_check("x", &listed, &tracked);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].contains("`src/missing.rs`"), "{findings:?}");
    }

    #[test]
    fn a_package_without_license_or_readme_is_refused() {
        let listed = ["src/lib.rs"];
        let tracked = ["src/lib.rs"];
        let findings = super::packaged_source_check("x", &listed, &tracked);
        assert_eq!(findings.len(), 2, "{findings:?}");
    }

    #[test]
    fn matching_sets_pass() {
        let listed = ["src/lib.rs", "LICENSE", "README.md", "Cargo.toml"];
        let tracked = ["src/lib.rs"];
        assert!(super::packaged_source_check("x", &listed, &tracked).is_empty());
    }

    #[test]
    fn cargo_refusal_reason_is_carried_into_the_finding() {
        let stderr = "error: 1 files in the working directory contain changes that were not yet committed into git:\n\nCargo.toml\n";
        assert_eq!(
            super::first_error_line(stderr),
            Some(
                "1 files in the working directory contain changes that were not yet committed into git:"
            )
        );
        assert_eq!(super::first_error_line("warning: nothing\n"), None);
    }

    fn features_readme(rows: &str) -> String {
        format!(
            "# x\n\n## Features\n\nOff unless marked *on*.\n\n| Feature | Default | What it does |\n|---|---|---|\n{rows}\nSee the workspace.\n"
        )
    }

    #[test]
    fn a_features_section_naming_every_feature_with_the_reserved_phrase_passes() {
        let readme = features_readme(
            "| `live` | off | does a thing |\n| `dormant` | off | reserved; no effect yet |\n",
        );
        let declared = vec![
            "default".to_owned(),
            "live".to_owned(),
            "dormant".to_owned(),
        ];
        let registry = [("x", "dormant")];
        assert!(super::features_section_check("x", &readme, &declared, &registry).is_empty());
    }

    #[test]
    fn a_missing_section_is_reported() {
        let findings = super::features_section_check("x", "# x\n\nno section\n", &[], &[]);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(
            findings[0].contains("no `## Features` section"),
            "{findings:?}"
        );
    }

    #[test]
    fn a_declared_feature_missing_from_the_section_is_named() {
        let readme = features_readme("| `live` | off | does a thing |\n");
        let declared = vec!["live".to_owned(), "forgotten".to_owned()];
        let findings = super::features_section_check("x", &readme, &declared, &[]);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].contains("`forgotten`"), "{findings:?}");
    }

    #[test]
    fn a_reserved_feature_that_drops_the_phrase_is_named() {
        let readme = features_readme("| `dormant` | off | does a thing now |\n");
        let declared = vec!["dormant".to_owned()];
        let registry = [("x", "dormant")];
        let findings = super::features_section_check("x", &readme, &declared, &registry);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(
            findings[0].contains("lacks `reserved; no effect yet`"),
            "{findings:?}"
        );
    }

    #[test]
    fn a_default_feature_needs_no_row() {
        let readme = features_readme("| `live` | off | does a thing |\n");
        let declared = vec!["default".to_owned(), "live".to_owned()];
        assert!(super::features_section_check("x", &readme, &declared, &[]).is_empty());
    }

    #[test]
    fn the_real_readmes_pass_the_features_section_rules() {
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/..");
        for name in super::PUBLISHED_CRATES {
            let dir = format!("{root}/crates/{name}");
            let manifest = std::fs::read_to_string(format!("{dir}/Cargo.toml")).expect("manifest");
            let declared = super::declared_features(&manifest).expect("features");
            let readme = std::fs::read_to_string(format!("{dir}/README.md")).expect("readme");
            assert!(
                super::features_section_check(
                    name,
                    &readme,
                    &declared,
                    super::RESERVED_INERT_FEATURES
                )
                .is_empty(),
                "{name}: {:?}",
                super::features_section_check(
                    name,
                    &readme,
                    &declared,
                    super::RESERVED_INERT_FEATURES
                )
            );
        }
    }

    #[test]
    fn a_toplevel_equal_to_the_working_directory_is_applicable() {
        let cwd = std::env::temp_dir()
            .canonicalize()
            .expect("temp dir resolves");
        assert_eq!(
            super::applicability(&cwd, cwd.to_str()),
            super::Applicability::Applicable
        );
    }

    #[test]
    fn a_toplevel_elsewhere_is_not_applicable_and_names_the_repository() {
        let cwd = std::env::temp_dir()
            .canonicalize()
            .expect("temp dir resolves");
        let elsewhere = std::env::current_dir()
            .expect("cwd")
            .canonicalize()
            .expect("cwd resolves");
        match super::applicability(&cwd, elsewhere.to_str()) {
            super::Applicability::OtherRepository(reason) => {
                assert!(reason.contains("another repository"), "{reason}")
            }
            other => panic!("expected OtherRepository, got {other:?}"),
        }
    }

    #[test]
    fn an_unresolvable_toplevel_is_a_failure_not_a_skip() {
        let cwd = std::env::temp_dir()
            .canonicalize()
            .expect("temp dir resolves");
        assert!(matches!(
            super::applicability(&cwd, Some("/no/such/toplevel/for/c9")),
            super::Applicability::GitUnusable(_)
        ));
    }

    #[test]
    fn a_git_that_reports_nothing_is_a_failure_not_a_skip() {
        let cwd = std::env::temp_dir()
            .canonicalize()
            .expect("temp dir resolves");
        assert!(matches!(
            super::applicability(&cwd, None),
            super::Applicability::GitUnusable(_)
        ));
    }

    #[test]
    fn an_empty_tracked_src_set_for_a_crate_with_sources_is_a_failure() {
        let listed = ["src/lib.rs", "LICENSE", "README.md"];
        let tracked: [&str; 0] = [];
        let findings = super::packaged_source_check("x", &listed, &tracked);
        assert!(
            findings.iter().any(|f| f.contains("tracks no `src/` file")),
            "{findings:?}"
        );
    }

    #[test]
    fn the_real_checkout_is_the_tree_under_test_when_run_from_the_root() {
        let cwd = std::env::current_dir().expect("cwd");
        // cargo runs tests from the crate directory; the repository root is its parent.
        let root = cwd.parent().expect("parent").canonicalize().expect("root");
        assert_eq!(
            super::applicability(&root, root.to_str()),
            super::Applicability::Applicable
        );
    }
}
