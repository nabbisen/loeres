//! Bounded RFC 021 conditional-release metadata and predicate validation.
//!
//! The tracked metadata file is intentionally optional before RFC 021 S2. If
//! it exists, every field is fail-closed and bound to the one reviewed
//! `0.20.2` correction. Repository bytes can describe a candidate, but they
//! can never prove that the external distribution predicate became true.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

pub(crate) const METADATA_PATH: &str = "release/conditional-finalization.toml";
pub(crate) const CONDITIONAL_STATUS: &str = "Implemented (conditional finalization for 0.20.2)";
pub(crate) const APEX_MARKER: &str = "**RFC 021 conditional release-finalization metadata.**";

const EXPECTED_SCHEMA_VERSION: u32 = 1;
const EXPECTED_RELEASE_VERSION: &str = "0.20.2";
const EXPECTED_CANONICAL_TAG: &str = "0.20.2";
const EXPECTED_PHASE: &str = "release-finalization-candidate";
const EXPECTED_REMOTE: &str = "origin";
const EXPECTED_BUNDLE: &str = "tag-push-release-workflow-v1";
const EXPECTED_RFCS: [u16; 3] = [19, 20, 21];
const EXPECTED_START_TIMEOUT_MINUTES: u32 = 30;
const EXPECTED_TERMINAL_TIMEOUT_MINUTES: u32 = 120;

const CONDITIONAL_RFC_FILES: [(&str, &str); 3] = [
    ("019", "019-release-integrity-and-msrv-recovery.md"),
    (
        "020",
        "020-normative-documentation-authority-and-currency.md",
    ),
    ("021", "021-conditional-release-finalization.md"),
];

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ConditionalMetadata {
    pub(crate) schema_version: u32,
    pub(crate) release_version: String,
    pub(crate) canonical_tag: String,
    pub(crate) phase: String,
    pub(crate) authoritative_remote: String,
    pub(crate) distribution_bundle: String,
    pub(crate) conditional_rfcs: Vec<u16>,
    pub(crate) workflow_start_timeout_minutes: u32,
    pub(crate) workflow_terminal_timeout_minutes: u32,
}

impl ConditionalMetadata {
    pub(crate) fn parse(source: &str) -> Result<Self, String> {
        let metadata = toml::from_str::<Self>(source)
            .map_err(|error| format!("cannot parse conditional metadata: {error}"))?;
        metadata.validate()?;
        Ok(metadata)
    }

    pub(crate) fn validate(&self) -> Result<(), String> {
        require_equal(
            "schema_version",
            self.schema_version,
            EXPECTED_SCHEMA_VERSION,
        )?;
        require_equal(
            "release_version",
            self.release_version.as_str(),
            EXPECTED_RELEASE_VERSION,
        )?;
        require_equal(
            "canonical_tag",
            self.canonical_tag.as_str(),
            EXPECTED_CANONICAL_TAG,
        )?;
        require_equal("phase", self.phase.as_str(), EXPECTED_PHASE)?;
        require_equal(
            "authoritative_remote",
            self.authoritative_remote.as_str(),
            EXPECTED_REMOTE,
        )?;
        require_equal(
            "distribution_bundle",
            self.distribution_bundle.as_str(),
            EXPECTED_BUNDLE,
        )?;
        require_equal(
            "workflow_start_timeout_minutes",
            self.workflow_start_timeout_minutes,
            EXPECTED_START_TIMEOUT_MINUTES,
        )?;
        require_equal(
            "workflow_terminal_timeout_minutes",
            self.workflow_terminal_timeout_minutes,
            EXPECTED_TERMINAL_TIMEOUT_MINUTES,
        )?;

        if self.conditional_rfcs.as_slice() != EXPECTED_RFCS {
            return Err(format!(
                "conditional_rfcs must be exactly [19, 20, 21], found {:?}",
                self.conditional_rfcs
            ));
        }
        Ok(())
    }
}

fn require_equal<T>(field: &str, actual: T, expected: T) -> Result<(), String>
where
    T: std::fmt::Debug + PartialEq,
{
    if actual == expected {
        Ok(())
    } else {
        Err(format!("{field} must be {expected:?}, found {actual:?}"))
    }
}

