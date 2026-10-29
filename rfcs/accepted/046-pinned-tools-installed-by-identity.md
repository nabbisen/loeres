# RFC 046 - Pinned Tools Installed By Identity, Not By Rebuild

**Status.** Accepted (design frozen 2026-10-29)

**Author tier.** `architect`.

**Governing scoping.** Architect review 108 §2, which named this as a candidate and declined to
smuggle it into RFC 045 because it touches `release.yml`. The owner authorized the `cargo-deny`
change on 2026-10-29 and, on accepting this RFC the same day, authorized §3.3 item 3 (`mdbook`)
and the landing order in §5's last row.
Every figure below was measured by the architect from this repository's own workflow runs, and
every hash was downloaded and recomputed by the architect, not copied from upstream prose.

## 1. Summary

Two pinned tools — `cargo-deny 0.20.2` and `mdbook 0.5.4` — are installed in CI by compiling
them from source. Upstream publishes statically linked binaries for both. Measured on this
repository:

| job | run | the source builds | the job |
| --- | --- | ---: | ---: |
| `release.yml` / `release-gate` | `37722084985` (the 0.24.0 cut) | `mdbook` **1m 19s** + `cargo-deny` **1m 53s** = **3m 12s** | 6m 20s |
| `ci.yml` / `gates` | `40792044395` | `cargo-deny` **1m 56s** | 2m 47s |

So **51%** of the release gate and **70%** of the push gate is spent rebuilding two tools whose
released binaries are a five-megabyte download. Replacing the rebuild with a hash-verified
download leaves `ci`'s twenty-gate job at **under a minute** — the workflow's bound becomes
`check-test` at 1m 7s, making `ci` with all twenty gates *faster than it was with three* before
RFC 045 — and the release gate at about **3m 18s**.

Speed is the smaller half of the argument. The larger half is §2.3.

## 2. Why this is an assurance change and not a one-line edit

### 2.1 Cost

`cargo install --locked` is paid on every push, every pull request, and every tag push, forever,
to produce a binary identical to one upstream already built.

### 2.2 The build's inputs are not pinned the way the version is

`cargo install cargo-deny --version 0.20.2 --locked` pins the tool's crate version and the
tool's own lockfile. It does not pin the compiler or the image that compile it. This repository's
own CI runs now carry the warning: *"The `ubuntu-latest` label will migrate to Ubuntu 26
beginning October 19, 2026."* A runner-image change we do not control can break a tool install
that has nothing to do with this workspace's code, and it would surface as a red gate on an
unchanged tree — the exact failure mode RFC 045 Amendment 1 spent three pages on.

Both published assets are `ELF 64-bit LSB pie executable, x86-64, static-pie linked` — verified
locally with `file` on the extracted binaries. A static-PIE binary has no dynamic loader and no
libc dependency, so it is indifferent to the image migration.

### 2.3 Nothing in this repository states which bytes ran

This is the part that makes it an RFC. Today the tracked answer to *"which `cargo-deny` executed
in the gate?"* is a crates.io version number. What actually executed is whatever that version
compiled to, under that day's toolchain, on that day's image. The project already has a standard
for this and it is stricter: architect review 105 withdrew the network argument against
`curl | sh` and established that the defensible requirement is **verifiable artifact identity**.
A version number is not an identity. A SHA-256 this repository committed after reviewing the
bytes once is.

The same standard already governs source identity at a cut — RFC 028's uncompressed-tar anchor,
hand-recomputed and verified three ways at every release, twelve consecutive holds. The tools
that *run* the gates are held to a weaker standard than the tree the gates check. S1 closes
that asymmetry.

## 3. Obligations

### 3.1 S1 — a composite action that installs a pinned tool by verified hash

`.github/actions/install-pinned-tool/action.yml`, a composite action with **one** input,
`tool`, whose accepted values are `cargo-deny` and `mdbook`. The pins themselves (§3.2) live in
a table at the top of that one file, not at the call sites.

**Why one input and not three.** The first form of this obligation took `url`, `sha256` and
`binary` at the call site, which reads cleanly — the job states what it installs. But
`cargo-deny` is installed by **two** workflows, so its pin would exist twice, and a pin that
exists twice is a pin that will eventually disagree with itself. One table in one file is also
the artifact a human reviews when a version moves, which is the review §2.3 exists to require.
The mechanism stays generic; only the data moved.

In order:

1. Download the asset into a fresh empty directory under `$RUNNER_TEMP` with
   `curl --proto '=https' --tlsv1.2 -sSfL`.
2. Compute its SHA-256 and compare against the `sha256` input. **On mismatch, print both values
   and fail the step** — before the archive is extracted and before anything in it is executed.
   The order is the whole point: an archive is attacker-controlled input until its hash matches.
3. Extract, locate the binary **by name** rather than by path, and install it into a tools
   directory appended to `$GITHUB_PATH`. By name is required: the two assets have different
   layouts (§3.2) — `cargo-deny`'s tarball nests the binary one directory deep, `mdbook`'s has
   it at the root.
4. Print the verified hash and the binary's own `--version`, so the run log records the identity
   that ran. **By absolute path**, because `$GITHUB_PATH` takes effect in later steps, not the
   one that writes it.

Nothing is piped to a shell. Review 105's objection was identity, and step 2 answers it.

**Not the upstream `.sha256` sidecar.** `cargo-deny` publishes one; it is signed by nobody and
served by the same party as the asset, so it detects transport corruption, not substitution. The
pin must be a value this repository reviewed and committed. mdBook publishes no sidecar at all,
which makes the point concrete rather than theoretical.

