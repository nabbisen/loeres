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

const RFC_DIRS: &[&str] = &["proposed", "done", "archive"];

pub fn run() -> bool {
    eprintln!("[check-rfcs] RFC lifecycle and index integrity");
    let mut ok = true;
    let rfcs = collect_rfcs();
    ok &= check_file_names_and_uniqueness(&rfcs);
    ok &= check_status_fields(&rfcs);
    ok &= check_readme_index(&rfcs);
    ok &= check_markdown_links(&rfc_markdown_files());
    eprintln!("[check-rfcs] {}", if ok { "PASS" } else { "FAIL" });
    ok
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
        let matches_folder = match (rfc.folder.as_str(), status) {
            ("proposed", Some(line)) => line.contains("Proposed"),
            ("done", Some(line)) => line.contains("Implemented"),
            ("archive", Some(line)) => line.contains("Withdrawn") || line.contains("Superseded"),
            (_, _) => false,
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
