//! `review-evidence` — RFC 022 architecture-review citation and provenance
//! integrity.
//!
//! Two assertions are enforced, fail-closed, because both depend only on
//! tracked bytes: every review citation in a tracked Markdown document
//! resolves to a row in `rfcs/review-evidence-index.md`, and every row
//! carries an author tier from the closed set (RFC 022 §0.1, §11.3). Hash
//! verification against the maintainer-held corpus runs when
//! `.git-exclude/reviewed/` is present and is reported `unavailable` — never
//! `passed` — when it is absent, per RFC 022 §11.3's evidence-class
//! discipline. A citation-shaped token that does not match the recognized
//! grammar is reported as a near-miss finding rather than silently ignored
//! (§11.4), but does not by itself fail the gate: RFC 022 draws that
//! distinction explicitly ("reported", not "fails").
//!
//! The authority-verb rule (§0.2 — a citation may not say a review
//! *accepted*/*approved*/*authorized* anything unless its tier is
//! `architect`, `owner`, or `external`) is deliberately not automated here.
//! It requires reading surrounding prose; a bounded lexical check would
//! produce false confidence, which RFC 022 §0.2 and this project's operating
//! rules treat as worse than no check.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::process::Command;

use super::link_audit;

const INDEX_PATH: &str = "rfcs/review-evidence-index.md";
const CORPUS_DIR: &str = ".git-exclude/reviewed";
const CLOSED_TIERS: &[&str] = &[
    "owner",
    "architect",
    "implementer",
    "external",
    "unrecorded",
];

pub fn run() -> bool {
    eprintln!("[review-evidence] RFC 022 citation and provenance integrity");
    let mut ok = true;

    let index_source = match fs::read_to_string(INDEX_PATH) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("  MISSING/UNREADABLE: {INDEX_PATH}: {error}");
            eprintln!("[review-evidence] FAIL");
            return false;
        }
    };
    let rows = match parse_index_rows(&index_source) {
        Ok(rows) => rows,
        Err(error) => {
            eprintln!("  INDEX PARSE: {error}");
            eprintln!("[review-evidence] FAIL");
            return false;
        }
    };

    for error in validate_tiers(&rows) {
        eprintln!("  TIER: {error}");
        ok = false;
    }

    let mut files = Vec::new();
    link_audit::collect_md(Path::new("."), &mut files);
    // The index registers citations; it does not make any of its own. Its
    // Subject column and illustrative prose are not scanned, so a line like
    // "a citation such as `architecture review 024`" cannot create a
    // self-referential near-miss or resolution dependency.
    files.retain(|path| !path.ends_with(INDEX_PATH));
    files.sort();

    let mut cited = BTreeSet::new();
    let mut near_miss_findings = Vec::new();
    for path in &files {
        let Ok(source) = fs::read_to_string(path) else {
            continue;
        };
        for (line_number, line) in source.lines().enumerate() {
            let (refs, near_misses) = find_citations_in_line(line);
            for reference in refs {
                cited.insert((reference, path.display().to_string()));
            }
            for token in near_misses {
                near_miss_findings.push(format!(
                    "{}:{}: citation-shaped token `{token}` near `review`/`reviews` does not match the NNN citation grammar",
                    path.display(),
                    line_number + 1,
                ));
            }
        }
    }

    let distinct_cited: BTreeSet<String> = cited
        .iter()
        .map(|(reference, _)| reference.clone())
        .collect();
    for (path, reference) in unresolved_citations(&cited, &rows) {
        eprintln!(
            "  UNRESOLVED CITATION: {path} cites review {reference}, no matching row in {INDEX_PATH}"
        );
        ok = false;
    }

    for finding in &near_miss_findings {
        eprintln!("  NEAR-MISS (reported, not blocking): {finding}");
    }

    match corpus_hashes(Path::new(CORPUS_DIR)) {
        Some(hashes) => {
            for error in verify_hashes(&rows, &hashes) {
                eprintln!("  HASH: {error}");
                ok = false;
            }
            eprintln!(
                "  hash verification: corpus present, {} file(s)",
                hashes.len()
            );
        }
        None => {
            eprintln!(
                "  hash verification: unavailable (corpus absent at {CORPUS_DIR}; this is not a pass)"
            );
        }
    }

    eprintln!(
        "  {} review document(s) registered, {} distinct citation(s) resolved",
        rows.len(),
        distinct_cited.len()
    );
    eprintln!("[review-evidence] {}", if ok { "PASS" } else { "FAIL" });
    ok
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct IndexRow {
    /// `None` for the `—` pre-numbering rows; otherwise the exact three-digit
    /// reference text as tracked (e.g. `"021"`), so comparison against a
    /// citation is a plain string match with no re-padding logic to get wrong.
    reference: Option<String>,
    sha256: String,
    tier: String,
}

