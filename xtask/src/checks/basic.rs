//! Host workspace type-check used by the aggregate gate.

use super::util::cargo;

pub fn run() -> bool {
    eprintln!("[host-check] cargo check --workspace --all-features");
    let ok = cargo(&["check", "--workspace", "--all-features"]);
    eprintln!("[host-check] {}", if ok { "PASS" } else { "FAIL" });
    ok
}
