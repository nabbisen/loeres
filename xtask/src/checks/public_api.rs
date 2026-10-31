//! `check-public-api` — source-level edge public API scan (RFC 010).

use std::fs;
use std::path::{Path, PathBuf};

use super::util::{collect_ext, strip_comment};

const EDGE_SRC: &[&str] = &[
    "crates/loeres/src",
    "crates/loeres-backend-static/src",
    "crates/loeres-device/src",
];

/// All five published crates' `src` trees (RFC 055 §3.1) — wider than `EDGE_SRC`,
/// which stays the zero-bleed token sweep's edge-three set and must not be widened.
const PUBLISHED_SRC: &[&str] = &[
    "crates/loeres/src",
    "crates/loeres-backend-static/src",
    "crates/loeres-backend-std/src",
    "crates/loeres-device/src",
    "crates/loeres-cluster/src",
];

const FORBIDDEN_PUBLIC_TOKENS: &[&str] = &[
    "Vec<", "Vec ", "String", "Box<", "Rc<", "Arc<", "HashMap", "BTreeMap", "std::", "alloc::",
    "dyn ",
];

const FORBIDDEN_ERROR_TOKENS: &[&str] = &["MaxIterationsReached", "ConvergenceStatus"];

pub fn run() -> bool {
    eprintln!("[check-public-api] scanning no_std public API source");
    let mut ok = true;
    let mut files = Vec::new();
    for root in EDGE_SRC {
        collect_ext(Path::new(root), "rs", &mut files);
    }
    files.retain(|p| p.file_name().and_then(|n| n.to_str()) != Some("tests.rs"));
    for path in files {
        ok &= audit_public_file(&path);
    }
    ok &= audit_solver_error_taxonomy();
    ok &= audit_cluster_runtime_surface();
    ok &= audit_empty_public_modules();
    eprintln!("[check-public-api] {}", if ok { "PASS" } else { "FAIL" });
    ok
}

/// `EMPTY PUBLIC MODULE` (RFC 055 §3) — every `pub mod <name>;` in the five published
/// crates' `src` trees (`PUBLISHED_SRC`), outside `tests.rs`, must resolve to
/// `<name>.rs` or `<name>/mod.rs` relative to the declaring file's own module
/// directory, and that file must declare at least one public item. A declaration
/// whose file cannot be found is a finding, not a skip.
///
/// **Limits, measured rather than assumed.** This does not handle an inline
/// `pub mod x { .. }` block, nor a module whose only public surface is a `pub use`
/// re-export — resolving either correctly would need parsing, not a line scan.
/// Neither shape exists among this project's 30 `pub mod` declarations today, so
/// the simple form is implemented and neither limit is worked around.
fn audit_empty_public_modules() -> bool {
    eprintln!("[check-public-api] scanning for empty public modules");
    let roots: Vec<&Path> = PUBLISHED_SRC.iter().map(Path::new).collect();
    let findings = empty_public_module_findings(&roots);
    for finding in &findings {
        eprintln!("  {finding}");
    }
    findings.is_empty()
}

fn empty_public_module_findings(roots: &[&Path]) -> Vec<String> {
    let mut files = Vec::new();
    for root in roots {
        collect_ext(root, "rs", &mut files);
    }
    files.retain(|p| p.file_name().and_then(|n| n.to_str()) != Some("tests.rs"));
    files.sort();
    let mut findings = Vec::new();
    for path in &files {
        let src = match fs::read_to_string(path) {
            Ok(src) => src,
            Err(e) => {
                findings.push(format!(
                    "EMPTY PUBLIC MODULE: cannot read {}: {e}",
                    path.display()
                ));
                continue;
            }
        };
        for (idx, raw) in src.lines().enumerate() {
            let code = strip_comment(raw).trim();
            let Some(name) = parse_pub_mod_decl(code) else {
                continue;
            };
            if let Some(finding) = empty_module_finding(path, idx + 1, name) {
                findings.push(finding);
            }
        }
    }
    findings
}

/// Matches `^\s*pub mod [a-z_][a-z0-9_]*;$` against already-trimmed code — the same
/// declaration shape the §0.1 re-derivation grep used.
fn parse_pub_mod_decl(code: &str) -> Option<&str> {
    let name = code.strip_prefix("pub mod ")?.strip_suffix(';')?;
    let mut chars = name.chars();
    let first = chars.next()?;
    if !(first.is_ascii_lowercase() || first == '_') {
        return None;
    }
    if chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_') {
        Some(name)
    } else {
        None
    }
}

