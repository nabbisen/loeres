//! `doc-currency` — bounded RFC 020 documentation currency assertions.
//!
//! This gate checks stable metadata, lifecycle/index facts, navigation, and a
//! small explicit stale-phrase ledger. It deliberately does not attempt to
//! infer arbitrary prose semantics; human architecture review remains required.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use super::check_rfcs::mask_code_spans;
use super::link_audit;

/// RFC 048: the three normative apex specs plus the four current-facing
/// documents that make release-scope claims of their own — a roadmap, a book
/// introduction, the book's specifications index, and the threat model — and
/// previously held none. Each carries the ordinary apex block (RFC 024 §3,
/// RFC 048 §3), byte-identical to the others, so a document making a
/// release-scope claim can no longer go stale without this gate seeing it.
const APEX_DOCS: &[&str] = &[
    "docs/specs/loeres-requirements-v1.md",
    "docs/specs/loeres-external-design-v1.md",
    "docs/specs/loeres-roadmap-milestones-v1.md",
    "ROADMAP.md",
    "docs/src/introduction.md",
    "docs/src/specifications.md",
    "docs/src/threat-model.md",
];

const RFC_FOLDERS: &[&str] = &["proposed", "accepted", "done", "archive"];

const LEGACY_APEX_MARKER: &str = "**RFC 020 shared currency metadata (draft).**";

/// Retired RFC 021 conditional apex marker. RFC 024 §11.3 retires the
/// one-release apparatus and Amendment 2 (§0.6) removes its module outright, so
/// this constant is the single owner of the string: the guard that the retired
/// marker never reappears in a tracked apex document.
const RETIRED_CONDITIONAL_APEX_MARKER: &str =
    "**RFC 021 conditional release-finalization metadata.**";

/// RFC 024 ordinary post-release apex marker.
///
/// The block it introduces records this tree's identity and the release this
/// documentation was last reconciled against — lineage, never release status
/// (Amendment 1 §0.1). Conflating identity and status was blocker B6; RFC 021
/// separated them for one finalization window, and RFC 024 makes the
/// separation the ordinary, permanent form.
const ORDINARY_APEX_MARKER: &str = "**Release currency metadata.**";

const STALE_PHRASES: &[&str] = &[
    "current as of v0.13.1",
    "current as of repository release v0.13.1",
    "current v0.13.1",
    "no std-side solver kernel exists yet",
    "release-gate remains an alias",
    "phase 0 skeleton",
    "all solver engines remain in design",
    "workspace is treated as reset-required",
    // RFC 026 made the supply-chain gate enforced; these are the exact
    // phrasings that carried "not configured" from v0.14 until then, kept so
    // the claim cannot return to a current-facing document.
    "cargo audit/deny: n/a",
    "cargo audit/cargo deny: n/a",
    "supply-chain vulnerability/license checks and enforced artifact",
    "dependency vulnerability and license-policy scanning are not enforced",
];

/// Documents that must no longer carry present-tense claims that the RFC 021
/// conditional apparatus is active, pending, or unresolved, now that `0.20.2`
/// has released (RFC 024 Amendment 1; review 036 blocker B1). Historical
/// descriptions of the protocol (framed as past events) are unaffected; only
/// the specific stale markers below are checked.
const CONDITIONAL_CURRENT_DOCS: &[&str] = &[
    "CHANGELOG.md",
    "README.md",
    "ROADMAP.md",
    "docs/src/introduction.md",
    "docs/src/threat-model.md",
    "docs/src/recovery-roadmap.md",
    "docs/src/specifications.md",
    "docs/specs/loeres-reconciliation-traceability-v020.md",
    "docs/specs/loeres-requirements-v1.md",
    "docs/specs/loeres-external-design-v1.md",
    "docs/specs/loeres-roadmap-milestones-v1.md",
    "rfcs/README.md",
    "rfcs/done/019-release-integrity-and-msrv-recovery.md",
    "rfcs/done/020-normative-documentation-authority-and-currency.md",
    "rfcs/done/021-conditional-release-finalization.md",
    "rfcs/handoffs/019-release-integrity-and-msrv-recovery/implementation-handoff.md",
    "rfcs/handoffs/020-normative-documentation-authority-and-currency/implementation-handoff.md",
    "rfcs/handoffs/021-conditional-release-finalization/implementation-handoff.md",
];

const CONDITIONAL_BOUNDARY_MARKERS: &[&str] = &[
    "Before external predicate `P` succeeds",
    "After `P` succeeds",
    "Tracked bytes alone do not establish whether `P` occurred",
    "GitHub release creation, registry publication, and certification remain separately authorized",
];

const CONDITIONAL_STALE_PHRASES: &[&str] = &[
    "The last externally activated repository release is v0.20.0",
    "Package/release readiness remains No-Go pending",
    "The repository remains No-Go for release/readiness claims",
    "Exact clean-tree Q2 evidence and review remain pending",
    "The owner must first make this atomic Q2 tree one clean exact revision",
    "actual release remain unestablished and unauthorized",
    // Extended below (review 036 B1) to match, verbatim, the specific
    // current-facing phrasing found and removed from each document. The
    // check is case-sensitive after whitespace normalization, so entries
    // mirror the deleted text exactly rather than a paraphrase.
    "implementation-complete and conditionally staged",
    "Implemented (conditional finalization for 0.20.2)",
    "Last released repository release: **",
    "do not prove activation",
    "does not prove which RFC 021 predicate branch applies",
    "not proof of which RFC 021 predicate branch applies",
    "RFC 021's conditional Status",
    "conditionally staged in `done/`",
];

#[derive(Clone, Debug, Eq, PartialEq)]
struct ApexCurrency {
    /// The version this tree carries. Bound to `workspace.package.version`.
    release: String,
    /// The repository release this documentation was last reconciled
    /// against (RFC 020 §11.3). Always strictly less than `release`, and
    /// must name a `CHANGELOG.md` record marked released (Amendment 1 §0.1).
    last_reconciled: String,
    normalized_block: String,
}

/// Parse `vX.Y.Z` or `X.Y.Z` into comparable components.
fn parse_version_triple(value: &str) -> Option<(u32, u32, u32)> {
    let trimmed = value.strip_prefix('v').unwrap_or(value);
    let mut parts = trimmed.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

/// An `NNN-slug.md` RFC file name's number, or `None` if the name does not
/// match that canonical shape. Mirrors `check_rfcs::valid_rfc_file_name`.
fn numbered_rfc_file(name: &str) -> Option<u32> {
    let stem = name.strip_suffix(".md")?;
    let (number, slug) = stem.split_once('-')?;
    if number.len() != 3 || !number.chars().all(|c| c.is_ascii_digit()) || slug.is_empty() {
        return None;
    }
    number.parse().ok()
}

/// Render an ascending set of RFC numbers as the canonical compact scope
/// string (Amendment 1 §0.2): every maximal run of two or more consecutive
/// numbers collapses to `NNN-NNN`; an isolated number stands alone. There is
/// exactly one rendering per set.
fn render_scope(numbers: &BTreeSet<u32>) -> String {
    let mut parts = Vec::new();
    let mut iter = numbers.iter().copied().peekable();
    while let Some(start) = iter.next() {
        let mut end = start;
        while iter.peek() == Some(&(end + 1)) {
            end = iter.next().expect("peeked value exists");
        }
        if end > start {
            parts.push(format!("{start:03}-{end:03}"));
        } else {
            parts.push(format!("{start:03}"));
        }
    }
    format!("RFCs {}", parts.join(", "))
}

/// The exact set of implemented RFC numbers under `<root>/rfcs/done`,
/// excluding RFC 000 (Amendment 1 §0.2: it governs the RFC directory itself,
/// not product scope), rendered canonically.
///
/// A maximum is not a set: this binds every number actually present, so a
/// withdrawn or accepted-but-not-done RFC below the highest implemented
/// number cannot be certified as implemented (review 036 blocker B2). Any
/// file under `done/` that is not a valid `NNN-slug.md` name fails closed
/// rather than being silently skipped.
fn implemented_scope_at(root: &Path, errors: &mut Vec<String>) -> Option<String> {
    let entries = match fs::read_dir(root.join("rfcs").join("done")) {
        Ok(entries) => entries,
        Err(error) => {
            errors.push(format!("RFC SCOPE: cannot read rfcs/done: {error}"));
            return None;
        }
    };
    let mut numbers = BTreeSet::new();
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            errors.push("RFC SCOPE: rfcs/done contains a non-UTF-8 file name".to_owned());
            return None;
        };
        if name == ".gitkeep" {
            continue;
        }
        let Some(number) = numbered_rfc_file(name) else {
            errors.push(format!(
                "RFC SCOPE: rfcs/done/{name} is not a valid `NNN-slug.md` name"
            ));
            return None;
        };
        if number != 0 {
            numbers.insert(number);
        }
    }
    if numbers.is_empty() {
        errors.push("RFC SCOPE: rfcs/done contains no numbered RFC".to_owned());
        return None;
    }
    Some(render_scope(&numbers))
}

fn implemented_scope(errors: &mut Vec<String>) -> Option<String> {
    implemented_scope_at(Path::new("."), errors)
}

const CHANGELOG_PATH: &str = "CHANGELOG.md";

/// Permitted `**Release status:**` values (developer handoff §4.2), in the
/// order checked. Order does not affect correctness, since each candidate is
/// matched as a whole token, but withdrawn/unreleased are checked first so
/// the more specific multi-word value is never shadowed by a shorter one.
const CHANGELOG_STATUS_VALUES: &[&str] = &["withdrawn candidate", "unreleased", "released"];

/// The `**Release status:**` value recorded immediately under a version's
/// `## [X]` heading in the given `CHANGELOG.md` source.
///
/// Pure function of the supplied source so tests can exercise every required
/// case (missing heading, missing marker, each permitted value) without a
/// real file on disk.
fn parse_changelog_release_status(source: &str, version: &str) -> Result<String, String> {
    let heading_prefix = format!("## [{version}]");
    let lines = source.lines().collect::<Vec<_>>();
    let heading_index = lines.iter().position(|line| {
        line.strip_prefix(&heading_prefix)
            .map(|suffix| suffix.is_empty() || suffix.starts_with(char::is_whitespace))
            .unwrap_or(false)
    });
    let Some(heading_index) = heading_index else {
        return Err(format!("no `{heading_prefix}` heading"));
    };
    let Some(status_line) = lines[heading_index + 1..]
        .iter()
        .find(|line| !line.trim().is_empty())
    else {
        return Err(format!(
            "`{heading_prefix}` heading has no release-status line"
        ));
    };
    let Some(value) = status_line.trim().strip_prefix("**Release status:** ") else {
        return Err(format!(
            "`{heading_prefix}` heading's first line is not a `**Release status:**` marker"
        ));
    };
    for candidate in CHANGELOG_STATUS_VALUES {
        if value == *candidate || value.starts_with(&format!("{candidate} (")) {
            return Ok((*candidate).to_owned());
        }
    }
    Err(format!(
        "`{heading_prefix}` release status `{value}` is not one of {CHANGELOG_STATUS_VALUES:?}"
    ))
}

fn changelog_marks_released(source: &str, version: &str) -> Result<(), String> {
    match parse_changelog_release_status(source, version) {
        Ok(status) if status == "released" => Ok(()),
        Ok(status) => Err(format!(
            "CHANGELOG.md marks `{version}` `{status}`, not `released`"
        )),
        Err(error) => Err(format!("CHANGELOG.md: {error}")),
    }
}

/// The repository release this documentation is currently reconciled against,
/// as recorded by the ordinary apex block.
///
/// Used by the release-candidate preflight so intended-tag validation depends
/// on reviewed normative documentation rather than one-release metadata.
pub(crate) fn ordinary_last_reconciled_release() -> Result<String, String> {
    let path = APEX_DOCS
        .first()
        .ok_or_else(|| "no apex document configured".to_owned())?;
    let source =
        fs::read_to_string(path).map_err(|error| format!("cannot read {path}: {error}"))?;
    let block = extract_currency_block(&source, ORDINARY_APEX_MARKER)?;
    bounded_value(
        &normalize_whitespace(&block),
        "Last reconciled repository release: **",
        "**",
    )
    .ok_or_else(|| format!("{path} has no last-reconciled field"))
}

pub fn run() -> bool {
    eprintln!("[doc-currency] bounded RFC 020 documentation assertions");
    let errors = validate_repository();
    for error in &errors {
        eprintln!("  {error}");
    }
    let ok = errors.is_empty();
    eprintln!("[doc-currency] {}", if ok { "PASS" } else { "FAIL" });
    ok
}

fn validate_repository() -> Vec<String> {
    let mut errors = Vec::new();
    let expected_release = workspace_release_marker(&mut errors);
    // Derived once and threaded to both callers (RFC 052 corrections §4):
    // `rfcs/done/` is read by `fs::read_dir` exactly once per run, and a
    // malformed file under it is reported once, not once per caller.
    let implemented_scope = implemented_scope(&mut errors);
    let last_reconciled = check_apex_currency(
        expected_release.as_deref(),
        implemented_scope.as_deref(),
        &mut errors,
    );
    check_rfc_index(&mut errors);
    check_root_roadmap(&mut errors);
    check_book_navigation(&mut errors);
    check_release_local_paths(&mut errors);
    check_stale_ledger(&mut errors);
    check_conditional_current_prose(&mut errors);
    check_readme_landing(&mut errors);
    check_readme_requirements(last_reconciled.as_deref(), &mut errors);
    check_import_model_requirements(&mut errors);
    check_release_table(
        last_reconciled.as_deref(),
        implemented_scope.as_deref(),
        &mut errors,
    );
    check_phase_sequence(&mut errors);
    check_example_guides(&mut errors);
    errors
}

fn workspace_release_marker(errors: &mut Vec<String>) -> Option<String> {
    let source = match fs::read_to_string("Cargo.toml") {
        Ok(source) => source,
        Err(error) => {
            errors.push(format!("CARGO: cannot read Cargo.toml: {error}"));
            return None;
        }
    };
    let document = match source.parse::<toml::Value>() {
        Ok(document) => document,
        Err(error) => {
            errors.push(format!("CARGO: cannot parse Cargo.toml: {error}"));
            return None;
        }
    };
    let version = document
        .get("workspace")
        .and_then(|workspace| workspace.get("package"))
        .and_then(|package| package.get("version"))
        .and_then(toml::Value::as_str);
    match version {
        Some(version) => Some(format!("v{version}")),
        None => {
            errors.push("CARGO: missing workspace.package.version".to_owned());
            None
        }
    }
}

/// Returns the apex block's `Last reconciled repository release` field, raw
/// and without a `v` prefix — the value `check_readme_requirements` (RFC 049
/// S4) binds its `MAJOR.MINOR` requirement to, so it is parsed once here
/// rather than a second time there.
fn check_apex_currency(
    expected_release: Option<&str>,
    scope: Option<&str>,
    errors: &mut Vec<String>,
) -> Option<String> {
    let changelog_source = match fs::read_to_string(CHANGELOG_PATH) {
        Ok(source) => Some(source),
        Err(error) => {
            errors.push(format!("MISSING/UNREADABLE: {CHANGELOG_PATH}: {error}"));
            None
        }
    };
    let mut found = Vec::new();
    for path in APEX_DOCS {
        let source = match read_required(path, errors) {
            Some(source) => source,
            None => continue,
        };
        let (Some(scope), Some(changelog_source)) = (scope, changelog_source.as_deref()) else {
            continue;
        };
        match parse_ordinary_apex_currency(&source, scope, changelog_source) {
            Ok(currency) => {
                if let Some(expected) = expected_release {
                    if currency.release != expected {
                        errors.push(format!(
                            "APEX RELEASE: {path} has `{}`, expected workspace `{expected}`",
                            currency.release
                        ));
                    }
                }
                found.push(((*path).to_owned(), currency));
            }
            Err(error) => errors.push(format!("APEX METADATA: {path}: {error}")),
        }
    }

    if let Some((_, first)) = found.first() {
        for (path, currency) in found.iter().skip(1) {
            if currency != first {
                errors.push(format!(
                    "APEX MISMATCH: {path} shared currency block differs from the first apex document (release `{}` vs `{}`)",
                    currency.release, first.release,
                ));
            }
        }
    }

    found
        .first()
        .map(|(_, currency)| currency.last_reconciled.clone())
}