/// Parse the index's single Markdown table. Pure function of the source text
/// so every required case is testable without a file on disk.
fn parse_index_rows(source: &str) -> Result<Vec<IndexRow>, String> {
    let mut rows = Vec::new();
    let mut in_table = false;
    for line in source.lines() {
        let trimmed = line.trim();
        if !in_table {
            if trimmed.starts_with("| Ref ") {
                in_table = true;
            }
            continue;
        }
        if trimmed.starts_with("|---") || trimmed.starts_with("|:--") || trimmed.starts_with("|-") {
            continue;
        }
        if !trimmed.starts_with('|') {
            break;
        }
        let cells: Vec<&str> = trimmed
            .trim_matches('|')
            .split('|')
            .map(str::trim)
            .collect();
        if cells.len() != 6 {
            return Err(format!(
                "expected 6 columns (Ref | Date | Subject | SHA-256 | Author tier | Cited in release), found {}: `{line}`",
                cells.len()
            ));
        }
        let raw_reference = cells[0].trim_matches('`');
        let reference = if raw_reference == "—" || raw_reference.is_empty() {
            None
        } else {
            Some(raw_reference.to_owned())
        };
        let sha256 = cells[3].trim_matches('`').to_owned();
        let tier = cells[4].trim_matches('`').to_owned();
        if sha256.is_empty() {
            return Err(format!("row has an empty SHA-256 field: `{line}`"));
        }
        rows.push(IndexRow {
            reference,
            sha256,
            tier,
        });
    }
    Ok(rows)
}

fn validate_tiers(rows: &[IndexRow]) -> Vec<String> {
    rows.iter()
        .filter(|row| !CLOSED_TIERS.contains(&row.tier.as_str()))
        .map(|row| {
            format!(
                "row `{}` carries tier `{}`, not one of {CLOSED_TIERS:?}",
                row.reference.as_deref().unwrap_or("—"),
                row.tier
            )
        })
        .collect()
}

/// Every `(reference, citing path)` pair that names a review not present as a
/// row in the index — the citation-resolution assertion, fail-closed because
/// it depends only on tracked bytes.
fn unresolved_citations(
    cited: &BTreeSet<(String, String)>,
    rows: &[IndexRow],
) -> Vec<(String, String)> {
    let known_refs: BTreeSet<&str> = rows
        .iter()
        .filter_map(|row| row.reference.as_deref())
        .collect();
    cited
        .iter()
        .filter(|(reference, _)| !known_refs.contains(reference.as_str()))
        .map(|(reference, path)| (path.clone(), reference.clone()))
        .collect()
}

fn verify_hashes(rows: &[IndexRow], corpus_hashes: &BTreeSet<String>) -> Vec<String> {
    rows.iter()
        .filter(|row| !corpus_hashes.contains(&row.sha256))
        .map(|row| {
            format!(
                "row `{}` SHA-256 `{}` matches no file in the present corpus",
                row.reference.as_deref().unwrap_or("—"),
                row.sha256
            )
        })
        .collect()
}

/// `None` when the corpus directory does not exist (legitimately absent from
/// a clean extraction — reported `unavailable`, never treated as a pass).
/// Shells out to the system `sha256sum`, matching `release_gate::package`'s
/// existing convention rather than adding a hashing crate dependency.
fn corpus_hashes(root: &Path) -> Option<BTreeSet<String>> {
    let entries = fs::read_dir(root).ok()?;
    let mut hashes = BTreeSet::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }
        if let Some(hash) = sha256sum(&path) {
            hashes.insert(hash);
        }
    }
    Some(hashes)
}

fn sha256sum(path: &Path) -> Option<String> {
    let output = Command::new("sha256sum").arg(path).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8(output.stdout).ok()?;
    let digest = stdout.split_whitespace().next()?;
    if digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Some(digest.to_ascii_lowercase())
    } else {
        None
    }
}

