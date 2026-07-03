//! `feature-matrix` — compile canonical feature profiles (RFC 010).

use std::fs;

use super::util::cargo;

pub fn run() -> bool {
    eprintln!("[feature-matrix] compiling canonical feature profiles");
    let mut profiles: Vec<(&str, &[&str])> = vec![
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
    ];
    if cluster_feature_exists("ffi-gateway") {
        eprintln!("  conditional profile enabled: cluster-ffi (ffi-gateway feature exists)");
        profiles.push((
            "cluster-ffi",
            &[
                "check",
                "-p",
                "loeres-cluster",
                "--no-default-features",
                "--features",
                "ffi-gateway",
            ],
        ));
    } else {
        eprintln!("  conditional profile skipped: cluster-ffi (ffi-gateway feature absent)");
    }
    let mut ok = true;
    for (name, args) in &profiles {
        eprintln!("  profile: {name}");
        ok &= cargo(args);
    }
    eprintln!("[feature-matrix] {}", if ok { "PASS" } else { "FAIL" });
    ok
}

fn cluster_feature_exists(feature: &str) -> bool {
    let Ok(manifest) = fs::read_to_string("crates/loeres-cluster/Cargo.toml") else {
        return false;
    };
    manifest
        .lines()
        .any(|line| line.trim_start().starts_with(&format!("{feature} =")))
}
