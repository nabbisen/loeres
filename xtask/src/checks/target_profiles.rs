//! `target-profiles` — build/check RFC 010 interim target profiles.

use super::util::{cargo, command_stdout};

const DEVICE_TARGET: &str = "thumbv7em-none-eabihf";

pub fn run() -> bool {
    eprintln!("[target-profiles] checking RFC 010 interim target profiles");
    eprintln!("  note: RFC 011 may expand or rename the full profile taxonomy");
    if let Some(version) = command_stdout("rustc", &["--version"]) {
        eprintln!("  rustc: {}", version.trim());
    }
    let host = std::env::consts::ARCH;
    eprintln!("  host profile: cluster-linux-{host}");
    let host_ok = cargo(&["check", "-p", "loeres-cluster", "--all-features"]);
    eprintln!("  device profile: device-thumbv7em-hardfloat ({DEVICE_TARGET})");
    let device_ok = cargo(&[
        "build",
        "--target",
        DEVICE_TARGET,
        "-p",
        "loeres-device",
        "--no-default-features",
    ]);
    let ok = host_ok && device_ok;
    eprintln!("[target-profiles] {}", if ok { "PASS" } else { "FAIL" });
    ok
}
