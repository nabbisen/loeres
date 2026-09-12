//! `check-rfcs` — RFC lifecycle, index, and link integrity (RFC 010).

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
struct RfcFile {
    number: String,
    path: PathBuf,
    folder: String,
    file_name: String,
}

const RFC_DIRS: &[&str] = &["proposed", "accepted", "done", "archive"];

pub fn run() -> bool {
    eprintln!("[check-rfcs] RFC lifecycle and index integrity");
    let mut ok = true;
    ok &= check_governed_directories();
    let rfcs = collect_rfcs();
    ok &= check_file_names_and_uniqueness(&rfcs);
    ok &= check_status_fields(&rfcs);
    ok &= check_done_rfc_amendments_are_named_by_status(&rfcs);
    ok &= check_readme_index(&rfcs);
    ok &= check_markdown_links(&rfc_markdown_files());
    eprintln!("[check-rfcs] {}", if ok { "PASS" } else { "FAIL" });
    ok
}

fn check_governed_directories() -> bool {
    let missing =
        missing_governed_directories(|folder| fs::read_dir(Path::new("rfcs").join(folder)).is_ok());
    for folder in &missing {
        eprintln!("  MISSING OR UNREADABLE RFC DIRECTORY: rfcs/{folder}/");
    }
    missing.is_empty()
}

fn missing_governed_directories<F>(mut is_dir: F) -> Vec<&'static str>
where
    F: FnMut(&str) -> bool,
{
    RFC_DIRS
        .iter()
        .copied()
        .filter(|folder| !is_dir(folder))
        .collect()
}