### 3.2 S2 — the pins, each downloaded and hashed by the architect

| tool | version | asset | bytes | SHA-256 |
| --- | --- | --- | ---: | --- |
| `cargo-deny` | `0.20.2` | `cargo-deny-0.20.2-x86_64-unknown-linux-musl.tar.gz` | `4936832` | `9f12ed4c49936e09b48bf862b595cde2fe64fcbd9d74dfacac6131ca824c8d5f` |
| `mdbook` | `0.5.4` | `mdbook-v0.5.4-x86_64-unknown-linux-musl.tar.gz` | `4944048` | `5222beabd3e37dc5be0d18ff99b79058469354db5c220153a1b92db5ba12be89` |

| tool | binary inside the archive | installed by |
| --- | --- | --- |
| `cargo-deny` | `<asset stem>/cargo-deny` (nested one level) | `ci.yml`, `release.yml` |
| `mdbook` | `mdbook` (archive root) | `release.yml` |

The two tag forms differ (`0.20.2` against `v0.5.4`) and so do the asset-name forms, which is why
the table holds whole URLs rather than a template and a version.

`cargo-deny`'s value independently matches its upstream sidecar and the asset size the GitHub API
reports. `mdbook`'s is hand-computed only, there being nothing to match it against.

### 3.3 S3 — the call sites

All three are authorized. Items 1 and 2 were authorized when this RFC was scoped; item 3 was
put to the owner separately, with its cost, and authorized on acceptance — `mdbook` is the
identical defect with the identical fix, and building the mechanism while declining to use it for
the second of its two callers would have left 1m 19s of the same job on the table for no reason
anyone could state later.

1. `ci.yml`'s `gates` job — replace `cargo install cargo-deny --version 0.20.2 --locked`.
2. `release.yml`'s `release-gate` job — replace
   `cargo +stable install cargo-deny --version 0.20.2 --locked`.
3. `release.yml`'s `release-gate` job — replace
   `cargo +stable install mdbook --version 0.5.4 --locked`.

Expected after all three: the release gate goes 6m 20s → about **3m 18s**, a 48% reduction, and
`mdbook` stops being a compiler's problem at the one moment a release cannot be cheaply retried;
`ci`'s `gates` job goes 2m 47s → **under a minute**, making the workflow's bound `check-test` at
1m 7s. These are predictions, not measurements. **S5 requires the real numbers.**

### 3.4 S4 — what this must not change

- **The pinned versions.** `0.20.2` and `0.5.4` stay exactly as they are. This RFC changes how
  they are installed, never which.
- **The version recording.** `release.yml`'s "Record tool versions" step keeps printing
  `cargo deny --version` and `mdbook --version`; S1 item 4 adds to that, it does not replace it.
- **The gates.** No gate is added, removed, or reordered. Twenty stay twenty, and
  `release-gate`'s own behaviour is untouched.
- **`no-std.yml` and `msrv.yml`** install neither tool and are not edited.

### 3.5 S5 — report the real numbers, and the landing order

§3.3's figures are arithmetic on measured step times, not measurements of the new workflow.
Report, from the first runs: `ci`'s `gates` job duration and the `ci` workflow total, and — at the
next cut, not on a tag created for the purpose — the `release-gate` job duration with its per-step
breakdown. Record them in §3.3 the way RFC 045 §3.4 records its own answer, so the next person
asking what this cost reads a number rather than a prediction.

**The action and item 1 land first, on an ordinary push**, because `ci`'s `gates` job is the
only thing that can exercise the action without creating a tag. Items 2 and 3 edit
`release.yml`, whose path cannot be exercised except at a cut; they land only after that push is
green, and their verification is the next `release-gate` run — not a tag created to test them.

## 4. Non-goals

**No gate asserts that a pin still matches upstream.** It would need network access from a gate,
which only `supply-chain` has today and only against an advisory database. This is a deliberate
exemption and belongs in `docs/src/development.md`'s RFC 045 S3 exemption list when S1 lands.
The failure mode is why it is acceptable: a stale or wrong pin fails **closed and loudly** at the
hash comparison, never silently, and the repair is a human updating one number after reviewing
new bytes — which is the review this RFC exists to require.

**No change to how this workspace's own dependencies are vetted.** `cargo deny` keeps doing that;
this is about the tool, not its input.

**No new tool, and no removal of `cargo install` as a mechanism.** If an asset is withdrawn
upstream, the one-line `cargo install` form remains a correct fallback (§5).

## 5. Risks

| risk | severity | why it is acceptable |
| --- | --- | --- |
| Upstream deletes or retags the release asset | low | The step fails closed; the fallback is the `cargo install` line, restored in one commit. `cargo install` depends on crates.io availability in exactly the same way. |
| A pin goes stale when a version is bumped | low | Fails closed at the hash comparison, with both values printed. Impossible to pass accidentally. |
| The downloaded binary is not what upstream built | low | This is the risk the RFC *reduces*. Today there is no check at all; after S1 a substituted asset fails unless it collides with a committed SHA-256. |
| musl/static binary misbehaves on the runner | low | Both verified `static-pie linked`, no dynamic loader. The architect verified identity and ELF type locally and did **not** execute the downloaded binaries; first execution is in CI, where S1 item 4 records the version. |
| `release.yml` is edited at all | medium | It gates releases. Mitigation: land S1 with items 1 and 2 on an ordinary push first, so `ci`'s `gates` job exercises the action before any tag depends on it, and verify the release path on the next cut's `release-gate` run rather than on a tag. |
