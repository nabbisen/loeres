# RFC 046 implementation handoff — Pinned Tools Installed By Identity, Not By Rebuild

**To:** implementer (mid-capability model)
**From:** architect
**Authorized by.** The owner accepted RFC 046 on 2026-10-29, including §3.3 item 3 (`mdbook`)
and §3.5's landing order. Read `rfcs/accepted/046-pinned-tools-installed-by-identity.md` in
full. Architect review 108 §2 holds the measurement that scoped it.
**Base revision:** `main` at or after the acceptance commit.
**Target release:** `0.24.1`. **Nothing here bumps anything** — CI configuration and
documentation, no callable API, no library change.

**Two review requests, back to the architect, in this order.**

- **A** — the composite action, its pin table, and `ci.yml`'s call site (§3.3 item 1), plus the
  `docs/src/development.md` exemption note (§4). The push that runs `ci` is what proves the
  action works; that is why A exists separately.
- **B** — `release.yml`'s two call sites (§3.3 items 2 and 3). **Open B only after A's push is
  green.** Nothing in B can be exercised without a tag, so do not create one; its verification
  is the next cut's `release-gate` run, which is the architect's and the owner's to schedule.

`cargo xtask release-gate --intended-tag 0.24.1` is required in each.

## 0. What the architect measured and verified, so you can check rather than discover

Reproduce these; do not take them.

1. **The step timings**, from this repository's own runs. `release.yml` / `release-gate`, run
   `37722084985` (the 0.24.0 cut): `mdbook` install **1m 19s** (03:19:07→03:20:26),
   `cargo-deny` install **1m 53s** (03:20:26→03:22:19), `release-gate` itself 2m 53s, whole job
   **6m 20s**. `ci.yml` / `gates`, run `40792044395`: `cargo-deny` **1m 56s** of **2m 47s**.
2. **Both assets exist and both hashes are the architect's own.** Downloaded and recomputed with
   `sha256sum`, not copied from upstream prose. `cargo-deny`'s matches its upstream `.sha256`
   sidecar and the byte count the GitHub API reports; **mdBook publishes no sidecar at all**.
3. **Both binaries are `ELF 64-bit LSB pie executable, x86-64, static-pie linked`**, confirmed
   with `file` on the extracted files. No dynamic loader, no libc dependency — which is why the
   announced `ubuntu-latest` → Ubuntu 26 migration cannot touch them.
4. **The archive layouts differ.** `cargo-deny`'s tarball contains
   `cargo-deny-0.20.2-x86_64-unknown-linux-musl/{cargo-deny,README.md,LICENSE-APACHE,LICENSE-MIT}`
   — the binary is one directory deep. `mdbook`'s contains exactly `mdbook`, at the root. Locate
   by name; do not hard-code either path.
5. **The architect did not execute either downloaded binary locally.** Identity and ELF type
   were verified statically. First execution is in CI.

### 0.1 The premises most likely to be wrong

- **The predicted durations are arithmetic, not measurements.** "Under a minute" for `gates` and
  "about 3m 18s" for `release-gate` are step-time subtraction. S5 exists because they are
  predictions. **If the real number is worse than predicted, that is the finding** — report it;
  do not reconcile it.
- **`sha256sum` is assumed present on `ubuntu-latest`.** It is coreutils, so it should be, but
  check rather than assume: if it is absent the step must fail loudly, never skip the
  comparison. A verification step that can silently not verify is worse than no step.

## 1. A — S1 and S2: the composite action and its pin table

Create `.github/actions/install-pinned-tool/action.yml`. One input, `tool`, accepted values
`cargo-deny` and `mdbook`. RFC 046 §3.1 explains why the pins live here and not at the call
sites: `cargo-deny` is installed by two workflows, and a pin that exists twice will eventually
disagree with itself.

**The pin table** (RFC 046 §3.2), to be reproduced exactly:

| tool | url | bytes | SHA-256 |
| --- | --- | ---: | --- |
| `cargo-deny` | `https://github.com/EmbarkStudios/cargo-deny/releases/download/0.20.2/cargo-deny-0.20.2-x86_64-unknown-linux-musl.tar.gz` | `4936832` | `9f12ed4c49936e09b48bf862b595cde2fe64fcbd9d74dfacac6131ca824c8d5f` |
| `mdbook` | `https://github.com/rust-lang/mdBook/releases/download/v0.5.4/mdbook-v0.5.4-x86_64-unknown-linux-musl.tar.gz` | `4944048` | `5222beabd3e37dc5be0d18ff99b79058469354db5c220153a1b92db5ba12be89` |

An unrecognised `tool` value must fail the step, not fall through to an empty URL.

### 1.1 The order, which is the whole obligation

1. Download into a **fresh, empty** directory under `$RUNNER_TEMP` — never a shared one.
   `curl --proto '=https' --tlsv1.2 -sSfL --retry 3`. The archive is untrusted input at this
   point.
