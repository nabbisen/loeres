//! Loeres repository automation.
//!
//! `xtask` is a `std` developer tool. It must never become a dependency of any
//! library crate (external design §1.1). It hosts the verification gates that
//! keep the server/edge boundary intact.
//!
//! Phase 0 implements the gates that the workspace skeleton can already satisfy
//! (`zero-bleed`, `no-std`, `check`) and registers the remaining gates from the
//! RFC 010 / roadmap §5.4 blueprint as scaffolds that land in later milestones.

mod checks;

use std::process::ExitCode;

const IMPLEMENTED: &[&str] = &[
    "zero-bleed",
    "no-std",
    "check",
    "check-rfcs",
    "release-gate",
];
const SCAFFOLD: &[&str] = &[
    "feature-matrix",
    "panic-audit",
    "size-budget",
    "check-public-api",
    "target-profiles",
    "link-audit",
    "unsafe-audit",
    "conformance",
];

fn main() -> ExitCode {
    let cmd = std::env::args().nth(1);
    let ok = match cmd.as_deref() {
        Some("zero-bleed") => checks::zero_bleed::run(),
        Some("no-std") => checks::no_std::run(),
        Some("check") => checks::basic::run(),
        Some("check-rfcs") => checks::check_rfcs::run(),
        Some("release-gate") => checks::release_gate::run(),
        Some(other) if SCAFFOLD.contains(&other) => checks::stubs::run(other),
        Some(other) => {
            eprintln!("xtask: unknown command `{other}`");
            usage();
            false
        }
        None => {
            usage();
            false
        }
    };
    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn usage() {
    eprintln!("usage: cargo xtask <command>\n");
    eprintln!("implemented:");
    for c in IMPLEMENTED {
        eprintln!("  {c}");
    }
    eprintln!("\nscaffolded (land in later milestones):");
    for c in SCAFFOLD {
        eprintln!("  {c}");
    }
}
