# RFC 036 implementation handoff — The Published Artifact

**To:** implementer (mid-capability model)
**From:** architect
**Authorized by.** Architect review 079 §6 and the owner's acceptance of RFC 036
on 2026-10-07. The RFC is `rfcs/accepted/036-the-published-artifact.md`; read it
first, this handoff does not restate its reasoning.
**Base revision:** `dfb5158` or later on `main`.

**Two review requests, not four.** Request **A** covers S1+S2 (packaging).
Request **B** covers S3+S4 (reference and vocabulary). Each slice must leave
`cargo xtask check` green on its own commit.

**Not your step.** Dropping tag `0.22.1`, re-cutting it, and publishing are the
architect's and owner's. Do not create, move or delete a tag, and do not run
`cargo publish` without `--dry-run`.

## 0. Values in this handoff are measured, not suggested

Everything below was verified by the architect on `dfb5158` before being written.
Where a plausible approach was tried and **rejected**, the reason is given so you
do not re-discover it. Three in particular:

1. **Do not symlink `LICENSE`.** A symlink *is* packaged, as a full regular file
   (11338 bytes) — tempting. But git records mode `120000` and `git archive` emits
   an `lrwxrwxrwx` entry, and `release_gate/package.rs:273-275` calls
   `fs::symlink_metadata` then rejects anything where `!file_type().is_file()`.
   A symlink breaks `release-gate`. Use real copies and the byte-equality gate in
   §1.2.
2. **Never pass `--cfg docsrs` on stable.** `RUSTDOCFLAGS="--cfg docsrs"` with
   `#![cfg_attr(docsrs, feature(doc_cfg))]` gives
   `error[E0554]: #![feature] may not be used on the stable release channel`.
   docs.rs builds on nightly, so the attribute is correct — but the `doc-build`
   gate runs stable and must omit the flag. Verified: stable without the flag
   passes under `-D warnings`; `cargo +1.85 check` is unaffected.
3. **`science::math` is not a category.** Only the slugs in §2.2 exist. Verified
   against the crates.io categories API, which lists 58 top-level slugs.

## 1. S1 — dependency requirements that mean what they say

### 1.1 The change

In `Cargo.toml` `[workspace.dependencies]`, the five internal entries carry the
workspace version instead of `"0"`:

```toml
loeres                      = { version = "0.22.1", path = "crates/loeres" }
loeres-backend-std          = { version = "0.22.1", path = "crates/loeres-backend-std" }
loeres-backend-static       = { version = "0.22.1", path = "crates/loeres-backend-static" }
loeres-cluster              = { version = "0.22.1", path = "crates/loeres-cluster" }
loeres-device               = { version = "0.22.1", path = "crates/loeres-device" }
```

Keep the column alignment the file already uses. Do not touch the external
dependencies below them.

**What this looks like when it works.** `cargo check --workspace --all-features`
still passes, because a `path` wins locally. And a dry-run of a dependent crate
now *refuses clearly* instead of silently substituting:

```text
error: failed to prepare local package for uploading
  failed to select a version for the requirement `loeres = "^0.22.1"`
  candidate versions found which didn't match: 0.20.2, 0.20.1, 0.20.0, ...
```

That error is the **correct** outcome and is not a defect to work around. It also
means `cargo publish --dry-run` can no longer verify a dependent crate before its
siblings are published. The whole-set check is §4.3's patched workspace.

### 1.2 The `published-metadata` gate, first assertion

New file `xtask/src/checks/published_metadata.rs`, following the shape of
`xtask/src/checks/link_audit.rs` (module doc comment, `pub fn run() -> bool`,
`[published-metadata] …` prefixed output, a `PASS`/`FAIL` last line, unit tests
in the same file as its neighbours do).

Registered in four places, matching how `link-audit` is wired:

| File | What to add |
|---|---|
| `xtask/src/checks.rs` | `pub mod published_metadata;` |
| `xtask/src/main.rs` (the `IMPLEMENTED` list, around line 31) | `"published-metadata",` |
| `xtask/src/main.rs` (the dispatch match, around line 55) | `Some("published-metadata") => checks::published_metadata::run(),` |
| `xtask/src/checks/release_gate.rs` (the gate tuple list, around line 198) | `("published-metadata", GateKind::Enforced, published_metadata::run()),` |

Place it **after** `link-audit` in the tuple list, so it reads last in the
summary. It is `Enforced`, not `Advisory`.

In S1 it asserts one thing: every internal entry in `[workspace.dependencies]`
has a `version` exactly equal to `[workspace.package] version`. Fail closed —
a missing `version` key, an extra internal crate, or a mismatch each fail, and the
message names the crate and both values.

## 2. S2 — a package a user can read

### 2.1 License text in every package

Copy the workspace `LICENSE` to `crates/<name>/LICENSE` for all five crates —
**real files**, for the reason in §0.1. Confirm with
`cargo package -p <name> --list`, which must contain a `LICENSE` line; today it
contains zero for every crate.

