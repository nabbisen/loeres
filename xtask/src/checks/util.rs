//! Shared helpers for running cargo subcommands.

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