2. Compute SHA-256 and compare with the table's value. **On mismatch print both values and fail
   the step.** This happens **before** extraction and before anything in the archive is
   executed. Getting this backwards is the one way to implement the whole RFC and deliver none
   of it.
3. Only then extract, `find` the binary by name, `chmod +x` it, move it into a tools directory,
   and append that directory to `$GITHUB_PATH`. Zero matches from `find` must fail the step.
4. Print the verified hash and the binary's `--version`, **invoking it by absolute path** —
   `$GITHUB_PATH` affects subsequent steps, not the step that writes it.

### 1.2 Three mechanical traps in composite actions

- **Every `run` step needs an explicit `shell:`** (`bash`). A composite action's `run` step
  without it is an error, unlike a workflow's.
- GitHub's `bash` default is `--noprofile --norc -eo pipefail`, which gives you `-e` and
  `pipefail` but **not** `-u`. Set what you rely on explicitly rather than inheriting it.
- `cargo deny` works as a cargo subcommand because cargo finds `cargo-deny` on `PATH`. The
  binary's name inside the archive is already `cargo-deny`; do not rename it.

### 1.3 `ci.yml`'s call site (§3.3 item 1)

In the `gates` job, replace `- run: cargo install cargo-deny --version 0.20.2 --locked` with a
`uses: ./.github/actions/install-pinned-tool` step and `with: tool: cargo-deny`. Keep the
`cargo deny --version` step. Keep the existing comment's point — the gate fails when cargo-deny
is absent (RFC 026 §11) — and update it to say the pin is now a hash, not only a version.

The toolchain step must still come first: the `gates` job needs `thumbv7em-none-eabihf` for
`no-std`, `target-profiles` and `size-budget`, and that is unrelated to this change.

### 1.4 The exemption note (§4)

`docs/src/development.md`'s "What is deliberately exempt (RFC 045 S3)" subsection gains one
item: **no gate asserts that a pinned tool hash still matches upstream.** It would need network
access from a gate, which only `supply-chain` has and only against an advisory database. State
the reason it is acceptable, which is the failure mode: a stale or wrong pin fails **closed and
loudly** at the hash comparison, with both values printed, and the repair is a human updating
one number after reviewing new bytes — which is the review RFC 046 §2.3 exists to require.

Write it in the same house style as the five items already there, each stated with the source
that backs it.

## 2. B — S3 items 2 and 3: `release.yml`

**Only after A's push is green.** In the `release-gate` job:

- Replace `- run: cargo +stable install cargo-deny --version 0.20.2 --locked` with the action,
  `tool: cargo-deny`.
- Replace `- run: cargo +stable install mdbook --version 0.5.4 --locked` with the action,
  `tool: mdbook`.

**Keep the "Record tool versions" step exactly as it is.** It prints both `rustc` versions,
`mdbook --version` and `cargo deny --version`, and it is the tracked record of what ran at a
cut. §3.1 item 4's print adds to that; it does not replace it.

The two new steps no longer need the Rust toolchain, so in principle they could move earlier in
the job. **Do not reorder them.** Keep them where the installs are now, so the diff a future
reader compares against the 0.24.0 run's step list stays readable.

## 3. Non-scope

- **Changing either pinned version.** `cargo-deny` stays `0.20.2`, `mdbook` stays `0.5.4`. This
  slice changes how they are installed, never which.
- **A gate that checks the pins against upstream.** RFC 046 §4 rules it out and §1.4 documents
  why. If you find yourself writing one, stop and return to the architect.
- **`no-std.yml` and `msrv.yml`.** They install neither tool. Do not touch them.
- **Adding, removing or reordering gates.** Twenty stay twenty. `release-gate`'s own behaviour
  is untouched.
- **Creating a tag**, for any reason, including to test B.
- Any library change, any version bump, any change to how this workspace's own dependencies are
  vetted.

## 4. Required evidence

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
3. **For A: the measurement S5 asks for** — `ci`'s `gates` job duration and the `ci` workflow
   total, from the Actions run, against 2m 47s and 2m 53s. Also state the **new step's own
   duration**, so the download-and-verify cost is on the record and not inferred.
4. **For A: that you reproduced both hashes yourself** rather than copying this file's table,
   and the command you used. If either disagrees with §0 item 2, **stop and report it** — a
   hash disagreement is not a detail to resolve on your own.
5. **For A: evidence that the mismatch path fails.** Run the action once against a deliberately
   wrong hash, locally or in a scratch branch, and show that it fails before extracting. A
   verification step nobody has seen fail is a verification step nobody has tested.
6. **For B: that the "Record tool versions" step is byte-for-byte unchanged**, and that you
   created no tag.
7. **A `supply-chain` failure on a dependency you did not touch is not yours.** Report it and
   fix it with a scoped `cargo update -p <crate>`; do not wonder what you broke and do not run
   a bare `cargo update` — review 102 measured that a bare one moves thirteen packages.
8. **Anything in this handoff that does not match the tree.** §0.1 names the premises the
   architect most expects to be wrong. The implementer found a wrong count in RFC 045's handoff
   before writing a line of code; that is the standard.
