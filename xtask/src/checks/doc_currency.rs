//! `doc-currency` — bounded RFC 020 documentation currency assertions.
//!
//! This gate checks stable metadata, lifecycle/index facts, navigation, and a
//! small explicit stale-phrase ledger. It deliberately does not attempt to
//! infer arbitrary prose semantics; human architecture review remains required.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

const APEX_DOCS: &[&str] = &[
    "docs/specs/loeres-requirements-v1.md",
    "docs/specs/loeres-external-design-v1.md",
    "docs/specs/loeres-roadmap-milestones-v1.md",
];

const RFC_FOLDERS: &[&str] = &["proposed", "accepted", "done", "archive"];

const LEGACY_APEX_MARKER: &str = "**RFC 020 shared currency metadata (draft).**";

/// Retired RFC 021 conditional apex marker. RFC 024 §11.3 retires the
/// one-release apparatus; this string must never reappear in a tracked apex
/// document, so it is checked for directly rather than re-importing the
/// (retained-as-historical-record, no-longer-called) `conditional_finalization`
/// module into the steady-state gate path.
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
    check_apex_currency(expected_release.as_deref(), &mut errors);
    check_rfc_index(&mut errors);
    check_root_roadmap(&mut errors);
    check_book_navigation(&mut errors);
    check_release_local_paths(&mut errors);
    check_stale_ledger(&mut errors);
    check_conditional_current_prose(&mut errors);
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

fn check_apex_currency(expected_release: Option<&str>, errors: &mut Vec<String>) {
    let scope = implemented_scope(errors);
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
        let (Some(scope), Some(changelog_source)) = (scope.as_deref(), changelog_source.as_deref())
        else {
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

    let whole_file_targets = [
        "README.md",
        "crates/loeres/README.md",
        "crates/loeres-backend-std/README.md",
        "crates/loeres-backend-static/README.md",
        "crates/loeres-cluster/README.md",
        "crates/loeres-device/README.md",
        "docs/src/threat-model.md",
        "docs/src/specifications.md",
    ];
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

fn bounded_value(source: &str, prefix: &str, suffix: &str) -> Option<String> {
    let (_, after_prefix) = source.split_once(prefix)?;
    let (value, _) = after_prefix.split_once(suffix)?;
    if value.is_empty() {
        None
    } else {
        Some(value.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BTreeSet, changelog_marks_released, current_before_historical, implemented_scope_at,
        index_status_matches, numbered_rfc_file, parse_changelog_release_status,
        parse_design_freeze, parse_ordinary_apex_currency, render_scope,
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
}