/// Find review citations on one line: resolved three-digit reference numbers,
/// and near-miss tokens (a `review`/`reviews`/`re-review` word immediately
/// followed by a purely numeric token that is not a clean three-digit
/// reference, or a slash-joined list of them).
///
/// A `review` word not immediately followed by a purely numeric token is
/// ordinary prose — "design review", "review the repository", "review B1",
/// "implementation-review (§12)", "review (BSTATIC-009)" — and produces
/// nothing. Mixed letter/digit tokens such as blocker IDs or section numbers
/// are never citation attempts, so they are neither resolved nor reported.
fn find_citations_in_line(line: &str) -> (Vec<String>, Vec<String>) {
    let words: Vec<&str> = line.split_whitespace().collect();
    let mut refs = Vec::new();
    let mut near_misses = Vec::new();
    let mut index = 0;
    while index < words.len() {
        if is_review_word(clean_word(words[index])) {
            let (found_refs, found_near, consumed) = scan_reference_tokens(&words[index + 1..]);
            refs.extend(found_refs);
            near_misses.extend(found_near);
            index += 1 + consumed;
        } else {
            index += 1;
        }
    }
    (refs, near_misses)
}

fn is_review_word(word: &str) -> bool {
    let lower = word.to_ascii_lowercase();
    lower == "review"
        || lower == "reviews"
        || lower.ends_with("-review")
        || lower.ends_with("-reviews")
}

/// Scans forward from just after a `review`/`reviews` word for a sequence of
/// number-groups joined by `,`/`and`/`&` (a list such as
/// "`021`, `022`, ..., and `033`"). Returns `(resolved, near_miss,
/// words_consumed)`. `words_consumed` is 0 when the very next word is not a
/// purely numeric citation attempt at all (ordinary prose, a blocker ID, a
/// section number) — trailing commas are already stripped by `clean_word`,
/// so a comma-separated list needs no explicit connector between numbers;
/// `and`/`&` is additionally accepted before the list's final item.
fn scan_reference_tokens(rest: &[&str]) -> (Vec<String>, Vec<String>, usize) {
    let mut refs = Vec::new();
    let mut near = Vec::new();
    let mut consumed = 0;
    for word in rest.iter().take(16) {
        let cleaned = clean_word(word);
        if cleaned.eq_ignore_ascii_case("and") || cleaned == "&" {
            consumed += 1;
            continue;
        }
        let Some((found_refs, found_near)) = classify_token(cleaned) else {
            break;
        };
        refs.extend(found_refs);
        near.extend(found_near);
        consumed += 1;
    }
    (refs, near, consumed)
}

/// A token already stripped of surrounding punctuation, e.g. `"031/032"` or
/// `"024"` or a malformed candidate like `"24"`. `None` when no `/`-separated
/// part is purely numeric — a mixed letter/digit token such as a blocker ID
/// (`B1`), a section number (`§12`), or an identifier (`BSTATIC-009`) is not
/// a citation attempt at all, so it is neither resolved nor near-missed.
/// Every purely numeric part that is exactly three digits resolves; every
/// other purely numeric part is a near miss.
fn classify_token(token: &str) -> Option<(Vec<String>, Vec<String>)> {
    let mut refs = Vec::new();
    let mut near = Vec::new();
    let mut any_part_numeric = false;
    for part in token.split('/') {
        if !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()) {
            any_part_numeric = true;
            if part.len() == 3 {
                refs.push(part.to_owned());
            } else {
                near.push(part.to_owned());
            }
        }
    }
    any_part_numeric.then_some((refs, near))
}

/// Strips markdown decoration (backticks, bold/italic asterisks, quotes,
/// brackets) and trailing punctuation/possessives from a whitespace-split
/// word, so `` `021` ``, `**036`, and `036's` all clean to a bare token.
fn clean_word(word: &str) -> &str {
    let word = word.trim_start_matches(['(', '[', '{', '"', '\'', '`', '*']);
    let word = word.trim_end_matches(['.', ',', ';', ':', ')', ']', '}', '"', '\'', '`', '*']);
    let word = word.strip_suffix("'s").unwrap_or(word);
    word.trim_end_matches(['.', ',', ';', ':'])
}

