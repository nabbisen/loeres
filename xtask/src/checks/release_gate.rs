//! `release-gate` — run every gate implemented at this milestone.
//!
//! Scaffolded gates (size-budget, …) are intentionally not run yet; they join
//! the aggregate as their owning milestones implement them. `panic-audit` is
//! implemented (RFC 006) and runs here.

use super::{basic, check_rfcs, no_std, panic_audit, zero_bleed};

pub fn run() -> bool {
    eprintln!("[release-gate] running Phase 0 gates");
    let results = [
        ("check", basic::run()),
        ("zero-bleed", zero_bleed::run()),
        ("no-std", no_std::run()),
        ("check-rfcs", check_rfcs::run()),
        ("panic-audit", panic_audit::run()),
    ];
    let ok = results.iter().all(|(_, r)| *r);
    eprintln!("[release-gate] summary:");
    for (name, r) in results {
        eprintln!("  {name}: {}", if r { "pass" } else { "FAIL" });
    }
    eprintln!("[release-gate] {}", if ok { "PASS" } else { "FAIL" });
    ok
}
