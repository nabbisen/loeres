//! `check-public-api` — source-level edge public API scan (RFC 010).

use std::fs;
use std::path::{Path, PathBuf};

use super::util::{collect_ext, strip_comment};

const EDGE_SRC: &[&str] = &[
    "crates/loeres/src",
    "crates/loeres-backend-static/src",
    "crates/loeres-device/src",
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
    eprintln!("[check-public-api] {}", if ok { "PASS" } else { "FAIL" });
    ok
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
