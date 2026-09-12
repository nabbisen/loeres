//! RFC 010 developer aggregate and RFC 019 release-candidate orchestration.

use std::env;
use std::path::{Component, Path};
use std::process::Command;

use super::{
    basic, check_rfcs, conformance, doc_currency, examples, feature_matrix, link_audit, no_std,
    panic_audit, public_api, review_evidence, size_budget, supply_chain, target_profiles,
    unsafe_audit, zero_bleed,
};

mod package;

/// The single reviewed remote to which a canonical tag may be published
/// (RFC 021 §7 "authoritative release remote"; retained by RFC 024).
///
/// Changing this value is an architecture-reviewed governance change, not a
/// routine refactor: RFC 021 §7 required ambiguity about the release remote
/// to fail closed, and moving this out of reviewed metadata into a tracked
/// code constant (RFC 024) preserves that only if edits to it get the same
/// review weight as any other release-governance change.
const AUTHORITATIVE_REMOTE: &str = "origin";

pub fn run_developer() -> bool {
    run_developer_named("check")
}

/// Run the complete non-publishing RFC 019 release-candidate gate.
pub fn run_release(args: &[String]) -> bool {
    eprintln!("[release-gate] RFC 019 candidate preconditions");
    let mode = match parse_release_mode(args) {
        Ok(mode) => mode,
        Err(error) => {
            eprintln!("  ARGUMENTS: {error}");
            eprintln!("[release-gate] FAIL (invalid invocation)");
            return false;
        }
    };
    let candidate = match release_preflight(&mode) {
        Ok(candidate) => candidate,
        Err(error) => {
            eprintln!("  {error}");
            eprintln!("[release-gate] FAIL (candidate preconditions not established)");
            return false;
        }
    };

    if !run_candidate_suite(Path::new("."), "source-tree") {
        eprintln!("[release-gate] FAIL (source-tree suite)");
        return false;
    }

    match package::certify(&candidate) {
        Ok(evidence) => {
            eprintln!("[release-gate] evidence: {}", evidence.display());
            eprintln!("[release-gate] PASS (non-publishing candidate evidence)");
            true
        }
        Err(error) => {
            eprintln!("[release-gate] PACKAGE/CLEAN EXTRACTION: {error}");
            eprintln!("[release-gate] FAIL");
            false
        }
    }
}

fn run_candidate_suite(root: &Path, name: &str) -> bool {
    eprintln!("[release-gate:{name}] complete applicable suite");
    let commands: &[(&str, &str, &[&str])] = &[
        (
            "fmt",
            "cargo",
            &["+stable", "fmt", "--all", "--", "--check"],
        ),
        (
            "clippy",
            "cargo",
            &[
                "+stable",
                "clippy",
                "--workspace",
                "--all-features",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ],
        ),
        (
            "test",
            "cargo",
            &["+stable", "test", "--workspace", "--all-features"],
        ),
        (
            "msrv",
            "cargo",
            &["+1.85.0", "check", "--workspace", "--all-features"],
        ),
        ("architecture", "cargo", &["+stable", "xtask", "check"]),
    ];
    for (label, program, args) in commands {
        if !command_success(root, label, program, args) {
            return false;
        }
    }
    run_mdbook(root, name)
}

fn command_success(root: &Path, label: &str, program: &str, args: &[&str]) -> bool {
    eprintln!("  [{label}] $ {program} {}", args.join(" "));
    match Command::new(program).args(args).current_dir(root).status() {
        Ok(status) if status.success() => true,
        Ok(status) => {
            eprintln!("  [{label}] command exited with {status}");
            false
        }
        Err(error) => {
            eprintln!("  [{label}] cannot run command: {error}");
            false
        }
    }
}