fn empty_module_finding(path: &Path, line: usize, name: &str) -> Option<String> {
    let dir = module_dir_for(path);
    let flat = dir.join(format!("{name}.rs"));
    let nested = dir.join(name).join("mod.rs");
    let resolved = if flat.is_file() {
        flat
    } else if nested.is_file() {
        nested
    } else {
        return Some(format!(
            "EMPTY PUBLIC MODULE at {}:{line}: `{name}` resolves to no file ({} or {})",
            path.display(),
            flat.display(),
            nested.display()
        ));
    };
    let src = fs::read_to_string(&resolved).unwrap_or_default();
    let declares_public_item = src.lines().any(|raw| {
        let trimmed = strip_comment(raw).trim_start();
        trimmed.starts_with("pub ") || trimmed.starts_with("pub(")
    });
    if declares_public_item {
        None
    } else {
        Some(format!(
            "EMPTY PUBLIC MODULE at {}:{line}: `{name}` ({}) declares no public item",
            path.display(),
            resolved.display()
        ))
    }
}

/// A module declared in `lib.rs`/`main.rs`/`mod.rs` resolves against that file's own
/// directory; one declared in any other file (`runtime.rs`) resolves against the
/// nested directory of the same name (`runtime/`).
fn module_dir_for(path: &Path) -> PathBuf {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    match path.file_stem().and_then(|s| s.to_str()) {
        Some("lib") | Some("main") | Some("mod") => parent.to_path_buf(),
        Some(stem) => parent.join(stem),
        None => parent.to_path_buf(),
    }
}

fn audit_public_file(path: &Path) -> bool {
    let src = match fs::read_to_string(path) {
        Ok(src) => src,
        Err(e) => {
            eprintln!("  ! cannot read {}: {e}", path.display());
            return false;
        }
    };
    let mut ok = true;
    let mut pending = String::new();
    let mut start_line = 0usize;
    for (idx, raw) in src.lines().enumerate() {
        let code = strip_comment(raw).trim();
        if pending.is_empty() {
            if starts_public_api(code) {
                pending.push_str(code);
                start_line = idx + 1;
            } else {
                continue;
            }
        } else {
            pending.push(' ');
            pending.push_str(code);
        }
        if code.ends_with(';') || code.ends_with('{') || code.contains(" where ") {
            ok &= audit_public_signature(path, start_line, &pending);
            pending.clear();
        }
    }
    if !pending.is_empty() {
        ok &= audit_public_signature(path, start_line, &pending);
    }
    ok
}

fn starts_public_api(code: &str) -> bool {
    code.starts_with("pub ")
        || code.starts_with("pub const ")
        || code.starts_with("pub fn ")
        || code.starts_with("pub struct ")
        || code.starts_with("pub enum ")
        || code.starts_with("pub trait ")
        || code.starts_with("pub type ")
        || code.starts_with("pub use ")
}

fn audit_public_signature(path: &Path, line: usize, signature: &str) -> bool {
    let mut ok = true;
    for token in FORBIDDEN_PUBLIC_TOKENS {
        if signature.contains(token) {
            eprintln!(
                "  FORBIDDEN public token `{token}` at {}:{line}",
                path.display()
            );
            ok = false;
        }
    }
    if signature.contains("dyn AsCoreReport") {
        eprintln!("  FORBIDDEN dyn AsCoreReport at {}:{line}", path.display());
        ok = false;
    }
    ok
}

fn audit_solver_error_taxonomy() -> bool {
    let path = PathBuf::from("crates/loeres/src/error.rs");
    let src = match fs::read_to_string(&path) {
        Ok(src) => src,
        Err(e) => {
            eprintln!("  ! cannot read {}: {e}", path.display());
            return false;
        }
    };
    let mut ok = true;
    for (idx, raw) in src.lines().enumerate() {
        let code = strip_comment(raw);
        for token in FORBIDDEN_ERROR_TOKENS {
            if code.contains(token) {
                eprintln!(
                    "  FORBIDDEN SolverError taxonomy token `{token}` at {}:{}",
                    path.display(),
                    idx + 1
                );
                ok = false;
            }
        }
    }
    ok
}

