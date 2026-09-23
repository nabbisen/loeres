//! RFC 019 tracked-input packaging and clean-extraction certification.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use super::{Candidate, TrackedEntry, run_candidate_suite};

const REQUIRED_FILES: &[&str] = &[
    "Cargo.toml",
    "Cargo.lock",
    "LICENSE",
    "NOTICE",
    // RFC 023 §11.4: the engineering-use terms are a user-facing obligation of
    // the release, not a convenience file.
    "TERMS_OF_USE.md",
    ".github/workflows/ci.yml",
    ".github/workflows/msrv.yml",
    ".github/workflows/release.yml",
    "docs/book.toml",
    "rfcs/README.md",
    "conformance/README.md",
];

pub(super) fn require_release_inputs(entries: &[TrackedEntry]) -> Result<(), String> {
    let paths = entries
        .iter()
        .map(|entry| entry.path.as_str())
        .collect::<BTreeSet<_>>();
    for required in REQUIRED_FILES {
        if !paths.contains(required) {
            return Err(format!(
                "required release input `{required}` is not tracked"
            ));
        }
    }
    // `examples/` joins the families because the clean extraction now runs the
    // RFC 023 examples gate; an extraction missing them would fail there with a
    // per-example finding instead of naming the absent family here.
    for prefix in [
        "crates/",
        "xtask/",
        "docs/",
        "rfcs/",
        "conformance/",
        "examples/",
    ] {
        if !paths.iter().any(|path| path.starts_with(prefix)) {
            return Err(format!("required release input family `{prefix}` is empty"));
        }
    }
    Ok(())
}

pub(super) fn certify(candidate: &Candidate) -> Result<PathBuf, String> {
    let source = std::env::current_dir()
        .map_err(|error| format!("cannot determine source directory: {error}"))?
        .canonicalize()
        .map_err(|error| format!("cannot canonicalize source directory: {error}"))?;
    let temp = source.join(".git-exclude/tmp/release-gate");
    let _temp_guard = TempGuard::create(&source, &temp, &candidate.revision)?;
    let extraction = temp.join("extracted");
    fs::create_dir(&extraction)
        .map_err(|error| format!("cannot create clean extraction directory: {error}"))?;

    let evidence = evidence_directory(&source, candidate)?;
    fs::create_dir_all(
        evidence
            .parent()
            .ok_or_else(|| "evidence directory has no parent".to_owned())?,
    )
    .map_err(|error| format!("cannot create evidence parent: {error}"))?;
    fs::create_dir(&evidence).map_err(|error| {
        format!(
            "cannot create fresh evidence directory {}: {error}",
            evidence.display()
        )
    })?;

    let archive_name = format!("loeres-v{}.tar.gz", candidate.version);
    let archive = evidence.join(&archive_name);
    write_evidence(
        candidate,
        &evidence,
        &archive_name,
        None,
        EvidenceStage::SourceSuitePassed,
    )?;

    if !super::git_success(&["diff", "--quiet"])
        || !super::git_success(&["diff", "--cached", "--quiet"])
    {
        return Err(
            "tracked tree changed during the source suite; packaging was not started".to_owned(),
        );
    }

    run_checked(
        &source,
        "git",
        &[
            "archive",
            "--format=tar.gz",
            &format!("--output={}", archive.display()),
            &candidate.revision,
        ],
    )?;

    validate_archive(&source, &archive, &candidate.entries)?;
    let digest = sha256(&source, &archive)?;
    // RFC 028: computed from the archive the gate itself just produced, before
    // clean extraction, so the anchor is bound to the exact bytes that ship —
    // never from a second `git archive` run, which would identify a tree
    // rather than the shipped file.
    let uncompressed_digest = uncompressed_tar_sha256(&source, &archive)?;
    let digests = ArchiveDigests {
        archive: &digest,
        uncompressed: &uncompressed_digest,
    };
    write_content_manifest(candidate, &evidence)?;
    write_evidence(
        candidate,
        &evidence,
        &archive_name,
        Some(&digests),
        EvidenceStage::ArchiveValidated,
    )?;

    run_checked(
        &source,
        "tar",
        &[
            "--extract",
            "--gzip",
            &format!("--file={}", archive.display()),
            &format!("--directory={}", extraction.display()),
        ],
    )?;
    verify_extracted_content(&extraction, &candidate.entries)?;
    write_evidence(
        candidate,
        &evidence,
        &archive_name,
        Some(&digests),
        EvidenceStage::ExtractedContentVerified,
    )?;
    if !run_candidate_suite(&extraction, "clean-extraction") {
        return Err(format!(
            "clean-extraction suite failed; incomplete evidence retained at {}",
            evidence.display()
        ));
    }

    write_evidence(
        candidate,
        &evidence,
        &archive_name,
        Some(&digests),
        EvidenceStage::Complete,
    )?;
    Ok(evidence)
}

