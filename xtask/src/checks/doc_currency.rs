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

#[derive(Clone, Debug, Eq, PartialEq)]
struct ApexCurrency {
    release: String,
    normalized_block: String,
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
    let mut found = Vec::new();
    for path in APEX_DOCS {
        let source = match read_required(path, errors) {
            Some(source) => source,
            None => continue,
        };
        let parsed = match conditional {
            Some(metadata) => parse_conditional_apex_currency(&source, metadata),
            None => parse_apex_currency(&source),
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

fn parse_apex_currency(source: &str) -> Result<ApexCurrency, String> {
    let header = source.lines().take(16).collect::<Vec<_>>().join(" ");
    let header_lower = header.to_ascii_lowercase();
    if !header_lower.contains("reconciliation draft (not yet the current marker)") {
        return Err("missing draft/not-current status qualifier".to_owned());
    }

    let block = extract_shared_currency_block(source)?;
    let normalized_block = normalize_whitespace(&block);
    if !normalized_block.contains("Implemented scope: **RFCs 001-018**") {
        return Err("missing implemented RFC 001-018 scope".to_owned());
    }
    if !normalized_block.contains("Accepted recovery work: **RFC 019 and RFC 020**") {
        return Err("missing accepted RFC 019/RFC 020 recovery scope".to_owned());
    }
    if !normalized_block.contains("this work is unshipped and in progress") {
        return Err("missing unshipped/in-progress recovery qualifier".to_owned());
    }
    if !normalized_block.contains("Activation as the current marker is pending") {
        return Err("missing pending current-marker activation qualifier".to_owned());
    }

    let prefix = "Proposed last-reconciled repository release: **";
    let release = bounded_value(&normalized_block, prefix, "**")
        .ok_or_else(|| "missing proposed last-reconciled release field".to_owned())?;
    Ok(ApexCurrency {
        release,
        normalized_block,
    })
}

fn extract_shared_currency_block(source: &str) -> Result<String, String> {
    const MARKER: &str = "**RFC 020 shared currency metadata (draft).**";
    if source.matches(MARKER).count() != 1 {
        return Err("expected exactly one shared draft metadata marker".to_owned());
    }

    let lines = source.lines().collect::<Vec<_>>();
    let Some(start) = lines.iter().position(|line| line.contains(MARKER)) else {
        return Err("missing shared draft metadata marker".to_owned());
    };
    if !lines[start].trim_start().starts_with('>') {
        return Err("shared draft metadata marker is not in a blockquote".to_owned());
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
    for marker in [
        format!(
            "Release-finalization marker for **{}**",
            metadata.release_version
        ),
        format!("Canonical tag: **{}**", metadata.canonical_tag),
        "Current only when this exact tree is distributed under the canonical tag after accepted tag-bound evidence, architecture release Go, and project-owner release authorization".to_owned(),
        "otherwise a non-current release-finalization candidate".to_owned(),
        "Implemented scope after activation: **RFCs 001-021**".to_owned(),
        "Stored lifecycle paths do not prove external activation".to_owned(),
    ] {
        if !normalized_block.contains(&marker) {
            return Err(format!("missing conditional apex field `{marker}`"));
        }
    }
    Ok(ApexCurrency {
        release: format!("v{}", metadata.release_version),
        normalized_block,
    })
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
        if !source.contains(marker) {
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
        current_before_historical, index_status_matches, parse_apex_currency,
        parse_conditional_apex_currency, parse_design_freeze, stale_phrases_in,
    };
    use crate::checks::conditional_finalization::ConditionalMetadata;

    fn valid_apex(release: &str) -> String {
        format!(
            "# Spec\nStatus: Accepted v1; RFC 020 S2 reconciliation draft (not yet the current marker)\n\n\
             > **RFC 020 shared currency metadata (draft).** Proposed last-reconciled\n\
             > repository release: **{release}**. Implemented scope: **RFCs 001-018** in done.\n\
             > Accepted recovery work: **RFC 019 and RFC 020**; this work is unshipped and in progress.\n\
             > Activation as the current marker is pending review."
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
    fn apex_parser_accepts_bounded_shared_fields() {
        let parsed = parse_apex_currency(&valid_apex("v0.20.0")).unwrap();
        assert_eq!(parsed.release, "v0.20.0");
        assert!(
            parsed
                .normalized_block
                .contains("Implemented scope: **RFCs 001-018**")
        );
    }

    #[test]
    fn apex_parser_rejects_missing_draft_qualifier() {
        let source = valid_apex("v0.20.0").replace(
            "reconciliation draft (not yet the current marker)",
            "reconciliation current",
        );
        assert!(parse_apex_currency(&source).is_err());
    }

    #[test]
    fn apex_parser_rejects_missing_recovery_scope() {
        let source = valid_apex("v0.20.0").replace("RFC 019 and RFC 020", "RFC 019");
        assert!(parse_apex_currency(&source).is_err());
    }

    #[test]
    fn apex_parser_rejects_incomplete_implemented_scope() {
        let source = valid_apex("v0.20.0").replace("RFCs 001-018", "RFCs 001-017");
        assert!(parse_apex_currency(&source).is_err());
    }

    #[test]
    fn apex_parser_rejects_duplicate_shared_markers() {
        let mut source = valid_apex("v0.20.0");
        source.push_str("\n\n> **RFC 020 shared currency metadata (draft).** duplicate");
        assert!(parse_apex_currency(&source).is_err());
    }

    #[test]
    fn apex_parser_rejects_field_displaced_outside_shared_block() {
        let source = valid_apex("v0.20.0").replace(
            "Proposed last-reconciled\n> repository release: **v0.20.0**.",
            "Last-reconciled\n> repository release: **v0.20.0**.",
        ) + "\n\nHistorical note: Proposed last-reconciled repository release: **v0.20.0**.";
        assert!(parse_apex_currency(&source).is_err());
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
