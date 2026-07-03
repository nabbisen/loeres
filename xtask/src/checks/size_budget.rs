//! `size-budget` — report current size-budget evidence (RFC 010).

use std::fs;
use std::path::Path;

use super::util::cargo;

const DEVICE_TARGET: &str = "thumbv7em-none-eabihf";

pub fn run() -> bool {
    eprintln!("[size-budget] reporting device artifact and public type budgets");
    let build_ok = cargo(&[
        "build",
        "--target",
        DEVICE_TARGET,
        "-p",
        "loeres-device",
        "--no-default-features",
    ]);
    if !build_ok {
        eprintln!("[size-budget] FAIL");
        return false;
    }
    report_file_size("target/thumbv7em-none-eabihf/debug/libloeres_device.rlib");
    eprintln!("  SolverError: <= 16 bytes (const assert in crates/loeres/src/error.rs)");
    eprintln!(
        "  DiagnosticSnapshot: pinned by core size tests in crates/loeres/src/error/tests.rs"
    );
    eprintln!("[size-budget] PASS");
    true
}

fn report_file_size(path: &str) {
    let path = Path::new(path);
    match fs::metadata(path) {
        Ok(meta) => eprintln!("  {}: {} bytes", path.display(), meta.len()),
        Err(e) => eprintln!("  {}: size unavailable ({e})", path.display()),
    }
}