/// Parse the RFC 024 ordinary apex block, as amended by Amendment 1 §0.1/§0.2.
///
/// The block records identity and lineage only, never release status: it
/// does not hard-code a scope or a version, and equality between `this tree`
/// and `last reconciled` is never valid (review 036 B3). The caller binds
/// `release` to the workspace version.
fn parse_ordinary_apex_currency(
    source: &str,
    expected_scope: &str,
    changelog_source: &str,
) -> Result<ApexCurrency, String> {
    if source.contains(LEGACY_APEX_MARKER) {
        return Err("retired RFC 020 draft metadata must be absent".to_owned());
    }
    if source.contains(RETIRED_CONDITIONAL_APEX_MARKER) {
        return Err("retired RFC 021 conditional metadata must be absent".to_owned());
    }

    let block = extract_currency_block(source, ORDINARY_APEX_MARKER)?;
    let normalized_block = normalize_whitespace(&block);

    let release = bounded_value(&normalized_block, "This tree: **", "**")
        .ok_or_else(|| "missing this-tree field".to_owned())?;
    let last_reconciled = bounded_value(
        &normalized_block,
        "Last reconciled repository release: **",
        "**",
    )
    .ok_or_else(|| "missing last-reconciled field".to_owned())?;

    let Some(tree_triple) = parse_version_triple(&release) else {
        return Err(format!("this-tree `{release}` is not `X.Y.Z`"));
    };
    let Some(reconciled_triple) = parse_version_triple(&last_reconciled) else {
        return Err(format!(
            "last-reconciled `{last_reconciled}` is not `X.Y.Z`"
        ));
    };
    // Strict, unconditionally: equality would let a tracked tree claim to be
    // a specific past release, which is exactly the identity conflation this
    // RFC exists to remove (Amendment 1 §0.1). There is no release-mode
    // exception and no authenticated context to establish.
    if tree_triple <= reconciled_triple {
        return Err(format!(
            "this-tree `{release}` does not strictly exceed last-reconciled `{last_reconciled}`"
        ));
    }
    changelog_marks_released(changelog_source, &last_reconciled)?;

    let scope_field = format!("Implemented scope: **{expected_scope}**");
    if !normalized_block.contains(&scope_field) {
        return Err(format!(
            "missing or stale scope field; expected `{scope_field}`"
        ));
    }

    Ok(ApexCurrency {
        release: format!("v{release}"),
        last_reconciled,
        normalized_block,
    })
}

fn extract_currency_block(source: &str, marker: &str) -> Result<String, String> {
    if source.matches(marker).count() != 1 {
        return Err(format!("expected exactly one `{marker}` metadata marker"));
    }

    let lines = source.lines().collect::<Vec<_>>();
    let Some(start) = lines.iter().position(|line| line.contains(marker)) else {
        return Err(format!("missing `{marker}` metadata marker"));
    };
    if !lines[start].trim_start().starts_with('>') {
        return Err(format!("`{marker}` metadata marker is not in a blockquote"));
    }

    let mut block = Vec::new();
    for line in lines.iter().skip(start) {
        if !line.trim_start().starts_with('>') {
            break;
        }
        if line.trim_start().trim_start_matches('>').trim().is_empty() {
            break;
        }
        block.push(*line);
    }
    if block.is_empty() {
        Err("shared draft metadata block is empty".to_owned())
    } else {
        Ok(block.join("\n"))
    }
}

fn check_rfc_index(errors: &mut Vec<String>) {
    let index = match read_required("rfcs/README.md", errors) {
        Some(index) => index,
        None => return,
    };

    for folder in RFC_FOLDERS {
        let root = Path::new("rfcs").join(folder);
        let entries = match fs::read_dir(&root) {
            Ok(entries) => entries,
            Err(error) => {
                errors.push(format!(
                    "RFC DIRECTORY: cannot read {}: {error}",
                    root.display()
                ));
                continue;
            }
        };
        let mut paths = Vec::new();
        for entry in entries {
            match entry {
                Ok(entry) => {
                    let path = entry.path();
                    if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
                        paths.push(path);
                    }
                }
                Err(error) => errors.push(format!(
                    "RFC DIRECTORY ENTRY: cannot read entry under {}: {error}",
                    root.display()
                )),
            }
        }
        paths.sort();
        for path in paths {
            check_rfc_index_entry(folder, &path, &index, errors);
        }
    }
}

fn check_rfc_index_entry(folder: &str, path: &Path, index: &str, errors: &mut Vec<String>) {
    let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
        errors.push(format!("RFC INDEX: invalid UTF-8 path {}", path.display()));
        return;
    };
    let number = file_name.chars().take(3).collect::<String>();
    let link = format!("[{number}]({folder}/{file_name})");
    let matching = index
        .lines()
        .filter(|line| line.contains(&link))
        .collect::<Vec<_>>();
    if matching.len() != 1 {
        errors.push(format!(
            "RFC INDEX: expected one row for `{link}`, found {}",
            matching.len()
        ));
        return;
    }
    let index_status = index_row_status(matching[0]);
    if !index_status_matches(folder, matching[0]) {
        errors.push(format!(
            "RFC INDEX STATUS: `{link}` row does not match `{folder}` lifecycle"
        ));
    }

    if folder == "accepted" {
        let source = match fs::read_to_string(path) {
            Ok(source) => source,
            Err(error) => {
                errors.push(format!(
                    "RFC ACCEPTED: cannot read {}: {error}",
                    path.display()
                ));
                return;
            }
        };
        let source_status = source
            .lines()
            .find(|line| line.starts_with("**Status.**"))
            .and_then(|line| line.strip_prefix("**Status.** "));
        let source_freeze = source_status.and_then(parse_design_freeze);
        let index_freeze = index_status.and_then(parse_design_freeze);
        match (source_freeze, index_freeze) {
            (Some(source_freeze), Some(index_freeze)) if source_freeze == index_freeze => {}
            (Some(source_freeze), Some(index_freeze)) => errors.push(format!(
                "RFC ACCEPTED: {} source freeze `{source_freeze}` differs from index `{index_freeze}`",
                path.display()
            )),
            _ => errors.push(format!(
                "RFC ACCEPTED: {} lacks complete matching YYYY-MM-DD design-freeze metadata",
                path.display()
            )),
        }
    }
}

fn index_status_matches(folder: &str, row: &str) -> bool {
    let Some(status) = index_row_status(row) else {
        return false;
    };
    match folder {
        "proposed" => status.starts_with("Proposed"),
        "accepted" => parse_design_freeze(status).is_some(),
        "done" => status.starts_with("Implemented"),
        "archive" => status.starts_with("Withdrawn") || status.starts_with("Superseded"),
        _ => false,
    }
}

fn index_row_status(row: &str) -> Option<&str> {
    let fields = row
        .split('|')
        .map(str::trim)
        .filter(|field| !field.is_empty())
        .collect::<Vec<_>>();
    fields.get(2).copied()
}

fn parse_design_freeze(status: &str) -> Option<&str> {
    let date = status
        .strip_prefix("Accepted (design frozen ")?
        .strip_suffix(')')?;
    if valid_date(date) { Some(date) } else { None }
}

fn valid_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return false;
    }
    if !bytes
        .iter()
        .enumerate()
        .all(|(index, byte)| index == 4 || index == 7 || byte.is_ascii_digit())
    {
        return false;
    }
    let Ok(year) = value[0..4].parse::<u32>() else {
        return false;
    };
    let Ok(month) = value[5..7].parse::<u32>() else {
        return false;
    };
    let Ok(day) = value[8..10].parse::<u32>() else {
        return false;
    };
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year % 400 == 0 || (year % 4 == 0 && year % 100 != 0) => 29,
        2 => 28,
        _ => return false,
    };
    year >= 2000 && day >= 1 && day <= days
}

fn check_root_roadmap(errors: &mut Vec<String>) {
    let source = match read_required("ROADMAP.md", errors) {
        Some(source) => source,
        None => return,
    };
    // Normalized: a bounded marker must survive ordinary re-wrapping, otherwise
    // the check enforces line breaks rather than content.
    let normalized = normalize_whitespace(&source);
    for marker in [
        "RFC 019",
        "RFC 020",
        "Accepted (design frozen",
        "No-Go",
        "release-gate",
        "fail-closed",
        "reviewed S2 apex reconciliation",
        "owner-durable",
    ] {
        if !normalized.contains(&normalize_whitespace(marker)) {
            errors.push(format!(
                "ROADMAP RECOVERY: missing bounded marker `{marker}`"
            ));
        }
    }
}

fn check_book_navigation(errors: &mut Vec<String>) {
    let source = match read_required("docs/src/SUMMARY.md", errors) {
        Some(source) => source,
        None => return,
    };
    for marker in [
        "[Threat Model](threat-model.md)",
        "[Architecture Recovery Roadmap](recovery-roadmap.md)",
        "[Specifications & RFCs](specifications.md)",
    ] {
        if !source.contains(marker) {
            errors.push(format!("MDBOOK NAVIGATION: missing `{marker}`"));
        }
    }
}

fn check_release_local_paths(errors: &mut Vec<String>) {
    let source = match read_required("docs/src/specifications.md", errors) {
        Some(source) => source,
        None => return,
    };
    for marker in [
        "docs/specs/loeres-requirements-v1.md",
        "docs/specs/loeres-external-design-v1.md",
        "docs/specs/loeres-roadmap-milestones-v1.md",
        "rfcs/README.md",
        "rfcs/done/000-rfc-lifecycle-policy.md",
        "moving-branch navigation",
    ] {
        if !source.contains(marker) {
            errors.push(format!("RELEASE-LOCAL DOCS: missing `{marker}`"));
        }
    }
}

fn check_stale_ledger(errors: &mut Vec<String>) {
    for path in APEX_DOCS {
        if let Some(source) = read_required(path, errors) {
            let current = source.lines().take(40).collect::<Vec<_>>().join("\n");
            stale_phrases_in(&current, path, errors);
        }
    }

    // RFC 051 §8: built from `CRATE_README_PATHS` rather than a second,
    // independently-ordered copy of the same five paths, so a sixth crate
    // cannot fall outside this ledger while remaining inside RFC 049's
    // wildcard denial. `CRATE_README_PATHS` orders `loeres-backend-static`
    // before `loeres-backend-std`; the reverse order held here before this
    // unification, so a finding naming more than one of these paths now
    // prints in a different order than before — the paths covered are
    // unchanged.
    let whole_file_targets = std::iter::once(README_PATH)
        .chain(CRATE_README_PATHS.iter().copied())
        .chain(["docs/src/threat-model.md", "docs/src/specifications.md"]);
    for path in whole_file_targets {
        if let Some(source) = read_required(path, errors) {
            stale_phrases_in(&source, path, errors);
        }
    }

    if let Some(source) = read_required("ROADMAP.md", errors) {
        let current = current_before_historical(&source);
        stale_phrases_in(current, "ROADMAP.md current-status prefix", errors);
    }
}

fn stale_phrases_in(source: &str, label: &str, errors: &mut Vec<String>) {
    let undecorated = source.to_ascii_lowercase().replace(['`', '*'], "");
    let lower = normalize_whitespace(&undecorated);
    for phrase in STALE_PHRASES {
        if lower.contains(phrase) {
            errors.push(format!("STALE CURRENT PROSE: {label} contains `{phrase}`"));
        }
    }
}

/// RFC 024 retires the RFC 021 conditional apparatus permanently (§11.3): no
/// tracked document may still claim, in the present tense, that the apex,
/// lifecycle, or recovery state is conditional, staged, or pending external
/// activation (review 036 blocker B1). Historical descriptions of the
/// protocol, framed as past events, are unaffected.
fn check_conditional_current_prose(errors: &mut Vec<String>) {
    for path in CONDITIONAL_CURRENT_DOCS {
        let Some(source) = read_required(path, errors) else {
            continue;
        };
        retired_conditional_prose_in(&source, path, errors);
    }
}

fn retired_conditional_prose_in(source: &str, label: &str, errors: &mut Vec<String>) {
    let normalized = normalize_whitespace(source);
    for marker in CONDITIONAL_BOUNDARY_MARKERS {
        if normalized.contains(&normalize_whitespace(marker)) {
            errors.push(format!(
                "RETIRED CONDITIONAL PROSE: {label} still carries `{marker}`"
            ));
        }
    }
    for phrase in CONDITIONAL_STALE_PHRASES {
        if normalized.contains(&normalize_whitespace(phrase)) {
            errors.push(format!(
                "RETIRED CONDITIONAL PROSE: {label} retains stale phrase `{phrase}`"
            ));
        }
    }
}

fn current_before_historical(source: &str) -> &str {
    source
        .split_once("### Historical completion:")
        .map(|(current, _)| current)
        .unwrap_or(source)
}

fn read_required(path: &str, errors: &mut Vec<String>) -> Option<String> {
    match fs::read_to_string(path) {
        Ok(source) => Some(source),
        Err(error) => {
            errors.push(format!("MISSING/UNREADABLE: {path}: {error}"));
            None
        }
    }
}

fn normalize_whitespace(source: &str) -> String {
    source
        .lines()
        .map(str::trim)
        .map(|line| line.strip_prefix('>').unwrap_or(line).trim_start())
        .flat_map(str::split_whitespace)
        .collect::<Vec<_>>()
        .join(" ")
}

/// RFC 035 §3.2: the README stays inside the reference guideline's band, and its
/// Quick Start example is the region marked in a gated example, byte-for-byte.
const README_PATH: &str = "README.md";
const README_LINE_RANGE: (usize, usize) = (100, 200);
const README_EXAMPLE_SOURCE: &str = "examples/device-mpc-step/src/main.rs";
const README_EXAMPLE_BEGIN: &str = "// README-EXAMPLE-BEGIN";
const README_EXAMPLE_END: &str = "// README-EXAMPLE-END";
/// The guideline's minimal working example is under 20 lines.
const README_EXAMPLE_MAX_LINES: usize = 19;

fn check_readme_landing(errors: &mut Vec<String>) {
    let Some(readme) = read_required(README_PATH, errors) else {
        return;
    };
    let Some(source) = read_required(README_EXAMPLE_SOURCE, errors) else {
        return;
    };
    errors.extend(readme_length_findings(&readme));
    errors.extend(readme_example_findings(&readme, &source));
}

fn readme_length_findings(readme: &str) -> Vec<String> {
    let lines = readme.lines().count();
    let (min, max) = README_LINE_RANGE;
    if (min..=max).contains(&lines) {
        Vec::new()
    } else {
        vec![format!(
            "README LENGTH: {README_PATH} has {lines} lines; RFC 035 requires {min}-{max}"
        )]
    }
}