Extend `published-metadata`: for each publishable crate, `crates/<name>/LICENSE`
exists and is **byte-identical** to the workspace `LICENSE`. Five copies that can
drift would be exactly the kind of debt this project refuses; the assertion is
what makes the duplication safe.

### 2.2 Keywords and categories

Add to each crate's `[package]`. Keywords: at most five, each at most 20
characters. Categories: every slug below is verified to exist.

| Crate | `keywords` | `categories` |
|---|---|---|
| `loeres` | `["optimization", "quadratic-program", "solver", "no-std", "math"]` | `["science", "mathematics", "no-std"]` |
| `loeres-backend-static` | `["no-std", "embedded", "matrix", "linear-algebra", "storage"]` | `["no-std", "embedded", "data-structures"]` |
| `loeres-backend-std` | `["linear-algebra", "matrix", "sparse", "storage", "optimization"]` | `["science", "mathematics", "data-structures"]` |
| `loeres-device` | `["embedded", "no-std", "optimization", "mpc", "solver"]` | `["embedded", "no-std", "science"]` |
| `loeres-cluster` | `["optimization", "solver", "batch", "quadratic-program", "server"]` | `["science", "mathematics", "concurrency"]` |

Extend `published-metadata`: each publishable crate declares 1-5 keywords and
1-5 categories, and every category is in a `const VALID_CATEGORIES: &[&str]` in
the gate holding exactly the slugs used above. An invalid slug is rejected by
crates.io at publish time, which is far too late to find out.

### 2.3 Absolute links in the packaged READMEs

Each of the five `crates/<name>/README.md` carries three relative links that 404
on crates.io, because the targets are not in the tarball. Replace them:

Only the link **targets** change; the link text stays as written. Targets are
shown bare below, without their surrounding brackets, so that `link-audit` does
not read this table as three real links of its own:

| Current target | Replacement target |
|---|---|
| `../../README.md` | `https://github.com/nabbisen/loeres` |
| `../../docs/src/architecture.md` | `https://github.com/nabbisen/loeres/blob/main/docs/src/architecture.md` |
| `../../rfcs/README.md` | `https://github.com/nabbisen/loeres/blob/main/rfcs/README.md` |

**Recorded decision, so you do not re-open it.** The two deep links track `main`,
not the released tag, so a reader of an old version may reach newer documents. The
architect considered pinning them to the release tag and rejected it: it would
mean editing fifteen links on every release for a mild and rarely harmful drift,
and the book is not hosted anywhere, so there is no version-stable target to point
at. If drift becomes a real complaint it is a later decision, not yours.

Extend `published-metadata`: no packaged README contains a relative Markdown link
target. "Packaged" means `crates/<name>/README.md` here; the **root** `README.md`
is deliberately exempt, because it ships with no crate and its relative links are
correct on GitHub. Say so in the gate's module doc, or the next reader will
"fix" the root README and break it.

## 3. S3 — an API reference that shows the whole API

### 3.1 docs.rs configuration

To each of the five crate manifests:

```toml
[package.metadata.docs.rs]
all-features = true
rustdoc-args = ["--cfg", "docsrs"]
```

### 3.2 Feature labels

At each crate root, as the first line:

```rust
#![cfg_attr(docsrs, feature(doc_cfg))]
```

On every feature-gated **public** item, beside its existing `#[cfg(…)]`:

```rust
#[cfg_attr(docsrs, doc(cfg(feature = "parallel-rayon")))]
```

The gated feature groups are `parallel-rayon`, `async-tokio`,
`observability-tracing`, `observability-metrics`, `serde`, `ffi-gateway`
(`loeres-cluster`); `owned-arrays`, `constant-iteration`, `diagnostic-snapshot`,
`panic-gate` (`loeres-device`); plus `owned-arrays` in `loeres-backend-static`.
Label what is public and gated. Private items need nothing.

Extend `published-metadata` with a lexical symmetry assertion: every
`doc(cfg(feature = "X"))` in a crate names an `X` that crate declares in
`[features]`. This is the only check that can catch a typo or a removed feature,
because the gate in §3.4 cannot build the `docsrs` path at all. Same spirit as
RFC 022's citation symmetry and RFC 030's coverage symmetry.

### 3.3 The five rustdoc diagnostics

| Location | Diagnostic | Required fix |
|---|---|---|
| `crates/loeres-backend-static/src/lib.rs:12:9` | `` `array` is both a module and a primitive type `` | disambiguate, e.g. `` [`mod@array`] `` |
| `crates/loeres-cluster/src/runtime.rs:6:59` | public docs link to **private** `executor` | **stop linking it** — see below |
| `crates/loeres-cluster/src/lib.rs:20:29` | redundant explicit link target | drop the redundant target |
| `crates/loeres-cluster/src/lib.rs:25:63` | redundant explicit link target | drop the redundant target |
| `crates/loeres-cluster/src/lib.rs:26:47` | redundant explicit link target | drop the redundant target |