#[cfg(test)]
mod tests {
    use super::{
        IndexRow, classify_token, corpus_hashes, find_citations_in_line, parse_index_rows,
        unresolved_citations, validate_tiers, verify_hashes,
    };
    use std::collections::BTreeSet;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn valid_index() -> String {
        "# Review Evidence Index\n\n\
         Prose above the table.\n\n\
         | Ref | Date | Subject | SHA-256 | Author tier | Cited in release |\n\
         |---|---|---|---|---|:--:|\n\
         | `021` | 2026-07-17 | joint closeout | `aaaa` | `unrecorded` | ✓ |\n\
         | `036` | 2026-07-31 | rfc024 review | `bbbb` | `implementer` | ✓ |\n\
         | `—` | — | pre-numbering review | `cccc` | `unrecorded` |  |\n\n\
         ## Summary\n\nRegistered: 3.\n"
            .to_owned()
    }

    #[test]
    fn parses_reference_sha_and_tier_ignoring_date_subject_and_cited_columns() {
        let rows = parse_index_rows(&valid_index()).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].reference.as_deref(), Some("021"));
        assert_eq!(rows[0].sha256, "aaaa");
        assert_eq!(rows[0].tier, "unrecorded");
        assert_eq!(rows[2].reference, None);
    }

    #[test]
    fn rejects_a_row_with_the_wrong_column_count() {
        let source = valid_index().replace(
            "| `021` | 2026-07-17 | joint closeout | `aaaa` | `unrecorded` | ✓ |",
            "| `021` | 2026-07-17 | joint closeout | `aaaa` |",
        );
        assert!(parse_index_rows(&source).is_err());
    }

    #[test]
    fn tier_validation_fails_on_missing_or_unrecognized_tier() {
        let rows = vec![
            IndexRow {
                reference: Some("021".to_owned()),
                sha256: "aaaa".to_owned(),
                tier: String::new(),
            },
            IndexRow {
                reference: Some("022".to_owned()),
                sha256: "bbbb".to_owned(),
                tier: "reviewer".to_owned(),
            },
            IndexRow {
                reference: Some("023".to_owned()),
                sha256: "cccc".to_owned(),
                tier: "unrecorded".to_owned(),
            },
        ];
        let errors = validate_tiers(&rows);
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn a_citation_with_no_index_row_fails_resolution() {
        let rows = vec![IndexRow {
            reference: Some("021".to_owned()),
            sha256: "aaaa".to_owned(),
            tier: "unrecorded".to_owned(),
        }];
        let mut cited = BTreeSet::new();
        cited.insert(("021".to_owned(), "README.md".to_owned()));
        cited.insert(("099".to_owned(), "ROADMAP.md".to_owned()));
        let unresolved = unresolved_citations(&cited, &rows);
        assert_eq!(
            unresolved,
            vec![("ROADMAP.md".to_owned(), "099".to_owned())]
        );
    }

    #[test]
    fn citation_scan_resolves_the_canonical_multi_reference_to_both_numbers() {
        let (refs, near) =
            find_citations_in_line("RFC 021's Design approval rests on reviews 031/032.");
        assert_eq!(refs, vec!["031".to_owned(), "032".to_owned()]);
        assert!(near.is_empty());
    }

    #[test]
    fn citation_scan_handles_and_joined_and_possessive_and_re_review_forms() {
        let (refs, _) = find_citations_in_line(
            "Reviews 036 and 037 were requested as independent architecture reviews.",
        );
        assert_eq!(refs, vec!["036".to_owned(), "037".to_owned()]);

        let (refs, _) =
            find_citations_in_line("review 036's own blocker B4 already requires this.");
        assert_eq!(refs, vec!["036".to_owned()]);

        let (refs, _) = find_citations_in_line("re-review 037 accepted the amendment.");
        assert_eq!(refs, vec!["037".to_owned()]);

        let (refs, _) =
            find_citations_in_line("architecture review 027 accepted the corrected design.");
        assert_eq!(refs, vec!["027".to_owned()]);
    }

    #[test]
    fn citation_scan_ignores_ordinary_prose_uses_of_review() {
        let (refs, near) =
            find_citations_in_line("Please review the repository directly at this revision.");
        assert!(refs.is_empty());
        assert!(near.is_empty());

        let (refs, near) =
            find_citations_in_line("A design review process governs every RFC 024 change.");
        assert!(refs.is_empty());
        assert!(near.is_empty());
    }

    #[test]
    fn citation_scan_reports_near_miss_without_resolving_it() {
        let (refs, near) = find_citations_in_line("See review 24 for the earlier finding.");
        assert!(refs.is_empty());
        assert_eq!(near, vec!["24".to_owned()]);

        let (refs, near) = find_citations_in_line("See reviews 24/032 for both findings.");
        assert_eq!(refs, vec!["032".to_owned()]);
        assert_eq!(near, vec!["24".to_owned()]);
    }

    #[test]
    fn classify_token_splits_slash_joined_groups() {
        let (refs, near) = classify_token("031/032").unwrap();
        assert_eq!(refs, vec!["031".to_owned(), "032".to_owned()]);
        assert!(near.is_empty());

        let (refs, near) = classify_token("31/032").unwrap();
        assert_eq!(refs, vec!["032".to_owned()]);
        assert_eq!(near, vec!["31".to_owned()]);
    }

    #[test]
    fn classify_token_is_none_for_mixed_letter_digit_identifiers() {
        // Blocker IDs, section numbers, and other identifiers are not
        // citation attempts at all, so they are neither resolved nor
        // reported as a near miss.
        for identifier in ["B1", "§12", "BSTATIC-009", "B1–B6"] {
            assert!(
                classify_token(identifier).is_none(),
                "treated `{identifier}` as a citation attempt"
            );
        }
    }

    #[test]
    fn citation_scan_ignores_blocker_ids_and_section_numbers_after_review() {
        for line in [
            "per review B1–B6",
            "(review B3)",
            "exposed for review (§12)",
            "catch accidental copies in API review (BSTATIC-009)",
            "implementation-review B1).",
        ] {
            let (refs, near) = find_citations_in_line(line);
            assert!(refs.is_empty(), "resolved a ref from: {line}");
            assert!(near.is_empty(), "reported a near miss from: {line}");
        }
    }

    #[test]
    fn citation_scan_strips_markdown_decoration_around_a_reference() {
        let (refs, _) =
            find_citations_in_line("cite independent architecture reviews `021`, `022`.");
        assert_eq!(refs, vec!["021".to_owned(), "022".to_owned()]);

        let (refs, _) = find_citations_in_line(
            "Reviews **036 and 037 are `implementer`**, on direct owner statement.",
        );
        assert_eq!(refs, vec!["036".to_owned(), "037".to_owned()]);
    }

    #[test]
    fn citation_scan_resolves_a_long_comma_separated_list_ending_in_and() {
        let (refs, _) = find_citations_in_line(
            "cite independent architecture reviews `021`, `022`, `024`, `025`, `027`, `030`, `031`, `032`, and `033` as the authority.",
        );
        assert_eq!(
            refs,
            vec![
                "021", "022", "024", "025", "027", "030", "031", "032", "033"
            ]
        );
    }

    #[test]
    fn hash_verification_fails_on_a_present_corpus_mismatch() {
        let rows = vec![
            IndexRow {
                reference: Some("021".to_owned()),
                sha256: "matching".to_owned(),
                tier: "unrecorded".to_owned(),
            },
            IndexRow {
                reference: Some("022".to_owned()),
                sha256: "stale".to_owned(),
                tier: "unrecorded".to_owned(),
            },
        ];
        let mut present = BTreeSet::new();
        present.insert("matching".to_owned());
        let errors = verify_hashes(&rows, &present);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("022"));
    }

    struct CorpusFixture {
        root: PathBuf,
    }

    impl CorpusFixture {
        fn with_one_file(contents: &[u8]) -> (Self, String) {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
                "../target/xtask-tests/review-evidence-corpus-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&root).unwrap();
            let file = root.join("001-example.md");
            fs::write(&file, contents).unwrap();
            let hash = super::sha256sum(&file).expect("sha256sum available in test environment");
            (Self { root }, hash)
        }
    }

    impl Drop for CorpusFixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn corpus_hashes_reports_none_when_the_directory_is_absent() {
        let missing = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../target/xtask-tests/definitely-absent-corpus");
        assert!(corpus_hashes(&missing).is_none());
    }

    #[test]
    fn corpus_hashes_recomputes_present_files() {
        let (fixture, expected_hash) = CorpusFixture::with_one_file(b"review body text");
        let hashes = corpus_hashes(&fixture.root).unwrap();
        assert!(hashes.contains(&expected_hash));
    }
}
