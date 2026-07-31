//! `doc-currency` — bounded RFC 020 documentation currency assertions.
//!
//! This gate checks stable metadata, lifecycle/index facts, navigation, and a
//! small explicit stale-phrase ledger. It deliberately does not attempt to
//! infer arbitrary prose semantics; human architecture review remains required.

use std::fs;
use std::path::Path;

use super::conditional_finalization::{self, ConditionalMetadata};

const APEX_DOCS: &[&str] = &[
    "docs/specs/loeres-requirements-v1.md",
    "docs/specs/loeres-external-design-v1.md",
    "docs/specs/loeres-roadmap-milestones-v1.md",
];

const RFC_FOLDERS: &[&str] = &["proposed", "accepted", "done", "archive"];

const LEGACY_APEX_MARKER: &str = "**RFC 020 shared currency metadata (draft).**";

/// RFC 024 ordinary post-release apex marker.
///
/// The block it introduces records the last released version and this tree's
/// version separately. Conflating them was blocker B6; RFC 021 separated them
/// for one finalization window, and RFC 024 makes the separation ordinary.
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
];

#[derive(Clone, Debug, Eq, PartialEq)]
struct ApexCurrency {
    /// The version this tree carries. Bound to `workspace.package.version`.
    release: String,
    /// The last version actually released. Never greater than `release`.
    last_released: String,
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

/// Highest RFC number recorded as implemented in `rfcs/done/`.
///
/// Derived rather than hard-coded: a literal scope is exactly why the
/// pre-release apex form expired the moment `0.20.2` shipped (RFC 024 §11.4).
fn implemented_scope(errors: &mut Vec<String>) -> Option<String> {
    let entries = match fs::read_dir(Path::new("rfcs").join("done")) {
        Ok(entries) => entries,
        Err(error) => {
            errors.push(format!("RFC SCOPE: cannot read rfcs/done: {error}"));
            return None;
        }
    };
    let mut highest = None;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if !name.ends_with(".md") {
            continue;
        }
        if let Ok(number) = name.chars().take(3).collect::<String>().parse::<u32>() {
            highest = Some(highest.map_or(number, |current: u32| current.max(number)));
        }
    }
    match highest {
        Some(highest) => Some(format!("RFCs 001-{highest:03}")),
        None => {
            errors.push("RFC SCOPE: rfcs/done contains no numbered RFC".to_owned());
            None
        }
    }
}

