//! `link-audit` — repository Markdown link integrity.

use std::fs;
use std::path::{Component, Path, PathBuf};

use super::check_rfcs::{check_markdown_links, external_or_anchor, markdown_links};

/// The book's own root. A relative `.md` target in a chapter under here is
/// rewritten to `.html` when the book is rendered, and nothing above this
/// directory exists in that rendered artifact (RFC 050 S1). Not derived
/// from `docs/book.toml`'s `src` key: that key is one word in a file
/// nobody is proposing to move, and reading it would add a TOML parse and
/// a failure mode to a check whose whole value is being three lines long.
const BOOK_SRC: &str = "docs/src";

pub fn run() -> bool {
    eprintln!("[link-audit] checking repository Markdown links");
    let mut files = Vec::new();
    collect_md(Path::new("."), &mut files);
    files.sort();
    // Evaluated independently, not short-circuited: a book-boundary
    // failure must not hide a broken-link report, or the other way round.
    let links_ok = check_markdown_links(&files);
    let boundary_ok = check_book_boundary(&files);
    eprintln!("  scanned {} Markdown file(s)", files.len());
    let ok = links_ok && boundary_ok;
    eprintln!("[link-audit] {}", if ok { "PASS" } else { "FAIL" });
    ok
}

fn check_book_boundary(files: &[PathBuf]) -> bool {
    let mut ok = true;
    for path in files {
        let Ok(source) = fs::read_to_string(path) else {
            continue; // `check_markdown_links` already reports this file.
        };
        for finding in book_boundary_findings(path, &source) {
            eprintln!("  {finding}");
            ok = false;
        }
    }
    ok
}

/// RFC 050 S1: a link in a `docs/src/` chapter must resolve inside the
/// book's own root — the artifact a reader opens is the rendered book,
/// where the target has been rewritten to `.html` and nothing above
/// `docs/src/` exists at all (architect review 117 F1). This composes
/// with `check_markdown_links` rather than replacing it: that function
/// keeps asserting the target exists in the repository; this asserts
/// that, for a book chapter, it also exists *within the book*.
fn book_boundary_findings(path: &Path, source: &str) -> Vec<String> {
    if !is_inside_book(path) {
        return Vec::new();
    }
    let base = path.parent().unwrap_or_else(|| Path::new("."));
    let mut findings = Vec::new();
    for target in markdown_links(source) {
        if external_or_anchor(&target) {
            continue;
        }
        let Some(target) = target.split('#').next().filter(|t| !t.is_empty()) else {
            continue;
        };
        // Normalised lexically, not with `canonicalize`: this must fire on
        // a link that escapes the book whether or not the file it names
        // exists, and must not touch the filesystem at all.
        let resolved = normalize_lexically(&base.join(target));
        if !is_inside_book(&resolved) {
            findings.push(format!(
                "BOOK BOUNDARY at {}: `{target}` resolves to `{}`, outside {BOOK_SRC}/ — a \
                 chapter names a repository path in backticks and links its absolute \
                 moving-branch URL instead ({BOOK_SRC}/specifications.md does this)",
                path.display(),
                resolved.display(),
            ));
        }
    }
    findings
}

/// Drop `.` components and pop the previous component on each `..`,
/// without touching the filesystem.
fn normalize_lexically(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Whether a path, once normalised lexically, sits inside (or is exactly)
/// `docs/src/` — component-wise, so `docs/srcfoo` is correctly outside.
fn is_inside_book(path: &Path) -> bool {
    let normalized = normalize_lexically(path);
    let book_src = Path::new(BOOK_SRC);
    normalized == book_src || normalized.starts_with(book_src)
}

/// Every tracked Markdown file under `dir`, skipping `.git`, `.git-exclude`,
/// and `target`. Shared with `review_evidence`, which must derive its
/// citation scan from the documents themselves rather than a fixed list
/// (RFC 022 §8).
pub(crate) fn collect_md(dir: &Path, out: &mut Vec<PathBuf>) {
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

#[cfg(test)]
mod tests {
    use super::book_boundary_findings;
    use std::path::Path;

    #[test]
    fn a_sibling_link_inside_the_book_is_accepted() {
        let findings = book_boundary_findings(
            Path::new("docs/src/getting-started.md"),
            "See the [Device User Guide](device-user-guide.md).\n",
        );
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn a_link_out_of_the_book_is_rejected_naming_the_target() {
        // RFC 050 F1 as a unit test: the committed defect, restored.
        let findings = book_boundary_findings(
            Path::new("docs/src/getting-started.md"),
            "See the [README](../../README.md).\n",
        );
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].contains("../../README.md"), "{findings:?}");
        assert!(findings[0].contains("README.md"), "{findings:?}");
    }

    #[test]
    fn the_same_target_outside_the_book_is_not_checked() {
        let findings = book_boundary_findings(
            Path::new("rfcs/README.md"),
            "See the [README](../../README.md).\n",
        );
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn an_absolute_url_and_an_anchor_only_target_are_both_accepted() {
        let findings = book_boundary_findings(
            Path::new("docs/src/getting-started.md"),
            "[external](https://example.com) and [here](#section).\n",
        );
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn an_anchor_is_stripped_before_resolving() {
        let findings = book_boundary_findings(
            Path::new("docs/src/getting-started.md"),
            "See [boundaries](architecture.md#boundaries).\n",
        );
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn an_escaping_target_is_rejected_whether_or_not_it_exists() {
        let findings = book_boundary_findings(
            Path::new("docs/src/getting-started.md"),
            "See [gone](../nonexistent/x.md).\n",
        );
        assert_eq!(findings.len(), 1, "{findings:?}");
    }
}
