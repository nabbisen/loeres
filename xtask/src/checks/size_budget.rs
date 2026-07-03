//! `size-budget` — report current size-budget evidence (RFC 010).

use std::fs;
use std::path::Path;

use super::util::cargo;

const DEVICE_TARGET: &str = "thumbv7em-none-eabihf";

pub fn run() -> bool {
    eprintln!("[size-budget] reporting device artifact and public type budgets");
    eprintln!("  mode: advisory baseline unless a measurement is unavailable");
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
    let size_ok = report_file_size("target/thumbv7em-none-eabihf/debug/libloeres_device.rlib");
    eprintln!("  enforced: SolverError <= 16 bytes (const assert in crates/loeres/src/error.rs)");
    eprintln!(
        "  advisory: DiagnosticSnapshot pinned by core size tests in crates/loeres/src/error/tests.rs"
    );
    eprintln!(
        "[size-budget] {}",
        if size_ok {
            "ADVISORY (no RFC 010 byte threshold)"
        } else {
            "FAIL"
        }
    );
    size_ok
}

fn report_file_size(path: &str) -> bool {
    let path = Path::new(path);
    match fs::metadata(path) {
        Ok(meta) => {
            eprintln!(
                "  advisory: {}: {} bytes (threshold pending owner RFC)",
                path.display(),
                meta.len()
            );
            true
        }
        Err(e) => {
            eprintln!("  unavailable: {}: size unavailable ({e})", path.display());
            false
        }
    }
}