fn collect_rfcs() -> Vec<RfcFile> {
    let mut out = Vec::new();
    for folder in RFC_DIRS {
        let root = Path::new("rfcs").join(folder);
        let entries = match fs::read_dir(&root) {
            Ok(entries) => entries,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let Some(file_name) = path.file_name().and_then(|n| n.to_str()).map(str::to_owned)
            else {
                continue;
            };
            if file_name == ".gitkeep" {
                continue;
            }
            let number = file_name.chars().take(3).collect::<String>();
            out.push(RfcFile {
                number,
                path,
                folder: (*folder).to_owned(),
                file_name,
            });
        }
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

fn rfc_markdown_files() -> Vec<PathBuf> {
    let mut files = vec![PathBuf::from("rfcs/README.md")];
    files.extend(collect_rfcs().into_iter().map(|r| r.path));
    files
}

fn check_file_names_and_uniqueness(rfcs: &[RfcFile]) -> bool {
    let mut ok = true;
    let mut by_number: BTreeMap<&str, Vec<&Path>> = BTreeMap::new();
    for rfc in rfcs {
        if !valid_rfc_file_name(&rfc.file_name) {
            eprintln!("  BAD NAME: {}", rfc.path.display());
            ok = false;
        }
        by_number.entry(&rfc.number).or_default().push(&rfc.path);
    }
    for (number, paths) in by_number {
        if paths.len() > 1 {
            eprintln!("  DUPLICATE RFC NUMBER {number}:");
            for path in paths {
                eprintln!("    {}", path.display());
            }
            ok = false;
        }
    }
    ok
}

fn valid_rfc_file_name(name: &str) -> bool {
    let Some((number, slug_with_ext)) = name.split_once('-') else {
        return false;
    };
    number.len() == 3
        && number.chars().all(|c| c.is_ascii_digit())
        && slug_with_ext.ends_with(".md")
        && slug_with_ext[..slug_with_ext.len() - 3]
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn check_status_fields(rfcs: &[RfcFile]) -> bool {
    let mut ok = true;
    for rfc in rfcs {
        let src = match fs::read_to_string(&rfc.path) {
            Ok(src) => src,
            Err(e) => {
                eprintln!("  ! cannot read {}: {e}", rfc.path.display());
                ok = false;
                continue;
            }
        };
        let status = src.lines().find(|line| line.starts_with("**Status.**"));
        let matches_folder = match status {
            Some(line) => status_matches_folder(&rfc.folder, line),
            None => false,
        };
        if !matches_folder {
            eprintln!(
                "  STATUS MISMATCH: {} in `{}`",
                rfc.path.display(),
                rfc.folder
            );
            ok = false;
        }
    }
    ok
}

fn status_matches_folder(folder: &str, status: &str) -> bool {
    let Some(state) = status
        .strip_prefix("**Status.** ")
        .and_then(|value| value.split(|c: char| !c.is_ascii_alphabetic()).next())
        .filter(|state| !state.is_empty())
    else {
        return false;
    };

    match folder {
        "proposed" => state == "Proposed",
        "accepted" => state == "Accepted",
        "done" => state == "Implemented",
        "archive" => state == "Withdrawn" || state == "Superseded",
        _ => false,
    }
}

/// RFC 000's in-place-amendment section (added by RFC 025) draws the boundary at
/// `done/`: an Accepted RFC may be corrected in place, a shipped one is
/// superseded by a new RFC instead. This asserts **accountability, not absence**
/// (RFC 025 §0, Amendment 1): a `done/` RFC may carry `0.N Amendment` headings —
/// an RFC amended while Accepted carries that record into `done/` legitimately —
/// but every one of them must be named by its Status line.
///
/// Asserting absence, as this check originally did, forbade the *presence* of a
/// heading where the rule forbids its *addition after shipping*, and so barred
/// every RFC that had used RFC 025's own rule from ever reaching `done/`. It
/// surfaced at the first such transition: the `0.21.0` finalization revision,
/// where RFCs 022, 023, and 024 could not move.
///
/// What this catches is the realistic failure: appending an amendment to a
/// shipped RFC and leaving the Status line untouched. What it cannot catch is an
/// editor who updates both — nothing readable from tracked bytes can, and RFC 000
/// condition (4) places that in the architect's recorded review.
fn check_done_rfc_amendments_are_named_by_status(rfcs: &[RfcFile]) -> bool {
    let mut ok = true;
    for rfc in rfcs.iter().filter(|rfc| rfc.folder == "done") {
        let src = match fs::read_to_string(&rfc.path) {
            Ok(src) => src,
            // `check_status_fields` already reported the read failure.
            Err(_) => continue,
        };
        for finding in unaccounted_amendments(&src) {
            eprintln!(
                "  AMENDMENT NOT IN STATUS: {} {finding}",
                rfc.path.display()
            );
            ok = false;
        }
    }
    ok
}

/// Amendment headings in a `done/` RFC that its Status line does not account for.
///
/// An RFC with no amendment heading is trivially accounted for and never needs a
/// Status line clause. One that carries headings but has no Status line at all is
/// a finding here as well as in `check_status_fields`: without a Status line
/// there is nothing to name them.
fn unaccounted_amendments(src: &str) -> Vec<String> {
    let headings = amendment_headings(src);
    if headings.is_empty() {
        return Vec::new();
    }
    let Some(status) = src.lines().find(|line| line.starts_with("**Status.**")) else {
        return vec![format!(
            "carries {} amendment heading(s) but has no `**Status.**` line to name them",
            headings.len()
        )];
    };
    let named = amendments_named_in_status(status);
    let mut findings = Vec::new();
    for heading in headings {
        match amendment_ordinal(&heading) {
            Some(ordinal) if named.contains(&ordinal) => {}
            Some(ordinal) => findings.push(format!(
                "carries `{heading}` but its Status line does not name amendment {ordinal}; \
                 a shipped RFC's amendments must be accounted for there (RFC 025 §0)"
            )),
            // A heading with no ordinal cannot be named by any Status line, so it
            // can never be accounted for.
            None => findings.push(format!(
                "carries `{heading}`, which has no amendment number for a Status line to name"
            )),
        }
    }
    findings
}

/// Amendment ordinals a Status line accounts for.
///
/// Accepts the natural phrasings a human writes: `Amendment 2`, `Amendments 1-3`
/// (hyphen or en dash, inclusive), and comma- or `and`-separated lists such as
/// `Amendments 1, 2 and 4`. Scanning is over the whole line, so the clause may
/// sit anywhere after the version.
fn amendments_named_in_status(status: &str) -> BTreeSet<u32> {
    let mut named = BTreeSet::new();
    let mut rest = status;
    while let Some(at) = rest.find("Amendment") {
        rest = &rest[at + "Amendment".len()..];
        rest = rest.strip_prefix('s').unwrap_or(rest);
        collect_ordinal_list(rest, &mut named);
    }
    named
}

/// Read a run of numbers, ranges, and separators, stopping at the first token
/// that is neither. `1-3, 4 and 6` yields `{1, 2, 3, 4, 6}`.
fn collect_ordinal_list(text: &str, out: &mut BTreeSet<u32>) {
    let mut rest = text.trim_start();
    loop {
        let (first, tail) = match leading_u32(rest) {
            Some(parsed) => parsed,
            None => return,
        };
        let tail_trimmed = tail.trim_start();
        // An inclusive range, written with a hyphen or an en dash.
        let range_tail = tail_trimmed
            .strip_prefix('-')
            .or_else(|| tail_trimmed.strip_prefix('\u{2013}'));
        let mut after = tail;
        match range_tail.map(str::trim_start).and_then(leading_u32) {
            Some((last, range_rest)) if last >= first => {
                for ordinal in first..=last {
                    out.insert(ordinal);
                }
                after = range_rest;
            }
            _ => {
                out.insert(first);
            }
        }
        // Continue only across a list separator; anything else ends the run.
        let mut next = after.trim_start();
        next = match next.strip_prefix(',') {
            Some(stripped) => stripped.trim_start(),
            None => next,
        };
        for connector in ["and ", "& "] {
            if let Some(stripped) = next.strip_prefix(connector) {
                next = stripped.trim_start();
            }
        }
        if next == after.trim_start() {
            return;
        }
        rest = next;
    }
}

fn leading_u32(text: &str) -> Option<(u32, &str)> {
    let digits = text
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>();
    if digits.is_empty() {
        return None;
    }
    digits
        .parse()
        .ok()
        .map(|value| (value, &text[digits.len()..]))
}

/// The `N` of an `Amendment N` heading.
fn amendment_ordinal(heading: &str) -> Option<u32> {
    let at = heading.find("Amendment")?;
    leading_u32(heading[at + "Amendment".len()..].trim_start()).map(|(ordinal, _)| ordinal)
}

/// Every amendment heading in a source, at any heading level.
///
/// RFC 000 names the `## 0.N Amendment` and `### 0.N Amendment` forms. Matching
/// any heading level keeps the check from being defeated by a deeper nesting, and
/// the section number is optional because RFCs in this repository write a first
/// amendment as `## 0. Amendment 1` and later ones as `## 0.K Amendment M` —
/// requiring the digits would have let every first amendment escape entirely.
fn amendment_headings(src: &str) -> Vec<String> {
    src.lines()
        .map(str::trim_end)
        .filter(|line| is_amendment_heading(line))
        .map(str::to_owned)
        .collect()
}

fn is_amendment_heading(line: &str) -> bool {
    let rest = line.trim_start_matches('#');
    if rest.len() == line.len() {
        return false;
    }
    let Some(rest) = rest.strip_prefix(' ') else {
        return false;
    };
    let rest = rest.trim_start();
    let Some(rest) = rest.strip_prefix("0.") else {
        return false;
    };
    // `0.` alone (a first amendment) or `0.` plus a section number.
    let digits = rest.chars().take_while(char::is_ascii_digit).count();
    rest[digits..].trim_start().starts_with("Amendment")
}

fn check_readme_index(rfcs: &[RfcFile]) -> bool {
    let readme = match fs::read_to_string("rfcs/README.md") {
        Ok(readme) => readme,
        Err(e) => {
            eprintln!("  ! cannot read rfcs/README.md: {e}");
            return false;
        }
    };
    let mut ok = true;
    for rfc in rfcs {
        let link = format!("({}/{})", rfc.folder, rfc.file_name);
        let count = readme.matches(&link).count();
        if count != 1 {
            eprintln!(
                "  README INDEX: expected one `{link}` entry for RFC {}, found {count}",
                rfc.number
            );
            ok = false;
        }
    }
    ok
}

pub(crate) fn check_markdown_links(files: &[PathBuf]) -> bool {
    let mut ok = true;
    for path in files {
        let src = match fs::read_to_string(path) {
            Ok(src) => src,
            Err(e) => {
                eprintln!("  ! cannot read {}: {e}", path.display());
                ok = false;
                continue;
            }
        };
        let base = path.parent().unwrap_or_else(|| Path::new("."));
        for target in markdown_links(&src) {
            if external_or_anchor(&target) {
                continue;
            }
            let target = target.split('#').next().unwrap_or("");
            if target.is_empty() {
                continue;
            }
            let resolved = base.join(target);
            if !resolved.exists() {
                eprintln!(
                    "  BROKEN LINK at {}: `{}` -> {}",
                    path.display(),
                    target,
                    resolved.display()
                );
                ok = false;
            }
        }
    }
    ok
}

pub(crate) fn markdown_links(src: &str) -> BTreeSet<String> {
    let mut links = BTreeSet::new();
    let mut in_fence = false;
    for line in src.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        let mut rest = line;
        while let Some(open) = rest.find("](") {
            rest = &rest[open + 2..];
            let Some(close) = rest.find(')') else {
                break;
            };
            let target = &rest[..close];
            if !target.is_empty() {
                links.insert(target.to_owned());
            }
            rest = &rest[close + 1..];
        }
    }
    links
}

fn external_or_anchor(target: &str) -> bool {
    target.starts_with("http://")
        || target.starts_with("https://")
        || target.starts_with("mailto:")
        || target.starts_with('#')
}

#[cfg(test)]
mod tests {
    use super::{
        RFC_DIRS, RfcFile, amendment_headings, amendment_ordinal, amendments_named_in_status,
        check_done_rfc_amendments_are_named_by_status, missing_governed_directories,
        status_matches_folder, unaccounted_amendments,
    };
    use std::path::PathBuf;

    /// A `done/` RFC amended while it was Accepted: the exact shape that
    /// `0.21.0`'s RFCs 022, 023, and 024 have, and that the original
    /// absence-asserting check barred from shipping.
    fn amended_body(status: &str) -> String {
        format!(
            "# RFC 099 — Example\n\n\
             {status}\n\n\
             ## 0. Amendment 1 — 2026-09-12: the first correction\n\n\
             Body.\n\n\
             ### 0.2 Amendment 2 — 2026-09-12: the second correction\n\n\
             Body.\n"
        )
    }

    const NAMES_BOTH: &str = "**Status.** Implemented (v0.21.0). Amendments 1-2 were made while \
                              Accepted, under RFC 000's in-place-amendment rule.";

    #[test]
    fn an_amended_rfc_reaching_done_passes_when_its_status_line_names_the_amendments() {
        // RFC 025 §15's first required case, and the one nobody wrote before `F`:
        // until the finalization revision no amended RFC had ever transitioned,
        // so asserting absence looked correct and shipped.
        assert!(unaccounted_amendments(&amended_body(NAMES_BOTH)).is_empty());
    }

    #[test]
    fn an_amended_rfc_reaching_done_fails_when_its_status_line_omits_an_amendment() {
        let partial = "**Status.** Implemented (v0.21.0). Amendment 1 was made while Accepted.";
        let findings = unaccounted_amendments(&amended_body(partial));
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].contains("Amendment 2"), "{findings:?}");
        assert!(
            findings[0].contains("does not name amendment 2"),
            "{findings:?}"
        );
    }

    #[test]
    fn a_done_rfc_naming_no_amendment_at_all_fails_for_every_heading_it_carries() {
        let silent = "**Status.** Implemented (v0.21.0)";
        let findings = unaccounted_amendments(&amended_body(silent));
        assert_eq!(findings.len(), 2, "{findings:?}");
    }

    #[test]
    fn an_unamended_done_rfc_needs_no_status_clause() {
        let plain = "# RFC 099 — Example\n\n**Status.** Implemented (v0.21.0)\n\n## 1. Summary\n";
        assert!(unaccounted_amendments(plain).is_empty());
    }

    #[test]
    fn amendment_headings_with_no_status_line_cannot_be_accounted_for() {
        let headless = amended_body("**Design approval.** Somebody.");
        let findings = unaccounted_amendments(&headless);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(
            findings[0].contains("no `**Status.**` line"),
            "{findings:?}"
        );
    }

    #[test]
    fn a_heading_with_no_amendment_number_can_never_be_named() {
        let unnumbered = "# RFC 099\n\n**Status.** Implemented (v0.21.0). Amendments 1-3.\n\n\
                          ## 0.4 Amendment — undated and unnumbered\n";
        let findings = unaccounted_amendments(unnumbered);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].contains("no amendment number"), "{findings:?}");
    }

    #[test]
    fn an_accepted_rfc_carrying_an_amendment_is_not_inspected() {
        // RFC 000 permits in-place amendment right up to `done/`, so an
        // `accepted/` file is never read — the check filters on the folder first.
        let accepted = RfcFile {
            number: "099".to_owned(),
            path: PathBuf::from("rfcs/accepted/099-example.md"),
            folder: "accepted".to_owned(),
            file_name: "099-example.md".to_owned(),
        };
        assert_ne!(accepted.folder, "done");
        assert!(check_done_rfc_amendments_are_named_by_status(&[accepted]));
    }

    #[test]
    fn status_line_ordinals_accept_the_phrasings_a_human_writes() {
        for (status, expected) in [
            (
                "**Status.** Implemented (v0.21.0). Amendment 2 was made while Accepted.",
                vec![2],
            ),
            (
                "**Status.** Implemented (v0.21.0). Amendments 1-3 were made while Accepted.",
                vec![1, 2, 3],
            ),
            (
                "**Status.** Implemented (v0.21.0). Amendments 1\u{2013}2 (en dash).",
                vec![1, 2],
            ),
            (
                "**Status.** Implemented (v0.21.0). Amendments 1, 2 and 4.",
                vec![1, 2, 4],
            ),
            (
                "**Status.** Implemented (v0.21.0). Amendments 1, 2 & 4.",
                vec![1, 2, 4],
            ),
            (
                "**Status.** Implemented (v0.21.0). Amendment 1 and Amendment 3.",
                vec![1, 3],
            ),
            ("**Status.** Implemented (v0.21.0)", vec![]),
        ] {
            let named: Vec<u32> = amendments_named_in_status(status).into_iter().collect();
            assert_eq!(named, expected, "{status}");
        }
    }

    #[test]
    fn a_version_number_is_not_mistaken_for_an_amendment_ordinal() {
        // `Implemented (v0.21.0)` must not contribute 0, 21, or 1: only digits
        // following the word `Amendment` count.
        let named = amendments_named_in_status("**Status.** Implemented (v0.21.0)");
        assert!(named.is_empty(), "{named:?}");
    }

    #[test]
    fn a_first_amendment_written_without_a_section_number_is_still_detected() {
        // `## 0. Amendment 1` is how every RFC here writes a first amendment.
        // Requiring digits after `0.` let all of them escape the check.
        let headings = amendment_headings("## 0. Amendment 1 — 2026-09-12: first\n");
        assert_eq!(headings.len(), 1, "{headings:?}");
        assert_eq!(amendment_ordinal(&headings[0]), Some(1));
    }

    #[test]
    fn amendment_heading_detection_rejects_near_misses_and_accepts_every_level() {
        for line in [
            "## 0.4 Amendment 2 — 2026-09-12: coverage symmetry",
            "### 0.5 Amendment 3",
            "#### 0.10 Amendment 4 (deeper nesting is still an amendment)",
            "##   0.1   Amendment 1",
            "## 0. Amendment 1",
        ] {
            assert_eq!(amendment_headings(line).len(), 1, "missed: {line}");
        }
        for line in [
            "## 0.4 Coverage symmetry",
            "## 1.0 Amendment-shaped but not a `0.N` section",
            "Prose mentioning ## 0.4 Amendment 2 inside a sentence",
            "##0.4 Amendment 2",
            "## Amendment 2",
        ] {
            assert!(
                amendment_headings(line).is_empty(),
                "false positive: {line}"
            );
        }
    }

    #[test]
    fn accepted_is_a_governed_rfc_directory() {
        assert!(RFC_DIRS.contains(&"accepted"));
    }

    #[test]
    fn accepted_status_matches_only_the_accepted_folder() {
        let status = "**Status.** Accepted (design frozen 2026-07-15)";
        assert!(status_matches_folder("accepted", status));
        assert!(!status_matches_folder("proposed", status));
        assert!(!status_matches_folder("done", status));
        assert!(!status_matches_folder("archive", status));
    }

    #[test]
    fn lifecycle_status_requires_the_expected_leading_state_token() {
        assert!(!status_matches_folder(
            "accepted",
            "**Status.** Proposed — not Accepted"
        ));
        assert!(!status_matches_folder(
            "accepted",
            "**Status.** Not Accepted"
        ));
        assert!(!status_matches_folder(
            "proposed",
            "**Status.** Accepted (design frozen 2026-07-15)"
        ));
        assert!(!status_matches_folder(
            "accepted",
            "**Status.**Accepted (missing required separator)"
        ));
    }

    #[test]
    fn every_lifecycle_state_uses_its_leading_token() {
        assert!(status_matches_folder("proposed", "**Status.** Proposed"));
        assert!(status_matches_folder("accepted", "**Status.** Accepted"));
        assert!(status_matches_folder(
            "done",
            "**Status.** Implemented (v1.0.0)"
        ));
        assert!(status_matches_folder(
            "archive",
            "**Status.** Withdrawn — rationale"
        ));
        assert!(status_matches_folder(
            "archive",
            "**Status.** Superseded by RFC 042"
        ));
    }

    #[test]
    fn missing_governed_directory_fails_the_invariant() {
        let missing = missing_governed_directories(|folder| folder != "accepted");
        assert_eq!(missing, vec!["accepted"]);
    }
}
