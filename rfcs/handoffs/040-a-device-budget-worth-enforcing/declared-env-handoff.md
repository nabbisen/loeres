# RFC 040 — the measurement declares its environment

**Author tier.** `architect`. **Date:** 2026-11-23.
**Governing text.** `rfcs/done/040-a-device-budget-worth-enforcing.md` — **unchanged, no
amendment** (third time). §2.1 already holds that a dependency of the number is *"a declared
constant of the measurement, printed beside every figure"*; this slice names the ones nobody had
named. Finding is architect review 129 F2.
**Base.** `4a2bf7f4e86675899a8640acdfe90020f8b8f404` — F1's `CARGO_INCREMENTAL` fix, accepted, not
reopened and not undone (§2).
**Release.** `0.25.0`. **No version bump.** Run `cargo xtask release-gate --intended-tag 0.25.0`.
**Not release-blocking** — this environment and CI leave all five variables unset — **and it is
the last change this gate gets before the cut.** If a sixth door appears afterwards it waits: a
gate edited four times in one day is its own risk.

## 0. The five doors, measured

Each from a removed `device-size-reference/target`, on your base revision. **Reproduce the three
marked "wrong number" before changing anything**; if any differs, report it rather than proceeding.

| ambient variable | measured | result |
| --- | --- | --- |
| *(none)* | 10 728 bytes, 0.1% | PASS |
| `CARGO_INCREMENTAL=1` | 10 728 bytes | PASS — closed by your previous slice |
| `RUSTFLAGS=-C opt-level=s` | **4 854 bytes, 54.8%** | FAIL, **wrong number** |
| `CARGO_PROFILE_RELEASE_OPT_LEVEL=z` | **4 914 bytes, 54.2%** | FAIL, **wrong number** |
| `CARGO_PROFILE_RELEASE_LTO=true` | `size -A` cannot read the emitted object | FAIL *(closed)* |
| `CARGO_TARGET_DIR=<elsewhere>` | object outside the searched tree | FAIL *(closed)* |

The three that yield a number are the dangerous ones: the gate's failure message tells the reader
to re-pin `REFERENCE_BASELINE_BYTES`, and a 54% move reads as "something really changed".

**Why this is not four more one-line fixes.** `CARGO_PROFILE_<name>_<key>` is an open family; an
exact list of it cannot be complete, and enumerating hazards as they are found is the
incident-driven list RFC 047 §1.1 criticises. **Declare the environment and remove the rest.**

## 1. The correction

### 1.1 A named helper, and `cargo_with_env` is left alone

Add to `xtask/src/checks/util.rs`:

```rust
pub fn cargo_in_declared_env(args: &[String], envs: &[(&str, String)]) -> bool
```

It removes the build-configuration variables below from the child's environment, then applies
`envs`, then runs cargo exactly as `cargo_with_env` does.

**Do not change `cargo_with_env`'s signature.** It has one other caller
(`target_profiles.rs:136`) and widening it would spread this change for no gain. Two helpers with
different purposes is correct here; a third general-purpose runner would not be.

### 1.2 What is cleared, and what is deliberately not

**Cleared — exact names:** `RUSTFLAGS`, `CARGO_ENCODED_RUSTFLAGS`, `CARGO_BUILD_RUSTFLAGS`,
`CARGO_TARGET_DIR`, `CARGO_BUILD_TARGET`.

**Cleared — prefix:** every variable whose name begins `CARGO_PROFILE_`. A prefix, not a list,
because the family is open (§0).

**Set:** `CARGO_INCREMENTAL=0`, explicitly, as your previous slice does. It stays set rather than
merely cleared, and stays in the printed `enforced:` line, because it is a **declared constant of
the measurement** and not only a hazard. Clearing it would give the same result today by relying on
cargo's release default; declaring it says what the number depends on.

**Not cleared, and this is a decision rather than an oversight:** `RUSTC`, `RUSTC_WRAPPER`, `PATH`,
`CARGO_HOME`, `RUSTUP_*`. Those select **which compiler** runs, not its flags, and which compiler
produced a number is the question architect review 128 §2.1 records as unowned. A gate fix must not
answer it quietly by pinning `RUSTC`. **State this in the helper's doc comment**, so the omission
reads as bounded rather than forgotten.

### 1.3 Print the environment the gate constructed

Before the build line, emit one line naming the policy and how many variables were actually
removed, for example:

```text
  environment: cleared RUSTFLAGS, CARGO_ENCODED_RUSTFLAGS, CARGO_BUILD_RUSTFLAGS, CARGO_TARGET_DIR, CARGO_BUILD_TARGET, CARGO_PROFILE_* (2 removed); set CARGO_INCREMENTAL=0
```

**Print the policy whether or not anything was present**, with the count of what was. A transcript
that shows the environment was constructed is the evidence; one that only shows it when something
was wrong leaves a reader guessing on the normal run.

## 2. What must not change

1. **`REFERENCE_BASELINE_BYTES` (10 736) and `REFERENCE_DELTA_FRACTION` (10%).** Correct. This
   slice makes the measured value reach them in more environments, not fewer.
2. **F1's `CARGO_INCREMENTAL=0`**, which stays set and stays printed (§1.2).
3. **The `unavailable → false` paths.** An enforced measurement that cannot be taken is still a
   failure. This slice removes causes of false failure; it does not make failure tolerable.
4. **`target_profiles.rs`.** Its builds pass rustflags explicitly, and it asserts *that a build
   succeeds* rather than pinning a number, so ambient flags change whether it compiles rather than
   invalidating a measurement. Lower stakes, out of scope, and **do not widen the slice to it** —
   say so if you disagree.
5. **The object search** from the slice before last.

## 3. Tests

The removal policy is pure and must be tested as such: a function that, given an iterator of
environment-variable **names**, returns those to remove. Four assertions on literals:

1. `RUSTFLAGS` → removed;
2. `CARGO_PROFILE_RELEASE_LTO` and `CARGO_PROFILE_DEV_DEBUG` → both removed, proving the prefix
   rather than a list;
3. `CARGO_HOME`, `PATH`, `RUSTC` → **kept**, proving §1.2's boundary;
4. `CARGO_INCREMENTAL` → not in the removal set, because it is set rather than cleared.

## 4. Verification

1. `cargo xtask check` → **PASS, 20 gates**, on the gate's own exit status, never through a pipe.
2. **Re-run all five rows of §0's table**, each from a removed `device-size-reference/target`, and
   quote the measured line for each. **The expected result is `10728` bytes and PASS in all six
   environments** including the two that previously failed closed. If `CARGO_PROFILE_RELEASE_LTO`
   or `CARGO_TARGET_DIR` still fails, say so with the message — one of them may need a sixth
   variable and I would rather know than have it worked around.
3. `cargo xtask release-gate --intended-tag 0.25.0` → PASS, whole stream to a file.
4. **The absence of the object still fails:** `mv` it aside, quote the `unavailable` line and the
   FAIL, move it back. Third slice in a row to check this, and the reason is that each of these
   slices touches the path between the build and the measurement.
5. `rustc -vV` and `cargo -V` quoted.
6. The review request at
   `.git-exclude/review-request/040-size-budget-declared-env/REVIEW_REQUEST.md`, with §0's figures
   as you reproduced them and §4 item 2's six lines.

**Not yours:** `RUSTC` and the toolchain-pin question (architect review 129 §4 — owner-facing,
unowned, and deliberately not settled here), the advisory rlib figure, the
`cargo::unused_dependencies` warning (review 127 E2, `0.25.1`), RFC 054's outstanding slice, moving
any RFC to `rfcs/done/`, the tag, the push, and publication.
