//! `panic-audit` — static panic/alloc/logging scan of the panic-averse crates.
//!
//! RFC 006 §6.2 requires `xtask` to scan the kernel and hot path for panicking
//! and non-`no_std` constructs. This gate scans the `no_std` production crates
//! (`loeres`, `loeres-backend-static`, `loeres-device`) — excluding colocated
//! `tests.rs` — for `unwrap` / `expect` / `panic!` / `todo!` / `unimplemented!`
//! and logging macros, on code lines only (comments stripped). The `std` crates
//! (`loeres-backend-std`, `loeres-cluster`) are out of scope; their governance
//! lands with RFC 010.

use std::fs;
use std::path::{Path, PathBuf};

/// `no_std`, panic-averse production crates whose `src/` is audited.
const AUDITED_CRATES: &[&str] = &["loeres", "loeres-backend-static", "loeres-device"];

/// Forbidden tokens on production code lines. `unwrap(` does not match the
/// non-panicking `unwrap_or(` / `unwrap_or_else(` family. `assert!` is not
/// listed: the const-assert dimension invariants are compile-time checks.
const FORBIDDEN: &[&str] = &[
    ".unwrap(",
    ".expect(",
    "panic!",
    "todo!",
    "unimplemented!",
    "dbg!",
    "println!",
    "eprintln!",
    "print!",
    "eprint!",
];

pub fn run() -> bool {
    eprintln!("[panic-audit] scanning no_std production crates (RFC 006 §6.2)");
    let mut ok = true;
    let mut scanned = 0usize;
    for krate in AUDITED_CRATES {
        let root = PathBuf::from("crates").join(krate).join("src");
        let mut files = Vec::new();
        collect_rs(&root, &mut files);
        for f in files {
            scanned += 1;
            ok &= audit_file(&f);
        }
    }
    eprintln!("  scanned {scanned} production file(s)");
    eprintln!("[panic-audit] {}", if ok { "PASS" } else { "FAIL" });
    ok
}

/// Recursively collect `.rs` files under `dir`, skipping colocated `tests.rs`.
fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rs(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs")
            && path.file_name().and_then(|n| n.to_str()) != Some("tests.rs")
        {
            out.push(path);
        }
    }
}

fn audit_file(path: &Path) -> bool {
    let src = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("  ! cannot read {}: {e}", path.display());
            return false;
        }
    };
    let mut ok = true;
    for (i, raw) in src.lines().enumerate() {
        let code = strip_comment(raw);
        for tok in FORBIDDEN {
            if code.contains(tok) {
                eprintln!("  FORBIDDEN `{tok}` at {}:{}", path.display(), i + 1);
                ok = false;
            }
        }
    }
    ok
}

/// Drop a doc/line comment so token scanning sees only code.
fn strip_comment(line: &str) -> &str {
    if line.trim_start().starts_with("//") {
        return "";
    }
    match line.find("//") {
        Some(idx) => &line[..idx],
        None => line,
    }
}