fn evidence_directory(source: &Path, candidate: &Candidate) -> Result<PathBuf, String> {
    if candidate.revision.len() != 40
        || !candidate
            .revision
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(format!(
            "candidate revision `{}` is not a full hexadecimal commit id",
            candidate.revision
        ));
    }
    Ok(source.join(format!(
        ".git-exclude/release-evidence/{}-v{}",
        candidate.revision, candidate.version
    )))
}

fn validate_archive(
    source: &Path,
    archive: &Path,
    expected: &[TrackedEntry],
) -> Result<(), String> {
    let listing = command_output(
        source,
        "tar",
        &[
            "--list",
            "--gzip",
            &format!("--file={}", archive.display()),
            "--quoting-style=literal",
        ],
    )?;
    let paths = listing.lines().collect::<Vec<_>>();
    validate_archive_listing(&paths, expected)?;

    let verbose = command_output(
        source,
        "tar",
        &[
            "--list",
            "--verbose",
            "--gzip",
            &format!("--file={}", archive.display()),
            "--quoting-style=literal",
        ],
    )?;
    let types = verbose.lines().collect::<Vec<_>>();
    validate_archive_types(&types, paths.len())?;
    Ok(())
}

fn validate_archive_types(lines: &[&str], expected_count: usize) -> Result<(), String> {
    if lines.len() != expected_count {
        return Err("archive path/type listing counts disagree".to_owned());
    }
    for line in lines {
        match line.as_bytes().first() {
            Some(b'-' | b'd') => {}
            Some(kind) => {
                return Err(format!(
                    "archive contains unsupported entry type `{}`",
                    char::from(*kind)
                ));
            }
            None => return Err("archive type listing contains an empty record".to_owned()),
        }
    }
    Ok(())
}

fn validate_archive_listing(paths: &[&str], expected: &[TrackedEntry]) -> Result<(), String> {
    let mut files = BTreeSet::new();
    let mut all = BTreeSet::new();
    for raw in paths {
        if raw.is_empty() || raw.bytes().any(|byte| byte.is_ascii_control()) {
            return Err("archive contains an empty/control-character path".to_owned());
        }
        let directory = raw.ends_with('/');
        let value = raw.trim_end_matches('/');
        super::validate_archive_path(value)?;
        if !all.insert(*raw) {
            return Err(format!("archive contains duplicate entry `{raw}`"));
        }
        if !directory && !files.insert(value) {
            return Err(format!("archive contains duplicate file `{value}`"));
        }
    }

    let expected_files = expected
        .iter()
        .map(|entry| entry.path.as_str())
        .collect::<BTreeSet<_>>();
    if files != expected_files {
        let missing = expected_files.difference(&files).next().copied();
        let extra = files.difference(&expected_files).next().copied();
        return Err(format!(
            "archive tracked-file set mismatch (missing: {}, extra: {})",
            missing.unwrap_or("none"),
            extra.unwrap_or("none")
        ));
    }
    Ok(())
}

fn verify_extracted_content(root: &Path, entries: &[TrackedEntry]) -> Result<(), String> {
    for entry in entries {
        let path = root.join(&entry.path);
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| format!("cannot inspect extracted `{}`: {error}", entry.path))?;
        if !metadata.file_type().is_file() {
            return Err(format!("extracted `{}` is not a regular file", entry.path));
        }
        let object = command_output(root, "git", &["hash-object", "--", &entry.path])?;
        if object != entry.object {
            return Err(format!(
                "extracted content mismatch at `{}` ({} != {})",
                entry.path, object, entry.object
            ));
        }
        verify_executable_mode(&metadata, entry)?;
    }
    Ok(())
}

