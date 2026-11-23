# RFC 040 — the pinned number must not depend on `CARGO_INCREMENTAL`

**Author tier.** `architect`. **Date:** 2026-11-23.
**Governing text.** `rfcs/done/040-a-device-budget-worth-enforcing.md` — **unchanged, no
amendment.** §2.1 already holds that the measurement's dependencies are *"declared constants of
the measurement, printed beside every figure"*; this slice adds one that was always a dependency
and was never declared. Finding is architect review 128 F1.
**Base.** `c8aae2c1bbd92132f7cfb352ab853d7f2f189109` — the object-path correction, accepted, not
reopened.
**Release.** `0.25.0`. **No version bump** — no public surface. Run
`cargo xtask release-gate --intended-tag 0.25.0`.
**Not release-blocking, and scheduled before the cut anyway.** This environment and CI both leave
the variable unset, so the cut would pass without this. It is one line against a helper that
already exists, and a cut is when gates get run in a shell nobody checked.

One item, in one call site. **Do not touch `REFERENCE_BASELINE_BYTES`.**

## 1. The defect, measured

From a removed `device-size-reference/target` each time:

| environment | measured | delta | result |
| --- | --- | --- | --- |
| as the project leaves it | 10 728 bytes | 0.1% | PASS |
| `CARGO_INCREMENTAL=1` exported | **11 944 bytes** | **11.3%** | **FAIL** |

**Reproduce both before changing anything.** If the two numbers differ from these, report it —
they are the measurement this slice exists to stabilise.

**It is not the object-lookup fix regressing, and not a shadowing problem.** The incremental
objects are named `3e37iajqy14v611svbhog97qk.o` and so on, so `newest_object_named`'s crate-name
filter excludes them, and

```sh
find device-size-reference/target/thumbv7em-none-eabihf/release -name 'device_size_reference*.o' | wc -l
```

is **1** in both environments. The gate measures the right object; **incremental compilation
changes the code inside it** — different codegen-unit partitioning, so different inlining.

**Nothing in the repository pins the variable:**

```sh
grep -rn CARGO_INCREMENTAL .github/workflows .cargo/config.toml Cargo.toml xtask/src
```

is empty. Release profiles default it off, which is the only reason this has never been seen.

**Why this is worth fixing rather than documenting.** In that state the gate prints

```text
FAIL: device-size-reference moved 11.3% from its pinned baseline, past the 10% bound.
Re-measure, understand the change, and update REFERENCE_BASELINE_BYTES in the same commit that moved it.
```

— a confident instruction to re-pin a baseline that was correct. Re-pinning to an incremental
number then fails for everyone else. A false failure naming the right remedy is survivable; one
naming the wrong remedy turns a clean tree into a corrupted pin.

## 2. The correction

In `xtask/src/checks/size_budget.rs`, the reference build at `:116-125` currently goes through
`util::cargo(&[...])`.

**Use `util::cargo_with_env` (`util.rs:17-25`) with `CARGO_INCREMENTAL=0`.** It already exists, it
already prints the variables it sets — `cargo_display` at `util.rs:27-35` puts them before the
command — so the gate's own transcript will show the setting rather than leave it implicit. Its
signature takes `&[String]` and `&[(&str, String)]`, so the argument list needs owning; that is the
whole of the mechanical work.

**Do not** add an env-var parameter to `util::cargo`, and do not write a third runner. One helper
per shape, and the right one is already written.

**Then declare it where the number is printed.** The `enforced:` line reads

```text
enforced: device-size-reference (N=8, M=4, release, panic=abort): .text+.rodata = 10728 bytes (...)
```

Add `incremental=0` to that parenthesised list. RFC 040 §2.1's rule is that a thing the number
depends on is *a declared constant of the measurement, printed beside every figure*, and this is
one. Update the module doc's description of the measurement in the same commit.

**Three things not to do:**

1. **Do not change `REFERENCE_BASELINE_BYTES` or `REFERENCE_DELTA_FRACTION`.** 10,736 and 10% are
   correct; this slice makes the measured value reach them reliably.
2. **Do not set the variable globally** — not in `.cargo/config.toml`, not in a workflow. The
   measurement is what needs pinning, not everyone's build. A global setting would also slow every
   developer's loop for one gate's benefit.
3. **Do not weaken the bound** to absorb the difference. An 11.3% move is a real difference in
   generated code; the answer is to measure one configuration, not to widen the tolerance until
   two configurations both pass.

## 3. The test

The build invocation is not unit-testable — it runs cargo. **Do not build a fixture that fakes it.**

What is testable is that the declared-constants string carries the new constant: if the
`enforced:` line is assembled by a pure function, add one assertion that its output contains
`incremental=0` alongside `N=8` and `panic=abort`. **If it is assembled inline in the reporting
function, say so and skip the test rather than refactoring the reporting path for one assertion** —
that refactor is bigger than this slice and I would rather review it on its own.

## 4. Verification

1. `rm -rf device-size-reference/target`, then `cargo xtask size-budget` → **PASS**, and quote the
   `enforced:` line showing `incremental=0` and the measured bytes.
2. **The same command with `CARGO_INCREMENTAL=1` exported** → **PASS, with the same byte count**.
   That is the point of the slice: quote both lines side by side, and if they differ the fix did
   not take.
3. `cargo xtask check` → **PASS, 20 gates**, on the gate's own exit status, never through a pipe.
   Note that `size-budget` prints `advisory baseline reported` in the summary on success
   (`release_gate.rs:583-590`), which is its classification string and not a failure.
4. `cargo xtask release-gate --intended-tag 0.25.0` → PASS, whole stream to a file.
5. **Confirm the absence of the object still fails:** `mv` the found object aside, run the gate,
   quote the `unavailable` line and the FAIL, then move it back. The object-path slice established
   this and the env change must not have loosened it.
6. `rustc -vV` and `cargo -V` quoted, as the previous slice did — the environment is part of the
   result in this corner of the tree.
7. The review request at
   `.git-exclude/review-request/040-size-budget-incremental-invariance/REVIEW_REQUEST.md`.

**Not yours:** `REFERENCE_BASELINE_BYTES`, the delta fraction, the advisory rlib figure, the
`cargo::unused_dependencies` warning (review 127 E2, recorded for `0.25.1`), the question of
whether `rust-toolchain.toml`'s moving `channel = "stable"` is the right pin (review 128 §2.1 —
recorded, unowned, and not a handoff item), RFC 054's outstanding slice, moving any RFC to
`rfcs/done/`, the tag, the push, and publication.
