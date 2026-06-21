//! `release-gate` — run every gate implemented at this milestone.
//!
//! Scaffolded gates (panic-audit, size-budget, …) are intentionally not run
//! yet; they join the aggregate as their owning milestones implement them.

use super::{basic, no_std, zero_bleed};

pub fn run() -> bool {
    eprintln!("[release-gate] running Phase 0 gates");
    let results = [
        ("check", basic::run()),
        ("zero-bleed", zero_bleed::run()),
        ("no-std", no_std::run()),
    ];
    let ok = results.iter().all(|(_, r)| *r);
    eprintln!("[release-gate] summary:");
    for (name, r) in results {
        eprintln!("  {name}: {}", if r { "pass" } else { "FAIL" });
    }
    eprintln!("[release-gate] {}", if ok { "PASS" } else { "FAIL" });
    ok
}