#[cfg(unix)]
fn verify_executable_mode(metadata: &fs::Metadata, entry: &TrackedEntry) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let executable = metadata.permissions().mode() & 0o111 != 0;
    let expected = entry.mode == "100755";
    if executable != expected {
        Err(format!("executable-mode mismatch at `{}`", entry.path))
    } else {
        Ok(())
    }
}

#[cfg(not(unix))]
fn verify_executable_mode(_metadata: &fs::Metadata, _entry: &TrackedEntry) -> Result<(), String> {
    Ok(())
}

fn sha256(root: &Path, archive: &Path) -> Result<String, String> {
    let archive_arg = archive.to_string_lossy();
    let output = command_output(root, "sha256sum", &[archive_arg.as_ref()])?;
    let digest = output
        .split_whitespace()
        .next()
        .ok_or_else(|| "sha256sum returned no digest".to_owned())?;
    if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!("sha256sum returned invalid digest `{digest}`"));
    }
    Ok(digest.to_ascii_lowercase())
}

/// The archive SHA-256 (this build's bytes) and the uncompressed tar SHA-256
/// (the cross-environment identity of the source artifact's content and
/// layout — RFC 028) for one produced archive.
struct ArchiveDigests<'a> {
    archive: &'a str,
    uncompressed: &'a str,
}

/// Cross-environment identity of the archive's content and layout (RFC 028).
///
/// `0.21.0`'s archive SHA-256 differed between CI and a local run of the same
/// revision, while the tracked content manifest and the uncompressed tar
/// stream were byte-identical: only gzip output differed by implementation or
/// level. This is the digest that actually reproduces.
///
/// Spawns `gzip -dc <archive>` with its stdout piped directly into a
/// `sha256sum` child's stdin — two processes, no shell string, no path ever
/// interpolated into a command line a shell would parse. Both children must
/// exit successfully: a `gzip` failure (not gzip data, missing tool) is an
/// `Err`, never an empty or partial digest, even though `sha256sum` would
/// otherwise happily hash whatever partial bytes it received before `gzip`
/// gave up.
fn uncompressed_tar_sha256(root: &Path, archive: &Path) -> Result<String, String> {
    let mut gzip = Command::new("gzip")
        .arg("-dc")
        .arg(archive)
        .current_dir(root)
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|error| format!("cannot run gzip: {error}"))?;
    let gzip_stdout = gzip
        .stdout
        .take()
        .ok_or_else(|| "gzip produced no stdout pipe".to_owned())?;

    let sha256sum = Command::new("sha256sum")
        .current_dir(root)
        .stdin(Stdio::from(gzip_stdout))
        .output()
        .map_err(|error| format!("cannot run sha256sum: {error}"))?;
    let gzip_status = gzip
        .wait()
        .map_err(|error| format!("cannot wait for gzip: {error}"))?;

    // `gzip`'s status is checked before trusting anything `sha256sum` produced:
    // a `gzip` failure partway through still leaves `sha256sum` a valid digest
    // of whatever partial (or empty) stream it saw, and that must not be
    // returned as the uncompressed identity.
    if !gzip_status.success() {
        return Err(format!("gzip exited with {gzip_status}"));
    }
    if !sha256sum.status.success() {
        return Err(format!("sha256sum exited with {}", sha256sum.status));
    }

    let stdout = String::from_utf8(sha256sum.stdout)
        .map_err(|_| "sha256sum output is not UTF-8".to_owned())?;
    let digest = stdout
        .split_whitespace()
        .next()
        .ok_or_else(|| "sha256sum returned no digest".to_owned())?;
    if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!("sha256sum returned invalid digest `{digest}`"));
    }
    Ok(digest.to_ascii_lowercase())
}

