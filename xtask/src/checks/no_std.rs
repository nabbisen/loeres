//! `no-std` — build the edge crates for a bare-metal target.
//!
//! If any edge crate accidentally pulls `std` or `alloc`, the build fails on a
//! `none` target. This is the runtime complement to `zero-bleed`.

use super::util::cargo;

const TARGET: &str = "thumbv7em-none-eabihf";
const EDGE_CRATES: &[&str] = &["loeres", "loeres-backend-static", "loeres-device"];

pub fn run() -> bool {
    eprintln!("[no-std] building edge crates for {TARGET}");
    let mut ok = true;
    for krate in EDGE_CRATES {
        if !cargo(&[
            "build",
            "--target",
            TARGET,
            "-p",
            krate,
            "--no-default-features",
        ]) {
            eprintln!("  FAIL: {krate} does not build no_std/no-alloc");
            ok = false;
        }
    }
    eprintln!("[no-std] {}", if ok { "PASS" } else { "FAIL" });
    ok
}
