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

/// Run a cargo subcommand with extra environment variables.
pub fn cargo_with_env(args: &[String], envs: &[(&str, String)]) -> bool {
    eprintln!("  $ {}", cargo_display(args, envs));
    let mut cmd = Command::new(env!("CARGO"));
    cmd.args(args);
    for (key, value) in envs {
        cmd.env(key, value);
    }
    cmd.status().map(|s| s.success()).unwrap_or(false)
}

fn cargo_display(args: &[String], envs: &[(&str, String)]) -> String {
    let mut parts = Vec::new();
    for (key, value) in envs {
        parts.push(format!("{key}={value}"));
    }
    parts.push("cargo".to_owned());
    parts.extend(args.iter().cloned());
    parts.join(" ")
}

/// Exact-name build-configuration variables a declared-environment build removes
/// before applying its own `envs` (RFC 040 §2.1; architect review 129 F2).
const CLEARED_BUILD_CONFIG_VARS: &[&str] = &[
    "RUSTFLAGS",
    "CARGO_ENCODED_RUSTFLAGS",
    "CARGO_BUILD_RUSTFLAGS",
    "CARGO_TARGET_DIR",
    "CARGO_BUILD_TARGET",
];

/// Pure: of a listing of ambient environment-variable **names**, which ones a
/// declared-environment build must remove. The exact names in
/// `CLEARED_BUILD_CONFIG_VARS`, plus any name starting `CARGO_PROFILE_` — a prefix,
/// not a list, because `CARGO_PROFILE_<name>_<key>` is an open family that an exact
/// list cannot cover (architect review 129 F2).
///
/// **Deliberately not removed:** `RUSTC`, `RUSTC_WRAPPER`, `PATH`, `CARGO_HOME`, and
/// any `RUSTUP_*` variable. Those select *which compiler* runs, not its flags, and
/// which compiler produced a measured number is a question (architect review 128
/// §2.1) this helper must not answer by quietly pinning one. `CARGO_INCREMENTAL` is
/// also not removed here — a caller that wants it fixed sets it explicitly via
/// `envs`, because it is a declared constant of a measurement, not a hazard to clear.
pub fn build_config_vars_to_remove<'a>(names: impl IntoIterator<Item = &'a str>) -> Vec<&'a str> {
    names
        .into_iter()
        .filter(|name| {
            CLEARED_BUILD_CONFIG_VARS.contains(name) || name.starts_with("CARGO_PROFILE_")
        })
        .collect()
}

/// Run a cargo subcommand in a **declared** environment: every ambient
/// build-configuration variable the ambient shell might have set
/// (`build_config_vars_to_remove`) is removed from the child first, then `envs` is
/// applied, then cargo runs exactly as `cargo_with_env` runs it. So the measurement
/// this produces depends only on what the call states, not on whatever the caller's
/// shell happens to have set (RFC 040 §2.1; architect review 129 F2: `RUSTFLAGS` and
/// `CARGO_PROFILE_RELEASE_OPT_LEVEL` each silently produced a wrong, confidently
/// reported number before this).
///
/// Prints the policy and how many variables were actually present and removed on
/// **every** run, not only when something was found — a normal run should show the
/// environment was constructed, not leave a reader to infer that from silence.
///
/// A second, narrower helper rather than a change to `cargo_with_env`'s signature:
/// that function has one other caller (`target_profiles.rs`) which asserts that a
/// build succeeds under caller-supplied flags, not a pinned number, so it has no
/// need of this cleaning and widening it would spread the change for no gain.
pub fn cargo_in_declared_env(args: &[String], envs: &[(&str, String)]) -> bool {
    let ambient: Vec<String> = std::env::vars().map(|(key, _)| key).collect();
    let to_remove = build_config_vars_to_remove(ambient.iter().map(String::as_str));
    eprintln!(
        "  environment: cleared RUSTFLAGS, CARGO_ENCODED_RUSTFLAGS, CARGO_BUILD_RUSTFLAGS, \
         CARGO_TARGET_DIR, CARGO_BUILD_TARGET, CARGO_PROFILE_* ({} removed); set {}",
        to_remove.len(),
        envs.iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    eprintln!("  $ {}", cargo_display(args, envs));
    let mut cmd = Command::new(env!("CARGO"));
    cmd.args(args);
    for name in &to_remove {
        cmd.env_remove(name);
    }
    for (key, value) in envs {
        cmd.env(key, value);
    }
    cmd.status().map(|s| s.success()).unwrap_or(false)
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

#[cfg(test)]
mod tests {
    use super::build_config_vars_to_remove;

    #[test]
    fn an_exact_named_rustflags_variable_is_removed() {
        assert_eq!(
            build_config_vars_to_remove(["RUSTFLAGS"]),
            vec!["RUSTFLAGS"]
        );
    }

    #[test]
    fn any_cargo_profile_prefixed_name_is_removed() {
        // Proves the prefix, not an exact-name list: both a known and an otherwise
        // unlisted CARGO_PROFILE_* name are removed.
        assert_eq!(
            build_config_vars_to_remove(["CARGO_PROFILE_RELEASE_LTO", "CARGO_PROFILE_DEV_DEBUG"]),
            vec!["CARGO_PROFILE_RELEASE_LTO", "CARGO_PROFILE_DEV_DEBUG"]
        );
    }

    #[test]
    fn compiler_selection_variables_are_kept() {
        assert_eq!(
            build_config_vars_to_remove(["CARGO_HOME", "PATH", "RUSTC"]),
            Vec::<&str>::new()
        );
    }

    #[test]
    fn cargo_incremental_is_not_in_the_removal_set() {
        // Set rather than cleared: it is a declared constant of a caller's
        // measurement, not a hazard this policy clears on its own.
        assert_eq!(
            build_config_vars_to_remove(["CARGO_INCREMENTAL"]),
            Vec::<&str>::new()
        );
    }
}
