//! `size-budget` — report current size-budget evidence (RFC 010).
//!
//! RFC 040 adds the measurement RFC 010 §3.7 asked for and nobody had built: `.text`
//! and `.rodata` of one concrete device-kernel instantiation, measured on the
//! instantiation's own compiled object (`cargo rustc -- --emit=obj`, then `size -A`),
//! summed by section-name prefix because the target emits one section per function
//! (`.text.<symbol>`, so an exact `.text` match reads zero — RFC 040 §2.3).
//!
//! **This figure is enforced**, not by an absolute ceiling but by a bounded delta from
//! a pinned baseline (RFC 040 §2.3): a move past the declared fraction fails. On a
//! legitimate failure — the measured code path genuinely changed — the remedy is to
//! re-measure, understand the change, and update `REFERENCE_BASELINE_BYTES` (and the
//! fraction, if that judgement changes too) in the same commit that moved it, exactly
//! as a `bench-baseline` pin is updated when counted work legitimately changes.
//!
//! The device **rlib** file size below measures something else — compiler metadata,
//! not code (RFC 040 §1): the device entry points are generic over `const N` and
//! `const M`, so a standalone build of the crate itself emits almost no instantiated
//! code. It stays, advisory, with its `.rmeta`-vs-other composition stated so it is
//! never mistaken for a device budget again.

use std::fs;
use std::path::{Path, PathBuf};

use super::util::{cargo, command_stdout};

const DEVICE_TARGET: &str = "thumbv7em-none-eabihf";
const REFERENCE_MANIFEST: &str = "device-size-reference/Cargo.toml";
const REFERENCE_CRATE: &str = "device_size_reference";
/// RFC 040 §2.1: a different instantiation is a different number, so this is a
/// declared constant of the measurement, printed beside every figure.
const REFERENCE_N: usize = 8;
const REFERENCE_M: usize = 4;
/// RFC 040 §2.3: the section-name prefixes this measurement sums.
const MEASURED_PREFIXES: &[&str] = &[".text", ".rodata"];
/// Pinned from the implementer's own measurement of this exact fixture (RFC 040
/// implementation review request A §2), not a guess: `.text + .rodata` of
/// `reference_solve`'s compiled object, release, `thumbv7em-none-eabihf`.
const REFERENCE_BASELINE_BYTES: u64 = 10_736;
/// Proposed by the implementer (RFC 040 §2.3 deliberately states no number, so the
/// architect rules on this one): loose enough to absorb ordinary rustc/LLVM codegen
/// jitter across a minor toolchain bump, tight enough that a real change to the
/// measured code path — even one small added branch, on a function this size —
/// moves the figure by more than this. Justified at length in the review request.
const REFERENCE_DELTA_FRACTION: f64 = 0.10;

pub fn run() -> bool {
    eprintln!("[size-budget] reporting device artifact and public type budgets");
    eprintln!("  mode: advisory, except the pinned reference-instantiation delta below");
    let build_ok = cargo(&[
        "build",
        "--target",
        DEVICE_TARGET,
        "-p",
        "loeres-device",
        "--no-default-features",
    ]);
    if !build_ok {
        eprintln!("[size-budget] FAIL");
        return false;
    }
    let rlib_ok =
        report_rlib_composition("target/thumbv7em-none-eabihf/debug/libloeres_device.rlib");
    let reference_ok = report_reference_instantiation();
    eprintln!("  enforced: SolverError <= 16 bytes (const assert in crates/loeres/src/error.rs)");
    eprintln!(
        "  advisory: DiagnosticSnapshot pinned by core size tests in crates/loeres/src/error/tests.rs"
    );
    let ok = rlib_ok && reference_ok;
    eprintln!("[size-budget] {}", if ok { "PASS" } else { "FAIL" });
    ok
}

