//! Shared helpers for running cargo subcommands.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Run a cargo subcommand, streaming its output. Returns `true` on success.
pub fn cargo(args: &[&str]) -> bool {
    eprintln!("  $ cargo {}", args.join(" "));
    Command::new(env!("CARGO"))
        .args(args)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Capture stdout of a cargo subcommand (used by the dependency-graph scan).
pub fn cargo_stdout(args: &[&str]) -> Option<String> {
    let out = Command::new(env!("CARGO")).args(args).output().ok()?;
    if out.status.success() {
        Some(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        None
    }
}

/// Capture stdout of a non-cargo command.
pub fn command_stdout(program: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(program).args(args).output().ok()?;
    if out.status.success() {
        Some(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        None
    }
}

/// Recursively collect files with a given extension.
pub fn collect_ext(root: &Path, ext: &str, out: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_ext(&path, ext, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some(ext) {
            out.push(path);
        }
    }
}

/// Drop a doc/line comment so token scanning sees only code.
pub fn strip_comment(line: &str) -> &str {
    if line.trim_start().starts_with("//") {
        return "";
    }
    match line.find("//") {
        Some(idx) => &line[..idx],
        None => line,
    }
}