fn write_content_manifest(candidate: &Candidate, evidence: &Path) -> Result<(), String> {
    let mut output = String::from("type\tmode\tgit_object\tpath\n");
    for entry in &candidate.entries {
        output.push_str(&format!(
            "file\t{}\t{}\t{}\n",
            entry.mode, entry.object, entry.path
        ));
    }
    fs::write(evidence.join("TRACKED_CONTENT_MANIFEST.tsv"), output)
        .map_err(|error| format!("cannot write tracked content manifest: {error}"))
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum EvidenceStage {
    SourceSuitePassed,
    ArchiveValidated,
    ExtractedContentVerified,
    Complete,
}

impl EvidenceStage {
    fn status(self) -> &'static str {
        match self {
            Self::SourceSuitePassed => "INCOMPLETE — SOURCE SUITE PASSED; ARCHIVE PENDING",
            Self::ArchiveValidated => "INCOMPLETE — ARCHIVE VALIDATED; EXTRACTION PENDING",
            Self::ExtractedContentVerified => {
                "INCOMPLETE — EXTRACTED CONTENT VERIFIED; CLEAN SUITE PENDING"
            }
            Self::Complete => "PASS",
        }
    }

    fn archive_result(self) -> &'static str {
        match self {
            Self::SourceSuitePassed => "pending",
            Self::ArchiveValidated | Self::ExtractedContentVerified | Self::Complete => "pass",
        }
    }

    fn extracted_result(self) -> &'static str {
        match self {
            Self::SourceSuitePassed | Self::ArchiveValidated => "pending",
            Self::ExtractedContentVerified | Self::Complete => "pass",
        }
    }

    fn clean_suite_result(self) -> &'static str {
        match self {
            Self::Complete => "pass",
            Self::SourceSuitePassed | Self::ArchiveValidated | Self::ExtractedContentVerified => {
                "pending"
            }
        }
    }
}

struct ToolVersions {
    stable_rustc: String,
    msrv_rustc: String,
    cargo: String,
    mdbook: String,
    git: String,
    tar: String,
    gzip: String,
    sha256sum: String,
    cargo_deny: String,
}

impl ToolVersions {
    fn observe(root: &Path) -> Result<Self, String> {
        Ok(Self {
            stable_rustc: command_output(root, "rustc", &["+stable", "--version", "--verbose"])?,
            msrv_rustc: command_output(root, "rustc", &["+1.85.0", "--version", "--verbose"])?,
            cargo: command_output(root, "cargo", &["+stable", "--version"])?,
            mdbook: command_output(root, "mdbook", &["--version"])?,
            git: command_output(root, "git", &["--version"])?,
            tar: first_line(&command_output(root, "tar", &["--version"])?).to_owned(),
            // RFC 028: `gzip` already runs on every clean extraction (`tar
            // --gzip`) and now also produces the uncompressed-tar identity
            // anchor, so it belongs in the evidence bundle beside `tar`.
            gzip: first_line(&command_output(root, "gzip", &["--version"])?).to_owned(),
            sha256sum: first_line(&command_output(root, "sha256sum", &["--version"])?).to_owned(),
            // RFC 026: the supply-chain gate is enforced, so the tool that
            // runs it belongs in the evidence bundle beside the compiler.
            cargo_deny: command_output(root, "cargo", &["deny", "--version"])?,
        })
    }
}

fn first_line(value: &str) -> &str {
    value.lines().next().unwrap_or(value)
}

fn write_evidence(
    candidate: &Candidate,
    evidence: &Path,
    archive_name: &str,
    digests: Option<&ArchiveDigests<'_>>,
    stage: EvidenceStage,
) -> Result<(), String> {
    let tools = ToolVersions::observe(evidence)?;
    let text = render_evidence(candidate, archive_name, digests, stage, &tools);
    fs::write(evidence.join("EVIDENCE.md"), text)
        .map_err(|error| format!("cannot write evidence record: {error}"))
}