/// Advisory (RFC 040 §2.4): the rlib file size, with its `.rmeta`-vs-other
/// composition measured and stated, so it is never read as a device code budget.
fn report_rlib_composition(path: &str) -> bool {
    let path_ref = Path::new(path);
    let total = match fs::metadata(path_ref) {
        Ok(meta) => meta.len(),
        Err(e) => {
            eprintln!("  unavailable: {path}: size unavailable ({e})");
            return false;
        }
    };
    match command_stdout("size", &["-A", path]) {
        Some(output) => {
            let (metadata, _other) = rlib_metadata_and_other_bytes(&output);
            let percent = if total == 0 {
                0.0
            } else {
                100.0 * metadata as f64 / total as f64
            };
            eprintln!(
                "  advisory: {path}: {total} bytes total, {metadata} ({percent:.0}%) is \
                 compiler metadata (.rmeta*) — not a device code budget; see \
                 device-size-reference below for one"
            );
        }
        None => {
            eprintln!(
                "  advisory: {path}: {total} bytes total (composition breakdown unavailable: \
                 `size -A` did not run)"
            );
        }
    }
    true
}

/// RFC 040 S1/S2: build the reference instantiation, find its own compiled object,
/// sum `.text*`/`.rodata*` with `size -A`, and enforce the pinned delta. An
/// unavailable tool, build or object **fails** — RFC 010 §3.7: an unavailable
/// required measurement must not pass silently.
fn report_reference_instantiation() -> bool {
    let build_ok = cargo(&[
        "rustc",
        "--locked",
        "--manifest-path",
        REFERENCE_MANIFEST,
        "--release",
        "--target",
        DEVICE_TARGET,
        "--",
        "--emit=obj",
    ]);
    if !build_ok {
        eprintln!("  unavailable: device-size-reference did not build");
        return false;
    }
    let Some(object) = find_reference_object() else {
        eprintln!("  unavailable: no compiled object found for device-size-reference");
        return false;
    };
    let Some(output) = command_stdout("size", &["-A", &object.to_string_lossy()]) else {
        eprintln!("  unavailable: `size -A` did not run (tool missing or object unreadable)");
        return false;
    };
    let total = sum_prefixed_sections(&output, MEASURED_PREFIXES);
    let delta = relative_delta(total, REFERENCE_BASELINE_BYTES);
    let within = delta <= REFERENCE_DELTA_FRACTION;
    eprintln!(
        "  enforced: device-size-reference (N={REFERENCE_N}, M={REFERENCE_M}, release, panic=abort): \
         .text+.rodata = {total} bytes (pinned {REFERENCE_BASELINE_BYTES}, delta {:.1}%, bound {:.0}%; tool: size -A)",
        delta * 100.0,
        REFERENCE_DELTA_FRACTION * 100.0
    );
    if !within {
        eprintln!(
            "  FAIL: device-size-reference moved {:.1}% from its pinned baseline, past the \
             {:.0}% bound. Re-measure, understand the change, and update \
             REFERENCE_BASELINE_BYTES in the same commit that moved it.",
            delta * 100.0,
            REFERENCE_DELTA_FRACTION * 100.0
        );
    }
    within
}

/// `|measured - baseline| / baseline`, as a fraction. `0.0` if `baseline` is `0`
/// (nothing to compare a delta against).
fn relative_delta(measured: u64, baseline: u64) -> f64 {
    if baseline == 0 {
        return 0.0;
    }
    (measured as f64 - baseline as f64).abs() / baseline as f64
}

/// The most recently built `device_size_reference-*.o` under the fixture's own
/// `target/`, since the hash in the file name changes between builds.
fn find_reference_object() -> Option<PathBuf> {
    let dir = Path::new("device-size-reference")
        .join("target")
        .join(DEVICE_TARGET)
        .join("release")
        .join("deps");
    let entries = fs::read_dir(dir).ok()?;
    entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("o"))
        .filter(|path| {
            path.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with(REFERENCE_CRATE))
        })
        .max_by_key(|path| fs::metadata(path).and_then(|m| m.modified()).ok())
}

