# RFC 045 implementation handoff — Gates That Fire Where They Matter

**To:** implementer (mid-capability model)
**From:** architect
**Authorized by.** The owner accepted RFC 045 on 2026-10-29. Read
`rfcs/accepted/045-gates-that-fire-where-they-matter.md` in full. Architect reviews 106 and
107 hold the measurements.
**Base revision:** `main` at or after the acceptance commit.
**Target release:** `0.24.1`. **Nothing here bumps anything** — CI configuration and
documentation, no callable API.

**Two review requests, back to the architect.** **A** is S1 (the aggregate on push/PR), with
S4's cost reporting folded into it. **B** is S3 (the exemptions, documentation only).
`cargo xtask release-gate --intended-tag 0.24.1` is required in each.

**S2, the scheduled run, is withdrawn** — RFC 045 Amendment 1. Do not add a `schedule:`
trigger. Its two replacements are documentation and are listed in §2 below.

## 0. What the architect measured, so you can check rather than discover

Every one of these was verified on the `0.24.1` tree. Reproduce them; do not take them.

1. **`cargo xtask check` is three seconds warm.** `cargo test --workspace --all-features` is
   18s. The `ci` workflow is 3m 00s, `no-std` 58s, `msrv` 39s, and `release` — the same twenty
   gates, cold, with `mdbook` and `cargo-deny` installs — **6m 57s**.
2. **`mdbook` is not needed by `check`.** `doc-build` runs
   `cargo doc --workspace --all-features --no-deps` under `RUSTDOCFLAGS=-D warnings`. A full
   `cargo xtask check` invokes `mdbook` **zero** times; only `release-gate`'s clean-extraction
   step does. **This corrects review 107 §6, which guessed the opposite.**
3. **`size` is GNU binutils** (`GNU size 2.47` locally) and `size_budget` returns `false` when
   it is missing, so absence fails the suite rather than silently skipping.
4. **`GateKind::Advisory` does not mean unenforced.** It changes only the label when the gate
   passes; a `false` prints `FAIL`, and the suite's result is the conjunction of every gate.
   So `size-budget` is enforced. §3 item 4 exists because that name misleads.

### 0.1 The one premise most likely to be wrong

**The three-second figure is warm and local. CI is cold.** Do not put it in a commit message
or a CI comment as though it transfers. What is known to transfer is `release.yml`'s 6m 57s
for the same gates. S4 (§4) is how the real number gets recorded.

## 1. A — S1: the aggregate on every push and pull request

Replace `ci.yml`'s **two** individually-named xtask invocations — `zero-bleed` and
`supply-chain` — with `cargo xtask check`. (**Corrected:** this said *three* and named
`no-std`. `cargo xtask no-std` is only in `no-std.yml`, which stays untouched; architect
review 108 §3. The implementer found it with `grep` before writing code.)

Provisioning, measured (RFC 045 §3.1):

- **`thumbv7em-none-eabihf`** — needed by `no-std`, `target-profiles`, `size-budget`.
  `no-std.yml` already does this with `dtolnay/rust-toolchain`'s `targets:` key; copy that
  form rather than inventing one.
- **`cargo-deny` pinned at `0.20.2`** — needed by `supply-chain`. `ci.yml` already installs it
  in one job; it now has to be available to the job that runs the aggregate.
- **binutils `size`** — present on `ubuntu-latest`. If it turns out not to be, the gate fails
  closed and you will see it; **report that rather than working around it.**
- **`mdbook` — do not add it.** See §0 item 2.

**Keep `no-std.yml` and `msrv.yml` as they are.** `no-std` being covered twice is cheap, and
`msrv.yml` pins a different toolchain that the aggregate does not exercise.

**Your own push validates this**, which is a nice property: the first push of this slice is the
first run of the new workflow. If it fails, that is the slice telling you something, not a
flake — read it before re-running.

## 2. Instead of a scheduled run — two pull-based notes (RFC 045 Amendment 1)

The architect proposed a scheduled job and the owner rejected it, correctly. The twenty days
`main` spent failing its own `supply-chain` gate were a **red gate, not an exposure** — `tokio`
is optional behind a default-off feature with only `rt`, and `AsyncFd` needs `net`, which this
workspace never enables. The only window in which an advisory could reach a user is a release,
and `release.yml` already runs all twenty gates on every tag push. And a timer is least
reliable exactly when it would matter most: this repository did 18 commits on 2026-10-08, then
nothing for twenty days.