fn readme_example_findings(readme: &str, source: &str) -> Vec<String> {
    let Some(quick_start) = quick_start_rust_block(readme) else {
        return vec![format!(
            "README EXAMPLE: no ```rust block under `## Quick Start` in {README_PATH}"
        )];
    };
    let Some(region) = marked_region(source) else {
        return vec![format!(
            "README EXAMPLE: {README_EXAMPLE_SOURCE} has no complete \
             `{README_EXAMPLE_BEGIN}` / `{README_EXAMPLE_END}` region"
        )];
    };
    let mut findings = Vec::new();
    if region.lines().count() > README_EXAMPLE_MAX_LINES {
        findings.push(format!(
            "README EXAMPLE: the marked region in {README_EXAMPLE_SOURCE} is {} lines; the limit is {README_EXAMPLE_MAX_LINES}",
            region.lines().count()
        ));
    }
    if quick_start != region {
        findings.push(format!(
            "README EXAMPLE: the Quick Start block in {README_PATH} does not match the region marked in {README_EXAMPLE_SOURCE}; copy the region again"
        ));
    }
    findings
}

/// The first ```rust block after the `## Quick Start` heading, body only, with
/// the newline that ends its last line.
fn quick_start_rust_block(readme: &str) -> Option<String> {
    let (_, after_heading) = readme.split_once("\n## Quick Start\n")?;
    let (_, after_open) = after_heading.split_once("\n```rust\n")?;
    let (body, _) = after_open.split_once("\n```\n")?;
    Some(format!("{body}\n"))
}

/// The lines between the begin and end markers, each ending in a newline.
/// `None` if a marker is missing, repeated, or the region never closes.
fn marked_region(source: &str) -> Option<String> {
    let mut inside = false;
    let mut region = String::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed == README_EXAMPLE_BEGIN {
            if inside {
                return None;
            }
            inside = true;
        } else if trimmed == README_EXAMPLE_END {
            return inside.then_some(region);
        } else if inside {
            region.push_str(line);
            region.push('\n');
        }
    }
    None
}

/// `crates/*/README.md`: RFC 049 §2.4/§4 — no install snippet is asked for,
/// but clause 2 covers them so that a future one cannot be a wildcard.
const CRATE_README_PATHS: &[&str] = &[
    "crates/loeres/README.md",
    "crates/loeres-backend-static/README.md",
    "crates/loeres-backend-std/README.md",
    "crates/loeres-cluster/README.md",
    "crates/loeres-device/README.md",
];

/// RFC 049 S4: `README.md`'s dependency requirement names the released
/// minor (clause 1), and no requirement site in `README.md` or any
/// `crates/*/README.md` may be a wildcard (clause 2) — `deny.toml:49`'s own
/// rule, applied to what a user is told to write rather than only to what
/// this workspace compiles. `last_reconciled` is the apex block's raw field
/// (`0.24.0`), reduced to `MAJOR.MINOR` (`0.24`) here — the single site of
/// that derivation — before being passed to the pure predicate below. It is
/// `None` when the apex block itself failed to parse; clause 1 is then
/// skipped (a second, derived complaint would be noise over an error
/// `check_apex_currency` already reported) and clause 2 still runs, since it
/// needs no version.
fn check_readme_requirements(last_reconciled: Option<&str>, errors: &mut Vec<String>) {
    let expected_minor = last_reconciled.and_then(major_minor);
    if let Some(readme) = read_required(README_PATH, errors) {
        errors.extend(readme_requirement_findings(
            README_PATH,
            &readme,
            expected_minor.as_deref(),
        ));
    }
    for path in CRATE_README_PATHS {
        if let Some(source) = read_required(path, errors) {
            errors.extend(readme_requirement_findings(path, &source, None));
        }
    }
}

/// `docs/specs/loeres-external-design-v1.md` §1.4 **Public Import Model** —
/// a normative spec, not an example a user is free to vary. RFC 051
/// treatment B: its `toml` fences name crates and features only; the
/// version requirement's one home is `README.md` (RFC 049), so a site here
/// is a finding regardless of what value it names, including the correct
/// one.
const IMPORT_MODEL_PATH: &str = "docs/specs/loeres-external-design-v1.md";

fn check_import_model_requirements(errors: &mut Vec<String>) {
    if let Some(source) = read_required(IMPORT_MODEL_PATH, errors) {
        errors.extend(import_model_findings(IMPORT_MODEL_PATH, &source));
    }
}

/// RFC 051 S2's predicate: `requirement_sites` over this file must be
/// empty. Every site found is a finding, never silently accepted — a
/// version named here, even `MAJOR.MINOR`-correct, contradicts treatment
/// B, which is why this does not reuse `readme_requirement_findings`.
fn import_model_findings(path: &str, source: &str) -> Vec<String> {
    requirement_sites(source)
        .into_iter()
        .map(|(line, crate_name, _value)| {
            format!(
                "IMPORT MODEL: {path}:{line} `{crate_name}` names a version requirement; the \
                 import model fixes crates and features only, and the requirement's one home is \
                 README.md (RFC 051)"
            )
        })
        .collect()
}

/// `MAJOR.MINOR` of an `X.Y.Z` version string, or `None` if it is not that
/// shape.
fn major_minor(version: &str) -> Option<String> {
    let mut parts = version.split('.');
    let major = parts.next()?;
    let minor = parts.next()?;
    parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    Some(format!("{major}.{minor}"))
}

/// `docs/src/specifications.md`'s `## Release currency` section: the one
/// home RFC 048 designated for release currency, carrying a table, a
/// crates.io paragraph, and (one file up) the apex block, none of which
/// were held against each other or against `CHANGELOG.md` before RFC 052.
const RELEASE_TABLE_PATH: &str = "docs/src/specifications.md";
const RELEASE_TABLE_HEADER: &str = "| Repository release | Shipped | Carries RFCs |";
const RELEASE_TABLE_SEPARATOR: &str = "|---|---|---|";

fn check_release_table(
    last_reconciled: Option<&str>,
    implemented_scope: Option<&str>,
    errors: &mut Vec<String>,
) {
    let Some(specifications) = read_required(RELEASE_TABLE_PATH, errors) else {
        return;
    };
    let Some(changelog) = read_required(CHANGELOG_PATH, errors) else {
        return;
    };
    errors.extend(release_table_findings(
        &specifications,
        &changelog,
        last_reconciled,
        implemented_scope,
    ));
}

/// RFC 052 §3's five clauses, as amended by its own §0.1 (architect review
/// 118). Pure over strings so every branch is exercisable on literals.
/// `last_reconciled` and `implemented_scope` are `None` when the apex block
/// itself already failed to parse; the clauses that need them are then
/// skipped rather than raising a second, derived complaint (the same rule
/// RFC 049 S4 applies).
fn release_table_findings(
    specifications: &str,
    changelog: &str,
    last_reconciled: Option<&str>,
    implemented_scope: Option<&str>,
) -> Vec<String> {
    // RFC 052 corrections §3 (F3): a header that cannot be found is one
    // finding, not twelve derived ones. The other clauses need rows to say
    // anything meaningful, so this returns early exactly as the `None`
    // apex-value case already does elsewhere in this file.
    let Some((rows, malformed)) = release_table_rows(specifications) else {
        return vec![format!(
            "RELEASE TABLE: {RELEASE_TABLE_PATH} has no table header; looked for \
             `{RELEASE_TABLE_HEADER}`"
        )];
    };
    let mut findings = malformed;
    findings.extend(release_table_row_set_symmetry(&rows, changelog));
    findings.extend(release_table_ascending(&rows));
    findings.extend(release_table_shipped_is_distributed(&rows, changelog));
    findings.extend(release_table_lineage(
        &rows,
        last_reconciled,
        implemented_scope,
    ));
    findings.extend(release_table_crates_io_paragraph(
        specifications,
        changelog,
        &rows,
    ));
    findings
}

/// `(version, Shipped cell, Carries RFCs cell)` for each row, plus any
/// malformed-row findings (RFC 052 §3.6).
type ReleaseTableRows = (Vec<(String, String, String)>, Vec<String>);

/// The `## Release currency` table's rows, in order. `None` if the
/// table's header and separator cannot be found at all — the caller turns
/// that into one finding rather than letting every other clause report a
/// derived, misleading complaint (RFC 052 corrections §3). RFC 052 §3.6: a
/// line between the header's separator and the table's end that looks
/// like a row but does not parse as exactly three `|`-delimited columns is
/// reported in the second return value, never silently skipped.
fn release_table_rows(source: &str) -> Option<ReleaseTableRows> {
    let mut rows = Vec::new();
    let mut malformed = Vec::new();
    let marker = format!("{RELEASE_TABLE_HEADER}\n{RELEASE_TABLE_SEPARATOR}\n");
    let (_, after_header) = source.split_once(&marker)?;
    for line in after_header.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') {
            break;
        }
        let fields: Vec<&str> = trimmed
            .trim_matches('|')
            .split('|')
            .map(str::trim)
            .collect();
        if fields.len() != 3 {
            malformed.push(format!(
                "RELEASE TABLE: malformed row in {RELEASE_TABLE_PATH}: `{trimmed}`"
            ));
            continue;
        }
        let version = fields[0].trim_matches('`').to_owned();
        rows.push((version, fields[1].to_owned(), fields[2].to_owned()));
    }
    Some((rows, malformed))
}

/// Every version named by a `## [X]` heading in `CHANGELOG.md`'s own
/// order — the heading grammar `parse_changelog_release_status` already
/// assumes (`## [` + version + `]` followed by end-of-line or whitespace),
/// read here only to enumerate the versions it does not itself enumerate.
fn changelog_heading_versions(source: &str) -> Vec<String> {
    let mut versions = Vec::new();
    for line in source.lines() {
        let Some(rest) = line.strip_prefix("## [") else {
            continue;
        };
        let Some((version, suffix)) = rest.split_once(']') else {
            continue;
        };
        if suffix.is_empty() || suffix.starts_with(char::is_whitespace) {
            versions.push(version.to_owned());
        }
    }
    versions
}

/// Clause 1: the table's versions are exactly the versions `CHANGELOG.md`
/// marks `released` — reusing `parse_changelog_release_status` per version
/// rather than a second classifier.
fn release_table_row_set_symmetry(
    rows: &[(String, String, String)],
    changelog: &str,
) -> Vec<String> {
    let mut findings = Vec::new();
    let is_released = |version: &str| {
        parse_changelog_release_status(changelog, version)
            .map(|status| status == "released")
            .unwrap_or(false)
    };
    for version in changelog_heading_versions(changelog) {
        if is_released(&version) && !rows.iter().any(|(v, _, _)| v == &version) {
            findings.push(format!(
                "RELEASE TABLE: CHANGELOG.md marks `{version}` released, but {RELEASE_TABLE_PATH}'s \
                 table has no row for it"
            ));
        }
    }
    for (version, _, _) in rows {
        let status = parse_changelog_release_status(changelog, version)
            .unwrap_or_else(|_| "no CHANGELOG.md status line".to_owned());
        if status != "released" {
            findings.push(format!(
                "RELEASE TABLE: {RELEASE_TABLE_PATH}'s table has a row for `{version}`, but \
                 CHANGELOG.md marks it `{status}`"
            ));
        }
    }
    findings
}

/// Clause 2: consecutive rows ascend by `parse_version_triple`.
fn release_table_ascending(rows: &[(String, String, String)]) -> Vec<String> {
    let mut findings = Vec::new();
    for index in 1..rows.len() {
        let (previous, _, _) = &rows[index - 1];
        let (current, _, _) = &rows[index];
        match (
            parse_version_triple(previous),
            parse_version_triple(current),
        ) {
            (Some(a), Some(b)) if a >= b => findings.push(format!(
                "RELEASE TABLE: {RELEASE_TABLE_PATH}'s table is not ascending: `{previous}` then \
                 `{current}`"
            )),
            (Some(_), Some(_)) => {}
            _ => findings.push(format!(
                "RELEASE TABLE: {RELEASE_TABLE_PATH}'s table has a row whose version is not \
                 `X.Y.Z`: `{previous}` or `{current}`"
            )),
        }
    }
    findings
}

/// The raw `**Release status:** …` text for one version's heading —
/// distinct from `parse_changelog_release_status`, which reduces that same
/// text to a bare token and discards the `distributed`/`published`
/// sub-text clauses 3 and 5 need.
fn changelog_status_line_value<'a>(source: &'a str, version: &str) -> Option<&'a str> {
    let heading_prefix = format!("## [{version}]");
    let lines: Vec<&str> = source.lines().collect();
    let heading_index = lines.iter().position(|line| {
        line.strip_prefix(&heading_prefix)
            .map(|suffix| suffix.is_empty() || suffix.starts_with(char::is_whitespace))
            .unwrap_or(false)
    })?;
    let status_line = lines[heading_index + 1..]
        .iter()
        .find(|line| !line.trim().is_empty())?;
    status_line.trim().strip_prefix("**Release status:** ")
}

/// The `distributed` date from a status line's parenthesised
/// `(tagged YYYY-MM-DD, distributed YYYY-MM-DD)` group — read by its
/// clause structure (split the group on `", "`, match each field's own
/// prefix) rather than searched for as a substring, which is the shape
/// F1 corrected one clause to the right (RFC 052 corrections §2). `None`
/// if the group, the `distributed` field, or its date is absent or fails
/// `valid_date`.
fn changelog_distributed_date(source: &str, version: &str) -> Option<String> {
    let line = changelog_status_line_value(source, version)?;
    let (_, after_open) = line.split_once('(')?;
    let (group, _) = after_open.split_once(')')?;
    for field in group.split(", ") {
        if let Some(date) = field.trim().strip_prefix("distributed ") {
            return valid_date(date).then(|| date.to_owned());
        }
    }
    None
}

/// Clause 3: each `Shipped` cell is that release's *distributed* date, not
/// its tagged date (architect review 118 F1 — `0.20.2` is the one row
/// where they differ).
fn release_table_shipped_is_distributed(
    rows: &[(String, String, String)],
    changelog: &str,
) -> Vec<String> {
    let mut findings = Vec::new();
    for (version, shipped, _) in rows {
        let Ok(status) = parse_changelog_release_status(changelog, version) else {
            continue; // row-set symmetry already reports this row
        };
        if status != "released" {
            continue; // row-set symmetry already reports this row
        }
        let Some(distributed) = changelog_distributed_date(changelog, version) else {
            findings.push(format!(
                "RELEASE TABLE: CHANGELOG.md's `{version}` status line has no valid \
                 `distributed YYYY-MM-DD` to check {RELEASE_TABLE_PATH}'s `Shipped` cell against"
            ));
            continue;
        };
        if shipped != &distributed {
            findings.push(format!(
                "RELEASE TABLE: {RELEASE_TABLE_PATH}'s `Shipped` cell for `{version}` is \
                 `{shipped}`, but CHANGELOG.md records it distributed `{distributed}`"
            ));
        }
    }
    findings
}

/// The largest decimal number appearing anywhere in a scope string.
/// `render_scope` emits comma-separated parts when `rfcs/done/` has a gap
/// (`RFCs 001-040, 042-044`), so this reads every digit run rather than
/// assuming a single `NNN-NNN` range or a `RFCs ` prefix.
fn max_scope_number(scope: &str) -> Option<u32> {
    let mut max = None;
    let mut current = String::new();
    for ch in scope.chars().chain(std::iter::once(' ')) {
        if ch.is_ascii_digit() {
            current.push(ch);
            continue;
        }
        if !current.is_empty() {
            if let Ok(number) = current.parse::<u32>() {
                max = Some(max.map_or(number, |previous: u32| previous.max(number)));
            }
            current.clear();
        }
    }
    max
}

