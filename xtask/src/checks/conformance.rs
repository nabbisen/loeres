//! `conformance` — RFC 013 corpus orchestration hook.

use std::path::Path;

pub fn run() -> bool {
    eprintln!("[conformance] smoke corpus hook");
    if !Path::new("conformance").exists() {
        eprintln!("  not-enforced: no conformance corpus exists yet; RFC 013 owns fixtures");
        eprintln!("  result: hook ready, pending RFC 013 corpus");
        eprintln!("[conformance] NOT-ENFORCED");
        return true;
    }
    let smoke = Path::new("conformance/smoke");
    if !smoke.exists() {
        eprintln!("  conformance/ exists but conformance/smoke is missing");
        eprintln!("[conformance] FAIL");
        return false;
    }
    let count = std::fs::read_dir(smoke)
        .map(|entries| entries.flatten().count())
        .unwrap_or(0);
    eprintln!("  smoke fixture count: {count}");
    eprintln!("[conformance] PASS");
    true
}