So **do not add a `schedule:` trigger.** Add these two instead, both documentation:

1. **In `rfcs/handoffs/release-0.24.0/finalization-checklist.md` §5** (and so in the template
   future cuts copy): advisory drift at cut time is **expected and is normal maintenance**, not
   an emergency. State the fix pattern — a **scoped** `cargo update -p <crate>`, never a bare
   one, then verify every changed lockfile line is a version string on one of this workspace's
   own crates (architect review 102).
2. **In this handoff's own §6 required-evidence section, as a standing line for future
   handoffs:** a `supply-chain` failure on a dependency you did not touch **is not yours**.
   Report it and fix it scoped. This is the one real cost the twenty-day window carried — an
   unrelated failure appearing mid-slice and reading as self-inflicted.

## 3. B — S3: write down what is deliberately exempt

Documentation only, in `docs/src/development.md`. An exemption that is written down is a
decision; one that is merely true is an accident waiting to be found by someone else. Record
all five from RFC 045 §3.3:

1. **The RFC 028 anchor is hand-recomputed and verified three ways at every cut, on purpose** —
   its value is that a machine did not do it. An exemption, not an omission.
2. **`no-std`, `panic-audit` and `check-public-api` cover only the edge crates** — `loeres`,
   `loeres-backend-static`, `loeres-device` — so `loeres-cluster` and `loeres-backend-std`
   have no equivalent assertion. State that, rather than leaving it to be inferred from a
   constant in `panic_audit.rs`.
3. **Two targets are asserted**: host and `thumbv7em-none-eabihf`. No other.
4. **`size-budget`'s name misleads; state the behaviour.** What is advisory is the *reported
   rlib figure*; the pinned reference-instantiation delta is **enforced**, and a missing `size`
   fails the suite. See §0 item 4 and verify it in `release_gate.rs`'s `GateKind` yourself.
5. **`bench`, `throughput` and `lp` are not gates** — standalone commands, so those figures
   are measured when someone chooses to, and `bench-baseline` is the only part that binds.

## 4. S4: report what it cost — folded into A

Not a separate request. Report in A:

- the `ci` workflow's duration **before and after** S1, from the Actions runs themselves;
- **and if the increase is material, say so plainly.** The fallback is to split the aggregate
  across the existing parallel jobs rather than to drop gates — and that decision needs a
  number, not an impression. Do not pre-emptively split it; measure first.

## 5. Non-scope

- **Architect review 106 §4's four new assertions** — an API-shape baseline, RFC-index section
  membership asserted against status, user-facing commands and version pins, and procedures
  verified rather than documented. Each needs its own design, and an API-shape baseline against
  a generic feature-gated surface is a real design problem (review 100 §8). **RFC 045 adds no
  new gate.** If you find yourself writing one, stop and return to the architect.
- Changing what any existing gate checks.
- Making the RFC 028 anchor automatic.
- Any library change, any version bump.

## 6. Required evidence

```text
cargo fmt --all -- --check          # run last, immediately before committing
cargo clippy --workspace --all-features --all-targets -- -D warnings
cargo test --workspace --all-features
cargo +1.85 check --workspace --all-features
cargo xtask check                   # 20 gates
cargo xtask release-gate --intended-tag 0.24.1      # must PASS
```

**Check `cargo xtask check`'s own exit status.** Never through a pipe: a pipeline's status is
the last command's, and the architect once pushed a commit on a failing gate exactly that way.

State in each request:

1. **Examples affected** — expected `none`.
2. **The `CHANGELOG.md` entry you added**, quoted.
3. For A: the workflow diff, the provisioning you added, and the **before/after `ci` duration**
   from the Actions runs.
4. For B: that you verified §0 item 4 against `GateKind` rather than taking it from this file.
5. **A `supply-chain` failure on a dependency you did not touch is not yours** (§2 item 2).
   Report it and fix it with a scoped `cargo update -p <crate>`; do not wonder what you broke
   and do not run a bare `cargo update`.
6. Anything in this handoff that does not match the tree. §0.1 names the premise the architect
   most expects to be wrong — the three-second figure does not transfer to CI, and if the real
   number is uncomfortable, that is the finding, not a failure.
