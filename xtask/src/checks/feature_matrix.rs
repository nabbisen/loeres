//! `feature-matrix` — compile canonical feature profiles (RFC 010).

use super::util::cargo;

pub fn run() -> bool {
    eprintln!("[feature-matrix] compiling canonical feature profiles");
    let profiles: &[(&str, &[&str])] = &[
        (
            "core-min",
            &["check", "-p", "loeres", "--no-default-features"],
        ),
        (
            "static-min",
            &[
                "check",
                "-p",
                "loeres-backend-static",
                "--no-default-features",
            ],
        ),
        (
            "device-min",
            &["check", "-p", "loeres-device", "--no-default-features"],
        ),
        (
            "cluster-default",
            &["check", "-p", "loeres-cluster", "--no-default-features"],
        ),
        (
            "cluster-ffi",
            &[
                "check",
                "-p",
                "loeres-cluster",
                "--no-default-features",
                "--features",
                "ffi-gateway",
            ],
        ),
    ];
    let mut ok = true;
    for (name, args) in profiles {
        eprintln!("  profile: {name}");
        ok &= cargo(args);
    }
    eprintln!("[feature-matrix] {}", if ok { "PASS" } else { "FAIL" });
    ok
}