/// Build the book into a gate-owned scratch directory under `target/`.
///
/// The gate used to build to `book.toml`'s default `docs/book/` and then delete
/// it, refusing to run at all if that directory already existed. But
/// `mdbook build docs` is also the documented developer command, so an ordinary
/// local build left output that blocked every later `release-gate` run, and the
/// gate would not clear a directory it had not created. Building to
/// `--dest-dir target/xtask-book/<suite>` removes the collision entirely: the
/// gate never reads, writes, or deletes anything outside `target/`, which it
/// already owns, and a developer's `docs/book/` is simply irrelevant to it
/// (architect review 045 follow-up 3).
///
/// Still `mdbook build`, and still the whole book — only the output path moves,
/// so RFC 019's documentation-build conformance is unchanged. The suite name
/// keeps the source-tree and clean-extraction builds from sharing a directory,
/// and the scratch directory is cleared first so each run proves the book builds
/// from nothing rather than incrementally over a previous result.
fn run_mdbook(root: &Path, suite: &str) -> bool {
    let dest = mdbook_dest_dir(suite);
    let absolute = root.join(&dest);
    if absolute.exists() {
        if let Err(error) = std::fs::remove_dir_all(&absolute) {
            eprintln!(
                "  [mdbook] cannot clear gate-owned {}: {error}",
                absolute.display()
            );
            return false;
        }
    }
    command_success(
        root,
        "mdbook",
        "mdbook",
        &["build", "docs", "--dest-dir", &dest],
    )
}

/// The gate-owned book output path for one suite, relative to the tree root.
///
/// Under `target/`, so it is ignored, disposable, and owned by tooling rather
/// than by the developer. Never `docs/book/`: that path belongs to whoever ran
/// `mdbook build docs` by hand.
fn mdbook_dest_dir(suite: &str) -> String {
    format!("target/xtask-book/{suite}")
}

fn run_developer_named(name: &str) -> bool {
    eprintln!("[{name}] running RFC 010 aggregate gates");
    let results = [
        ("host-check", GateKind::Enforced, basic::run()),
        ("zero-bleed", GateKind::Enforced, zero_bleed::run()),
        ("no-std", GateKind::Enforced, no_std::run()),
        ("feature-matrix", GateKind::Enforced, feature_matrix::run()),
        (
            "target-profiles",
            GateKind::Enforced,
            target_profiles::run(),
        ),
        ("check-rfcs", GateKind::Enforced, check_rfcs::run()),
        ("doc-currency", GateKind::Enforced, doc_currency::run()),
        (
            "review-evidence",
            GateKind::Enforced,
            review_evidence::run(),
        ),
        ("check-public-api", GateKind::Enforced, public_api::run()),
        ("panic-audit", GateKind::Enforced, panic_audit::run()),
        ("size-budget", GateKind::Advisory, size_budget::run()),
        ("unsafe-audit", GateKind::Enforced, unsafe_audit::run()),
        ("supply-chain", GateKind::Enforced, supply_chain::run()),
        ("examples", GateKind::Enforced, examples::run()),
        ("conformance", GateKind::Enforced, conformance::run(&[])),
        ("link-audit", GateKind::Enforced, link_audit::run()),
    ];
    let ok = results.iter().all(|(_, _, result)| *result);
    eprintln!("[{name}] summary:");
    for (gate, kind, result) in results {
        eprintln!("  {gate}: {}", kind.status(result));
    }
    eprintln!("[{name}] {}", if ok { "PASS" } else { "FAIL" });
    ok
}