/// Clause 4, as amended (review 118 F2): bounded and ordered, not equated
/// to `implemented_scope` — that field runs ahead of every row during a
/// release cut, so equality would fail the gate mid-cut.
fn release_table_lineage(
    rows: &[(String, String, String)],
    last_reconciled: Option<&str>,
    implemented_scope: Option<&str>,
) -> Vec<String> {
    let mut findings = Vec::new();
    let Some((last_version, _, last_carries)) = rows.last() else {
        return findings;
    };

    if let Some(last_reconciled) = last_reconciled {
        if last_version != last_reconciled {
            findings.push(format!(
                "RELEASE TABLE: {RELEASE_TABLE_PATH}'s last row is `{last_version}`, but the apex \
                 block's last reconciled release is `{last_reconciled}`"
            ));
        }
    }

    let mut previous_max: Option<u32> = None;
    for (version, _, carries) in rows {
        let Some(max) = max_scope_number(carries) else {
            findings.push(format!(
                "RELEASE TABLE: {RELEASE_TABLE_PATH}'s `Carries RFCs` cell for `{version}` has no \
                 number to read: `{carries}`"
            ));
            continue;
        };
        if let Some(previous) = previous_max {
            if max < previous {
                findings.push(format!(
                    "RELEASE TABLE: {RELEASE_TABLE_PATH}'s `Carries RFCs` column decreases at \
                     `{version}` ({max} after {previous})"
                ));
            }
        }
        previous_max = Some(max);
    }

    if let (Some(last_max), Some(scope)) = (max_scope_number(last_carries), implemented_scope) {
        if let Some(scope_max) = max_scope_number(scope) {
            if last_max > scope_max {
                findings.push(format!(
                    "RELEASE TABLE: {RELEASE_TABLE_PATH}'s last row claims RFCs up to {last_max}, \
                     which exceeds the apex block's implemented scope maximum {scope_max}"
                ));
            }
        }
    }
    findings
}

/// The two literals a `released` version's status line's final
/// `"; "`-separated clause must equal — read as the clause, never searched
/// for as a substring, because `CHANGELOG_NOT_PUBLISHED` contains
/// `CHANGELOG_PUBLISHED` (architect review 119 F1: the substring form
/// admits an unpublished release as published).
const CHANGELOG_PUBLISHED: &str = "published to crates.io";
const CHANGELOG_NOT_PUBLISHED: &str = "not published to crates.io";

/// A `released` version's publication verdict, read from its status
/// line's final clause (after the last `"; "`) and compared for equality
/// against the two literals above. `Err` when that clause is neither —
/// fail closed, naming the version and what its line says, rather than
/// silently defaulting to "not published."
fn is_published(changelog: &str, version: &str) -> Result<bool, String> {
    let line = changelog_status_line_value(changelog, version).ok_or_else(|| {
        format!("`{version}` is released but has no status line to read a publication clause from")
    })?;
    let clause = line.rsplit("; ").next().unwrap_or(line).trim();
    match clause {
        CHANGELOG_PUBLISHED => Ok(true),
        CHANGELOG_NOT_PUBLISHED => Ok(false),
        other => Err(format!(
            "`{version}`'s status line's final clause is neither `{CHANGELOG_PUBLISHED}` nor \
             `{CHANGELOG_NOT_PUBLISHED}`: `{other}`"
        )),
    }
}

/// The newest (by `parse_version_triple`) *released* version whose
/// publication clause is `published to crates.io` — published and
/// released are independent (review 118 F3): five of this project's own
/// releases were distributed and never published. The second return
/// value is any fail-closed finding `is_published` raised along the way;
/// every one of the released entries in the committed tree carries one of
/// the two literals, so this is unreachable there and is exercised only by
/// a unit test (RFC 052 corrections §1.1).
fn newest_published_version(changelog: &str) -> (Option<String>, Vec<String>) {
    let mut findings = Vec::new();
    let newest = changelog_heading_versions(changelog)
        .into_iter()
        .filter(|version| {
            parse_changelog_release_status(changelog, version)
                .map(|status| status == "released")
                .unwrap_or(false)
        })
        .filter_map(|version| match is_published(changelog, &version) {
            Ok(true) => parse_version_triple(&version).map(|triple| (triple, version)),
            Ok(false) => None,
            Err(reason) => {
                findings.push(format!("RELEASE TABLE: {reason}"));
                None
            }
        })
        .max()
        .map(|(_, version)| version);
    (newest, findings)
}

/// The `**What is on crates.io.**` paragraph's text, up to the next blank
/// line.
fn crates_io_paragraph(source: &str) -> Option<&str> {
    let (_, after) = source.split_once("**What is on crates.io.**")?;
    Some(match after.find("\n\n") {
        Some(end) => &after[..end],
        None => after,
    })
}

/// Clause 5: the crates.io paragraph names the newest *published* version
/// (not `last_reconciled`) and that version's own row's `Carries RFCs`.
fn release_table_crates_io_paragraph(
    specifications: &str,
    changelog: &str,
    rows: &[(String, String, String)],
) -> Vec<String> {
    let (newest_published, publication_findings) = newest_published_version(changelog);
    let mut findings = publication_findings;
    let Some(newest_published) = newest_published else {
        findings
            .push("RELEASE TABLE: CHANGELOG.md names no version published to crates.io".to_owned());
        return findings;
    };
    let Some((_, _, carries)) = rows.iter().find(|(v, _, _)| v == &newest_published) else {
        findings.push(format!(
            "RELEASE TABLE: `{newest_published}` is the newest version CHANGELOG.md marks \
             published to crates.io, but {RELEASE_TABLE_PATH}'s table has no row for it"
        ));
        return findings;
    };
    let Some(paragraph) = crates_io_paragraph(specifications) else {
        findings.push(format!(
            "RELEASE TABLE: {RELEASE_TABLE_PATH} has no `What is on crates.io.` paragraph to check"
        ));
        return findings;
    };
    if !paragraph.contains(&format!("`{newest_published}`")) {
        findings.push(format!(
            "RELEASE TABLE: the `What is on crates.io.` paragraph in {RELEASE_TABLE_PATH} must \
             name `{newest_published}`, the newest version CHANGELOG.md marks published to \
             crates.io"
        ));
    }
    if !paragraph.contains(&format!("RFCs {carries}")) {
        findings.push(format!(
            "RELEASE TABLE: the `What is on crates.io.` paragraph in {RELEASE_TABLE_PATH} must \
             state `RFCs {carries}`, matching `{newest_published}`'s own row"
        ));
    }
    findings
}

/// RFC 049 S4's predicate, as a pure function over strings so every branch
/// can be exercised on literals (architect review 113 F5: an unexercised
/// branch is how a predicate goes vacuous). A wildcard site is reported
/// instead of a clause-1 mismatch against it — the wildcard is the defect;
/// comparing a wildcard's literal text to a specific minor would just add a
/// second, redundant message for the one broken line.
fn readme_requirement_findings(
    path: &str,
    readme: &str,
    expected_minor: Option<&str>,
) -> Vec<String> {
    let mut findings = Vec::new();
    for (line, crate_name, value) in requirement_sites(readme) {
        if crate_name == "version" {
            findings.push(format!(
                "README VERSION: {path}:{line} a `version = ...` line with no crate name \
                 before it is not a shape this assertion reads"
            ));
            continue;
        }
        if is_wildcard_requirement(value) {
            findings.push(format!(
                "README VERSION: {path}:{line} `{crate_name}` must not advertise a wildcard \
                 requirement (found `{value}`)"
            ));
            continue;
        }
        if let Some(expected_minor) = expected_minor {
            if value != expected_minor {
                findings.push(format!(
                    "README VERSION: {path}:{line} `{crate_name}` requires `{value}`, expected \
                     the released minor `{expected_minor}`"
                ));
            }
        }
    }
    findings
}

/// Every `version = "…"` line inside a ` ```toml ` fence, as
/// `(1-based line number, identifier before the line's first '=', the
/// quoted value)`. The identifier is the crate name for an ordinary site
/// (`crate = { version = "…" }`) and the literal `version` for the
/// `[dependencies.<crate>]` form, which `readme_requirement_findings`
/// reports rather than silently skips (RFC 049 §4.2).
fn requirement_sites(source: &str) -> Vec<(usize, &str, &str)> {
    let mut sites = Vec::new();
    let mut in_fence = false;
    for (index, line) in source.lines().enumerate() {
        match line.trim() {
            "```toml" => {
                in_fence = true;
                continue;
            }
            "```" => {
                in_fence = false;
                continue;
            }
            _ => {}
        }
        if !in_fence {
            continue;
        }
        let Some(value) = requirement_value(line) else {
            continue;
        };
        let Some(equals) = line.find('=') else {
            continue;
        };
        sites.push((index + 1, line[..equals].trim(), value));
    }
    sites
}

/// The quoted value of a `version = "…"` occurrence on one line, or `None`
/// if the line does not contain the literal `version` followed by `=` and a
/// double-quoted string.
fn requirement_value(line: &str) -> Option<&str> {
    let (_, after_version) = line.split_once("version")?;
    let after_eq = after_version.trim_start().strip_prefix('=')?.trim_start();
    let (value, _) = after_eq.strip_prefix('"')?.split_once('"')?;
    Some(value)
}

/// `deny.toml:49`'s own rule (`wildcards = "deny"`), applied to a version
/// requirement string: a `*`, or any dot-separated component equal to `x` or
/// `X`.
fn is_wildcard_requirement(value: &str) -> bool {
    value.contains('*') || value.split('.').any(|part| part.eq_ignore_ascii_case("x"))
}

fn bounded_value(source: &str, prefix: &str, suffix: &str) -> Option<String> {
    let (_, after_prefix) = source.split_once(prefix)?;
    let (value, _) = after_prefix.split_once(suffix)?;
    if value.is_empty() {
        None
    } else {
        Some(value.to_owned())
    }
}

/// RFC 038 §2.3: every example under `examples/` carries a `README.md`, is
/// named in both the root `README.md` table and `examples/README.md`, and —
/// for a worked problem — that README's problem paragraph is the second
/// paragraph structurally extracted from its `src/main.rs`.
///
/// Amendment 1 drops the `README-EXAMPLE-BEGIN`/`END` marker mechanism
/// originally specified for this: inside a `//!` block a bare marker-comment
/// line contributes nothing to rustdoc, so the paragraphs either side of it
/// would silently merge in the rendered documentation, and `doc-build`'s
/// `-D warnings` would not necessarily catch the merge. Structural extraction
/// needs no source edit at all.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ExampleCategory {
    WorkedProblem,
    ApiArtifact,
}

const EXAMPLE_REGISTRY: &[(&str, ExampleCategory)] = &[
    ("device-mpc-step", ExampleCategory::WorkedProblem),
    ("device-box-pfo", ExampleCategory::ApiArtifact),
    ("cluster-capacity-dispatch", ExampleCategory::WorkedProblem),
    ("cluster-qp-constrained", ExampleCategory::ApiArtifact),
    ("cluster-batch-solve", ExampleCategory::ApiArtifact),
    ("cluster-counted-work", ExampleCategory::WorkedProblem),
];

/// RFC 053: the single, non-RFC definition of RFC 047's phases. Not in
/// `APEX_DOCS` and carries no apex currency block — it makes no release
/// or scope claim.
const DESTINATION_SEQUENCE_PATH: &str = "rfcs/destination-sequence.md";

/// The three state literals, checked by equality on the trimmed value,
/// never `contains` — `unscheduled` contains `scheduled`, which is how
/// RFC 052's clause 5 shipped a defect past its own gate (architect
/// review 119 F1), and the literals here are pairwise non-substrings as a
/// second line of defence.
const PHASE_STATES: &[&str] = &["deferred", "scheduled", "shipped"];

/// One phase's own held fields, parsed from `rfcs/destination-sequence.md`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct DestinationPhase {
    numeral: String,
    state: String,
    rfcs: Vec<String>,
}

/// Parse `rfcs/destination-sequence.md`'s `## Phase <roman> — <title>`
/// sections into their own `**State.**` and `**RFCs.**` fields. Pure over
/// the file's own text so every branch is exercisable on literals.
fn parse_destination_phases(source: &str) -> Vec<DestinationPhase> {
    let mut phases = Vec::new();
    let mut current: Option<DestinationPhase> = None;
    for line in source.lines() {
        if let Some(rest) = line.strip_prefix("## Phase ") {
            if let Some(phase) = current.take() {
                phases.push(phase);
            }
            let numeral = rest.split_whitespace().next().unwrap_or("").to_owned();
            current = Some(DestinationPhase {
                numeral,
                state: String::new(),
                rfcs: Vec::new(),
            });
            continue;
        }
        let Some(phase) = current.as_mut() else {
            continue;
        };
        if let Some(value) = line.strip_prefix("**State.**") {
            phase.state = value.trim().to_owned();
        } else if let Some(value) = line.strip_prefix("**RFCs.**") {
            let value = value.trim();
            if value != "—" {
                phase.rfcs = value.split(',').map(|s| s.trim().to_owned()).collect();
            }
        }
    }
    if let Some(phase) = current.take() {
        phases.push(phase);
    }
    phases
}

/// Every `Phase <roman-run>` reference on a line already passed through
/// `mask_code_spans` — a maximal run of `IVXLCDM` characters immediately
/// after the literal `"Phase "`, non-empty, and followed by a word
/// boundary, so `Phase IV9` is not read as `Phase IV` and arabic
/// `Phase 0`-`3` never match at all (RFC 053 S3 clause 1). This is a
/// roman-numeral *run*, not an enumeration of defined phases: a future
/// `Phase VII` is matched here and only fails later, for want of a
/// definition, which is the point.
fn phase_references(line: &str) -> Vec<&str> {
    let mut found = Vec::new();
    for (pos, _) in line.match_indices("Phase ") {
        let start = pos + "Phase ".len();
        let tail = &line[start..];
        let end = tail
            .find(|c: char| !"IVXLCDM".contains(c))
            .unwrap_or(tail.len());
        if end == 0 {
            continue;
        }
        let boundary = tail[end..]
            .chars()
            .next()
            .is_none_or(|c| !c.is_alphanumeric());
        if boundary {
            found.push(&tail[..end]);
        }
    }
    found
}

/// Clause 1: every roman-numeral phase reference, in every file
/// `collect_md` finds, resolves to a phase `defined` names. One finding
/// per undefined phase, naming every site it is cited from — not one
/// finding per occurrence, which is architect review 119 F3's mistake
/// repeated (a renamed table header produced twelve findings for one
/// cause). Historical documents are in scope by design: the point is that
/// `rfcs/done/049-…:188`'s reference resolves, and exempting `done/`
/// would exempt the citations that motivated this RFC.
fn phase_reference_findings(
    files: &[(PathBuf, String)],
    defined: &BTreeSet<String>,
) -> Vec<String> {
    let mut sites: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (path, source) in files {
        for (index, line) in source.lines().enumerate() {
            let masked = mask_code_spans(line);
            for numeral in phase_references(&masked) {
                if !defined.contains(numeral) {
                    sites.entry(numeral.to_owned()).or_default().push(format!(
                        "{}:{}",
                        path.display(),
                        index + 1
                    ));
                }
            }
        }
    }
    sites
        .into_iter()
        .map(|(numeral, cited_at)| {
            format!(
                "PHASE REFERENCE: `Phase {numeral}` is not defined in \
                 {DESTINATION_SEQUENCE_PATH}; cited at {}",
                cited_at.join(", ")
            )
        })
        .collect()
}

