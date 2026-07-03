//! `target-profiles` — manifest-driven target-profile checks (RFC 011).

use std::collections::BTreeSet;
use std::fs;

use super::util::{cargo_with_env, command_stdout};

const MANIFEST: &str = "xtask/target-profiles.toml";

pub fn run() -> bool {
    eprintln!("[target-profiles] checking RFC 011 target profile manifest");
    let rustc_verbose = command_stdout("rustc", &["-vV"]).unwrap_or_default();
    if !rustc_verbose.is_empty() {
        for line in rustc_verbose.lines() {
            eprintln!("  rustc: {line}");
        }
    }
    let host = host_triple(&rustc_verbose).unwrap_or("unknown-host");
    eprintln!("  host triple: {host}");

    let manifest = match fs::read_to_string(MANIFEST) {
        Ok(src) => src,
        Err(e) => {
            eprintln!("  ! cannot read {MANIFEST}: {e}");
            eprintln!("[target-profiles] FAIL");
            return false;
        }
    };
    let profiles = match parse_manifest(&manifest) {
        Ok(profiles) => profiles,
        Err(e) => {
            eprintln!("  ! invalid {MANIFEST}: {e}");
            eprintln!("[target-profiles] FAIL");
            return false;
        }
    };

    let installed_targets = installed_targets();
    let mut summary = Summary::default();
    let mut ok = true;
    for profile in &profiles {
        eprintln!("  profile: {}", profile.name);
        eprintln!("    class: {}", profile.class.as_str());
        eprintln!("    environment: {}", profile.environment);
        eprintln!("    target: {}", profile.effective_target(host));
        eprintln!("    panic_strategy: {}", profile.panic_strategy);
        eprintln!("    fpu: {}", profile.fpu);
        eprintln!("    scalar_family: {}", profile.scalar_family);
        eprintln!("    size_budget_group: {}", profile.size_budget_group);
        eprintln!(
            "    conformance_group: {} (metadata only; RFC 013 owns fixtures)",
            profile.conformance_group
        );
        if !profile.rustflags.is_empty() {
            eprintln!("    rustflags: {}", profile.rustflags.join(" "));
        }
        match profile.class {
            ProfileClass::Mandatory => {
                let passed = run_buildable(profile, host, &installed_targets);
                summary.mandatory_total += 1;
                if passed {
                    summary.mandatory_passed += 1;
                } else {
                    summary.mandatory_failed += 1;
                    ok = false;
                }
            }
            ProfileClass::AdvisoryInstalled => {
                summary.advisory_total += 1;
                if target_available(profile, &installed_targets) {
                    let passed = run_buildable(profile, host, &installed_targets);
                    if passed {
                        summary.advisory_passed += 1;
                    } else {
                        summary.advisory_failed += 1;
                        ok = false;
                    }
                } else {
                    eprintln!("    result: advisory unavailable (target/tool not installed)");
                    summary.advisory_unavailable += 1;
                }
            }
            ProfileClass::DocumentedOnly => {
                eprintln!("    result: documented-only (not compiled by v0.17.0 local xtask)");
                summary.documented_total += 1;
            }
        }
    }

    summary.print();
    eprintln!("[target-profiles] {}", if ok { "PASS" } else { "FAIL" });
    ok
}