fn release_preflight(mode: &ReleaseMode) -> Result<Candidate, String> {
    let version = workspace_version().map_err(|error| format!("VERSION: {error}"))?;
    require_single_changelog_heading(&version).map_err(|error| format!("CHANGELOG: {error}"))?;
    let reference = match mode {
        ReleaseMode::Normal => validate_ci_tag(&version),
        ReleaseMode::IntendedTag(tag) => validate_intended_tag(tag, &version),
    }
    .map_err(|error| format!("TAG: {error}"))?;
    if !git_success(&["diff", "--quiet"]) || !git_success(&["diff", "--cached", "--quiet"]) {
        return Err("CLEANLINESS: staged or unstaged tracked changes exist".to_owned());
    }
    let entries = tracked_manifest().map_err(|error| format!("MANIFEST: {error}"))?;
    package::require_release_inputs(&entries).map_err(|error| format!("MANIFEST: {error}"))?;
    let revision =
        git_output(&["rev-parse", "HEAD"]).map_err(|error| format!("REVISION: {error}"))?;
    eprintln!("  version: {version}");
    eprintln!("  revision: {revision}");
    eprintln!("  tracked regular files: {}", entries.len());
    eprintln!("  candidate identity: {}", reference.label());
    Ok(Candidate {
        version,
        revision,
        reference,
        entries,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ReleaseMode {
    Normal,
    IntendedTag(String),
}

fn parse_release_mode(args: &[String]) -> Result<ReleaseMode, String> {
    match args {
        [] => Ok(ReleaseMode::Normal),
        [flag, tag] if flag == "--intended-tag" => {
            parse_stable_version(tag)?;
            Ok(ReleaseMode::IntendedTag(tag.clone()))
        }
        _ => Err("expected no arguments or exactly `--intended-tag MAJOR.MINOR.PATCH`".to_owned()),
    }
}

fn workspace_version() -> Result<String, String> {
    let source = std::fs::read_to_string("Cargo.toml")
        .map_err(|error| format!("cannot read Cargo.toml: {error}"))?;
    let document = source
        .parse::<toml::Value>()
        .map_err(|error| format!("cannot parse Cargo.toml: {error}"))?;
    let version = document
        .get("workspace")
        .and_then(|workspace| workspace.get("package"))
        .and_then(|package| package.get("version"))
        .and_then(toml::Value::as_str)
        .ok_or_else(|| "missing workspace.package.version".to_owned())?;
    parse_stable_version(version)?;
    Ok(version.to_owned())
}

fn require_single_changelog_heading(version: &str) -> Result<(), String> {
    let source = std::fs::read_to_string("CHANGELOG.md")
        .map_err(|error| format!("cannot read CHANGELOG.md: {error}"))?;
    let expected = format!("## [{version}]");
    let count = source
        .lines()
        .filter(|line| {
            line.strip_prefix(&expected)
                .map(|suffix| suffix.is_empty() || suffix.starts_with(char::is_whitespace))
                .unwrap_or(false)
        })
        .count();
    if count == 1 {
        Ok(())
    } else {
        Err(format!(
            "expected exactly one `{expected}` heading, found {count}"
        ))
    }
}

fn validate_ci_tag(version: &str) -> Result<CandidateRef, String> {
    match (env::var("GITHUB_REF_TYPE"), env::var("GITHUB_REF_NAME")) {
        (Ok(kind), Ok(tag)) if kind == "tag" => {
            parse_stable_version(&tag)?;
            if tag != version {
                return Err(format!("tag `{tag}` does not match version `{version}`"));
            }
            let reference = format!("refs/tags/{tag}^{{commit}}");
            let tag_commit = git_output(&["rev-parse", &reference])?;
            let head = git_output(&["rev-parse", "HEAD"])?;
            if tag_commit != head {
                return Err(format!(
                    "peeled tag {tag_commit} does not equal HEAD {head}"
                ));
            }
            Ok(CandidateRef::Tag(tag))
        }
        (Ok(kind), _) if kind == "tag" => Err("GITHUB_REF_NAME is missing for tag run".to_owned()),
        _ => {
            eprintln!("  tag assertion: not performed (local non-tagged dry run)");
            Ok(CandidateRef::LocalDryRun)
        }
    }
}

fn validate_intended_tag(tag: &str, version: &str) -> Result<CandidateRef, String> {
    if env::var("GITHUB_REF_TYPE").as_deref() == Ok("tag") {
        return Err("--intended-tag is host-only and forbidden in a tag run".to_owned());
    }

    // RFC 024: the preflight no longer depends on one-release conditional
    // metadata. Every RFC 021 §7 property is preserved, sourced from the
    // ordinary apex block and the workspace manifest instead.
    let last_reconciled = doc_currency::ordinary_last_reconciled_release()?;
    validate_intended_binding(tag, version, &last_reconciled)?;

    require_local_tag_absent(tag)?;
    require_remote_tag_absent(AUTHORITATIVE_REMOTE, tag)?;
    Ok(CandidateRef::IntendedTag(tag.to_owned()))
}

fn validate_intended_binding(
    tag: &str,
    version: &str,
    last_reconciled: &str,
) -> Result<(), String> {
    let intended = parse_stable_version(tag)?;
    if tag != version {
        return Err(format!(
            "intended tag `{tag}` does not match workspace version `{version}`"
        ));
    }
    let reconciled = parse_stable_version(last_reconciled).map_err(|error| {
        format!("apex last-reconciled `{last_reconciled}` is not a canonical version: {error}")
    })?;
    if intended <= reconciled {
        return Err(format!(
            "intended tag `{tag}` does not advance past last reconciled release `{last_reconciled}`"
        ));
    }
    Ok(())
}

fn require_local_tag_absent(tag: &str) -> Result<(), String> {
    let reference = format!("refs/tags/{tag}");
    let status = Command::new("git")
        .args(["show-ref", "--verify", "--quiet", &reference])
        .status()
        .map_err(|error| format!("cannot query local tag `{tag}`: {error}"))?;
    classify_local_tag_status(tag, status.code())
        .map_err(|error| format!("{error} (query status {status})"))
}

fn classify_local_tag_status(tag: &str, code: Option<i32>) -> Result<(), String> {
    match code {
        Some(1) => Ok(()),
        Some(0) => Err(format!("local tag `{tag}` already exists")),
        _ => Err(format!("local tag query for `{tag}` failed")),
    }
}

fn require_remote_tag_absent(remote: &str, tag: &str) -> Result<(), String> {
    let urls = git_output(&["remote", "get-url", "--push", "--all", remote])?;
    validate_remote_push_urls(remote, &urls)?;

    let direct = format!("refs/tags/{tag}");
    let peeled = format!("{direct}^{{}}");
    let output = Command::new("git")
        .args(["ls-remote", "--tags", remote, &direct, &peeled])
        .output()
        .map_err(|error| format!("cannot query authoritative remote `{remote}`: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "authoritative remote `{remote}` tag query failed with {}",
            output.status
        ));
    }
    let listing = std::str::from_utf8(&output.stdout)
        .map_err(|_| "authoritative remote tag query was not UTF-8".to_owned())?;
    validate_remote_tag_listing(tag, listing)
}

fn validate_remote_push_urls(remote: &str, urls: &str) -> Result<(), String> {
    let configured = urls.lines().filter(|line| !line.trim().is_empty()).count();
    if configured == 1 {
        Ok(())
    } else {
        Err(format!(
            "authoritative remote `{remote}` must have exactly one push URL, found {configured}"
        ))
    }
}

fn validate_remote_tag_listing(tag: &str, listing: &str) -> Result<(), String> {
    if listing.trim().is_empty() {
        return Ok(());
    }
    let direct = format!("refs/tags/{tag}");
    let peeled = format!("{direct}^{{}}");
    let mut found = Vec::new();
    for line in listing.lines() {
        let fields = line.split_whitespace().collect::<Vec<_>>();
        if fields.len() != 2
            || fields[0].len() != 40
            || !fields[0].bytes().all(|byte| byte.is_ascii_hexdigit())
            || (fields[1] != direct && fields[1] != peeled)
        {
            return Err(format!("ambiguous remote tag response `{line}`"));
        }
        if found.iter().any(|reference| reference == &fields[1]) {
            return Err(format!("duplicate remote tag response for `{}`", fields[1]));
        }
        found.push(fields[1]);
    }
    Err(format!(
        "authoritative remote already contains {} for `{tag}`",
        found.join(" and ")
    ))
}

#[derive(Debug)]
struct Candidate {
    version: String,
    revision: String,
    reference: CandidateRef,
    entries: Vec<TrackedEntry>,
}

#[derive(Clone, Debug)]
enum CandidateRef {
    LocalDryRun,
    IntendedTag(String),
    Tag(String),
}

impl CandidateRef {
    fn label(&self) -> String {
        match self {
            Self::LocalDryRun => "local dry run (no tag assertion)".to_owned(),
            Self::IntendedTag(tag) => {
                format!("validated unused intended tag `{tag}` targeting HEAD (tag not created)")
            }
            Self::Tag(tag) => format!("validated tag `{tag}`"),
        }
    }
}

fn parse_stable_version(value: &str) -> Result<(u64, u64, u64), String> {
    let parts = value.split('.').collect::<Vec<_>>();
    if parts.len() != 3 {
        return Err(format!("`{value}` is not stable MAJOR.MINOR.PATCH"));
    }
    let parse = |part: &str| -> Result<u64, String> {
        if part.is_empty()
            || !part.bytes().all(|byte| byte.is_ascii_digit())
            || (part.len() > 1 && part.starts_with('0'))
        {
            return Err(format!("`{value}` is not stable MAJOR.MINOR.PATCH"));
        }
        part.parse::<u64>()
            .map_err(|_| format!("`{value}` contains an overflowing component"))
    };
    Ok((parse(parts[0])?, parse(parts[1])?, parse(parts[2])?))
}

#[derive(Debug, Eq, PartialEq)]
pub(super) struct TrackedEntry {
    pub(super) mode: String,
    pub(super) object: String,
    pub(super) path: String,
}

fn tracked_manifest() -> Result<Vec<TrackedEntry>, String> {
    let output = Command::new("git")
        .args(["ls-tree", "-rz", "--full-tree", "HEAD"])
        .output()
        .map_err(|error| format!("cannot run git ls-tree: {error}"))?;
    if !output.status.success() {
        return Err("git ls-tree failed".to_owned());
    }
    parse_tracked_manifest(&output.stdout)
}

fn parse_tracked_manifest(output: &[u8]) -> Result<Vec<TrackedEntry>, String> {
    let mut entries = Vec::new();
    for raw in output
        .split(|byte| *byte == 0)
        .filter(|entry| !entry.is_empty())
    {
        let record = std::str::from_utf8(raw)
            .map_err(|_| "tracked path or metadata is not UTF-8".to_owned())?;
        let (metadata, path) = record
            .split_once('\t')
            .ok_or_else(|| format!("malformed git ls-tree record `{record}`"))?;
        let fields = metadata.split_whitespace().collect::<Vec<_>>();
        if fields.len() != 3 || fields[1] != "blob" {
            return Err(format!("unsupported tracked entry `{record}`"));
        }
        if fields[0] != "100644" && fields[0] != "100755" {
            return Err(format!(
                "non-regular tracked mode `{}` at `{path}`",
                fields[0]
            ));
        }
        validate_archive_path(path)?;
        entries.push(TrackedEntry {
            mode: fields[0].to_owned(),
            object: fields[2].to_owned(),
            path: path.to_owned(),
        });
    }
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    if entries.windows(2).any(|pair| pair[0].path == pair[1].path) {
        return Err("duplicate tracked path".to_owned());
    }
    Ok(entries)
}

fn validate_archive_path(value: &str) -> Result<(), String> {
    let path = Path::new(value);
    if value.contains('\\') || path.is_absolute() {
        return Err(format!("unsafe archive path `{value}`"));
    }
    if path
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(format!("unsafe archive path `{value}`"));
    }
    let forbidden = [".git", ".git-exclude", "target", "docs/book"];
    if forbidden
        .iter()
        .any(|prefix| path == Path::new(prefix) || path.starts_with(prefix))
        || value.ends_with(".tar.gz")
    {
        return Err(format!("excluded archive path `{value}`"));
    }
    Ok(())
}

fn git_success(args: &[&str]) -> bool {
    Command::new("git")
        .args(args)
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn git_output(args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .output()
        .map_err(|error| format!("cannot run git {}: {error}", args.join(" ")))?;
    if !output.status.success() {
        return Err(format!("git {} failed", args.join(" ")));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

#[derive(Copy, Clone)]
enum GateKind {
    Enforced,
    Advisory,
}

impl GateKind {
    fn status(self, ok: bool) -> &'static str {
        match (self, ok) {
            (Self::Enforced, true) => "pass",
            (Self::Enforced, false) => "FAIL",
            (Self::Advisory, true) => "advisory baseline reported",
            (Self::Advisory, false) => "FAIL",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ReleaseMode, classify_local_tag_status, mdbook_dest_dir, parse_release_mode,
        parse_stable_version, parse_tracked_manifest, validate_archive_path,
        validate_intended_binding, validate_remote_push_urls, validate_remote_tag_listing,
    };

    #[test]
    fn the_book_is_built_under_target_never_into_the_developer_path() {
        // The collision architect review 045 follow-up 3 removes: a local
        // `mdbook build docs` must not be able to block the gate.
        for suite in ["source-tree", "clean-extraction"] {
            let dest = mdbook_dest_dir(suite);
            assert!(dest.starts_with("target/"), "{dest}");
            assert!(!dest.contains("docs/book"), "{dest}");
            assert!(dest.ends_with(suite), "{dest}");
        }
        // The two suites must not share one output directory.
        assert_ne!(
            mdbook_dest_dir("source-tree"),
            mdbook_dest_dir("clean-extraction")
        );
    }

    #[test]
    fn canonical_tags_are_stable_unprefixed_semver() {
        assert_eq!(parse_stable_version("0.20.1"), Ok((0, 20, 1)));
        assert_eq!(parse_stable_version("1.0.0"), Ok((1, 0, 0)));
        for invalid in ["v0.20.1", "0.20", "release-0.20.1", "01.0.0", "1.0.0-rc.1"] {
            assert!(
                parse_stable_version(invalid).is_err(),
                "accepted `{invalid}`"
            );
        }
    }

    #[test]
    fn intended_tag_cli_is_exact_and_host_oriented() {
        assert_eq!(parse_release_mode(&[]), Ok(ReleaseMode::Normal));
        assert_eq!(
            parse_release_mode(&["--intended-tag".to_owned(), "0.20.2".to_owned()]),
            Ok(ReleaseMode::IntendedTag("0.20.2".to_owned()))
        );
        for invalid in [
            vec!["--intended-tag".to_owned()],
            vec!["--intended-tag".to_owned(), "v0.20.2".to_owned()],
            vec![
                "--intended-tag".to_owned(),
                "0.20.2".to_owned(),
                "extra".to_owned(),
            ],
            vec!["--unknown".to_owned(), "0.20.2".to_owned()],
        ] {
            assert!(parse_release_mode(&invalid).is_err());
        }
    }

    #[test]
    fn intended_tag_remote_query_fails_closed() {
        assert!(validate_remote_tag_listing("0.20.2", "").is_ok());
        for listing in [
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\trefs/tags/0.20.2\n",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\trefs/tags/0.20.2\n\
             bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\trefs/tags/0.20.2^{}\n",
            "not-a-hash\trefs/tags/0.20.2\n",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\trefs/tags/other\n",
            "ambiguous output\n",
        ] {
            assert!(validate_remote_tag_listing("0.20.2", listing).is_err());
        }
    }

    #[test]
    fn intended_tag_binding_and_local_collision_are_fail_closed() {
        // Tag must equal the workspace version and advance past last released.
        assert!(validate_intended_binding("0.20.3", "0.20.3", "0.20.2").is_ok());
        assert!(validate_intended_binding("0.20.3", "0.20.2", "0.20.2").is_err());
        assert!(validate_intended_binding("0.20.2", "0.20.3", "0.20.2").is_err());
        // Never re-release or regress past what already shipped.
        assert!(validate_intended_binding("0.20.2", "0.20.2", "0.20.2").is_err());
        assert!(validate_intended_binding("0.20.1", "0.20.1", "0.20.2").is_err());
        assert!(validate_intended_binding("0.20.3", "0.20.3", "not-a-version").is_err());

        assert!(classify_local_tag_status("0.20.2", Some(1)).is_ok());
        assert!(classify_local_tag_status("0.20.2", Some(0)).is_err());
        assert!(classify_local_tag_status("0.20.2", Some(2)).is_err());
        assert!(classify_local_tag_status("0.20.2", None).is_err());
    }

    #[test]
    fn authoritative_remote_requires_exactly_one_push_url() {
        assert!(validate_remote_push_urls("origin", "git@example/repo.git\n").is_ok());
        assert!(validate_remote_push_urls("origin", "").is_err());
        assert!(
            validate_remote_push_urls(
                "origin",
                "git@example/repo.git\nssh://mirror.example/repo.git\n"
            )
            .is_err()
        );
    }

    #[test]
    fn tracked_manifest_accepts_only_regular_unique_safe_paths() {
        let input = b"100755 blob bbbb\tscripts/check.sh\0\
                      100644 blob aaaa\tCargo.toml\0";
        let entries = parse_tracked_manifest(input).unwrap();
        assert_eq!(entries[0].path, "Cargo.toml");
        assert_eq!(entries[1].path, "scripts/check.sh");

        assert!(parse_tracked_manifest(b"120000 blob aaaa\tlink\0").is_err());
        assert!(
            parse_tracked_manifest(
                b"100644 blob aaaa\tCargo.toml\0\
              100644 blob bbbb\tCargo.toml\0"
            )
            .is_err()
        );
    }

    #[test]
    fn archive_paths_reject_escape_and_excluded_content() {
        for invalid in [
            "/absolute",
            "../escape",
            "docs/../escape",
            "windows\\path",
            ".git/config",
            ".git-exclude/review.md",
            "target/debug/file",
            "docs/book/index.html",
            "loeres-v0.20.0.tar.gz",
        ] {
            assert!(
                validate_archive_path(invalid).is_err(),
                "accepted `{invalid}`"
            );
        }
        assert!(validate_archive_path("crates/loeres/src/lib.rs").is_ok());
    }

    #[test]
    fn release_workflow_selects_canonical_tags_and_pins_actions() {
        let workflow = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../.github/workflows/release.yml");
        let source = std::fs::read_to_string(workflow).unwrap();
        assert!(source.contains("tags: [\"[0-9]+.[0-9]+.[0-9]+\"]"));
        assert!(!source.contains("tags: [\"v*\"]"));
        assert!(source.contains("cargo +stable install mdbook --version 0.5.4 --locked"));

        let mut action_count = 0;
        for line in source.lines().map(str::trim) {
            let Some(reference) = line.strip_prefix("uses: ") else {
                continue;
            };
            action_count += 1;
            let revision = reference
                .split_once('@')
                .map(|(_, value)| value.split_whitespace().next().unwrap_or(""))
                .unwrap_or("");
            assert_eq!(revision.len(), 40, "action is not pinned: `{line}`");
            assert!(
                revision.bytes().all(|byte| byte.is_ascii_hexdigit()),
                "action is not pinned: `{line}`"
            );
        }
        assert_eq!(action_count, 4);
        let gate = source.find("cargo +stable xtask release-gate").unwrap();
        let upload = source.find("actions/upload-artifact@").unwrap();
        assert!(
            upload > gate,
            "evidence upload must follow the successful gate"
        );
        assert!(source.contains("include-hidden-files: true"));
        assert!(!source.contains("cargo publish"));
        assert!(!source.contains("softprops/action-gh-release"));
    }
}
