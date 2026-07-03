//! Aggregate RFC 010 release gate.

use super::{
    basic, check_rfcs, conformance, feature_matrix, link_audit, no_std, panic_audit, public_api,
    size_budget, target_profiles, unsafe_audit, zero_bleed,
};

pub fn run(name: &str) -> bool {
    eprintln!("[{name}] running RFC 010 aggregate gates");
    let results = [
        ("host-check", basic::run()),
        ("zero-bleed", zero_bleed::run()),
        ("no-std", no_std::run()),
        ("feature-matrix", feature_matrix::run()),
        ("target-profiles", target_profiles::run()),
        ("check-rfcs", check_rfcs::run()),
        ("check-public-api", public_api::run()),
        ("panic-audit", panic_audit::run()),
        ("size-budget", size_budget::run()),
        ("unsafe-audit", unsafe_audit::run()),
        ("conformance", conformance::run()),
        ("link-audit", link_audit::run()),
    ];
    let ok = results.iter().all(|(_, r)| *r);
    eprintln!("[{name}] summary:");
    for (name, r) in results {
        eprintln!("  {name}: {}", if r { "pass" } else { "FAIL" });
    }
    eprintln!("[{name}] {}", if ok { "PASS" } else { "FAIL" });
    ok
}