fn audit_cluster_runtime_surface() -> bool {
    eprintln!("  scanning cluster baseline public surface for runtime leaks");
    let mut files = Vec::new();
    collect_ext(Path::new("crates/loeres-cluster/src"), "rs", &mut files);
    files.retain(|p| p.file_name().and_then(|n| n.to_str()) != Some("tests.rs"));
    let mut ok = true;
    for path in files {
        ok &= audit_cluster_public_file(&path);
    }
    ok
}

fn audit_cluster_public_file(path: &Path) -> bool {
    let src = match fs::read_to_string(path) {
        Ok(src) => src,
        Err(e) => {
            eprintln!("  ! cannot read {}: {e}", path.display());
            return false;
        }
    };
    let mut ok = true;
    let mut pending = String::new();
    let mut start_line = 0usize;
    for (idx, raw) in src.lines().enumerate() {
        let code = strip_comment(raw).trim();
        if pending.is_empty() {
            if starts_public_api(code) {
                pending.push_str(code);
                start_line = idx + 1;
            } else {
                continue;
            }
        } else {
            pending.push(' ');
            pending.push_str(code);
        }
        if code.ends_with(';') || code.ends_with('{') || code.contains(" where ") {
            ok &= audit_cluster_public_signature(path, start_line, &pending);
            pending.clear();
        }
    }
    if !pending.is_empty() {
        ok &= audit_cluster_public_signature(path, start_line, &pending);
    }
    ok
}

fn audit_cluster_public_signature(path: &Path, line: usize, signature: &str) -> bool {
    let mut ok = true;
    for token in CLUSTER_RUNTIME_TOKENS {
        if signature.contains(token) {
            eprintln!(
                "  FORBIDDEN cluster runtime public token `{token}` at {}:{line}",
                path.display()
            );
            ok = false;
        }
    }
    ok
}

const CLUSTER_RUNTIME_TOKENS: &[&str] = &[
    "tokio::",
    "rayon::",
    "Runtime",
    "Handle",
    "JoinHandle",
    "ThreadPool",
    "ThreadPoolBuilder",
];

#[cfg(test)]
mod tests {
    use super::empty_public_module_findings;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct ModuleFixture {
        root: PathBuf,
    }

    impl ModuleFixture {
        fn new() -> Self {
            static NEXT_FIXTURE: AtomicUsize = AtomicUsize::new(0);
            let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
                "../target/xtask-tests/public-api-empty-module-{}-{}",
                std::process::id(),
                NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&root).unwrap();
            Self { root }
        }

        fn write(&self, relative: &str, contents: &str) {
            let path = self.root.join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, contents).unwrap();
        }
    }

    impl Drop for ModuleFixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn an_empty_module_is_one_finding_naming_it() {
        let fixture = ModuleFixture::new();
        fixture.write("lib.rs", "pub mod empty;\n");
        fixture.write("empty.rs", "fn helper() {}\n");
        let findings = empty_public_module_findings(&[fixture.root.as_path()]);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(
            findings[0].contains("EMPTY PUBLIC MODULE"),
            "{}",
            findings[0]
        );
        assert!(findings[0].contains("empty"), "{}", findings[0]);
    }

    #[test]
    fn a_module_with_one_pub_fn_is_not_a_finding() {
        let fixture = ModuleFixture::new();
        fixture.write("lib.rs", "pub mod occupied;\n");
        fixture.write("occupied.rs", "pub fn do_it() {}\n");
        let findings = empty_public_module_findings(&[fixture.root.as_path()]);
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn the_nested_shape_resolves_against_the_declaring_files_own_directory() {
        let fixture = ModuleFixture::new();
        fixture.write("runtime.rs", "pub mod cancel;\n");
        fixture.write("runtime/cancel.rs", "pub struct Token;\n");
        let findings = empty_public_module_findings(&[fixture.root.as_path()]);
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn a_declaration_whose_file_is_missing_fails_closed() {
        let fixture = ModuleFixture::new();
        fixture.write("lib.rs", "pub mod ghost;\n");
        let findings = empty_public_module_findings(&[fixture.root.as_path()]);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].contains("ghost"), "{}", findings[0]);
    }
}
