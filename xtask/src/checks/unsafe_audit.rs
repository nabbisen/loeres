//! `unsafe-audit` — source-level unsafe/FFI scan (RFC 010).

use std::path::Path;

use super::util::{collect_ext, strip_comment};

const TOKENS: &[&str] = &[
    "unsafe ",
    "unsafe{",
    "unsafe(",
    "unsafe impl",
    "extern \"",
    "*const ",
    "*mut ",
    "from_raw",
    "as_ptr(",
    "as_mut_ptr(",
];

pub fn run() -> bool {
    eprintln!("[unsafe-audit] scanning Rust sources for unsafe/FFI markers");
    let mut files = Vec::new();
    collect_ext(Path::new("crates"), "rs", &mut files);
    collect_ext(Path::new("xtask"), "rs", &mut files);
    files.retain(|p| {
        !p.components()
            .any(|c| c.as_os_str() == "target" || c.as_os_str() == ".git-exclude")
    });
    let mut findings = 0usize;
    let mut ok = true;
    for path in files {
        let src = match std::fs::read_to_string(&path) {
            Ok(src) => src,
            Err(e) => {
                eprintln!("  ! cannot read {}: {e}", path.display());
                ok = false;
                continue;
            }
        };
        for (idx, raw) in src.lines().enumerate() {
            let code = strip_comment(raw);
            let code = strip_string_literals(code);
            for token in TOKENS {
                if code.contains(token) {
                    findings += 1;
                    eprintln!("  {token} at {}:{}", path.display(), idx + 1);
                }
            }
        }
    }
    eprintln!("  findings: {findings}");
    eprintln!("[unsafe-audit] {}", if ok { "PASS" } else { "FAIL" });
    ok
}

fn strip_string_literals(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut in_string = false;
    let mut escaped = false;
    for ch in line.chars() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            out.push(' ');
        } else if ch == '"' {
            in_string = true;
            out.push(' ');
        } else {
            out.push(ch);
        }
    }
    out
}