/// Last released version recorded by the ordinary apex block.
///
/// Used by the release-candidate preflight so intended-tag validation depends
/// on reviewed normative documentation rather than one-release metadata.
pub(crate) fn ordinary_last_released() -> Result<String, String> {
    let path = APEX_DOCS
        .first()
        .ok_or_else(|| "no apex document configured".to_owned())?;
    let source =
        fs::read_to_string(path).map_err(|error| format!("cannot read {path}: {error}"))?;
    let block = extract_currency_block(&source, ORDINARY_APEX_MARKER)?;
    bounded_value(
        &normalize_whitespace(&block),
        "Last released repository release: **",
        "**",
    )
    .ok_or_else(|| format!("{path} has no last-released field"))
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
    let conditional = match conditional_finalization::load_optional(Path::new(".")) {
        Ok(metadata) => metadata,
        Err(error) => {
            errors.push(format!("CONDITIONAL METADATA: {error}"));
            None
        }
    };
    let expected_release = workspace_release_marker(&mut errors);
    check_apex_currency(
        expected_release.as_deref(),
        conditional.as_ref(),
        &mut errors,
    );
    check_rfc_index(&mut errors);
    check_root_roadmap(&mut errors);
    check_book_navigation(&mut errors);
    check_release_local_paths(&mut errors);
    check_stale_ledger(&mut errors);
    check_conditional_current_prose(conditional.as_ref(), &mut errors);
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

fn check_apex_currency(
    expected_release: Option<&str>,
    conditional: Option<&ConditionalMetadata>,
    errors: &mut Vec<String>,
) {
    let scope = implemented_scope(errors);
    let mut found = Vec::new();
    for path in APEX_DOCS {
        let source = match read_required(path, errors) {
            Some(source) => source,
            None => continue,
        };
        let parsed = match (conditional, scope.as_deref()) {
            (Some(metadata), _) => parse_conditional_apex_currency(&source, metadata),
            (None, Some(scope)) => parse_ordinary_apex_currency(&source, scope),
            (None, None) => continue,
        };
        match parsed {
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
    if conditional.is_some() && errors.is_empty() {
        eprintln!("  conditional apex structure is staged; external activation is not inferred");
    }
}

/// Parse the RFC 024 ordinary apex block.
///
/// Unlike the retired forms this does not hard-code a scope or a version. It
/// asserts the block's internal consistency; the caller binds `release` to the
/// workspace version.
fn parse_ordinary_apex_currency(
    source: &str,
    expected_scope: &str,
) -> Result<ApexCurrency, String> {
    if source.contains(LEGACY_APEX_MARKER) {
        return Err("retired RFC 020 draft metadata must be absent".to_owned());
    }
    if source.contains(conditional_finalization::APEX_MARKER) {
        return Err("retired RFC 021 conditional metadata must be absent".to_owned());
    }

    let block = extract_currency_block(source, ORDINARY_APEX_MARKER)?;
    let normalized_block = normalize_whitespace(&block);

    let last_released = bounded_value(
        &normalized_block,
        "Last released repository release: **",
        "**",
    )
    .ok_or_else(|| "missing last-released field".to_owned())?;
    let release = bounded_value(&normalized_block, "This tree: **", "**")
        .ok_or_else(|| "missing this-tree field".to_owned())?;

    let Some(last_triple) = parse_version_triple(&last_released) else {
        return Err(format!("last-released `{last_released}` is not `X.Y.Z`"));
    };
    let Some(tree_triple) = parse_version_triple(&release) else {
        return Err(format!("this-tree `{release}` is not `X.Y.Z`"));
    };
    if tree_triple < last_triple {
        return Err(format!(
            "this-tree `{release}` precedes last-released `{last_released}`"
        ));
    }

    // The released/unreleased qualifier must agree with the two versions, so a
    // tree can never silently describe itself as released while ahead of the
    // last release.
    let expected_qualifier = if tree_triple == last_triple {
        "(released)"
    } else {
        "(unreleased)"
    };
    if !normalized_block.contains(&format!("This tree: **{release}** {expected_qualifier}")) {
        return Err(format!(
            "this-tree qualifier must be `{expected_qualifier}` for `{release}` against `{last_released}`"
        ));
    }

    let scope_field = format!("Implemented scope: **{expected_scope}**");
    if !normalized_block.contains(&scope_field) {
        return Err(format!(
            "missing or stale scope field; expected `{scope_field}`"
        ));
    }

    Ok(ApexCurrency {
        release: format!("v{release}"),
        last_released,
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

fn parse_conditional_apex_currency(
    source: &str,
    metadata: &ConditionalMetadata,
) -> Result<ApexCurrency, String> {
    metadata.validate()?;
    if source.contains(LEGACY_APEX_MARKER) {
        return Err("legacy RFC 020 draft metadata must be absent in conditional mode".to_owned());
    }
    if source
        .matches(conditional_finalization::APEX_MARKER)
        .count()
        != 1
    {
        return Err("expected exactly one RFC 021 conditional metadata marker".to_owned());
    }
    let lines = source.lines().collect::<Vec<_>>();
    let start = lines
        .iter()
        .position(|line| line.contains(conditional_finalization::APEX_MARKER))
        .ok_or_else(|| "missing RFC 021 conditional metadata marker".to_owned())?;
    if !lines[start].trim_start().starts_with('>') {
        return Err("RFC 021 conditional metadata marker is not in a blockquote".to_owned());
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
    let normalized_block = normalize_whitespace(&block.join("\n"));
    let expected = canonical_conditional_apex_block(metadata);
    if normalized_block != expected {
        return Err("conditional apex block differs from the canonical RFC 021 block".to_owned());
    }
    Ok(ApexCurrency {
        release: format!("v{}", metadata.release_version),
        last_released: metadata.release_version.clone(),
        normalized_block,
    })
}

fn canonical_conditional_apex_block(metadata: &ConditionalMetadata) -> String {
    normalize_whitespace(&format!(
        "> {}\n\
         > Release-finalization marker for **{}**. Canonical tag: **{}**.\n\
         > Current only when this exact tree is distributed under the canonical tag after accepted\n\
         > tag-bound evidence, architecture release Go, and project-owner release authorization;\n\
         > otherwise a non-current release-finalization candidate.\n\
         > Implemented scope after activation: **RFCs 001-021**.\n\
         > Stored lifecycle paths do not prove external activation.",
        conditional_finalization::APEX_MARKER,
        metadata.release_version,
        metadata.canonical_tag,
    ))
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

fn check_conditional_current_prose(
    conditional: Option<&ConditionalMetadata>,
    errors: &mut Vec<String>,
) {
    for path in CONDITIONAL_CURRENT_DOCS {
        let Some(source) = read_required(path, errors) else {
            continue;
        };
        match conditional {
            Some(_) => conditional_current_prose_in(&source, path, errors),
            // RFC 024: once the conditional apparatus is retired the boundary
            // prose describes a resolved condition. Requiring its absence is
            // what stops it being copied forward into the next release.
            None => retired_conditional_prose_in(&source, path, errors),
        }
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

fn conditional_current_prose_in(source: &str, label: &str, errors: &mut Vec<String>) {
    let normalized = normalize_whitespace(source);
    for marker in CONDITIONAL_BOUNDARY_MARKERS {
        if !normalized.contains(&normalize_whitespace(marker)) {
            errors.push(format!(
                "CONDITIONAL CURRENT PROSE: {label} is missing `{marker}`"
            ));
        }
    }
    for phrase in CONDITIONAL_STALE_PHRASES {
        if normalized.contains(&normalize_whitespace(phrase)) {
            errors.push(format!(
                "CONDITIONAL CURRENT PROSE: {label} retains stale unconditional phrase `{phrase}`"
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
        conditional_current_prose_in, current_before_historical, index_status_matches,
        parse_conditional_apex_currency, parse_design_freeze, parse_ordinary_apex_currency,
        retired_conditional_prose_in, stale_phrases_in,
    };
    use crate::checks::conditional_finalization::ConditionalMetadata;

    fn valid_apex(last_released: &str, this_tree: &str, qualifier: &str) -> String {
        format!(
            "# Spec\nStatus: Accepted v1\n\n\
             > **Release currency metadata.**\n\
             > Last released repository release: **{last_released}**.\n\
             > This tree: **{this_tree}** {qualifier}.\n\
             > Implemented scope: **RFCs 001-021**."
        )
    }

    fn valid_conditional_metadata() -> ConditionalMetadata {
        ConditionalMetadata::parse(
            r#"
schema_version = 1
release_version = "0.20.2"
canonical_tag = "0.20.2"
phase = "release-finalization-candidate"
authoritative_remote = "origin"
distribution_bundle = "tag-push-release-workflow-v1"
conditional_rfcs = [19, 20, 21]
workflow_start_timeout_minutes = 30
workflow_terminal_timeout_minutes = 120
"#,
        )
        .unwrap()
    }

    fn valid_conditional_apex() -> String {
        "# Spec\n\n\
         > **RFC 021 conditional release-finalization metadata.**\n\
         > Release-finalization marker for **0.20.2**. Canonical tag: **0.20.2**.\n\
         > Current only when this exact tree is distributed under the canonical tag after accepted\n\
         > tag-bound evidence, architecture release Go, and project-owner release authorization;\n\
         > otherwise a non-current release-finalization candidate.\n\
         > Implemented scope after activation: **RFCs 001-021**.\n\
         > Stored lifecycle paths do not prove external activation."
            .to_owned()
    }

    #[test]
    fn ordinary_apex_accepts_unreleased_tree_ahead_of_last_release() {
        let parsed = parse_ordinary_apex_currency(
            &valid_apex("0.20.2", "0.20.3", "(unreleased)"),
            "RFCs 001-021",
        )
        .unwrap();
        assert_eq!(parsed.release, "v0.20.3");
        assert_eq!(parsed.last_released, "0.20.2");
    }

    #[test]
    fn ordinary_apex_accepts_released_tree_at_the_last_release() {
        let parsed = parse_ordinary_apex_currency(
            &valid_apex("0.20.3", "0.20.3", "(released)"),
            "RFCs 001-021",
        )
        .unwrap();
        assert_eq!(parsed.release, "v0.20.3");
    }

    #[test]
    fn ordinary_apex_rejects_tree_behind_the_last_release() {
        let source = valid_apex("0.20.3", "0.20.2", "(unreleased)");
        assert!(parse_ordinary_apex_currency(&source, "RFCs 001-021").is_err());
    }

    #[test]
    fn ordinary_apex_rejects_released_qualifier_while_ahead() {
        let source = valid_apex("0.20.2", "0.20.3", "(released)");
        assert!(parse_ordinary_apex_currency(&source, "RFCs 001-021").is_err());
    }

    #[test]
    fn ordinary_apex_rejects_unreleased_qualifier_at_the_release() {
        let source = valid_apex("0.20.3", "0.20.3", "(unreleased)");
        assert!(parse_ordinary_apex_currency(&source, "RFCs 001-021").is_err());
    }

    #[test]
    fn ordinary_apex_rejects_scope_disagreeing_with_the_lifecycle() {
        let source = valid_apex("0.20.2", "0.20.3", "(unreleased)");
        assert!(parse_ordinary_apex_currency(&source, "RFCs 001-022").is_err());
    }

    #[test]
    fn ordinary_apex_rejects_retired_markers() {
        let legacy = valid_apex("0.20.2", "0.20.3", "(unreleased)")
            + "\n\n> **RFC 020 shared currency metadata (draft).** leftover";
        assert!(parse_ordinary_apex_currency(&legacy, "RFCs 001-021").is_err());
        let conditional = valid_apex("0.20.2", "0.20.3", "(unreleased)")
            + "\n\n> **RFC 021 conditional release-finalization metadata.** leftover";
        assert!(parse_ordinary_apex_currency(&conditional, "RFCs 001-021").is_err());
    }

    #[test]
    fn ordinary_apex_rejects_duplicate_markers() {
        let source = valid_apex("0.20.2", "0.20.3", "(unreleased)")
            + "\n\n> **Release currency metadata.** duplicate";
        assert!(parse_ordinary_apex_currency(&source, "RFCs 001-021").is_err());
    }

    #[test]
    fn ordinary_apex_rejects_field_displaced_outside_block() {
        let source = valid_apex("0.20.2", "0.20.3", "(unreleased)")
            .replace("> Last released repository release: **0.20.2**.\n", "")
            + "\n\nHistorical: Last released repository release: **0.20.2**.";
        assert!(parse_ordinary_apex_currency(&source, "RFCs 001-021").is_err());
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

    #[test]
    fn conditional_apex_parser_accepts_exact_shared_candidate_semantics() {
        let parsed = parse_conditional_apex_currency(
            &valid_conditional_apex(),
            &valid_conditional_metadata(),
        )
        .unwrap();
        assert_eq!(parsed.release, "v0.20.2");
    }

    #[test]
    fn conditional_apex_never_treats_tracked_lifecycle_as_activation_proof() {
        let source = valid_conditional_apex().replace(
            "Stored lifecycle paths do not prove external activation.",
            "",
        );
        assert!(parse_conditional_apex_currency(&source, &valid_conditional_metadata()).is_err());
    }

    #[test]
    fn conditional_apex_rejects_scope_or_predicate_drift() {
        for source in [
            valid_conditional_apex().replace("RFCs 001-021", "RFCs 001-020"),
            valid_conditional_apex().replace("otherwise a non-current", "already current"),
            valid_conditional_apex().replace("architecture release Go", "architecture review"),
        ] {
            assert!(
                parse_conditional_apex_currency(&source, &valid_conditional_metadata()).is_err()
            );
        }
    }

    #[test]
    fn conditional_apex_rejects_legacy_or_extra_claims() {
        let legacy = format!(
            "{}\n\n> **RFC 020 shared currency metadata (draft).** retained",
            valid_conditional_apex()
        );
        let extra = valid_conditional_apex().replace(
            "> Stored lifecycle paths do not prove external activation.",
            "> Extra statement copied to every apex document.\n\
             > Stored lifecycle paths do not prove external activation.",
        );
        let unconditional = valid_conditional_apex().replace(
            "> Stored lifecycle paths do not prove external activation.",
            "> This release is current and released.\n\
             > Stored lifecycle paths do not prove external activation.",
        );
        for source in [legacy, extra, unconditional] {
            assert!(
                parse_conditional_apex_currency(&source, &valid_conditional_metadata()).is_err()
            );
        }
    }

    #[test]
    fn conditional_apex_rejects_required_field_displaced_outside_block() {
        let source = valid_conditional_apex().replace(" Canonical tag: **0.20.2**.", "")
            + "\n\nCanonical tag: **0.20.2**.";
        assert!(parse_conditional_apex_currency(&source, &valid_conditional_metadata()).is_err());
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
    fn conditional_current_prose_accepts_complete_timeless_boundary() {
        let source = "Before external predicate `P` succeeds, this is non-current.\n\
            After `P` succeeds, these same bytes are current.\n\
            Tracked bytes alone do not establish whether `P` occurred.\n\
            GitHub release creation, registry publication, and certification remain separately authorized.";
        let mut errors = Vec::new();
        conditional_current_prose_in(source, "synthetic", &mut errors);
        assert!(errors.is_empty());
    }

    #[test]
    fn conditional_current_prose_rejects_missing_branch_and_stale_no_go() {
        let source = "Before external predicate `P` succeeds, this is non-current.\n\
            Tracked bytes alone do not establish whether `P` occurred.\n\
            GitHub release creation, registry publication, and certification remain separately authorized.\n\
            The repository remains No-Go for release/readiness claims.";
        let mut errors = Vec::new();
        conditional_current_prose_in(source, "synthetic", &mut errors);
        assert_eq!(errors.len(), 2);
    }
}