fn run_buildable(profile: &Profile, host: &str, installed_targets: &BTreeSet<String>) -> bool {
    if !target_available(profile, installed_targets) {
        eprintln!("    result: FAIL (required target/tool unavailable)");
        return false;
    }
    let Some(package) = profile.package.as_deref() else {
        eprintln!("    result: FAIL (missing package)");
        return false;
    };
    let Some(command) = profile.command.as_deref() else {
        eprintln!("    result: FAIL (missing command)");
        return false;
    };
    let Some(default_features) = profile.default_features else {
        eprintln!("    result: FAIL (missing default_features)");
        return false;
    };
    let Some(features) = profile.features.as_ref() else {
        eprintln!("    result: FAIL (missing features)");
        return false;
    };

    let mut args = vec![command.to_owned(), "-p".to_owned(), package.to_owned()];
    if profile.target != "host" {
        args.push("--target".to_owned());
        args.push(profile.effective_target(host).to_owned());
    }
    if !default_features {
        args.push("--no-default-features".to_owned());
    }
    if features.iter().any(|f| f == "all") {
        args.push("--all-features".to_owned());
    } else if !features.is_empty() {
        args.push("--features".to_owned());
        args.push(features.join(","));
    }

    let mut envs = Vec::new();
    if !profile.rustflags.is_empty() {
        envs.push(("RUSTFLAGS", profile.rustflags.join(" ")));
    }
    let passed = cargo_with_env(&args, &envs);
    eprintln!("    result: {}", if passed { "pass" } else { "FAIL" });
    passed
}

fn target_available(profile: &Profile, installed_targets: &BTreeSet<String>) -> bool {
    profile.target == "host" || installed_targets.contains(&profile.target)
}

fn installed_targets() -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    if let Some(stdout) = command_stdout("rustup", &["target", "list", "--installed"]) {
        for line in stdout.lines() {
            let target = line.trim();
            if !target.is_empty() {
                out.insert(target.to_owned());
            }
        }
    }
    out
}

fn host_triple(rustc_verbose: &str) -> Option<&str> {
    rustc_verbose
        .lines()
        .find_map(|line| line.strip_prefix("host: ").map(str::trim))
}

#[derive(Default)]
struct Summary {
    mandatory_total: usize,
    mandatory_passed: usize,
    mandatory_failed: usize,
    advisory_total: usize,
    advisory_passed: usize,
    advisory_failed: usize,
    advisory_unavailable: usize,
    documented_total: usize,
}

impl Summary {
    fn print(&self) {
        eprintln!("  evidence-status summary:");
        eprintln!(
            "    mandatory: {} passed / {} failed / {} total",
            self.mandatory_passed, self.mandatory_failed, self.mandatory_total
        );
        eprintln!(
            "    advisory-installed: {} passed / {} failed / {} unavailable / {} total",
            self.advisory_passed,
            self.advisory_failed,
            self.advisory_unavailable,
            self.advisory_total
        );
        eprintln!("    documented-only: {} listed", self.documented_total);
        eprintln!("    conformance groups: metadata only; RFC 013 owns fixtures");
    }
}

#[derive(Clone, Debug)]
struct Profile {
    name: String,
    class: ProfileClass,
    environment: String,
    target: String,
    package: Option<String>,
    command: Option<String>,
    default_features: Option<bool>,
    features: Option<Vec<String>>,
    panic_strategy: String,
    fpu: String,
    scalar_family: String,
    size_budget_group: String,
    conformance_group: String,
    rustflags: Vec<String>,
}

impl Profile {
    fn effective_target<'a>(&'a self, host: &'a str) -> &'a str {
        if self.target == "host" {
            host
        } else {
            &self.target
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum ProfileClass {
    Mandatory,
    AdvisoryInstalled,
    DocumentedOnly,
}

impl ProfileClass {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "mandatory" => Ok(Self::Mandatory),
            "advisory-installed" => Ok(Self::AdvisoryInstalled),
            "documented-only" => Ok(Self::DocumentedOnly),
            _ => Err(format!("unknown profile class `{value}`")),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Mandatory => "mandatory",
            Self::AdvisoryInstalled => "advisory-installed",
            Self::DocumentedOnly => "documented-only",
        }
    }

    fn buildable(self) -> bool {
        matches!(self, Self::Mandatory | Self::AdvisoryInstalled)
    }
}

fn parse_manifest(src: &str) -> Result<Vec<Profile>, String> {
    let mut schema_version = None;
    let mut raw_profiles = Vec::new();
    let mut current = RawProfile::default();
    let mut in_profile = false;

    for (idx, raw) in src.lines().enumerate() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if line == "[[profiles]]" {
            if in_profile {
                raw_profiles.push(current);
                current = RawProfile::default();
            }
            in_profile = true;
            continue;
        }
        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| format!("line {}: expected key = value", idx + 1))?;
        let key = key.trim();
        let value = value.trim();
        if in_profile {
            current.set(key, value, idx + 1)?;
        } else if key == "schema_version" {
            schema_version = Some(parse_integer(value, idx + 1)?);
        } else {
            return Err(format!(
                "line {}: unexpected top-level key `{key}`",
                idx + 1
            ));
        }
    }
    if in_profile {
        raw_profiles.push(current);
    }
    match schema_version {
        Some(1) => {}
        Some(other) => return Err(format!("unsupported schema_version `{other}`")),
        None => return Err("missing schema_version".to_owned()),
    }
    validate_profiles(raw_profiles)
}

