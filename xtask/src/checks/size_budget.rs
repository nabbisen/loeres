//! `size-budget` — report current size-budget evidence (RFC 010).
//!
//! RFC 040 adds the measurement RFC 010 §3.7 asked for and nobody had built: `.text`
//! and `.rodata` of one concrete device-kernel instantiation, measured on the
//! instantiation's own compiled object (`cargo rustc -- --emit=obj`, then `size -A`),
//! summed by section-name prefix because the target emits one section per function
//! (`.text.<symbol>`, so an exact `.text` match reads zero — RFC 040 §2.3). In S1 this
//! is advisory; S2 pins a baseline and bounds the delta.
//!
//! The device **rlib** file size below measures something else — compiler metadata,
//! not code (RFC 040 §1): the device entry points are generic over `const N` and
//! `const M`, so a standalone build of the crate itself emits almost no instantiated
//! code, and most of the rlib's bytes are `.rmeta`. It stays, advisory, so it is never
//! mistaken for a device budget again.

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

pub fn run() -> bool {
    eprintln!("[size-budget] reporting device artifact and public type budgets");
    eprintln!("  mode: advisory baseline unless a measurement is unavailable");
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
    let rlib_ok = report_file_size("target/thumbv7em-none-eabihf/debug/libloeres_device.rlib");
    let reference_ok = report_reference_instantiation();
    eprintln!("  enforced: SolverError <= 16 bytes (const assert in crates/loeres/src/error.rs)");
    eprintln!(
        "  advisory: DiagnosticSnapshot pinned by core size tests in crates/loeres/src/error/tests.rs"
    );
    let ok = rlib_ok && reference_ok;
    eprintln!(
        "[size-budget] {}",
        if ok {
            "ADVISORY (no RFC 010 byte threshold)"
        } else {
            "FAIL"
        }
    );
    ok
}

fn report_file_size(path: &str) -> bool {
    let path = Path::new(path);
    match fs::metadata(path) {
        Ok(meta) => {
            eprintln!(
                "  advisory: {}: {} bytes (threshold pending owner RFC)",
                path.display(),
                meta.len()
            );
            true
        }
        Err(e) => {
            eprintln!("  unavailable: {}: size unavailable ({e})", path.display());
            false
        }
    }
}

/// RFC 040 S1: build the reference instantiation, find its own compiled object, and
/// sum `.text*`/`.rodata*` with `size -A`. An unavailable tool, build or object
/// **fails** — RFC 010 §3.7: an unavailable required measurement must not pass
/// silently.
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
    eprintln!(
        "  advisory: device-size-reference (N={REFERENCE_N}, M={REFERENCE_M}, release, panic=abort): \
         .text+.rodata = {total} bytes (tool: size -A; threshold pending RFC 040 S2)"
    );
    true
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

/// Sum every section whose name begins with one of `prefixes` (RFC 040 §2.3): the
/// target emits one section per function, so an exact name match would read zero.
fn sum_prefixed_sections(size_a_output: &str, prefixes: &[&str]) -> u64 {
    size_a_output
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let name = parts.next()?;
            let size: u64 = parts.next()?.parse().ok()?;
            prefixes.iter().any(|p| name.starts_with(p)).then_some(size)
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::sum_prefixed_sections;

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
}
