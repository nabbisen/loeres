//! `link-audit` — repository Markdown link integrity.

use std::path::{Path, PathBuf};

use super::check_rfcs::check_markdown_links;

pub fn run() -> bool {
    eprintln!("[link-audit] checking repository Markdown links");
    let mut files = Vec::new();
    collect_md(Path::new("."), &mut files);
    files.sort();
    let ok = check_markdown_links(&files);
    eprintln!("  scanned {} Markdown file(s)", files.len());
    eprintln!("[link-audit] {}", if ok { "PASS" } else { "FAIL" });
    ok
}

fn collect_md(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if skip_dir(&path) {
                continue;
            }
            collect_md(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
            out.push(path);
        }
    }
}

fn skip_dir(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|n| n.to_str()),
        Some(".git" | ".git-exclude" | "target")
    )
}