fn render_evidence(
    candidate: &Candidate,
    archive_name: &str,
    digests: Option<&ArchiveDigests<'_>>,
    stage: EvidenceStage,
    tools: &ToolVersions,
) -> String {
    let (digest, uncompressed_digest) = match digests {
        Some(digests) => (digests.archive, digests.uncompressed),
        None => ("pending", "pending"),
    };
    format!(
        "# RFC 019 Release-Gate Evidence\n\n\
         **Status:** {status}  \n\
         **Revision:** `{revision}`  \n\
         **Candidate version:** `{version}`  \n\
         **Candidate identity:** {identity}  \n\
         **Archive:** `{archive_name}`  \n\
         **Archive SHA-256:** `{digest}`  \n\
         **Uncompressed tar SHA-256:** `{uncompressed_digest}`  \n\
         **Tracked regular files:** {tracked_files}  \n\
         **Archive layout/type/file-set/digest validation:** {archive_result}  \n\
         **Extracted Git-object content and executable-mode identity:** {extracted_result}  \n\
         **Publication:** not performed or authorized\n\n\
         The uncompressed tar SHA-256 is the cross-environment identity of this source\n\
         artifact's content and layout. The archive SHA-256 identifies this build's bytes\n\
         and is not expected to reproduce in another environment (RFC 028).\n\n\
         ## Gate results\n\n\
         - Source-tree format, Clippy, tests/doc-tests, Rust 1.85 check, architecture aggregate, and mdBook: pass.\n\
         - Package construction plus path/type/file-set/digest validation: {archive_result}.\n\
         - Extracted Git-object content and executable-mode identity: {extracted_result}.\n\
         - Clean-extraction complete applicable suite: {clean_suite_result}.\n\n\
         ## Tool versions\n\n\
         ```text\n{stable_rustc}\n{msrv_rustc}\n{cargo}\n{mdbook}\n{git}\n{tar}\n{gzip}\n{sha256sum}\n{cargo_deny}\n```\n\n\
         ## Evidence classes\n\n\
         Source-tree and clean-extraction results are separate phases of the same command.\n\
         Target-profile advisory-unavailable/documented-only and size-budget advisory\n\
         classifications remain as printed by `cargo xtask check`; they are not promoted\n\
         to enforced evidence. See `TRACKED_CONTENT_MANIFEST.tsv` for normalized tracked\n\
         file mode/object/path identity.\n",
        status = stage.status(),
        revision = candidate.revision,
        version = candidate.version,
        identity = candidate.reference.label(),
        tracked_files = candidate.entries.len(),
        archive_result = stage.archive_result(),
        extracted_result = stage.extracted_result(),
        clean_suite_result = stage.clean_suite_result(),
        stable_rustc = tools.stable_rustc,
        msrv_rustc = tools.msrv_rustc,
        cargo = tools.cargo,
        mdbook = tools.mdbook,
        git = tools.git,
        tar = tools.tar,
        gzip = tools.gzip,
        sha256sum = tools.sha256sum,
        cargo_deny = tools.cargo_deny,
    )
}

fn run_checked(root: &Path, program: &str, args: &[&str]) -> Result<(), String> {
    eprintln!("  $ {program} {}", args.join(" "));
    let status = Command::new(program)
        .args(args)
        .current_dir(root)
        .status()
        .map_err(|error| format!("cannot run {program}: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{program} exited with {status}"))
    }
}

fn command_output(root: &Path, program: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(program)
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|error| format!("cannot run {program}: {error}"))?;
    if !output.status.success() {
        return Err(format!("{program} exited with {}", output.status));
    }
    String::from_utf8(output.stdout)
        .map(|value| value.trim_end().to_owned())
        .map_err(|_| format!("{program} output is not UTF-8"))
}

struct TempGuard {
    source: PathBuf,
    path: PathBuf,
    marker: String,
}

