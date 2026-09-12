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
    ok &= check_done_rfcs_carry_no_amendment(&rfcs);
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
/// superseded by a new RFC instead. This asserts the second half — no file under
/// `rfcs/done/` carries a numbered amendment heading. Without it the rule is
/// remembered rather than enforced, and amending a shipped contract is exactly
/// the history rewrite RFC 000 forbids.
fn check_done_rfcs_carry_no_amendment(rfcs: &[RfcFile]) -> bool {
    let mut ok = true;
    for rfc in rfcs.iter().filter(|rfc| rfc.folder == "done") {
        let src = match fs::read_to_string(&rfc.path) {
            Ok(src) => src,
            // `check_status_fields` already reported the read failure.
            Err(_) => continue,
        };
        for heading in amendment_headings(&src) {
            eprintln!(
                "  AMENDMENT IN `done/`: {} carries `{heading}`; a shipped RFC is superseded, never amended in place",
                rfc.path.display()
            );
            ok = false;
        }
    }
    ok
}

/// Every `0.N Amendment` heading in a source, at any heading level. RFC 000
/// names the `## 0.N Amendment` and `### 0.N Amendment` forms; matching any
/// level keeps the assertion fail-closed against a deeper nesting rather than
/// letting `#### 0.7 Amendment 4` through on a formatting choice.
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
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        return false;
    }
    rest[digits.len()..].trim_start().starts_with("Amendment")
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
        RFC_DIRS, RfcFile, amendment_headings, check_done_rfcs_carry_no_amendment,
        missing_governed_directories, status_matches_folder,
    };
    use std::path::PathBuf;

    const AMENDED_BODY: &str = concat!(
        "# RFC 099 — Example\n\n",
        "**Status.** Accepted (design frozen 2026-09-12; Amendment 1)\n\n",
        "### 0.1 Amendment 1 — 2026-09-12: a corrected provision\n\n",
        "Body text.\n",
    );

    #[test]
    fn a_numbered_amendment_heading_is_detected_so_a_done_rfc_carrying_one_fails() {
        let headings = amendment_headings(AMENDED_BODY);
        assert_eq!(
            headings,
            vec!["### 0.1 Amendment 1 — 2026-09-12: a corrected provision".to_owned()]
        );
        // `check_done_rfcs_carry_no_amendment` reports one finding per heading
        // and fails, so a non-empty result under `done/` is a gate failure.
        assert!(!headings.is_empty());
    }

    #[test]
    fn an_accepted_rfc_carrying_an_amendment_is_not_a_finding() {
        // The same body under `accepted/` is never inspected: the check filters
        // on `folder == "done"` before reading. RFC 000 permits in-place
        // amendment right up to `done/`, which is what RFCs 022 and 024 rely on.
        let accepted = RfcFile {
            number: "099".to_owned(),
            path: PathBuf::from("rfcs/accepted/099-example.md"),
            folder: "accepted".to_owned(),
            file_name: "099-example.md".to_owned(),
        };
        assert_ne!(accepted.folder, "done");
        assert!(check_done_rfcs_carry_no_amendment(&[accepted]));
    }

    #[test]
    fn amendment_heading_detection_rejects_near_misses_and_accepts_every_level() {
        for line in [
            "## 0.4 Amendment 2 — 2026-09-12: coverage symmetry",
            "### 0.5 Amendment 3",
            "#### 0.10 Amendment 4 (deeper nesting is still an amendment)",
            "##   0.1   Amendment 1",
        ] {
            assert_eq!(amendment_headings(line).len(), 1, "missed: {line}");
        }
        for line in [
            "## 0.4 Coverage symmetry",
            "## 1.0 Amendment-shaped but not a `0.N` section",
            "Prose mentioning ## 0.4 Amendment 2 inside a sentence",
            "##0.4 Amendment 2",
            "## 0. Amendment",
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
