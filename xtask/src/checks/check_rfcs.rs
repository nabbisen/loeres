//! `check-rfcs` — core error/diagnostic source hygiene.
//!
//! Enforces the gates RFC 003 mandates against the `loeres` error and
//! diagnostic modules:
//!
//! * §6.2 — no `Display` impl, no `error::Error` impl, and no
//!   formatting/allocation tokens (`format!`, `String`, `Vec<`, `Box<`,
//!   `alloc::`) in core error code;
//! * §6.4 — every public error/diagnostic `enum` carries `#[non_exhaustive]`.
//!
//! Broader RFC-index and link-integrity checks are owned by RFC 010 and land
//! with it; this gate is intentionally scoped to the implemented core modules
//! above (RFC 003 error/diagnostic, RFC 014 solver, RFC 001 scalar).

use std::fs;

const CORE_MODULES: &[&str] = &[
    "crates/loeres/src/error.rs",
    "crates/loeres/src/diagnostic.rs",
    "crates/loeres/src/solver.rs",
    "crates/loeres/src/scalar.rs",
    "crates/loeres/src/scalar/primitive.rs",
];

/// Tokens forbidden on core error/diagnostic *code* lines (comments excluded).
const FORBIDDEN: &[&str] = &[
    "Display for",
    "error::Error",
    "format!",
    "String",
    "Vec<",
    "Box<",
    "alloc::",
];

pub fn run() -> bool {
    eprintln!("[check-rfcs] core module hygiene (RFC 003 §6.2/§6.4, RFC 014 §4.3, RFC 001 §6.2)");
    let mut ok = true;
    for rel in CORE_MODULES {
        match fs::read_to_string(rel) {
            Ok(src) => {
                ok &= audit_forbidden(rel, &src);
                ok &= audit_non_exhaustive(rel, &src);
            }
            Err(e) => {
                eprintln!("  ! cannot read {rel}: {e}");
                ok = false;
            }
        }
    }
    eprintln!("[check-rfcs] {}", if ok { "PASS" } else { "FAIL" });
    ok
}

/// Forbidden formatting / allocation tokens, ignoring comment and doc lines.
fn audit_forbidden(path: &str, src: &str) -> bool {
    let mut ok = true;
    for (i, raw) in src.lines().enumerate() {
        let code = strip_comment(raw);
        for tok in FORBIDDEN {
            if code.contains(tok) {
                eprintln!("  FORBIDDEN `{tok}` at {path}:{}", i + 1);
                ok = false;
            }
        }
    }
    ok
}

/// Public error/diagnostic enums must carry `#[non_exhaustive]` (RFC 003 §6.4).
fn audit_non_exhaustive(path: &str, src: &str) -> bool {
    let lines: Vec<&str> = src.lines().collect();
    let mut ok = true;
    for (i, line) in lines.iter().enumerate() {
        if !line.trim_start().starts_with("pub enum ") {
            continue;
        }
        // Walk back over the contiguous attribute / doc / blank block.
        let guarded = lines[..i]
            .iter()
            .rev()
            .take_while(|l| {
                let t = l.trim_start();
                t.starts_with('#') || t.starts_with("//") || t.is_empty()
            })
            .any(|l| l.contains("non_exhaustive"));
        if !guarded {
            eprintln!(
                "  MISSING `#[non_exhaustive]` on `{}` at {path}:{}",
                line.trim(),
                i + 1
            );
            ok = false;
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