/// Clause 2: every phase's `State.` equals one of `PHASE_STATES`, by
/// equality on the trimmed value, failing closed on anything else —
/// `unscheduled` included.
fn phase_state_findings(phases: &[DestinationPhase]) -> Vec<String> {
    phases
        .iter()
        .filter(|phase| !PHASE_STATES.contains(&phase.state.as_str()))
        .map(|phase| {
            format!(
                "PHASE STATE: {DESTINATION_SEQUENCE_PATH}'s Phase {} state `{}` is not one of \
                 {PHASE_STATES:?}",
                phase.numeral, phase.state
            )
        })
        .collect()
}

/// Clause 3: a `shipped` phase names at least one RFC, and every RFC it
/// names resolves to a file in `rfcs/done/` — `deferred` and `scheduled`
/// phases may name none. `done_numbers` is read by `numbered_rfc_file`'s
/// own filename grammar (RFC 053 asks whether an RFC's *file* is present,
/// a narrower question than `implemented_scope`'s rendered range), not
/// re-derived from that range.
fn phase_rfc_findings(phases: &[DestinationPhase], done_numbers: &BTreeSet<u32>) -> Vec<String> {
    let mut findings = Vec::new();
    for phase in phases {
        if phase.state != "shipped" {
            continue;
        }
        if phase.rfcs.is_empty() {
            findings.push(format!(
                "PHASE STATE: {DESTINATION_SEQUENCE_PATH}'s Phase {} is `shipped` but names no \
                 RFC",
                phase.numeral
            ));
            continue;
        }
        for rfc in &phase.rfcs {
            let resolves = rfc
                .parse::<u32>()
                .is_ok_and(|number| done_numbers.contains(&number));
            if !resolves {
                findings.push(format!(
                    "PHASE STATE: {DESTINATION_SEQUENCE_PATH}'s Phase {} is `shipped` but names \
                     RFC {rfc}, which does not resolve to a file in rfcs/done/",
                    phase.numeral
                ));
            }
        }
    }
    findings
}

/// The RFC numbers present as files under `rfcs/done/`, by file name
/// alone. A non-`NNN-slug.md` name is skipped silently here rather than
/// reported: `implemented_scope` already reports that defect as its own
/// `RFC SCOPE` finding, and reporting it a second time from here would be
/// architect review 119 F4's mistake again (the same cause, found twice).
fn done_rfc_numbers(errors: &mut Vec<String>) -> Option<BTreeSet<u32>> {
    let entries = match fs::read_dir(Path::new("rfcs").join("done")) {
        Ok(entries) => entries,
        Err(error) => {
            errors.push(format!("PHASE STATE: cannot read rfcs/done: {error}"));
            return None;
        }
    };
    let mut numbers = BTreeSet::new();
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if let Some(number) = numbered_rfc_file(name) {
            numbers.insert(number);
        }
    }
    Some(numbers)
}

fn check_phase_sequence(errors: &mut Vec<String>) {
    let Some(sequence_source) = read_required(DESTINATION_SEQUENCE_PATH, errors) else {
        return;
    };
    let phases = parse_destination_phases(&sequence_source);
    let defined: BTreeSet<String> = phases.iter().map(|phase| phase.numeral.clone()).collect();

    let mut paths = Vec::new();
    link_audit::collect_md(Path::new("."), &mut paths);
    let files: Vec<(PathBuf, String)> = paths
        .into_iter()
        .filter_map(|path| fs::read_to_string(&path).ok().map(|source| (path, source)))
        .collect();
    errors.extend(phase_reference_findings(&files, &defined));

    errors.extend(phase_state_findings(&phases));

    if let Some(done_numbers) = done_rfc_numbers(errors) {
        errors.extend(phase_rfc_findings(&phases, &done_numbers));
    }
}

const EXAMPLES_INDEX_PATH: &str = "examples/README.md";

fn check_example_guides(errors: &mut Vec<String>) {
    check_example_guides_at(Path::new("."), errors);
}

fn check_example_guides_at(root: &Path, errors: &mut Vec<String>) {
    let Some(directories) = example_directories_at(root, errors) else {
        return;
    };
    errors.extend(registry_symmetry_findings(&directories));
    errors.extend(readme_presence_findings(root, &directories));
    for &(name, category) in EXAMPLE_REGISTRY {
        if category == ExampleCategory::WorkedProblem && directories.iter().any(|d| d == name) {
            errors.extend(extraction_findings(root, name));
        }
    }
    // The two tables are read and checked independently, each against the
    // real directory set: a missing or unreadable `examples/README.md` must
    // not suppress the root `README.md` table's own symmetry findings (or
    // vice versa).
    if let Some(root_readme) = read_example_file(root, README_PATH, errors) {
        errors.extend(table_symmetry_findings(
            &directories,
            README_PATH,
            section_after_heading(&root_readme, "## Examples").unwrap_or_default(),
        ));
    }
    if let Some(examples_index) = read_example_file(root, EXAMPLES_INDEX_PATH, errors) {
        errors.extend(table_symmetry_findings(
            &directories,
            EXAMPLES_INDEX_PATH,
            &examples_index,
        ));
    }
}

fn read_example_file(root: &Path, relative: &str, errors: &mut Vec<String>) -> Option<String> {
    let path = root.join(relative);
    match fs::read_to_string(&path) {
        Ok(source) => Some(source),
        Err(error) => {
            errors.push(format!("MISSING/UNREADABLE: {}: {error}", path.display()));
            None
        }
    }
}

fn example_directories_at(root: &Path, errors: &mut Vec<String>) -> Option<Vec<String>> {
    let examples_dir = root.join("examples");
    let entries = match fs::read_dir(&examples_dir) {
        Ok(entries) => entries,
        Err(error) => {
            errors.push(format!(
                "EXAMPLE REGISTRY: cannot read {}: {error}",
                examples_dir.display()
            ));
            return None;
        }
    };
    let mut directories = Vec::new();
    for entry in entries.flatten() {
        if !entry.path().is_dir() {
            continue;
        }
        match entry.file_name().to_str() {
            Some(name) => directories.push(name.to_owned()),
            None => errors.push(format!(
                "EXAMPLE REGISTRY: {} contains a non-UTF-8 directory name",
                examples_dir.display()
            )),
        }
    }
    directories.sort();
    Some(directories)
}

/// RFC 038 §2.3 item 1: every registered name is a directory that exists, and
/// every directory is registered, in both directions.
fn registry_symmetry_findings(directories: &[String]) -> Vec<String> {
    let mut findings = Vec::new();
    for &(name, _) in EXAMPLE_REGISTRY {
        if !directories.iter().any(|directory| directory == name) {
            findings.push(format!(
                "EXAMPLE REGISTRY: `{name}` is registered but examples/{name} does not exist"
            ));
        }
    }
    for directory in directories {
        if !EXAMPLE_REGISTRY.iter().any(|&(name, _)| name == directory) {
            findings.push(format!(
                "EXAMPLE REGISTRY: examples/{directory} exists but is not registered"
            ));
        }
    }
    findings
}

/// RFC 038 §2.3 item 2: every example directory has a `README.md`.
fn readme_presence_findings(root: &Path, directories: &[String]) -> Vec<String> {
    directories
        .iter()
        .filter(|name| !root.join("examples").join(name).join("README.md").is_file())
        .map(|name| format!("EXAMPLE README: examples/{name} has no README.md"))
        .collect()
}

/// Paragraphs of a `//!` module doc block, read from the top: strip `//!` and
/// one following space from each leading `//!` line, then split into
/// paragraphs on blank stripped lines. RFC 038 §0.1's extraction rule.
///
/// A bare `//!` line (no trailing space) strips to an empty `rest`, which
/// must still count as a blank separator line — the fallback on a missing
/// space must stay on `rest`, not revert to the original `//!`-prefixed
/// `line`, or a bare marker line would be swallowed into the paragraph text
/// instead of splitting it.
fn module_doc_paragraphs(source: &str) -> Vec<String> {
    let mut paragraphs = Vec::new();
    let mut current: Vec<&str> = Vec::new();
    for line in source.lines() {
        let Some(rest) = line.strip_prefix("//!") else {
            break;
        };
        let rest = rest.strip_prefix(' ').unwrap_or(rest);
        if rest.is_empty() {
            if !current.is_empty() {
                paragraphs.push(current.join(" "));
                current.clear();
            }
        } else {
            current.push(rest);
        }
    }
    if !current.is_empty() {
        paragraphs.push(current.join(" "));
    }
    paragraphs
}

/// The problem statement (worked problems) or the fixed API-artifact line
/// (RFC 038 §0.1), or `None` if the block has no second paragraph.
fn module_doc_second_paragraph(source: &str) -> Option<String> {
    module_doc_paragraphs(source).into_iter().nth(1)
}

/// Markdown paragraphs: runs of non-blank lines separated by one or more
/// blank lines, each paragraph's lines joined with a space.
fn markdown_paragraphs(source: &str) -> Vec<String> {
    let mut paragraphs = Vec::new();
    let mut current: Vec<&str> = Vec::new();
    for line in source.lines() {
        if line.trim().is_empty() {
            if !current.is_empty() {
                paragraphs.push(current.join(" "));
                current.clear();
            }
        } else {
            current.push(line.trim());
        }
    }
    if !current.is_empty() {
        paragraphs.push(current.join(" "));
    }
    paragraphs
}

/// The first paragraph in a per-example `README.md` that is not itself a
/// heading — the problem paragraph, by the convention §1 of the handoff
/// requires every worked-problem README to follow (title heading, then the
/// problem paragraph, then the run command and the links).
fn readme_problem_paragraph(readme: &str) -> Option<String> {
    markdown_paragraphs(readme)
        .into_iter()
        .find(|paragraph| !paragraph.starts_with('#'))
}

/// RFC 038 §2.3 item 3, worked problems only.
fn extraction_findings(root: &Path, name: &str) -> Vec<String> {
    let main_rs_path = root.join("examples").join(name).join("src").join("main.rs");
    let readme_path = root.join("examples").join(name).join("README.md");
    let main_rs = match fs::read_to_string(&main_rs_path) {
        Ok(source) => source,
        Err(error) => {
            return vec![format!(
                "EXAMPLE EXTRACTION: cannot read {}: {error}",
                main_rs_path.display()
            )];
        }
    };
    let readme = match fs::read_to_string(&readme_path) {
        Ok(source) => source,
        Err(error) => {
            return vec![format!(
                "EXAMPLE EXTRACTION: cannot read {}: {error}",
                readme_path.display()
            )];
        }
    };
    extraction_findings_from(name, &main_rs, &readme)
}

fn extraction_findings_from(name: &str, main_rs: &str, readme: &str) -> Vec<String> {
    let Some(expected) = module_doc_second_paragraph(main_rs) else {
        return vec![format!(
            "EXAMPLE EXTRACTION: `{name}`'s src/main.rs has no second `//!` paragraph to extract"
        )];
    };
    let Some(actual) = readme_problem_paragraph(readme) else {
        return vec![format!(
            "EXAMPLE EXTRACTION: `{name}`'s README.md has no problem paragraph to compare"
        )];
    };
    if normalize_whitespace(&expected) == normalize_whitespace(&actual) {
        Vec::new()
    } else {
        vec![format!(
            "EXAMPLE EXTRACTION: `{name}`'s README problem paragraph does not match the module doc's second paragraph — extracted: \"{expected}\" — readme: \"{actual}\""
        )]
    }
}

/// The directory names linked as `` [`name`] `` in the table rows of
/// `markdown` — lines starting with `|`, so a prose "see also" link using the
/// same backtick-code style elsewhere in the document is not mistaken for a
/// table entry.
///
/// GitHub-flavoured Markdown also permits a table whose rows carry no leading
/// pipe, and this scanner would find no rows in one. That cannot pass silently:
/// the two directions interlock. With no rows found, the directory-to-table
/// direction reports *every* example as unlisted and fails loudly, long before
/// the table-to-directory direction could miss a row naming something absent.
fn linked_directory_names(markdown: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for line in markdown.lines() {
        if !line.trim_start().starts_with('|') {
            continue;
        }
        let mut rest = line;
        while let Some(start) = rest.find("[`") {
            let after = &rest[start + 2..];
            let Some(end) = after.find("`]") else {
                break;
            };
            names.insert(after[..end].to_owned());
            rest = &after[end..];
        }
    }
    names
}

/// The body of the first section under `heading`: the text up to the next
/// `#` line, or the end of the document if there is none.
fn section_after_heading<'a>(markdown: &'a str, heading: &str) -> Option<&'a str> {
    let marker = format!("\n{heading}\n");
    let (_, after) = markdown.split_once(&marker)?;
    Some(match after.find("\n#") {
        Some(end) => &after[..end],
        None => after,
    })
}