**The `executor` decision is made: do not make it public.** RFC 036 §4 routed
this to review because publicising a type is an API change, and the architect has
ruled. Rewrite the sentence so it does not link a private item — name it in prose
or in backticks without a link. If you believe the documentation genuinely needs
the type to be public, stop and say so in request B rather than changing the
surface; `check-public-api` would catch it anyway.

### 3.4 The `doc-build` gate

New `xtask/src/checks/doc_build.rs`, wired at the same four sites as §1.2, placed
after `published-metadata`, `Enforced`. It runs:

```text
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps
```

**No `--cfg docsrs`** — §0.2. Report the failing crate names on failure. State in
the module doc that the `docsrs` path is deliberately unbuilt, that this is why
§3.2's symmetry assertion exists, and that building it would require nightly,
which the project does not pin.

## 4. S4 — say "published" when publication is meant

### 4.1 CHANGELOG vocabulary

Release-status lines state both facts, so neither can be read as the other. For
the five affected releases the fact does not change, only its precision:

```text
**Release status:** released (tagged 2026-09-24, distributed 2026-09-24); not published to crates.io
```

Apply to `0.21.0` (tagged/distributed `2026-09-12`), `0.21.1`, `0.21.2`, `0.21.3`
and `0.22.0` (all `2026-09-24`). Leave `0.20.2` and earlier alone — they *are*
published, and if you wish to mark them, `; published to crates.io` is the form.

Add, directly under the file's existing preamble, a short note defining both
words: **distributed** is RFC 021 §7's `D_release` — the authoritative remote
accepted the canonical tag and the tagged CI `release-gate` job succeeded;
**published** means the version is present in the crates.io index for every
publishable crate. Do not restate RFC 021's predicate algebra; a user is reading
this, not an auditor.

`0.22.1`'s own section keeps `**Release status:** unreleased` — the architect sets
it at the cut.

### 4.2 A note in the book

In `docs/src/specifications.md`, near the release-currency table, state plainly
that the newest version on crates.io is `0.20.2`, that releases since then are
tagged but not published, and what that means for installing today. Keep it to a
few sentences and do not duplicate the CHANGELOG.

The getting-started tutorial already tells a reader to `git clone` and needs no
change in this RFC.

### 4.3 The publication procedure

Add a section to `docs/src/development.md`, next to the two subsections on
`release-gate` behaviour:

- Publication is authorized **separately** from distribution (RFC 021 §7), is a
  human step, and is run by the architect and owner — not CI, not the dev team.
- Dependency order: `loeres`, `loeres-backend-static`, `loeres-backend-std`,
  `loeres-device`, `loeres-cluster`.
- `cargo publish --dry-run` **cannot** verify a dependent crate before its
  siblings are published, and after S1 it will say so plainly. Record why, and
  record the check that does work, concretely: `cargo package` all five, extract
  each `.crate`, write a workspace whose `[patch.crates-io]` maps every crate name
  to its extracted directory, and `cargo check --workspace --all-features`. The
  architect verified the `0.22.1` set compiles this way.
- `cargo publish` is irreversible: a version can be yanked but never replaced, so
  metadata ships permanently.

## 5. Explicit non-scope

- **No API change.** `check-public-api` must stay green across all four slices.
- No backfill of `0.21.0`-`0.22.0` to crates.io; no yanking of `0.20.2`.
- No publish automation or credentials in CI. RFC 021 kept this out and RFC 036 §4
  does not reopen it.
- No change to `link-audit`. It reasons about the repository; the new gate reasons
  about the package. Leave the existing one alone.
- No `#[allow(…)]` anywhere, including on new gate code.
- Do not touch `rust-toolchain.toml` or the MSRV.

## 6. Required evidence

For each request:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-features --all-targets -- -D warnings
cargo test --workspace --all-features
cargo +1.85 check --workspace --all-features
cargo xtask check                                   # 18 gates after S1, 19 after S3
cargo xtask published-metadata                      # standalone
cargo xtask doc-build                               # standalone, request B
cargo package -p <each crate> --list                # must show LICENSE (request A)
```

State in the request:

1. **Examples affected:** expected `none` for every slice. Say so explicitly.
2. For request A: the `cargo package --list` LICENSE line for all five crates, and
   the dry-run refusal text from §1.1 showing the requirement is now enforced.
3. For request B: that `cargo doc` passes under `-D warnings`, and the count of
   items you labeled per crate.
4. **That each new gate actually bites.** Do not assert it — break each assertion
   once, record the exact failure message, restore, and confirm the tree is clean
   with `git status`. A gate nobody has seen fail is not known to work. This is the
   same standard C2 was held to, and the architect will re-measure it.
5. Any value in this handoff you found to be wrong. They were measured, but
   measurement is not infallibility — review 077 and 078 both corrected the
   architect's own numbers.

Redirect long gate output to a file; do not pipe it through `head` or `tail`
(`docs/src/development.md`, "A candidate run is once-per-revision").
