//! `zero-bleed` — assert no forbidden internal crate dependency edge exists.
//!
//! Encodes the forbidden edges from roadmap §5.5. `std`/`alloc` leakage is not
//! checked here (those are sysroot crates and never appear in `cargo tree`);
//! the `no-std` gate covers them by building for a bare-metal target.

use super::util::cargo_stdout;

/// `(crate, forbidden direct-or-transitive dependency)` pairs.
const FORBIDDEN: &[(&str, &[&str])] = &[
    (
        "loeres",
        &[
            "loeres-backend-std",
            "loeres-backend-static",
            "loeres-cluster",
            "loeres-device",
        ],
    ),
    (
        "loeres-backend-static",
        &["loeres-backend-std", "loeres-cluster", "loeres-device"],
    ),
    ("loeres-device", &["loeres-backend-std", "loeres-cluster"]),
    ("loeres-backend-std", &["loeres-device", "loeres-cluster"]),
    ("loeres-cluster", &["loeres-device"]),
];

pub fn run() -> bool {
    eprintln!("[zero-bleed] checking forbidden dependency edges");
    let mut ok = true;
    for (krate, forbidden) in FORBIDDEN {
        let tree = match cargo_stdout(&[
            "tree",
            "-p",
            krate,
            "--edges",
            "normal,build",
            "--prefix",
            "none",
            "--no-dedupe",
        ]) {
            Some(t) => t,
            None => {
                eprintln!("  ! could not resolve dependency tree for {krate}");
                ok = false;
                continue;
            }
        };
        // Dependency crate names appear at the start of a line, e.g. "loeres v0.0.0".
        let deps: Vec<&str> = tree
            .lines()
            .filter_map(|l| l.split_whitespace().next())
            .filter(|name| *name != *krate)
            .collect();
        for bad in *forbidden {
            if deps.contains(bad) {
                eprintln!("  FORBIDDEN EDGE: {krate} -> {bad}");
                ok = false;
            }
        }
        if ok {
            eprintln!("  ok: {krate}");
        }
    }
    if ok {
        eprintln!("[zero-bleed] PASS");
    } else {
        eprintln!("[zero-bleed] FAIL");
    }
    ok
}
