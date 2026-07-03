//! Aggregate RFC 010 release gate.

use super::{
    basic, check_rfcs, conformance, feature_matrix, link_audit, no_std, panic_audit, public_api,
    size_budget, target_profiles, unsafe_audit, zero_bleed,
};

pub fn run(name: &str) -> bool {
    eprintln!("[{name}] running RFC 010 aggregate gates");
    let results = [
        ("host-check", GateKind::Enforced, basic::run()),
        ("zero-bleed", GateKind::Enforced, zero_bleed::run()),
        ("no-std", GateKind::Enforced, no_std::run()),
        ("feature-matrix", GateKind::Enforced, feature_matrix::run()),
        (
            "target-profiles",
            GateKind::Enforced,
            target_profiles::run(),
        ),
        ("check-rfcs", GateKind::Enforced, check_rfcs::run()),
        ("check-public-api", GateKind::Enforced, public_api::run()),
        ("panic-audit", GateKind::Enforced, panic_audit::run()),
        ("size-budget", GateKind::Advisory, size_budget::run()),
        ("unsafe-audit", GateKind::Enforced, unsafe_audit::run()),
        ("conformance", GateKind::Hook, conformance::run()),
        ("link-audit", GateKind::Enforced, link_audit::run()),
    ];
    let ok = results.iter().all(|(_, _, r)| *r);
    eprintln!("[{name}] summary:");
    for (name, kind, r) in results {
        eprintln!("  {name}: {}", kind.status(r));
    }
    eprintln!("[{name}] {}", if ok { "PASS" } else { "FAIL" });
    ok
}

#[derive(Copy, Clone)]
enum GateKind {
    Enforced,
    Advisory,
    Hook,
}

impl GateKind {
    fn status(self, ok: bool) -> &'static str {
        match (self, ok) {
            (Self::Enforced, true) => "pass",
            (Self::Enforced, false) => "FAIL",
            (Self::Advisory, true) => "advisory baseline reported",
            (Self::Advisory, false) => "FAIL",
            (Self::Hook, true) => "not-enforced hook ready",
            (Self::Hook, false) => "FAIL",
        }
    }
}