/// RFC 038 §2.3 item 4: every example directory has a row in both tables,
/// and every row in either table names a directory that exists.
fn table_symmetry_findings(directories: &[String], label: &str, table: &str) -> Vec<String> {
    let actual: BTreeSet<String> = directories.iter().cloned().collect();
    let linked = linked_directory_names(table);
    let mut findings = Vec::new();
    for name in &actual {
        if !linked.contains(name) {
            findings.push(format!(
                "EXAMPLE TABLE: examples/{name} has no row in {label}"
            ));
        }
    }
    for name in &linked {
        if !actual.contains(name) {
            findings.push(format!(
                "EXAMPLE TABLE: {label} links examples/{name}, which does not exist"
            ));
        }
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::{
        BTreeSet, DestinationPhase, changelog_marks_released, current_before_historical,
        implemented_scope_at, import_model_findings, index_status_matches, numbered_rfc_file,
        parse_changelog_release_status, parse_design_freeze, parse_ordinary_apex_currency,
        phase_reference_findings, phase_rfc_findings, phase_state_findings,
        readme_requirement_findings, release_table_findings, render_scope,
        retired_conditional_prose_in, stale_phrases_in,
    };
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn valid_apex(last_reconciled: &str, this_tree: &str) -> String {
        format!(
            "# Spec\nStatus: Accepted v1\n\n\
             > **Release currency metadata.**\n\
             > This tree: **{this_tree}**.\n\
             > Last reconciled repository release: **{last_reconciled}**.\n\
             > Implemented scope: **RFCs 001-021**."
        )
    }

    fn valid_changelog() -> String {
        "## [0.20.3] — unreleased\n\
         **Release status:** unreleased\n\n\
         ## [0.20.2] — 2026-07-22 — RFC 021 conditional finalization\n\
         **Release status:** released (tagged 2026-07-22, distributed 2026-07-30)\n\n\
         ## [0.20.1] — 2026-07-17 — RFC 019/RFC 020 architecture recovery (candidate)\n\
         **Release status:** withdrawn candidate\n"
            .to_owned()
    }

    #[test]
    fn ordinary_apex_accepts_tree_strictly_ahead_of_released_last_reconciled() {
        let parsed = parse_ordinary_apex_currency(
            &valid_apex("0.20.2", "0.20.3"),
            "RFCs 001-021",
            &valid_changelog(),
        )
        .unwrap();
        assert_eq!(parsed.release, "v0.20.3");
        assert_eq!(parsed.last_reconciled, "0.20.2");
    }

    #[test]
    fn ordinary_apex_rejects_equality_between_this_tree_and_last_reconciled() {
        // Equality is never valid (Amendment 1 §0.1) — not even at the moment
        // of a release, and not tied to any authenticated context.
        let source = valid_apex("0.20.3", "0.20.3");
        assert!(parse_ordinary_apex_currency(&source, "RFCs 001-021", &valid_changelog()).is_err());
    }

    #[test]
    fn ordinary_apex_rejects_tree_behind_last_reconciled() {
        let source = valid_apex("0.20.3", "0.20.2");
        assert!(parse_ordinary_apex_currency(&source, "RFCs 001-021", &valid_changelog()).is_err());
    }

    #[test]
    fn ordinary_apex_rejects_last_reconciled_naming_an_unreleased_or_withdrawn_record() {
        for version in ["0.20.1", "0.20.3"] {
            let source = valid_apex(version, "0.20.4");
            assert!(
                parse_ordinary_apex_currency(&source, "RFCs 001-021", &valid_changelog()).is_err(),
                "accepted last-reconciled = {version}"
            );
        }
    }

    #[test]
    fn ordinary_apex_rejects_last_reconciled_with_no_changelog_record() {
        let source = valid_apex("0.9.9", "0.20.3");
        assert!(parse_ordinary_apex_currency(&source, "RFCs 001-021", &valid_changelog()).is_err());
    }

    #[test]
    fn ordinary_apex_rejects_scope_disagreeing_with_the_lifecycle() {
        let source = valid_apex("0.20.2", "0.20.3");
        assert!(parse_ordinary_apex_currency(&source, "RFCs 001-022", &valid_changelog()).is_err());
    }

    #[test]
    fn ordinary_apex_rejects_retired_markers() {
        let legacy = valid_apex("0.20.2", "0.20.3")
            + "\n\n> **RFC 020 shared currency metadata (draft).** leftover";
        assert!(parse_ordinary_apex_currency(&legacy, "RFCs 001-021", &valid_changelog()).is_err());
        let conditional = valid_apex("0.20.2", "0.20.3")
            + "\n\n> **RFC 021 conditional release-finalization metadata.** leftover";
        assert!(
            parse_ordinary_apex_currency(&conditional, "RFCs 001-021", &valid_changelog()).is_err()
        );
    }

    #[test]
    fn ordinary_apex_rejects_duplicate_markers() {
        let source =
            valid_apex("0.20.2", "0.20.3") + "\n\n> **Release currency metadata.** duplicate";
        assert!(parse_ordinary_apex_currency(&source, "RFCs 001-021", &valid_changelog()).is_err());
    }

    #[test]
    fn ordinary_apex_rejects_field_displaced_outside_block() {
        let source = valid_apex("0.20.2", "0.20.3")
            .replace("> Last reconciled repository release: **0.20.2**.\n", "")
            + "\n\nHistorical: Last reconciled repository release: **0.20.2**.";
        assert!(parse_ordinary_apex_currency(&source, "RFCs 001-021", &valid_changelog()).is_err());
    }

    #[test]
    fn ordinary_apex_rejects_missing_block_rather_than_passing_vacuously() {
        let source = "# Spec\nNo currency block here.";
        assert!(parse_ordinary_apex_currency(source, "RFCs 001-021", &valid_changelog()).is_err());
    }

    #[test]
    fn retired_prose_check_rejects_leftover_conditional_boundary() {
        let mut errors = Vec::new();
        retired_conditional_prose_in(
            "Before external predicate `P` succeeds, things are pending.",
            "doc.md",
            &mut errors,
        );
        assert!(!errors.is_empty());
    }

    #[test]
    fn retired_prose_check_accepts_plain_released_wording() {
        let mut errors = Vec::new();
        retired_conditional_prose_in(
            "0.20.2 is released; 0.20.3 is in development.",
            "doc.md",
            &mut errors,
        );
        assert!(errors.is_empty());
    }

    /// Representative present-tense conditional claims actually found and
    /// removed from current-facing documents (review 036 B1); each must
    /// individually fail the retired-prose check so a regression is caught.
    #[test]
    fn retired_prose_check_rejects_each_representative_b1_marker() {
        let representative_stale_sentences = [
            "RFCs 019-021 are implementation-complete and conditionally staged.",
            "**Status.** Implemented (conditional finalization for 0.20.2)",
            "> Last released repository release: **0.20.2**.",
            "RFC 021's exact conditional Status and do not prove activation.",
            "Their stored lifecycle paths are not proof of which RFC 021 predicate branch applies.",
            "RFCs 019/020/021 use RFC 021's conditional Status here.",
            "RFCs 019/020/021 are conditionally staged in `done/` under the tracked metadata.",
        ];
        for sentence in representative_stale_sentences {
            let mut errors = Vec::new();
            retired_conditional_prose_in(sentence, "doc.md", &mut errors);
            assert!(!errors.is_empty(), "accepted stale sentence: {sentence}");
        }
    }

    #[test]
    fn changelog_status_parses_each_permitted_value() {
        let source = valid_changelog();
        assert_eq!(
            parse_changelog_release_status(&source, "0.20.2").unwrap(),
            "released"
        );
        assert_eq!(
            parse_changelog_release_status(&source, "0.20.3").unwrap(),
            "unreleased"
        );
        assert_eq!(
            parse_changelog_release_status(&source, "0.20.1").unwrap(),
            "withdrawn candidate"
        );
    }

    #[test]
    fn changelog_status_rejects_missing_heading_missing_marker_and_unknown_value() {
        let source = valid_changelog();
        assert!(parse_changelog_release_status(&source, "9.9.9").is_err());
        assert!(parse_changelog_release_status("## [0.20.2]\nno marker here\n", "0.20.2").is_err());
        assert!(
            parse_changelog_release_status("## [0.20.2]\n**Release status:** maybe\n", "0.20.2")
                .is_err()
        );
    }

    #[test]
    fn changelog_marks_released_distinguishes_released_from_other_statuses() {
        let source = valid_changelog();
        assert!(changelog_marks_released(&source, "0.20.2").is_ok());
        assert!(changelog_marks_released(&source, "0.20.3").is_err());
        assert!(changelog_marks_released(&source, "0.20.1").is_err());
    }

    #[test]
    fn numbered_rfc_file_parses_canonical_names_and_rejects_others() {
        assert_eq!(numbered_rfc_file("001-stratified-scalar.md"), Some(1));
        assert_eq!(
            numbered_rfc_file("024-post-release-documentation-steady-state.md"),
            Some(24)
        );
        for invalid in [
            "README.md",
            "1-x.md",
            "0012-x.md",
            "abc-x.md",
            "001-.md",
            "001x.md",
        ] {
            assert_eq!(numbered_rfc_file(invalid), None, "accepted `{invalid}`");
        }
    }

    #[test]
    fn render_scope_collapses_runs_of_two_or_more_and_keeps_isolated_numbers_bare() {
        let contiguous_with_gaps: BTreeSet<u32> = (1..=18).chain([21, 24]).collect();
        assert_eq!(
            render_scope(&contiguous_with_gaps),
            "RFCs 001-018, 021, 024"
        );

        // The two-element boundary: a run of exactly two collapses, never
        // renders as a comma-separated pair (re-review 037 N1).
        let two_element_boundary: BTreeSet<u32> = [20, 21].into_iter().collect();
        assert_eq!(render_scope(&two_element_boundary), "RFCs 020-021");

        let isolated: BTreeSet<u32> = [5].into_iter().collect();
        assert_eq!(render_scope(&isolated), "RFCs 005");
    }

    struct ScopeFixture {
        root: PathBuf,
    }

    impl ScopeFixture {
        fn new(done_files: &[&str]) -> Self {
            static NEXT_FIXTURE: AtomicUsize = AtomicUsize::new(0);
            let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
                "../target/xtask-tests/doc-currency-scope-{}-{}",
                std::process::id(),
                NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(root.join("rfcs/done")).unwrap();
            for name in done_files {
                fs::write(root.join("rfcs/done").join(name), "# RFC\n").unwrap();
            }
            Self { root }
        }

        fn write_other_folder(&self, folder: &str, name: &str) {
            let dir = self.root.join("rfcs").join(folder);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join(name), "# RFC\n").unwrap();
        }
    }

    impl Drop for ScopeFixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn implemented_scope_binds_the_exact_done_set_and_excludes_rfc_000() {
        let fixture = ScopeFixture::new(&["000-a.md", "001-a.md", "002-a.md", "003-a.md"]);
        let mut errors = Vec::new();
        assert_eq!(
            implemented_scope_at(&fixture.root, &mut errors),
            Some("RFCs 001-003".to_owned())
        );
        assert!(errors.is_empty());
    }

    #[test]
    fn implemented_scope_reflects_accepted_and_archived_gaps() {
        // An RFC sitting in accepted/ or archive/ must never appear in scope,
        // even though a neighbouring number is implemented: only done/ counts
        // (review 036 B2 — a maximum is not a set).
        let fixture = ScopeFixture::new(&["001-a.md", "002-a.md", "004-a.md"]);
        fixture.write_other_folder("accepted", "003-a.md");
        fixture.write_other_folder("archive", "005-a.md");
        let mut errors = Vec::new();
        assert_eq!(
            implemented_scope_at(&fixture.root, &mut errors),
            Some("RFCs 001-002, 004".to_owned())
        );
        assert!(errors.is_empty());
    }

    #[test]
    fn implemented_scope_fails_closed_on_a_malformed_done_file_name() {
        let fixture = ScopeFixture::new(&["001-a.md", "not-a-valid-name.md"]);
        let mut errors = Vec::new();
        assert_eq!(implemented_scope_at(&fixture.root, &mut errors), None);
        assert!(!errors.is_empty());
    }

    #[test]
    fn index_status_is_folder_specific() {
        let accepted =
            "| [020](accepted/020-x.md) | X | Accepted (design frozen 2026-07-15) | note |";
        assert!(index_status_matches("accepted", accepted));
        assert!(!index_status_matches("proposed", accepted));
        assert!(index_status_matches(
            "done",
            "| [018](done/018-x.md) | X | Implemented (v0.20.0) | note |"
        ));
        assert!(index_status_matches(
            "proposed",
            "| [021](proposed/021-x.md) | X | Proposed | note |"
        ));
        assert!(index_status_matches(
            "archive",
            "| [022](archive/022-x.md) | X | Superseded by RFC 023 | note |"
        ));
        assert!(!index_status_matches(
            "done",
            "| [018](done/018-x.md) | X | Proposed but Implemented elsewhere | note |"
        ));
    }

    #[test]
    fn design_freeze_requires_complete_valid_date() {
        assert_eq!(
            parse_design_freeze("Accepted (design frozen 2026-07-15)"),
            Some("2026-07-15")
        );
        assert_eq!(parse_design_freeze("Accepted (design frozen )"), None);
        assert_eq!(
            parse_design_freeze("Accepted (design frozen 2026-02-30)"),
            None
        );
        assert_eq!(
            parse_design_freeze("Accepted (design frozen 2026/07/15)"),
            None
        );
    }

    #[test]
    fn source_and_index_freeze_dates_can_be_compared_exactly() {
        let source = parse_design_freeze("Accepted (design frozen 2026-07-15)");
        let matching_index = parse_design_freeze("Accepted (design frozen 2026-07-15)");
        let mismatched_index = parse_design_freeze("Accepted (design frozen 2026-07-16)");
        assert_eq!(source, matching_index);
        assert_ne!(source, mismatched_index);
    }

    #[test]
    fn stale_phrase_scan_catches_current_claims() {
        let mut errors = Vec::new();
        stale_phrases_in("Status: current as of v0.13.1", "synthetic", &mut errors);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn stale_phrase_scan_ignores_markdown_decoration() {
        let mut errors = Vec::new();
        stale_phrases_in("`release-gate` remains an alias", "synthetic", &mut errors);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn stale_phrase_scan_normalizes_wrapped_decorated_text() {
        let mut errors = Vec::new();
        stale_phrases_in(
            "Current: no **std-side** solver\n`kernel` exists yet.",
            "synthetic",
            &mut errors,
        );
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn historical_suffix_is_outside_root_current_status_scan() {
        let source = "Current status is No-Go.\n### Historical completion: old\nno std-side solver kernel exists yet";
        let mut errors = Vec::new();
        stale_phrases_in(current_before_historical(source), "synthetic", &mut errors);
        assert!(errors.is_empty());
    }

    #[test]
    fn stale_phrase_scan_rejects_same_phrase_before_history() {
        let source = "no std-side solver kernel exists yet\n### Historical completion: old";
        let mut errors = Vec::new();
        stale_phrases_in(current_before_historical(source), "synthetic", &mut errors);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn readme_length_accepts_the_band_and_rejects_either_side() {
        let body = |n: usize| "line\n".repeat(n);
        assert!(super::readme_length_findings(&body(100)).is_empty());
        assert!(super::readme_length_findings(&body(200)).is_empty());
        assert_eq!(super::readme_length_findings(&body(99)).len(), 1);
        assert_eq!(super::readme_length_findings(&body(201)).len(), 1);
    }

    fn readme_with(block: &str) -> String {
        format!(
            "# Loeres\n\n## Overview\n\n```rust\nnot the example\n```\n\n\
             ## Quick Start\n\n```sh\ncargo check\n```\n\n```rust\n{block}```\n\n## More\n"
        )
    }

    fn source_with(region: &str) -> String {
        format!(
            "fn a() {{}}\n// README-EXAMPLE-BEGIN\n{region}// README-EXAMPLE-END\nfn b() {{}}\n"
        )
    }

    #[test]
    fn readme_example_matches_the_marked_region_and_reports_a_changed_digit() {
        let region = "fn step() -> f64 {\n    0.9\n}\n";
        assert!(
            super::readme_example_findings(&readme_with(region), &source_with(region)).is_empty()
        );

        let changed = readme_with("fn step() -> f64 {\n    0.8\n}\n");
        let findings = super::readme_example_findings(&changed, &source_with(region));
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].contains("does not match"), "{findings:?}");
    }

    #[test]
    fn readme_example_reports_an_over_long_region_and_missing_markers() {
        let long: String = "let x = 0;\n".repeat(20);
        let findings = super::readme_example_findings(&readme_with(&long), &source_with(&long));
        assert!(
            findings.iter().any(|f| f.contains("limit is 19")),
            "{findings:?}"
        );

        let no_markers = super::readme_example_findings(&readme_with("x\n"), "fn a() {}\n");
        assert_eq!(no_markers.len(), 1, "{no_markers:?}");
        assert!(no_markers[0].contains("no complete"), "{no_markers:?}");

        let unclosed = "// README-EXAMPLE-BEGIN\nx\n";
        assert!(super::marked_region(unclosed).is_none());
        assert!(
            super::marked_region(
                "// README-EXAMPLE-BEGIN\n// README-EXAMPLE-BEGIN\n// README-EXAMPLE-END\n"
            )
            .is_none()
        );
    }

    #[test]
    fn quick_start_block_ignores_rust_blocks_before_the_heading() {
        let readme = readme_with("fn real() {}\n");
        assert_eq!(
            super::quick_start_rust_block(&readme).as_deref(),
            Some("fn real() {}\n")
        );
        assert!(super::quick_start_rust_block("# no quick start\n```rust\nx\n```\n").is_none());
    }

    // RFC 038: the per-example README guide and its doc-currency assertions.

    struct ExampleFixture {
        root: PathBuf,
    }

    impl ExampleFixture {
        fn new(directories: &[&str]) -> Self {
            static NEXT_FIXTURE: AtomicUsize = AtomicUsize::new(0);
            let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
                "../target/xtask-tests/doc-currency-examples-{}-{}",
                std::process::id(),
                NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
            ));
            for name in directories {
                fs::create_dir_all(root.join("examples").join(name)).unwrap();
            }
            Self { root }
        }

        fn write_readme(&self, name: &str, body: &str) {
            fs::write(
                self.root.join("examples").join(name).join("README.md"),
                body,
            )
            .unwrap();
        }
    }

    impl Drop for ExampleFixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn module_doc_second_paragraph_extracts_a_multi_line_paragraph() {
        let source = "//! Title.\n//!\n//! Line one.\n//! Line two.\n//!\n//! Third paragraph.\n";
        assert_eq!(
            super::module_doc_second_paragraph(source).as_deref(),
            Some("Line one. Line two.")
        );
    }

    #[test]
    fn module_doc_second_paragraph_is_none_when_the_block_has_only_a_title() {
        assert!(super::module_doc_second_paragraph("//! Title only.\n").is_none());
        assert!(super::module_doc_second_paragraph("//! Title only.\n\nfn main() {}\n").is_none());
    }

    #[test]
    fn module_doc_paragraphs_treats_a_bare_marker_line_as_a_blank_separator() {
        // A bare `//!` line (no trailing space) must split paragraphs rather
        // than being swallowed into the surrounding text: `rest` is already
        // `//!`-stripped, and the fallback on a missing space must stay on
        // `rest`, not revert to the original `//!`-prefixed line.
        let source = "//! Title.\n//!\n//! Body.\n";
        assert_eq!(
            super::module_doc_paragraphs(source),
            vec!["Title.".to_owned(), "Body.".to_owned()]
        );
    }

    #[test]
    fn readme_problem_paragraph_skips_the_title_heading() {
        let readme = "# Title\n\nThe problem, reflowed\nto one paragraph.\n\nRun:\n";
        assert_eq!(
            super::readme_problem_paragraph(readme).as_deref(),
            Some("The problem, reflowed to one paragraph.")
        );
        assert!(super::readme_problem_paragraph("# Title only\n").is_none());
    }

    #[test]
    fn extraction_matches_a_reflowed_paragraph_regardless_of_line_wrap() {
        let main_rs = "//! Title.\n//!\n//! Line one.\n//! Line two.\n";
        let readme = "# Title\n\nLine one.\nLine two.\n\nRun it.\n";
        assert!(super::extraction_findings_from("x", main_rs, readme).is_empty());
    }

    #[test]
    fn extraction_reports_a_mismatch_naming_the_example_and_both_texts() {
        let main_rs = "//! Title.\n//!\n//! The real problem statement.\n";
        let readme = "# Title\n\nA different sentence entirely.\n";
        let findings = super::extraction_findings_from("cluster-counted-work", main_rs, readme);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].contains("cluster-counted-work"), "{findings:?}");
        assert!(
            findings[0].contains("The real problem statement."),
            "{findings:?}"
        );
        assert!(
            findings[0].contains("A different sentence entirely."),
            "{findings:?}"
        );
    }

    #[test]
    fn extraction_reports_a_missing_second_paragraph_as_a_finding_not_a_panic() {
        let main_rs = "//! Title only, no second paragraph.\n";
        let readme = "# Title\n\nSome problem text.\n";
        let findings = super::extraction_findings_from("x", main_rs, readme);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].contains("no second"), "{findings:?}");
    }

    #[test]
    fn extraction_reports_a_missing_readme_paragraph() {
        let main_rs = "//! Title.\n//!\n//! A problem statement.\n";
        let readme = "# Title only\n";
        let findings = super::extraction_findings_from("x", main_rs, readme);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].contains("no problem paragraph"), "{findings:?}");
    }

    #[test]
    fn registry_symmetry_accepts_the_real_six_registered_directories() {
        let directories: Vec<String> = super::EXAMPLE_REGISTRY
            .iter()
            .map(|&(name, _)| name.to_owned())
            .collect();
        assert!(super::registry_symmetry_findings(&directories).is_empty());
    }

    #[test]
    fn registry_symmetry_reports_a_registered_name_with_no_directory() {
        let directories: Vec<String> = super::EXAMPLE_REGISTRY
            .iter()
            .skip(1)
            .map(|&(name, _)| name.to_owned())
            .collect();
        let findings = super::registry_symmetry_findings(&directories);
        let missing = super::EXAMPLE_REGISTRY[0].0;
        assert!(
            findings
                .iter()
                .any(|f| f.contains(missing) && f.contains("is registered but")),
            "{findings:?}"
        );
    }

    #[test]
    fn registry_symmetry_reports_an_unregistered_directory() {
        let mut directories: Vec<String> = super::EXAMPLE_REGISTRY
            .iter()
            .map(|&(name, _)| name.to_owned())
            .collect();
        directories.push("not-registered".to_owned());
        let findings = super::registry_symmetry_findings(&directories);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].contains("not-registered"), "{findings:?}");
        assert!(findings[0].contains("not registered"), "{findings:?}");
    }

    #[test]
    fn readme_presence_reports_each_directory_missing_a_readme() {
        let fixture = ExampleFixture::new(&["a", "b"]);
        fixture.write_readme("a", "# A\n\nProblem.\n");
        let findings =
            super::readme_presence_findings(&fixture.root, &["a".to_owned(), "b".to_owned()]);
        assert_eq!(
            findings,
            vec!["EXAMPLE README: examples/b has no README.md".to_owned()]
        );
    }

    #[test]
    fn section_after_heading_stops_at_the_next_heading() {
        let markdown = "# Top\n\n## Examples\n\nrow one\nrow two\n\n## Design Notes\n\nother\n";
        assert_eq!(
            super::section_after_heading(markdown, "## Examples"),
            Some("\nrow one\nrow two\n")
        );
        assert!(super::section_after_heading(markdown, "## Missing").is_none());
    }

    #[test]
    fn table_symmetry_reports_both_directions_in_both_tables() {
        let directories = vec!["a".to_owned(), "b".to_owned()];
        let root_readme =
            "# Loeres\n\n## Examples\n\n| [`a`](examples/a/) | p | device |\n\n## Design Notes\n";
        let root_table = super::section_after_heading(root_readme, "## Examples").unwrap();
        let examples_index =
            "# Examples\n\n| [`a`](a/) | p | device |\n| [`c`](c/) | p | device |\n";
        let mut findings = super::table_symmetry_findings(&directories, "README.md", root_table);
        findings.extend(super::table_symmetry_findings(
            &directories,
            "examples/README.md",
            examples_index,
        ));
        assert!(
            findings
                .iter()
                .any(|f| f.contains("examples/b has no row in README.md")),
            "{findings:?}"
        );
        assert!(
            findings
                .iter()
                .any(|f| f.contains("examples/b has no row in examples/README.md")),
            "{findings:?}"
        );
        assert!(
            findings
                .iter()
                .any(|f| f.contains("examples/README.md links examples/c, which does not exist")),
            "{findings:?}"
        );
    }

    #[test]
    fn table_symmetry_ignores_a_backtick_code_link_outside_a_table_row() {
        // A prose "see also" link in the same backtick-code style as a table
        // cell (e.g. `[`README`](../README.md)`) must not be read as a table
        // row naming a directory `README`.
        let directories = vec!["a".to_owned()];
        let index = "# Examples\n\n\
                      | [`a`](a/) | p | device |\n\n\
                      See also: the root [`README`](../README.md).\n";
        let findings = super::table_symmetry_findings(&directories, "examples/README.md", index);
        assert!(findings.is_empty(), "{findings:?}");
    }

    /// RFC 049 S4 §4.3's seven required branches, each on a literal rather
    /// than a real file, so the predicate is exercised rather than merely
    /// satisfied by today's tree.
    fn real_readme_fence() -> String {
        "```toml\n\
         # Cluster / server user\n\
         loeres-cluster        = { version = \"0.24\", features = [\"parallel-rayon\"] }\n\
         loeres-backend-std    = { version = \"0.24\", features = [\"dense\"] }\n\n\
         # Device / edge user\n\
         loeres-device         = { version = \"0.24\", default-features = false }\n\
         loeres-backend-static = { version = \"0.24\", default-features = false, features = [\"owned-arrays\"] }\n\
         ```\n"
            .to_owned()
    }

    #[test]
    fn readme_requirements_accepts_the_four_real_sites_at_the_released_minor() {
        let findings = readme_requirement_findings("README.md", &real_readme_fence(), Some("0.24"));
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn readme_requirements_reports_a_wildcard_naming_the_crate_and_line() {
        let readme = "```toml\nloeres-device = { version = \"0.x\" }\n```\n";
        let findings = readme_requirement_findings("README.md", readme, Some("0.24"));
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].contains("README.md:2"), "{findings:?}");
        assert!(findings[0].contains("loeres-device"), "{findings:?}");
        assert!(findings[0].contains("wildcard"), "{findings:?}");
    }

    #[test]
    fn readme_requirements_reports_a_stale_minor_and_reads_the_value_rather_than_a_literal() {
        let readme = "```toml\nloeres-device = { version = \"0.23\" }\n```\n";
        let stale = readme_requirement_findings("README.md", readme, Some("0.24"));
        assert_eq!(stale.len(), 1, "{stale:?}");
        assert!(stale[0].contains("README.md:2"), "{stale:?}");
        assert!(stale[0].contains("loeres-device"), "{stale:?}");

        // Same input, the value the site actually names: no finding. This is
        // what proves the function compares against `expected_minor` rather
        // than a literal `"0.24"` baked into the predicate.
        let matching = readme_requirement_findings("README.md", readme, Some("0.23"));
        assert!(matching.is_empty(), "{matching:?}");
    }

    #[test]
    fn readme_requirements_reports_a_wildcard_in_a_per_crate_readme() {
        // Clause 2 over `crates/*/README.md` matches nothing in the tree
        // today — no per-crate README contains `version =` at all — so this
        // synthetic site is the only thing that has ever exercised it.
        let readme = "```toml\nloeres = { version = \"0.x\" }\n```\n";
        let findings = readme_requirement_findings("crates/loeres/README.md", readme, None);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(
            findings[0].contains("crates/loeres/README.md:2"),
            "{findings:?}"
        );
        assert!(findings[0].contains("wildcard"), "{findings:?}");
    }

    #[test]
    fn readme_requirements_with_no_expected_minor_reports_the_wildcard_and_nothing_else() {
        let readme = "```toml\nloeres-device = { version = \"0.x\" }\n```\n";
        let findings = readme_requirement_findings("README.md", readme, None);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].contains("wildcard"), "{findings:?}");
        assert!(
            !findings[0].contains("expected"),
            "a clause-1 finding must not also fire: {findings:?}"
        );
    }

    #[test]
    fn readme_requirements_ignores_a_version_line_outside_any_toml_fence() {
        let readme = "Prose mentioning version = \"0.x\" is not a requirement site.\n";
        let findings = readme_requirement_findings("README.md", readme, Some("0.24"));
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn readme_requirements_fails_closed_on_the_dependencies_table_shape() {
        let readme = "```toml\n[dependencies.loeres-device]\nversion = \"0.24\"\n```\n";
        let findings = readme_requirement_findings("README.md", readme, Some("0.24"));
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].contains("README.md:3"), "{findings:?}");
        assert!(
            findings[0].contains("not a shape this assertion reads"),
            "{findings:?}"
        );
    }

    /// RFC 051 §3.2's five required branches.
    #[test]
    fn import_model_accepts_fences_with_no_version_site() {
        let source = "```toml\n\
                       [dependencies]\n\
                       loeres-cluster = { features = [\"parallel-rayon\"] }  # version: see README.md\n\
                       loeres-backend-std = { features = [\"dense\"] }       # version: see README.md\n\
                       ```\n";
        let findings = import_model_findings("docs/specs/loeres-external-design-v1.md", source);
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn import_model_rejects_a_correct_version_too() {
        // Treatment B asserts the absence of a requirement site, not the
        // wildcard — a *correct* version is still a finding, which is the
        // whole point of B over A.
        let source = "```toml\nloeres-cluster = { version = \"0.24\", features = [\"parallel-rayon\"] }\n```\n";
        let findings = import_model_findings("docs/specs/loeres-external-design-v1.md", source);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(
            findings[0].contains("docs/specs/loeres-external-design-v1.md:2"),
            "{findings:?}"
        );
        assert!(findings[0].contains("loeres-cluster"), "{findings:?}");
    }

    #[test]
    fn import_model_rejects_the_historical_wildcard() {
        let source =
            "```toml\nloeres-device = { version = \"0.x\", default-features = false }\n```\n";
        let findings = import_model_findings("docs/specs/loeres-external-design-v1.md", source);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].contains("loeres-device"), "{findings:?}");
    }

    #[test]
    fn import_model_ignores_prose_naming_a_version_outside_a_fence() {
        let source = "The requirement, `version = \"0.24\"`, lives in README.md.\n";
        let findings = import_model_findings("docs/specs/loeres-external-design-v1.md", source);
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn import_model_accepts_the_s1_comment_as_prose_not_a_site() {
        // The one false positive this slice could plausibly ship: the
        // trailing `# version: see README.md` comment must not itself read
        // as a requirement site.
        let source = "```toml\nloeres-cluster = { features = [\"parallel-rayon\"] }  # version: see README.md\n```\n";
        let findings = import_model_findings("docs/specs/loeres-external-design-v1.md", source);
        assert!(findings.is_empty(), "{findings:?}");
    }

    /// RFC 052 §3.7's twelve required branches, each on a literal
    /// reproduction of the real table and the real `CHANGELOG.md` status
    /// set (test 1) or a minimal perturbation of it.
    fn release_table_valid_specifications() -> String {
        "## Release currency\n\n\
         This is the one home for release currency.\n\n\
         | Repository release | Shipped | Carries RFCs |\n\
         |---|---|---|\n\
         | `0.20.2` | 2026-07-30 | 001-021 |\n\
         | `0.21.0` | 2026-09-12 | 001-026 |\n\
         | `0.21.1` | 2026-09-24 | 001-029 |\n\
         | `0.21.2` | 2026-09-24 | 001-030 |\n\
         | `0.21.3` | 2026-09-24 | 001-033 |\n\
         | `0.22.0` | 2026-09-24 | 001-034 |\n\
         | `0.22.1` | 2026-10-07 | 001-036 |\n\
         | `0.22.2` | 2026-10-07 | 001-037 |\n\
         | `0.22.3` | 2026-10-07 | 001-040 |\n\
         | `0.23.0` | 2026-10-08 | 001-042 |\n\
         | `0.24.0` | 2026-10-08 | 001-044 |\n\n\
         **What is on crates.io.** The newest published version of every Loeres crate is \
         `0.24.0`, published on 2026-10-08. It carries RFCs 001-044.\n\n\
         Historical notes follow.\n"
            .to_owned()
    }

    fn release_table_valid_changelog() -> String {
        "## [0.24.1] — unreleased\n\
         **Release status:** unreleased\n\n\
         ## [0.24.0] — 2026-10-08\n\
         **Release status:** released (tagged 2026-10-08, distributed 2026-10-08); published to \
         crates.io\n\n\
         ## [0.23.0] — 2026-10-08\n\
         **Release status:** released (tagged 2026-10-08, distributed 2026-10-08); published to \
         crates.io\n\n\
         ## [0.22.3] — 2026-10-07\n\
         **Release status:** released (tagged 2026-10-07, distributed 2026-10-07); published to \
         crates.io\n\n\
         ## [0.22.2] — 2026-10-07\n\
         **Release status:** released (tagged 2026-10-07, distributed 2026-10-07); published to \
         crates.io\n\n\
         ## [0.22.1] — 2026-10-07\n\
         **Release status:** released (tagged 2026-10-07, distributed 2026-10-07); published to \
         crates.io\n\n\
         ## [0.22.0] — 2026-09-24\n\
         **Release status:** released (tagged 2026-09-24, distributed 2026-09-24); not published \
         to crates.io\n\n\
         ## [0.21.3] — 2026-09-24\n\
         **Release status:** released (tagged 2026-09-24, distributed 2026-09-24); not published \
         to crates.io\n\n\
         ## [0.21.2] — 2026-09-24\n\
         **Release status:** released (tagged 2026-09-24, distributed 2026-09-24); not published \
         to crates.io\n\n\
         ## [0.21.1] — 2026-09-24\n\
         **Release status:** released (tagged 2026-09-24, distributed 2026-09-24); not published \
         to crates.io\n\n\
         ## [0.21.0] — 2026-09-12\n\
         **Release status:** released (tagged 2026-09-12, distributed 2026-09-12); not published \
         to crates.io\n\n\
         ## [0.20.2] — 2026-07-22\n\
         **Release status:** released (tagged 2026-07-22, distributed 2026-07-30); published to \
         crates.io\n\n\
         ## [0.20.1] — 2026-07-17\n\
         **Release status:** withdrawn candidate\n"
            .to_owned()
    }

    #[test]
    fn release_table_accepts_the_real_table_and_the_real_status_set() {
        let findings = release_table_findings(
            &release_table_valid_specifications(),
            &release_table_valid_changelog(),
            Some("0.24.0"),
            Some("RFCs 001-044"),
        );
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn release_table_reports_a_released_version_with_no_row() {
        let specifications = release_table_valid_specifications()
            .replace("| `0.22.1` | 2026-10-07 | 001-036 |\n", "");
        let findings = release_table_findings(
            &specifications,
            &release_table_valid_changelog(),
            Some("0.24.0"),
            Some("RFCs 001-044"),
        );
        assert!(
            findings
                .iter()
                .any(|f| f.contains("0.22.1") && f.contains("no row")),
            "{findings:?}"
        );
    }

    #[test]
    fn release_table_reports_an_extra_row_for_an_unreleased_version() {
        let specifications = release_table_valid_specifications().replace(
            "| `0.24.0` | 2026-10-08 | 001-044 |\n",
            "| `0.24.0` | 2026-10-08 | 001-044 |\n| `0.24.1` | 2026-10-30 | 001-044 |\n",
        );
        let findings = release_table_findings(
            &specifications,
            &release_table_valid_changelog(),
            Some("0.24.0"),
            Some("RFCs 001-044"),
        );
        assert!(
            findings
                .iter()
                .any(|f| f.contains("0.24.1") && f.contains("unreleased")),
            "{findings:?}"
        );
    }

    #[test]
    fn release_table_reports_two_rows_out_of_order() {
        let specifications = release_table_valid_specifications().replace(
            "| `0.20.2` | 2026-07-30 | 001-021 |\n| `0.21.0` | 2026-09-12 | 001-026 |\n",
            "| `0.21.0` | 2026-09-12 | 001-026 |\n| `0.20.2` | 2026-07-30 | 001-021 |\n",
        );
        let findings = release_table_findings(
            &specifications,
            &release_table_valid_changelog(),
            Some("0.24.0"),
            Some("RFCs 001-044"),
        );
        assert!(
            findings
                .iter()
                .any(|f| f.contains("0.21.0") && f.contains("0.20.2") && f.contains("ascending")),
            "{findings:?}"
        );
    }

    #[test]
    fn release_table_reports_a_shipped_cell_carrying_the_tagged_date() {
        // Review 118 F1 as a test: `0.20.2` is tagged `2026-07-22` and
        // distributed `2026-07-30`; the cell must be the latter.
        let specifications = release_table_valid_specifications().replace(
            "| `0.20.2` | 2026-07-30 | 001-021 |",
            "| `0.20.2` | 2026-07-22 | 001-021 |",
        );
        let findings = release_table_findings(
            &specifications,
            &release_table_valid_changelog(),
            Some("0.24.0"),
            Some("RFCs 001-044"),
        );
        assert!(
            findings
                .iter()
                .any(|f| f.contains("0.20.2") && f.contains("distributed `2026-07-30`")),
            "{findings:?}"
        );
    }

    #[test]
    fn release_table_reports_a_last_row_not_the_reconciled_release() {
        let findings = release_table_findings(
            &release_table_valid_specifications(),
            &release_table_valid_changelog(),
            Some("0.23.0"),
            Some("RFCs 001-044"),
        );
        assert!(
            findings
                .iter()
                .any(|f| f.contains("0.24.0") && f.contains("0.23.0")),
            "{findings:?}"
        );
    }

    #[test]
    fn release_table_accepts_the_cut_window_with_no_findings() {
        // Review 118 F2 as a test, and the one not to skip: `implemented_scope`
        // runs ahead of every row during a release cut. Equality would fail
        // this; the inequality must not.
        let findings = release_table_findings(
            &release_table_valid_specifications(),
            &release_table_valid_changelog(),
            Some("0.24.0"),
            Some("RFCs 001-052"),
        );
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn release_table_reports_a_last_row_maximum_exceeding_the_scope() {
        let findings = release_table_findings(
            &release_table_valid_specifications(),
            &release_table_valid_changelog(),
            Some("0.24.0"),
            Some("RFCs 001-040"),
        );
        assert!(
            findings
                .iter()
                .any(|f| f.contains("44") && f.contains("40")),
            "{findings:?}"
        );
    }

    #[test]
    fn release_table_reports_a_decreasing_carries_sequence() {
        let specifications = release_table_valid_specifications().replace(
            "| `0.22.0` | 2026-09-24 | 001-034 |",
            "| `0.22.0` | 2026-09-24 | 001-030 |",
        );
        let findings = release_table_findings(
            &specifications,
            &release_table_valid_changelog(),
            Some("0.24.0"),
            Some("RFCs 001-044"),
        );
        assert!(
            findings
                .iter()
                .any(|f| f.contains("0.22.0") && f.contains("decreases")),
            "{findings:?}"
        );
    }

    #[test]
    fn release_table_scope_with_a_gap_parses_to_its_true_maximum() {
        let findings = release_table_findings(
            &release_table_valid_specifications(),
            &release_table_valid_changelog(),
            Some("0.24.0"),
            Some("RFCs 001-040, 042-044"),
        );
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn release_table_reports_the_crates_io_paragraph_naming_an_unpublished_version() {
        let specifications = release_table_valid_specifications().replace(
            "The newest published version of every Loeres crate is `0.24.0`, published on \
             2026-10-08. It carries RFCs 001-044.",
            "The newest published version of every Loeres crate is `0.22.0`, published on \
             2026-09-24. It carries RFCs 001-034.",
        );
        let findings = release_table_findings(
            &specifications,
            &release_table_valid_changelog(),
            Some("0.24.0"),
            Some("RFCs 001-044"),
        );
        assert!(
            findings
                .iter()
                .any(|f| f.contains("0.24.0") && f.contains("crates.io")),
            "{findings:?}"
        );
    }

    #[test]
    fn release_table_reports_a_malformed_row() {
        let specifications = release_table_valid_specifications().replace(
            "| `0.24.0` | 2026-10-08 | 001-044 |\n",
            "| `0.24.0` | 2026-10-08 |\n",
        );
        let findings = release_table_findings(
            &specifications,
            &release_table_valid_changelog(),
            Some("0.24.0"),
            Some("RFCs 001-044"),
        );
        assert!(
            findings.iter().any(|f| f.contains("malformed row")),
            "{findings:?}"
        );
    }

    /// RFC 052 corrections §5's six required branches. The twelve tests
    /// above are unedited, per §7.
    #[test]
    fn release_table_crates_io_names_the_newest_published_not_the_newest_released() {
        // Architect review 119 F1 as a test: `0.24.0` is the newest
        // *released* version but (here) not published; clause 5 must
        // demand the paragraph name `0.23.0`, the newest version that
        // *is* published, not accept the paragraph's claim of `0.24.0`
        // just because `0.24.0` is the newest release.
        let changelog = release_table_valid_changelog().replace(
            "## [0.24.0] — 2026-10-08\n\
             **Release status:** released (tagged 2026-10-08, distributed 2026-10-08); \
             published to crates.io\n\n",
            "## [0.24.0] — 2026-10-08\n\
             **Release status:** released (tagged 2026-10-08, distributed 2026-10-08); not \
             published to crates.io\n\n",
        );
        let findings = release_table_findings(
            &release_table_valid_specifications(),
            &changelog,
            Some("0.24.0"),
            Some("RFCs 001-044"),
        );
        assert!(
            findings
                .iter()
                .any(|f| f.contains("0.23.0") && f.contains("crates.io")),
            "{findings:?}"
        );
    }

    #[test]
    fn release_table_crates_io_reports_no_published_version_when_none_is() {
        let changelog = release_table_valid_changelog()
            .replace("; published to crates.io", "; not published to crates.io");
        let findings = release_table_findings(
            &release_table_valid_specifications(),
            &changelog,
            Some("0.24.0"),
            Some("RFCs 001-044"),
        );
        assert!(
            findings
                .iter()
                .any(|f| f.contains("names no version published to crates.io")),
            "{findings:?}"
        );
    }

    #[test]
    fn release_table_crates_io_fails_closed_on_an_unrecognized_publication_clause() {
        let changelog = release_table_valid_changelog().replace(
            "## [0.22.0] — 2026-09-24\n\
             **Release status:** released (tagged 2026-09-24, distributed 2026-09-24); not \
             published to crates.io\n\n",
            "## [0.22.0] — 2026-09-24\n\
             **Release status:** released (tagged 2026-09-24, distributed 2026-09-24); \
             published on request\n\n",
        );
        let findings = release_table_findings(
            &release_table_valid_specifications(),
            &changelog,
            Some("0.24.0"),
            Some("RFCs 001-044"),
        );
        assert!(
            findings
                .iter()
                .any(|f| f.contains("0.22.0") && f.contains("neither")),
            "{findings:?}"
        );
    }

    #[test]
    fn release_table_shipped_reports_a_missing_distributed_field() {
        // Pins F2's refactor against regression: the existing clause-3
        // finding must still fire when the parenthesised group has no
        // `distributed` field at all.
        let changelog = release_table_valid_changelog().replace(
            "## [0.20.2] — 2026-07-22\n\
             **Release status:** released (tagged 2026-07-22, distributed 2026-07-30); \
             published to crates.io\n\n",
            "## [0.20.2] — 2026-07-22\n\
             **Release status:** released (tagged 2026-07-22); published to crates.io\n\n",
        );
        let findings = release_table_findings(
            &release_table_valid_specifications(),
            &changelog,
            Some("0.24.0"),
            Some("RFCs 001-044"),
        );
        assert!(
            findings
                .iter()
                .any(|f| f.contains("0.20.2") && f.contains("no valid")),
            "{findings:?}"
        );
    }

    #[test]
    fn release_table_shipped_reports_an_unparseable_distributed_date() {
        let changelog = release_table_valid_changelog().replace(
            "## [0.20.2] — 2026-07-22\n\
             **Release status:** released (tagged 2026-07-22, distributed 2026-07-30); \
             published to crates.io\n\n",
            "## [0.20.2] — 2026-07-22\n\
             **Release status:** released (tagged 2026-07-22, distributed 2026-13-99); \
             published to crates.io\n\n",
        );
        let findings = release_table_findings(
            &release_table_valid_specifications(),
            &changelog,
            Some("0.24.0"),
            Some("RFCs 001-044"),
        );
        assert!(
            findings
                .iter()
                .any(|f| f.contains("0.20.2") && f.contains("no valid")),
            "{findings:?}"
        );
    }

    #[test]
    fn release_table_reports_exactly_one_finding_when_the_header_is_missing() {
        let specifications = release_table_valid_specifications().replace(
            "| Repository release | Shipped | Carries RFCs |",
            "| Repository release | Shipped date | Carries RFCs |",
        );
        let findings = release_table_findings(
            &specifications,
            &release_table_valid_changelog(),
            Some("0.24.0"),
            Some("RFCs 001-044"),
        );
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].contains("no table header"), "{findings:?}");
    }

    /// RFC 053 S3.4's eight required branches.
    #[test]
    fn phase_reference_to_an_undefined_phase_fails() {
        let defined: BTreeSet<String> = ["I".to_owned()].into_iter().collect();
        let files = vec![(
            PathBuf::from("doc.md"),
            "See Phase II for details.\n".to_owned(),
        )];
        let findings = phase_reference_findings(&files, &defined);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].contains("Phase II"), "{findings:?}");
        assert!(findings[0].contains("doc.md:1"), "{findings:?}");
    }

    #[test]
    fn phase_vii_fails_proving_a_roman_run_not_an_enumeration() {
        let defined: BTreeSet<String> = ["I", "II", "III", "IV", "V", "VI"]
            .iter()
            .map(|s| (*s).to_owned())
            .collect();
        let files = vec![(
            PathBuf::from("doc.md"),
            "A future Phase VII would need defining.\n".to_owned(),
        )];
        let findings = phase_reference_findings(&files, &defined);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].contains("Phase VII"), "{findings:?}");
    }

    #[test]
    fn an_undefined_reference_inside_backticks_passes_and_unbackticked_fails() {
        let defined: BTreeSet<String> = ["I".to_owned()].into_iter().collect();
        let quoted = vec![(
            PathBuf::from("doc.md"),
            "Quoting `Phase IX` as text.\n".to_owned(),
        )];
        assert!(
            phase_reference_findings(&quoted, &defined).is_empty(),
            "a backticked reference must not be checked"
        );

        let real = vec![(
            PathBuf::from("doc.md"),
            "Citing Phase IX for real.\n".to_owned(),
        )];
        let findings = phase_reference_findings(&real, &defined);
        assert_eq!(findings.len(), 1, "{findings:?}");
    }

    #[test]
    fn arabic_phase_2_does_not_match() {
        let defined: BTreeSet<String> = BTreeSet::new();
        let files = vec![(
            PathBuf::from("doc.md"),
            "Phase 2 is the static backend.\n".to_owned(),
        )];
        let findings = phase_reference_findings(&files, &defined);
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn a_state_of_in_progress_fails_closed() {
        let phases = vec![DestinationPhase {
            numeral: "I".to_owned(),
            state: "in progress".to_owned(),
            rfcs: Vec::new(),
        }];
        let findings = phase_state_findings(&phases);
        assert_eq!(findings.len(), 1, "{findings:?}");
    }

    #[test]
    fn a_state_of_unscheduled_fails_the_substring_trap() {
        // The literal a `contains("scheduled")` implementation would pass.
        let phases = vec![DestinationPhase {
            numeral: "II".to_owned(),
            state: "unscheduled".to_owned(),
            rfcs: Vec::new(),
        }];
        let findings = phase_state_findings(&phases);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].contains("unscheduled"), "{findings:?}");
    }

    #[test]
    fn a_shipped_phase_naming_an_accepted_not_done_rfc_fails() {
        let phases = vec![DestinationPhase {
            numeral: "I".to_owned(),
            state: "shipped".to_owned(),
            rfcs: vec!["047".to_owned()],
        }];
        let done_numbers: BTreeSet<u32> = [49, 50, 51].into_iter().collect();
        let findings = phase_rfc_findings(&phases, &done_numbers);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].contains("047"), "{findings:?}");
    }

    #[test]
    fn a_deferred_phase_naming_no_rfc_passes() {
        let phases = vec![DestinationPhase {
            numeral: "II".to_owned(),
            state: "deferred".to_owned(),
            rfcs: Vec::new(),
        }];
        let done_numbers: BTreeSet<u32> = BTreeSet::new();
        let findings = phase_rfc_findings(&phases, &done_numbers);
        assert!(findings.is_empty(), "{findings:?}");
    }
}