pub(crate) fn load_optional(root: &Path) -> Result<Option<ConditionalMetadata>, String> {
    let path = root.join(METADATA_PATH);
    match fs::read_to_string(&path) {
        Ok(source) => ConditionalMetadata::parse(&source).map(Some),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("cannot read {}: {error}", path.display())),
    }
}

pub(crate) fn validate_lifecycle(root: &Path, metadata: &ConditionalMetadata) -> Vec<String> {
    let mut errors = Vec::new();
    if let Err(error) = metadata.validate() {
        errors.push(format!("CONDITIONAL METADATA: {error}"));
        return errors;
    }

    let index_path = root.join("rfcs/README.md");
    let index = match fs::read_to_string(&index_path) {
        Ok(index) => index,
        Err(error) => {
            errors.push(format!(
                "CONDITIONAL RFC INDEX: cannot read {}: {error}",
                index_path.display()
            ));
            return errors;
        }
    };

    for (number, file_name) in CONDITIONAL_RFC_FILES {
        let accepted = root.join("rfcs/accepted").join(file_name);
        let done = root.join("rfcs/done").join(file_name);
        if accepted.exists() {
            errors.push(format!(
                "CONDITIONAL RFC {number}: accepted path still exists"
            ));
        }
        let source = match fs::read_to_string(&done) {
            Ok(source) => source,
            Err(error) => {
                errors.push(format!(
                    "CONDITIONAL RFC {number}: cannot read required done path {}: {error}",
                    done.display()
                ));
                continue;
            }
        };
        let expected_source = format!("**Status.** {CONDITIONAL_STATUS}");
        let statuses = source
            .lines()
            .filter(|line| line.starts_with("**Status.**"))
            .collect::<Vec<_>>();
        if statuses.len() != 1 || statuses[0] != expected_source {
            errors.push(format!(
                "CONDITIONAL RFC {number}: expected exactly one Status field equal to `{expected_source}`, found {statuses:?}"
            ));
        }

        let expected_link = format!("[{number}](done/{file_name})");
        let rows = index
            .lines()
            .filter(|line| line.contains(&expected_link))
            .collect::<Vec<_>>();
        if rows.len() != 1 || !rows[0].contains(&format!("| {CONDITIONAL_STATUS} |")) {
            errors.push(format!(
                "CONDITIONAL RFC {number}: index must contain one exact done/status row"
            ));
        }
    }

    let allowed_paths = CONDITIONAL_RFC_FILES
        .iter()
        .map(|(_, file_name)| root.join("rfcs/done").join(file_name))
        .collect::<BTreeSet<_>>();
    for path in rfc_markdown_paths(root) {
        if allowed_paths.contains(&path) {
            continue;
        }
        if let Ok(source) = fs::read_to_string(&path) {
            let expected_source = format!("**Status.** {CONDITIONAL_STATUS}");
            if source
                .lines()
                .take_while(|line| !line.starts_with("## "))
                .any(|line| line == expected_source)
            {
                errors.push(format!(
                    "CONDITIONAL RFC ALLOWLIST: unreviewed use at {}",
                    path.display()
                ));
            }
        }
    }
    errors
}

fn rfc_markdown_paths(root: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for folder in ["proposed", "accepted", "done", "archive"] {
        let directory = root.join("rfcs").join(folder);
        let Ok(entries) = fs::read_dir(directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|extension| extension.to_str()) == Some("md") {
                paths.push(path);
            }
        }
    }
    paths.sort();
    paths
}

#[cfg(test)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum WorkflowConclusion {
    Success,
    Cancelled,
    TimedOut,
    GateFailed,
    UploadFailed,
}

#[cfg(test)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) struct DistributionObservation {
    pub(crate) q_established_at_minutes: u32,
    pub(crate) remote_accepted_at_minutes: u32,
    pub(crate) workflow_started_at_minutes: Option<u32>,
    pub(crate) workflow_finished_at_minutes: Option<u32>,
    pub(crate) conclusion: Option<WorkflowConclusion>,
}

