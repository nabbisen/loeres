//! `check` — type-check the whole workspace on the host target.

use super::util::cargo;

pub fn run() -> bool {
    eprintln!("[check] cargo check --workspace --all-features");
    let ok = cargo(&["check", "--workspace", "--all-features"]);
    eprintln!("[check] {}", if ok { "PASS" } else { "FAIL" });
    ok
}