/// One `(name, size)` pair per real section line in `size -A` output. A "real"
/// section line's name begins `.`; this excludes the `Total` line (whose size would
/// otherwise parse as a valid number too) and the member/header lines archive output
/// adds, without needing to match them by content.
fn parsed_sections(size_a_output: &str) -> impl Iterator<Item = (&str, u64)> {
    size_a_output.lines().filter_map(|line| {
        let mut parts = line.split_whitespace();
        let name = parts.next()?;
        if !name.starts_with('.') {
            return None;
        }
        let size: u64 = parts.next()?.parse().ok()?;
        Some((name, size))
    })
}

/// Sum every section whose name begins with one of `prefixes` (RFC 040 §2.3): the
/// target emits one section per function, so an exact name match would read zero.
fn sum_prefixed_sections(size_a_output: &str, prefixes: &[&str]) -> u64 {
    parsed_sections(size_a_output)
        .filter(|(name, _)| prefixes.iter().any(|p| name.starts_with(p)))
        .map(|(_, size)| size)
        .sum()
}

/// `(.rmeta* bytes, everything else)`, over every member of an archive's `size -A`
/// output (an rlib is an `ar` archive of several objects plus the `.rmeta`/
/// `.rmeta-link` metadata members; GNU `size` reports each member's sections).
fn rlib_metadata_and_other_bytes(size_a_output: &str) -> (u64, u64) {
    let mut metadata = 0u64;
    let mut other = 0u64;
    for (name, size) in parsed_sections(size_a_output) {
        if name.starts_with(".rmeta") {
            metadata += size;
        } else {
            other += size;
        }
    }
    (metadata, other)
}

#[cfg(test)]
mod tests {
    use super::{relative_delta, rlib_metadata_and_other_bytes, sum_prefixed_sections};

    #[test]
    fn sums_only_text_and_rodata_prefixed_sections() {
        let output = "\
object.o  :\n\
section                          size   addr\n\
.text                                0      0\n\
.text.reference_solve             9024      0\n\
.ARM.exidx.text.reference_solve      8      0\n\
.rodata.big_table                  512      0\n\
.comment                            45      0\n\
Total                            10873\n";
        assert_eq!(sum_prefixed_sections(output, &[".text", ".rodata"]), 9536);
    }

    #[test]
    fn an_exact_name_match_alone_would_read_zero() {
        // The reason the sum is by prefix, not by exact name: the target emits one
        // section per function (`.text.<symbol>`), so an exact `.text` match on its
        // own reads only the always-empty umbrella section.
        let output = ".text    0    0\n.text.reference_solve   9024   0\n";
        assert_eq!(
            output
                .lines()
                .find(|l| l.split_whitespace().next() == Some(".text"))
                .and_then(|l| l.split_whitespace().nth(1))
                .and_then(|s| s.parse::<u64>().ok()),
            Some(0)
        );
        assert_eq!(sum_prefixed_sections(output, &[".text"]), 9024);
    }

    #[test]
    fn an_empty_or_header_only_output_sums_to_zero() {
        assert_eq!(sum_prefixed_sections("", &[".text", ".rodata"]), 0);
        assert_eq!(
            sum_prefixed_sections("section size addr\nTotal 0\n", &[".text", ".rodata"]),
            0
        );
    }

    #[test]
    fn relative_delta_is_symmetric_around_the_baseline() {
        assert_eq!(relative_delta(11_000, 10_000), 0.1);
        assert_eq!(relative_delta(9_000, 10_000), 0.1);
        assert_eq!(relative_delta(10_000, 10_000), 0.0);
        assert_eq!(relative_delta(5, 0), 0.0);
    }

    #[test]
    fn rlib_composition_excludes_the_total_line_from_either_bucket() {
        // A naive "not .rmeta" filter would double-count the `Total` line, since its
        // size parses as a valid number too; `parsed_sections` excludes it by
        // requiring the name to start with `.`.
        let output = "\
lib.rmeta   (ex target/.../libloeres_device.rlib):\n\
section    size   addr\n\
.rmeta   23631      0\n\
Total    23631\n\
\n\
loeres_device-abc.o   (ex target/.../libloeres_device.rlib):\n\
section           size   addr\n\
.text                0      0\n\
.comment            45      0\n\
Total               45\n";
        assert_eq!(rlib_metadata_and_other_bytes(output), (23631, 45));
    }
}