/// Evaluate only a supplied observation. This never discovers or persists `P`.
#[cfg(test)]
pub(crate) fn observed_distribution_succeeded(observation: DistributionObservation) -> bool {
    let Some(started) = observation.workflow_started_at_minutes else {
        return false;
    };
    let Some(finished) = observation.workflow_finished_at_minutes else {
        return false;
    };
    observation.remote_accepted_at_minutes > observation.q_established_at_minutes
        && started >= observation.remote_accepted_at_minutes
        && started - observation.remote_accepted_at_minutes <= EXPECTED_START_TIMEOUT_MINUTES
        && finished >= started
        && finished - observation.remote_accepted_at_minutes <= EXPECTED_TERMINAL_TIMEOUT_MINUTES
        && observation.conclusion == Some(WorkflowConclusion::Success)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::{
        CONDITIONAL_RFC_FILES, CONDITIONAL_STATUS, ConditionalMetadata, DistributionObservation,
        WorkflowConclusion, observed_distribution_succeeded, validate_lifecycle,
    };

    static NEXT_FIXTURE: AtomicUsize = AtomicUsize::new(0);

    struct LifecycleFixture {
        root: PathBuf,
    }

    impl LifecycleFixture {
        fn valid() -> Self {
            let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
                "../target/xtask-tests/conditional-{}-{}",
                std::process::id(),
                NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
            ));
            for folder in ["accepted", "done"] {
                fs::create_dir_all(root.join("rfcs").join(folder)).unwrap();
            }
            let mut index = String::new();
            for (number, file_name) in CONDITIONAL_RFC_FILES {
                fs::write(
                    root.join("rfcs/done").join(file_name),
                    format!("# RFC {number}\n\n**Status.** {CONDITIONAL_STATUS}\n"),
                )
                .unwrap();
                index.push_str(&format!(
                    "| [{number}](done/{file_name}) | Title | {CONDITIONAL_STATUS} | note |\n"
                ));
            }
            fs::write(root.join("rfcs/README.md"), index).unwrap();
            Self { root }
        }
    }

    impl Drop for LifecycleFixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn valid_metadata() -> String {
        r#"
schema_version = 1
release_version = "0.20.2"
canonical_tag = "0.20.2"
phase = "release-finalization-candidate"
authoritative_remote = "origin"
distribution_bundle = "tag-push-release-workflow-v1"
conditional_rfcs = [19, 20, 21]
workflow_start_timeout_minutes = 30
workflow_terminal_timeout_minutes = 120
"#
        .to_owned()
    }

    #[test]
    fn exact_reviewed_metadata_is_accepted() {
        let parsed = ConditionalMetadata::parse(&valid_metadata()).unwrap();
        assert_eq!(parsed.conditional_rfcs, vec![19, 20, 21]);
    }

    #[test]
    fn metadata_rejects_omitted_added_duplicate_and_reordered_scope() {
        for replacement in [
            "conditional_rfcs = [19, 20]",
            "conditional_rfcs = [19, 20, 21, 22]",
            "conditional_rfcs = [19, 20, 21, 21]",
        ] {
            let source = valid_metadata().replace("conditional_rfcs = [19, 20, 21]", replacement);
            assert!(ConditionalMetadata::parse(&source).is_err());
        }
        let reordered = valid_metadata().replace("[19, 20, 21]", "[21, 20, 19]");
        assert!(ConditionalMetadata::parse(&reordered).is_err());
    }

    #[test]
    fn metadata_rejects_protocol_drift_and_self_referential_fields() {
        for source in [
            valid_metadata().replace("0.20.2", "0.20.3"),
            valid_metadata().replace("origin", "upstream"),
            valid_metadata().replace("= 30", "= 31"),
            valid_metadata().replace("= 120", "= 121"),
            format!("{}\nfinalization_commit = \"deadbeef\"\n", valid_metadata()),
        ] {
            assert!(ConditionalMetadata::parse(&source).is_err());
        }
    }

    #[test]
    fn lifecycle_accepts_only_the_atomic_reviewed_three_rfc_move() {
        let fixture = LifecycleFixture::valid();
        let metadata = ConditionalMetadata::parse(&valid_metadata()).unwrap();
        fs::write(
            fixture.root.join("rfcs/done/000-policy.md"),
            format!(
                "# RFC 000\n\n**Status.** Implemented\n\n## Policy\n\n```markdown\n**Status.** {CONDITIONAL_STATUS}\n```\n"
            ),
        )
        .unwrap();
        assert!(validate_lifecycle(&fixture.root, &metadata).is_empty());
    }

    #[test]
    fn lifecycle_rejects_partial_move_status_drift_and_unreviewed_use() {
        let fixture = LifecycleFixture::valid();
        let metadata = ConditionalMetadata::parse(&valid_metadata()).unwrap();

        fs::rename(
            fixture
                .root
                .join("rfcs/done/020-normative-documentation-authority-and-currency.md"),
            fixture
                .root
                .join("rfcs/accepted/020-normative-documentation-authority-and-currency.md"),
        )
        .unwrap();
        fs::write(
            fixture.root.join("rfcs/done/022-unreviewed.md"),
            format!("# RFC 022\n\n**Status.** {CONDITIONAL_STATUS}\n"),
        )
        .unwrap();
        fs::write(
            fixture
                .root
                .join("rfcs/done/019-release-integrity-and-msrv-recovery.md"),
            "# RFC 019\n\n**Status.** Implemented (v0.20.2)\n",
        )
        .unwrap();

        let errors = validate_lifecycle(&fixture.root, &metadata);
        assert!(errors.iter().any(|error| error.contains("accepted path")));
        assert!(
            errors
                .iter()
                .any(|error| error.contains("expected exactly one Status field"))
        );
        assert!(errors.iter().any(|error| error.contains("unreviewed use")));
    }

    #[test]
    fn lifecycle_requires_one_exact_status_field_total() {
        let metadata = ConditionalMetadata::parse(&valid_metadata()).unwrap();
        let expected = format!("**Status.** {CONDITIONAL_STATUS}");
        for statuses in [
            format!("{expected}\n**Status.** Implemented (v0.20.2)"),
            format!("{expected}\n{expected}"),
            "No status field".to_owned(),
            "**Status.** Implemented (v0.20.2)".to_owned(),
        ] {
            let fixture = LifecycleFixture::valid();
            fs::write(
                fixture
                    .root
                    .join("rfcs/done/019-release-integrity-and-msrv-recovery.md"),
                format!("# RFC 019\n\n{statuses}\n"),
            )
            .unwrap();
            let errors = validate_lifecycle(&fixture.root, &metadata);
            assert!(
                errors
                    .iter()
                    .any(|error| error.contains("expected exactly one Status field")),
                "accepted invalid Status fields: {statuses}"
            );
        }
    }

    fn successful_observation() -> DistributionObservation {
        DistributionObservation {
            q_established_at_minutes: 10,
            remote_accepted_at_minutes: 11,
            workflow_started_at_minutes: Some(20),
            workflow_finished_at_minutes: Some(100),
            conclusion: Some(WorkflowConclusion::Success),
        }
    }

    #[test]
    fn distribution_requires_success_strictly_after_q_within_both_windows() {
        assert!(observed_distribution_succeeded(successful_observation()));

        let mut not_after_q = successful_observation();
        not_after_q.remote_accepted_at_minutes = 10;
        assert!(!observed_distribution_succeeded(not_after_q));

        let mut late_start = successful_observation();
        late_start.workflow_started_at_minutes = Some(42);
        assert!(!observed_distribution_succeeded(late_start));

        let mut late_finish = successful_observation();
        late_finish.workflow_finished_at_minutes = Some(132);
        assert!(!observed_distribution_succeeded(late_finish));
    }

    #[test]
    fn distribution_rejects_no_start_and_every_non_success_terminal_state() {
        let mut no_start = successful_observation();
        no_start.workflow_started_at_minutes = None;
        assert!(!observed_distribution_succeeded(no_start));

        let mut no_finish = successful_observation();
        no_finish.workflow_finished_at_minutes = None;
        assert!(!observed_distribution_succeeded(no_finish));

        for conclusion in [
            WorkflowConclusion::Cancelled,
            WorkflowConclusion::TimedOut,
            WorkflowConclusion::GateFailed,
            WorkflowConclusion::UploadFailed,
        ] {
            let mut observation = successful_observation();
            observation.conclusion = Some(conclusion);
            assert!(!observed_distribution_succeeded(observation));
        }
    }
}