fn validate_profiles(raw_profiles: Vec<RawProfile>) -> Result<Vec<Profile>, String> {
    if raw_profiles.is_empty() {
        return Err("manifest must contain at least one profile".to_owned());
    }
    let mut names = BTreeSet::new();
    let mut out = Vec::new();
    for raw in raw_profiles {
        let name = required(raw.name, "name")?;
        if !names.insert(name.clone()) {
            return Err(format!("duplicate profile name `{name}`"));
        }
        let class = ProfileClass::parse(&required(raw.class, "class")?)?;
        let environment = required_enum(
            raw.environment,
            "environment",
            &["cluster", "device", "portability"],
        )?;
        let target = required(raw.target, "target")?;
        if environment == "device" && target == "host" {
            return Err(format!(
                "profile `{name}`: device profiles cannot use target = \"host\""
            ));
        }
        let panic_strategy = required_enum(
            raw.panic_strategy,
            "panic_strategy",
            &["abort", "unwind", "target-default"],
        )?;
        let fpu = required_enum(
            raw.fpu,
            "fpu",
            &["hardware-single", "software", "host", "none", "unspecified"],
        )?;
        let scalar_family = required_enum(
            raw.scalar_family,
            "scalar_family",
            &["float", "fixed-point-future", "integer-like-future", "host"],
        )?;
        let size_budget_group = required(raw.size_budget_group, "size_budget_group")?;
        let conformance_group = required(raw.conformance_group, "conformance_group")?;
        if class.buildable()
            && (raw.package.is_none()
                || raw.command.is_none()
                || raw.default_features.is_none()
                || raw.features.is_none())
        {
            return Err(format!(
                "profile `{name}`: buildable profiles require package, command, default_features, and features"
            ));
        }
        if let Some(command) = raw.command.as_deref()
            && command != "check"
            && command != "build"
        {
            return Err(format!("profile `{name}`: unknown command `{command}`"));
        }
        out.push(Profile {
            name,
            class,
            environment,
            target,
            package: raw.package,
            command: raw.command,
            default_features: raw.default_features,
            features: raw.features,
            panic_strategy,
            fpu,
            scalar_family,
            size_budget_group,
            conformance_group,
            rustflags: raw.rustflags.unwrap_or_default(),
        });
    }
    Ok(out)
}

#[derive(Default)]
struct RawProfile {
    name: Option<String>,
    class: Option<String>,
    environment: Option<String>,
    target: Option<String>,
    package: Option<String>,
    command: Option<String>,
    default_features: Option<bool>,
    features: Option<Vec<String>>,
    panic_strategy: Option<String>,
    fpu: Option<String>,
    scalar_family: Option<String>,
    size_budget_group: Option<String>,
    conformance_group: Option<String>,
    rustflags: Option<Vec<String>>,
}

