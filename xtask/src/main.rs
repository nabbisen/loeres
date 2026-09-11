//! Loeres repository automation.
//!
//! `xtask` is a `std` developer tool. It must never become a dependency of any
//! library crate (external design §1.1). It hosts the verification gates that
//! keep the server/edge boundary intact.
//!
//! RFC 010 defines `check` as the developer aggregate. Accepted RFC 019
//! separates `release-gate` into a complete, non-publishing candidate gate.

mod checks;

use std::process::ExitCode;

const IMPLEMENTED: &[&str] = &[
    "check",
    "release-gate",
    "check-rfcs",
    "zero-bleed",
    "no-std",
    "feature-matrix",
    "target-profiles",
    "panic-audit",
    "check-public-api",
    "size-budget",
    "supply-chain",
    "unsafe-audit",
    "conformance",
    "doc-currency",
    "review-evidence",
    "link-audit",
];

fn main() -> ExitCode {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let cmd = args.first().map(String::as_str);
    let ok = match cmd {
        Some("check") => checks::release_gate::run_developer(),
        Some("release-gate") => checks::release_gate::run_release(&args[1..]),
        Some("zero-bleed") => checks::zero_bleed::run(),
        Some("no-std") => checks::no_std::run(),
        Some("check-rfcs") => checks::check_rfcs::run(),
        Some("feature-matrix") => checks::feature_matrix::run(),
        Some("target-profiles") => checks::target_profiles::run(),
        Some("panic-audit") => checks::panic_audit::run(),
        Some("check-public-api") => checks::public_api::run(),
        Some("size-budget") => checks::size_budget::run(),
        Some("supply-chain") => checks::supply_chain::run(),
        Some("unsafe-audit") => checks::unsafe_audit::run(),
        Some("conformance") => checks::conformance::run(&args[1..]),
        Some("doc-currency") => checks::doc_currency::run(),
        Some("review-evidence") => checks::review_evidence::run(),
        Some("link-audit") => checks::link_audit::run(),
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
    eprintln!("usage: cargo xtask <command> [args]\n");
    eprintln!("implemented:");
    for c in IMPLEMENTED {
        eprintln!("  {c}");
    }
}