impl TempGuard {
    fn create(source: &Path, path: &Path, revision: &str) -> Result<Self, String> {
        let expected = source.join(".git-exclude/tmp/release-gate");
        if path != expected {
            return Err("refusing unexpected release-gate temporary path".to_owned());
        }
        fs::create_dir_all(
            path.parent()
                .ok_or_else(|| "temporary path has no parent".to_owned())?,
        )
        .map_err(|error| format!("cannot create temporary parent: {error}"))?;
        fs::create_dir(path).map_err(|error| {
            format!(
                "temporary directory {} must be absent before the gate: {error}",
                path.display()
            )
        })?;
        let marker = format!("loeres-release-gate:{revision}\n");
        fs::write(path.join("OWNED"), &marker)
            .map_err(|error| format!("cannot write temporary ownership marker: {error}"))?;
        Ok(Self {
            source: source.to_owned(),
            path: path.to_owned(),
            marker,
        })
    }
}

impl Drop for TempGuard {
    fn drop(&mut self) {
        let expected = self.source.join(".git-exclude/tmp/release-gate");
        let owned = fs::read_to_string(self.path.join("OWNED"));
        let marker_matches = owned
            .as_deref()
            .map(|value| value == self.marker)
            .unwrap_or(false);
        if self.path == expected && marker_matches {
            if let Err(error) = fs::remove_dir_all(&self.path) {
                eprintln!(
                    "[release-gate] warning: cannot remove owned temporary directory {}: {error}",
                    self.path.display()
                );
            }
        } else {
            eprintln!(
                "[release-gate] warning: ownership check failed; retained {}",
                self.path.display()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ArchiveDigests, EvidenceStage, ToolVersions, render_evidence, require_release_inputs,
        sha256, uncompressed_tar_sha256, validate_archive_listing, validate_archive_types,
    };
    use crate::checks::release_gate::{Candidate, CandidateRef, TrackedEntry};
    use std::fs;
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Stdio};
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn entry(path: &str) -> TrackedEntry {
        TrackedEntry {
            mode: "100644".to_owned(),
            object: "a".repeat(40),
            path: path.to_owned(),
        }
    }

    fn candidate(reference: CandidateRef) -> Candidate {
        Candidate {
            version: "0.20.1".to_owned(),
            revision: "b".repeat(40),
            reference,
            entries: vec![entry("Cargo.toml")],
        }
    }

    fn tools() -> ToolVersions {
        ToolVersions {
            stable_rustc: "rustc stable".to_owned(),
            msrv_rustc: "rustc 1.85.0".to_owned(),
            cargo: "cargo stable".to_owned(),
            mdbook: "mdbook 0.5.4".to_owned(),
            git: "git version test".to_owned(),
            tar: "tar test".to_owned(),
            gzip: "gzip test".to_owned(),
            sha256sum: "sha256sum test".to_owned(),
            cargo_deny: "cargo-deny test".to_owned(),
        }
    }

    #[test]
    fn archive_listing_requires_exact_tracked_file_set() {
        let expected = [entry("Cargo.toml"), entry("docs/src/index.md")];
        assert!(
            validate_archive_listing(
                &["Cargo.toml", "docs/", "docs/src/", "docs/src/index.md"],
                &expected
            )
            .is_ok()
        );
        assert!(validate_archive_listing(&["Cargo.toml", "docs/src/extra.md"], &expected).is_err());
        assert!(validate_archive_listing(&["Cargo.toml", "Cargo.toml"], &expected).is_err());
    }

    #[test]
    fn archive_types_allow_only_regular_files_and_directories() {
        assert!(validate_archive_types(&["-rw-r--r-- file", "drwxr-xr-x dir/"], 2).is_ok());
        for special in [
            "lrwxrwxrwx symlink",
            "hrw-r--r-- hardlink",
            "prw-r--r-- fifo",
            "crw-r--r-- device",
        ] {
            assert!(validate_archive_types(&[special], 1).is_err());
        }
        assert!(validate_archive_types(&["-rw-r--r-- file"], 2).is_err());
    }

    #[test]
    fn release_inputs_require_lock_license_workflows_and_source_families() {
        let mut entries = super::REQUIRED_FILES
            .iter()
            .map(|path| entry(path))
            .collect::<Vec<_>>();
        for path in [
            "crates/loeres/src/lib.rs",
            "xtask/src/main.rs",
            "docs/src/index.md",
            "rfcs/README.md",
            "conformance/README.md",
            "examples/device-box-pfo/Cargo.toml",
        ] {
            if !entries.iter().any(|entry| entry.path == path) {
                entries.push(entry(path));
            }
        }
        assert!(require_release_inputs(&entries).is_ok());
        entries.retain(|entry| entry.path != "Cargo.lock");
        assert!(require_release_inputs(&entries).is_err());
    }

    fn digests<'a>(archive: &'a str, uncompressed: &'a str) -> ArchiveDigests<'a> {
        ArchiveDigests {
            archive,
            uncompressed,
        }
    }

    #[test]
    fn pre_extraction_evidence_keeps_content_and_mode_pending() {
        let text = render_evidence(
            &candidate(CandidateRef::LocalDryRun),
            "loeres-v0.20.1.tar.gz",
            Some(&digests(&"c".repeat(64), &"d".repeat(64))),
            EvidenceStage::ArchiveValidated,
            &tools(),
        );
        assert!(text.contains("ARCHIVE VALIDATED; EXTRACTION PENDING"));
        assert!(text.contains("Archive layout/type/file-set/digest validation:** pass"));
        assert!(
            text.contains("Extracted Git-object content and executable-mode identity:** pending")
        );
        assert!(text.contains("Clean-extraction complete applicable suite: pending"));
    }

    #[test]
    fn post_verification_evidence_promotes_only_extracted_identity() {
        let text = render_evidence(
            &candidate(CandidateRef::LocalDryRun),
            "loeres-v0.20.1.tar.gz",
            Some(&digests(&"c".repeat(64), &"d".repeat(64))),
            EvidenceStage::ExtractedContentVerified,
            &tools(),
        );
        assert!(text.contains("EXTRACTED CONTENT VERIFIED; CLEAN SUITE PENDING"));
        assert!(text.contains("Extracted Git-object content and executable-mode identity:** pass"));
        assert!(text.contains("Clean-extraction complete applicable suite: pending"));
    }

    #[test]
    fn evidence_heading_is_neutral_and_identity_is_candidate_specific() {
        let local = render_evidence(
            &candidate(CandidateRef::LocalDryRun),
            "loeres-v0.20.1.tar.gz",
            None,
            EvidenceStage::SourceSuitePassed,
            &tools(),
        );
        let tagged = render_evidence(
            &candidate(CandidateRef::Tag("0.20.1".to_owned())),
            "loeres-v0.20.1.tar.gz",
            Some(&digests(&"c".repeat(64), &"d".repeat(64))),
            EvidenceStage::Complete,
            &tools(),
        );
        assert!(local.starts_with("# RFC 019 Release-Gate Evidence"));
        assert!(local.contains("local dry run (no tag assertion)"));
        assert!(tagged.starts_with("# RFC 019 Release-Gate Evidence"));
        assert!(tagged.contains("validated tag `0.20.1`"));
        assert!(tagged.contains("**Status:** PASS"));
        assert!(tagged.contains("Clean-extraction complete applicable suite: pass"));
    }

    /// RFC 028 §13: the archive SHA-256 identifies this build's bytes; the
    /// uncompressed tar SHA-256 is the cross-environment identity. Both must
    /// appear, in that order, with the paragraph naming what each means.
    #[test]
    fn rendered_evidence_orders_archive_digest_before_uncompressed_digest() {
        let text = render_evidence(
            &candidate(CandidateRef::LocalDryRun),
            "loeres-v0.20.1.tar.gz",
            Some(&digests(&"c".repeat(64), &"d".repeat(64))),
            EvidenceStage::Complete,
            &tools(),
        );
        let archive_at = text
            .find("**Archive SHA-256:**")
            .expect("archive digest label present");
        let uncompressed_at = text
            .find("**Uncompressed tar SHA-256:**")
            .expect("uncompressed digest label present");
        assert!(
            archive_at < uncompressed_at,
            "Archive SHA-256 must precede Uncompressed tar SHA-256"
        );
        assert!(text.contains(&format!("`{}`", "c".repeat(64))));
        assert!(text.contains(&format!("`{}`", "d".repeat(64))));
        assert!(text.contains(
            "The uncompressed tar SHA-256 is the cross-environment identity of this source\n\
             artifact's content and layout. The archive SHA-256 identifies this build's bytes\n\
             and is not expected to reproduce in another environment (RFC 028)."
        ));
    }

    #[test]
    fn pending_evidence_marks_both_digests_pending() {
        let text = render_evidence(
            &candidate(CandidateRef::LocalDryRun),
            "loeres-v0.20.1.tar.gz",
            None,
            EvidenceStage::SourceSuitePassed,
            &tools(),
        );
        assert!(text.contains("**Archive SHA-256:** `pending`"));
        assert!(text.contains("**Uncompressed tar SHA-256:** `pending`"));
    }

    // -------------------------------------------------------------------
    // RFC 028: uncompressed_tar_sha256
    // -------------------------------------------------------------------

    struct GzipFixture {
        root: PathBuf,
    }

    impl GzipFixture {
        /// Compress `data` at gzip `level` (e.g. `"-1"`, `"-9"`) with `-n` (no
        /// name/mtime, for a deterministic header) and write the result to a
        /// fresh scratch file. Returns the fixture (whose `Drop` cleans up)
        /// and the archive's path.
        fn compress(data: &[u8], level: &str) -> (Self, PathBuf) {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
                "../target/xtask-tests/release-gate-gzip-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&root).expect("scratch directory");

            let mut gzip = Command::new("gzip")
                .arg(level)
                .arg("-n")
                .arg("-c")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()
                .expect("gzip available in test environment");
            gzip.stdin
                .take()
                .expect("gzip stdin pipe")
                .write_all(data)
                .expect("write to gzip stdin");
            let output = gzip.wait_with_output().expect("gzip terminates");
            assert!(output.status.success(), "gzip {level} failed");

            let archive = root.join("archive.gz");
            fs::write(&archive, &output.stdout).expect("write compressed archive");
            (Self { root }, archive)
        }
    }

    impl Drop for GzipFixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    /// Deterministic, non-trivially-compressible bytes, large enough that
    /// `gzip -1` and `gzip -9` produce different compressed output — the
    /// stated precondition this test asserts rather than assumes.
    fn incompressible_bytes(len: usize) -> Vec<u8> {
        let mut data = Vec::with_capacity(len);
        let mut state = 0x2545_F491_4F6C_DD1Du64;
        while data.len() < len {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            data.extend_from_slice(&state.to_le_bytes());
        }
        data.truncate(len);
        data
    }

    #[test]
    fn uncompressed_digest_matches_across_gzip_levels_while_archive_digests_differ() {
        let data = incompressible_bytes(64 * 1024);
        let (_low_fixture, archive_low) = GzipFixture::compress(&data, "-1");
        let (_high_fixture, archive_high) = GzipFixture::compress(&data, "-9");

        let root = Path::new(".");
        let archive_digest_low = sha256(root, &archive_low).expect("sha256 of gzip -1 output");
        let archive_digest_high = sha256(root, &archive_high).expect("sha256 of gzip -9 output");
        assert_ne!(
            archive_digest_low, archive_digest_high,
            "gzip -1 and gzip -9 must compress this input to different bytes, \
             or this test proves nothing"
        );

        let uncompressed_low =
            uncompressed_tar_sha256(root, &archive_low).expect("uncompressed digest of gzip -1");
        let uncompressed_high =
            uncompressed_tar_sha256(root, &archive_high).expect("uncompressed digest of gzip -9");
        assert_eq!(
            uncompressed_low, uncompressed_high,
            "the same content compressed at two levels must yield the same \
             uncompressed-tar identity"
        );
    }

    #[test]
    fn uncompressed_digest_fails_closed_on_non_gzip_input() {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
            "../target/xtask-tests/release-gate-not-gzip-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).expect("scratch directory");
        let plain = root.join("plain.txt");
        fs::write(&plain, b"not a gzip stream at all").expect("write plain file");

        let result = uncompressed_tar_sha256(Path::new("."), &plain);
        assert!(
            result.is_err(),
            "a non-gzip file must fail closed, never return a digest"
        );

        let _ = fs::remove_dir_all(&root);
    }
}