impl RawProfile {
    fn set(&mut self, key: &str, value: &str, line: usize) -> Result<(), String> {
        match key {
            "name" => self.name = Some(parse_string(value, line)?),
            "class" => self.class = Some(parse_string(value, line)?),
            "environment" => self.environment = Some(parse_string(value, line)?),
            "target" => self.target = Some(parse_string(value, line)?),
            "package" => self.package = Some(parse_string(value, line)?),
            "command" => self.command = Some(parse_string(value, line)?),
            "default_features" => self.default_features = Some(parse_bool(value, line)?),
            "features" => self.features = Some(parse_string_array(value, line)?),
            "panic_strategy" => self.panic_strategy = Some(parse_string(value, line)?),
            "fpu" => self.fpu = Some(parse_string(value, line)?),
            "scalar_family" => self.scalar_family = Some(parse_string(value, line)?),
            "size_budget_group" => self.size_budget_group = Some(parse_string(value, line)?),
            "conformance_group" => self.conformance_group = Some(parse_string(value, line)?),
            "rustflags" => self.rustflags = Some(parse_string_array(value, line)?),
            _ => return Err(format!("line {line}: unknown profile key `{key}`")),
        }
        Ok(())
    }
}

fn required(value: Option<String>, field: &str) -> Result<String, String> {
    value.ok_or_else(|| format!("missing required field `{field}`"))
}

fn required_enum(value: Option<String>, field: &str, allowed: &[&str]) -> Result<String, String> {
    let value = required(value, field)?;
    if allowed.contains(&value.as_str()) {
        Ok(value)
    } else {
        Err(format!("unknown {field} `{value}`"))
    }
}

fn parse_integer(value: &str, line: usize) -> Result<u32, String> {
    value
        .parse()
        .map_err(|_| format!("line {line}: expected integer"))
}

fn parse_bool(value: &str, line: usize) -> Result<bool, String> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!("line {line}: expected boolean")),
    }
}

fn parse_string(value: &str, line: usize) -> Result<String, String> {
    value
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .map(str::to_owned)
        .ok_or_else(|| format!("line {line}: expected quoted string"))
}

fn parse_string_array(value: &str, line: usize) -> Result<Vec<String>, String> {
    let inner = value
        .strip_prefix('[')
        .and_then(|v| v.strip_suffix(']'))
        .ok_or_else(|| format!("line {line}: expected string array"))?
        .trim();
    if inner.is_empty() {
        return Ok(Vec::new());
    }
    inner
        .split(',')
        .map(|part| parse_string(part.trim(), line))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_requires_schema_version() {
        let err = parse_manifest("[[profiles]]\nname = \"x\"").unwrap_err();
        assert!(err.contains("missing schema_version"));
    }

    #[test]
    fn manifest_rejects_unknown_class() {
        let err = parse_manifest(
            r#"
schema_version = 1

[[profiles]]
name = "x"
class = "optional"
environment = "cluster"
target = "host"
panic_strategy = "target-default"
fpu = "host"
scalar_family = "host"
size_budget_group = "x"
conformance_group = "x"
"#,
        )
        .unwrap_err();
        assert!(err.contains("unknown profile class"));
    }

    #[test]
    fn buildable_profiles_require_build_fields() {
        let err = parse_manifest(
            r#"
schema_version = 1

[[profiles]]
name = "x"
class = "mandatory"
environment = "cluster"
target = "host"
panic_strategy = "target-default"
fpu = "host"
scalar_family = "host"
size_budget_group = "x"
conformance_group = "x"
"#,
        )
        .unwrap_err();
        assert!(err.contains("buildable profiles require"));
    }

    #[test]
    fn documented_profiles_may_omit_build_fields() {
        let profiles = parse_manifest(
            r#"
schema_version = 1

[[profiles]]
name = "x"
class = "documented-only"
environment = "cluster"
target = "aarch64-unknown-linux-gnu"
panic_strategy = "target-default"
fpu = "host"
scalar_family = "host"
size_budget_group = "x"
conformance_group = "x"
"#,
        )
        .unwrap();
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].class, ProfileClass::DocumentedOnly);
    }
}
