# Local Development

Loeres is a Cargo **workspace**. Install the toolchain with
[rustup](https://rustup.rs/); the pinned channel, components, and bare-metal
target are declared in `rust-toolchain.toml`.

## Layout

```text
crates/loeres             # no_std, no alloc — shared contracts
crates/loeres-backend-static   # no_std, no alloc — fixed-size storage / views
crates/loeres-device           # no_std, no alloc — deterministic edge solvers
crates/loeres-backend-std      # std — dynamic storage (server-only)
crates/loeres-cluster          # std — server solving / orchestration
examples/                      # runnable examples, excluded from the workspace (RFC 023)
xtask/                         # repository automation (never a library dependency)
```

The three crates under `examples/` are **not** workspace members. They are listed
in the root manifest's `[workspace] exclude` and carry their own lockfiles, so
each one's resolved dependency graph is independent evidence of isolation rather
than a product of shared resolution. Build one with
`cargo build --locked --manifest-path examples/<name>/Cargo.toml`.

## Everyday commands

```sh
cargo check --workspace --all-features
cargo test  --workspace
cargo fmt --all            # run once after implementation, before checks
cargo clippy --workspace --all-features -- -D warnings
mdbook build docs --dest-dir target/xtask-book/local      # see below
```

Build the book with an explicit `--dest-dir` under `target/`. `book.toml`'s
default output is `docs/book/`, and while that path is ignored it is not
gate-owned: `release-gate` will not delete a directory it did not create, so a
plain `mdbook build docs` used to leave output that blocked every later candidate
run. The gate now builds to its own scratch directory under `target/` and never
touches `docs/book/`, so a stale one is harmless — but keeping your own output
under `target/` too means one `cargo clean` disposes of everything.

`--dest-dir` resolves against the **current directory**, not against the book
directory, so run the command above from the repository root and pass no `../`.

## Verification gates (`xtask`)

```sh
cargo xtask zero-bleed       # no forbidden server <-> edge dependency edge exists
cargo xtask no-std           # edge crates build for thumbv7em-none-eabihf (no std/alloc)
cargo xtask doc-currency     # bounded RFC 020/024 apex metadata/lifecycle/navigation assertions
cargo xtask review-evidence  # RFC 022 architecture-review citation and provenance integrity
cargo xtask supply-chain     # RFC 026 dependency advisories, licenses, bans, sources
cargo xtask examples         # RFC 023/035 per-example build, run, resolved-graph isolation, captured output
cargo xtask check            # canonical developer architecture aggregate
cargo xtask release-gate     # complete non-publishing RFC 019 candidate evidence
```

RFC 010 implements the stable command namespace: `check-rfcs`, `zero-bleed`,
`check-public-api`, `feature-matrix`, `target-profiles`, `panic-audit`,
`size-budget`, `unsafe-audit`, `conformance`, `doc-currency`, `review-evidence`,
`supply-chain`, `examples`, `link-audit`, (RFC 030) `differential`, and (RFC 036)
`published-metadata` and `doc-build`.
The aggregate summary labels commands as enforced, advisory/reporting, or
owner-RFC hooks; threshold-less baselines and missing future corpora are not
reported as enforced verification passes.

RFC 020 adds `doc-currency` to the developer aggregate. It checks the shared
apex release/draft fields, RFC folder/index status agreement and accepted
design-freeze metadata, recovery markers, mdBook navigation, release-local
normative paths, and an explicit current-status stale-phrase ledger. It ignores
the labeled historical root-roadmap suffix and does not attempt to infer
arbitrary prose semantics; human architecture review remains required.

`release-gate` is not an everyday check alias. On a clean tracked revision it
runs the ordered RFC 019 §11.3 source suite, creates a tracked-input root-layout
`loeres-v<version>.tar.gz`, and validates its paths, types, exact tracked-file
set, and SHA-256 before extraction. It then verifies the extracted Git-object
content and executable modes before repeating the applicable suite in the
gate-owned clean extraction.
Evidence records two digests for that archive: the archive SHA-256 identifies
this build's compressed bytes, and the uncompressed tar SHA-256 is the
cross-environment identity of its content and layout, reproducible across a
different gzip implementation or level (RFC 028).
Local runs record that no tag assertion occurred; tagged CI additionally proves
that the peeled canonical unprefixed SemVer tag equals `HEAD`. Evidence and the
archive remain in gate-owned ignored workspace state; temporary extraction
state is removed only after an ownership check. A passing local dry run is not
tagged-release evidence and neither mode publishes crates or creates a release.
Tracked changes fail closed. Its `mdbook build` step writes to
`target/xtask-book/<suite>` — cleared before each run, so the book is proven to
build from nothing — and the gate neither reads nor deletes anything outside
`target/`. The candidate gate expects the canonical Linux workflow tools `git`,
GNU `tar`, and `sha256sum`; the release workflow supplies the reviewed execution
environment.

### A candidate run is once-per-revision, and does not clean up after itself

`release-gate` writes its evidence to
`.git-exclude/release-evidence/<revision>-v<version>/` and creates that directory
with `create_dir`, not `create_dir_all`. A second candidate run on the **same
revision** therefore stops with

```text
cannot create fresh evidence directory .../<revision>-v<version>: File exists
```

and a non-zero exit. This is the gate refusing to overwrite evidence, not a gate
failure: RFC 019 evidence is meant to be written once and read afterwards, and
silently merging a new run into an older directory would make it untrustworthy.

The consequence is that re-running a candidate on an unchanged revision needs that
one directory removed first. Remove only the directory named for the revision you
are re-running, after looking at what is in it. Nothing else under
`.git-exclude/release-evidence/` belongs to the new run.

Do not pipe a candidate run through `head`, `tail` or any other command that can
close the pipe early — that kills the run partway through and leaves exactly the
part-written evidence directory described above. Redirect the whole stream to a
file and read the file afterwards.

### Clippy runs in `release-gate`, on a floating channel

`cargo xtask check` does not run clippy. `release-gate` does, as
`cargo +stable clippy --workspace --all-features --all-targets -- -D warnings`.
Two consequences follow, and both are accepted rather than worked around.

A lint failure is a **release blocker and nothing less**: it cannot be found by
the aggregate check, so it surfaces for the first time when a release candidate
is run. Run `release-gate` before scheduling a release, not after.

`rust-toolchain.toml` pins `channel = "stable"`, which floats. A new stable
toolchain can therefore turn a passing tree into a failing one with no
repository change, because `-D warnings` promotes a newly added lint — a fresh
deprecation, most often — into an error. The channel is deliberately not pinned:
an upstream deprecation is information worth receiving early, and pinning would
defer it to a larger upgrade. The cost is that a release may need a lint-fixing
commit before it can be cut, which is ordinary work, not an incident.

RFC 011 makes `target-profiles` manifest-driven through
`xtask/target-profiles.toml`. Mandatory profiles fail the aggregate on missing
targets or failed commands; advisory-installed profiles report unavailable when
optional targets are not installed; documented-only profiles are listed without
compilation.

RFC 013 makes the default `conformance` path enforced. `cargo xtask
conformance` runs the smoke corpus under `conformance/smoke/`, comparing the
real device and cluster projected-first-order solvers. `extended` and
`adversarial` remain placeholder suites until populated by later work.

RFC 015 adds the cluster-only validation evidence cache. The cached
projected-first-order path is carrier-only and `f64`-only in v0.19.0; the
generic RFC 016 solve path remains source-compatible and non-cacheable. The
carrier advances mutation epochs before mutable model access, so stale cached
evidence fails closed after failed or panicking mutation closures.

RFC 017 extends the enforced smoke corpus with validation-cache conformance
fixtures. `schema_version = 2` fixtures exercise cache hit/miss, insufficient
scope, stale/wrong evidence, current-iterate scan retention, hot-loop
numerical-domain retention, and reusable-cache insertion rejection.

RFC 024 gives `doc-currency` a permanent post-release apex form: the shared
apex block records this tree's identity (`This tree`) and the release this
documentation was last reconciled against (`Last reconciled repository
release`), never release status, with a strict `>` invariant and no equality
case. `Implemented scope` binds the exact `rfcs/done/` set, excluding RFC 000,
rendered as a canonical compact set (a run of two or more consecutive numbers
collapses to `NNN-NNN`). The one-release RFC 021 conditional apparatus is gone:
its metadata file, its `xtask` module, and that module's call site were all
removed (Amendment 2, §0.6). RFC 021's own document and git history are its
record — unused code is deleted rather than suppressed, so nothing is carried
under a lint allowance. `doc-currency` keeps one constant for the retired apex
marker, as the guard that it never reappears.

RFC 025 codifies in-place amendment of Accepted RFCs in RFC 000 and gives
`check-rfcs` the enforcing assertion: an RFC in `accepted/` may be corrected in
place through a numbered, dated `## 0.N Amendment` section under RFC 000's stated
conditions, while no file under `rfcs/done/` may carry such a heading — a shipped
RFC is superseded by a new one, never amended in place.

RFC 022 adds `review-evidence`. Normative documents cite architecture reviews
as evidence, not authority; the check resolves every `review NNN` /
`reviews NNN/NNN` citation in a tracked Markdown document against
`rfcs/review-evidence-index.md`, requires every index row to carry an author
tier from a closed set (`owner`, `architect`, `implementer`, `external`,
`unrecorded`), and asserts the index's **Cited in release** ticks equal the
cited set it derives, failing in either direction (Amendment 3, §0.5). All three
depend only on tracked bytes and are enforced, fail-closed. Hash verification
against the maintainer-held corpus (`.git-exclude/reviewed/`), coverage symmetry
(Amendment 2, §0.4), and row count against corpus file count (Amendment 3) run
when the corpus is present and report `unavailable` — never a pass — when it is
absent, such as in a clean extraction. The count assertion catches the
duplicated row that set-based hash verification and symmetry cannot see. A
citation-shaped token that does not match the recognized grammar is reported as
a near-miss finding rather than silently ignored, but does not by itself fail
the gate.

RFC 026 adds `supply-chain`, enforced in the aggregate and in `release-gate`'s
source-tree and clean-extraction suites. It runs `cargo deny --all-features
check` against the tracked `deny.toml`: RustSec advisories, an exhaustive
license allow-list, duplicate/wildcard bans, and crates.io-only sources. It then
runs `cargo deny check bans` per edge crate with `--no-default-features` and
asserts each has an empty external dependency set — a second zero-bleed witness
independent of the `zero-bleed` gate's internal-edge scan.

Install the tool before running the aggregate:

```sh
cargo install cargo-deny --version 0.20.2 --locked
```

If `cargo-deny` is absent the gate reports `unavailable` **and fails**. That is
deliberately unlike RFC 022's maintainer-held review corpus, which is
legitimately missing from a clean extraction: a missing tool is an environment
defect. Policy lives in `deny.toml` and is never relaxed to make a tree pass — a
failing tree is a finding, not a reason to add a skip.

RFC 023 adds `examples`, enforced in the aggregate and in `release-gate`'s
source-tree and clean-extraction suites. Per example it builds the crate under
its declared feature set with `--locked`, then reads the **resolved** dependency
graph against that example's own lockfile and asserts no forbidden crate appears
in it. Resolved, not declared: a manifest lists direct dependencies while the
resolve shows what is reachable, including transitively and through feature
activation. The device forbidden set is external design §1.1's —
`loeres-cluster`, `loeres-backend-std`, `tokio`, `rayon`, `tracing`. An absent
example directory, manifest, lockfile, or `src/main.rs` fails; a gate that passes
when its subject is missing proves nothing.

What this establishes is **dependency reachability**, not bare-metal
buildability. An example is a host program and its own `main` may use `std`; the
edge crates remain `#![no_std]` with no `alloc`, and that claim belongs to
`no-std` against `thumbv7em-none-eabihf`. Keep the two apart in prose.

### Publication is a separate, human step

Publishing to crates.io is authorized separately from distribution (RFC 021 §7). It is run
by the architect and the project owner. No workflow in this repository publishes, and the
development team does not publish.

- **Order.** Publish `loeres`, then `loeres-backend-static`, `loeres-backend-std`,
  `loeres-device` and `loeres-cluster`. A dependent needs its siblings in the registry at
  the same version first.
- **`cargo publish --dry-run` cannot verify a dependent before its siblings are published.**
  On `0.22.1` it refuses, as it should:

  ```text
  error: failed to select a version for the requirement `loeres = "^0.22.1"`
    candidate versions found which didn't match: 0.20.2, 0.20.1, 0.20.0, ...
  ```

  `[patch.crates-io]` does not change this: cargo resolves a publishable manifest against the
  real registry and ignores patches. `cargo package` of a dependent refuses in the same way,
  so no tarball of a dependent can be built before its siblings are published. A dry-run of a
  root crate with no internal dependencies succeeds.
- **What establishes the packaged set compiles, before publication.** `cargo package -p <crate>
  --list --offline` needs no registry resolution. For all five crates together it takes about
  0.14 seconds. The listing shows that the packaged `src/` set is identical to the tracked `src/`
  set (18, 9, 9, 13 and 25 files), and that no crate has a `build.rs`. So the published crates'
  compilable content is byte-identical to the workspace members', and the published set compiles
  if `cargo check --workspace --all-features` passes. A separate source check, made by reading the
  sources and not by the listing, found that no source uses `include_str!` or `include_bytes!`.
  `cargo xtask published-metadata` asserts the file-set equality per commit. It also fails if a
  package lists a file git does not track, and asserts that the package carries `LICENSE` and
  `README.md`.
- **The real whole-set verification happens during publication.** Each crate's own
  `cargo publish` verify build resolves its siblings from the real registry, so publish in
  dependency order and let each step verify against what is already published.
- **The residual risk, plainly.** If crate *N* fails verification, crates *1* to *N-1* are
  already published, and a published version can be yanked but never replaced. The per-commit
  file-set assertion is what makes that outcome unlikely, because the only thing that could
  differ between the workspace build and the published build is a missing file.
- **Publication is irreversible.** Check `cargo xtask published-metadata` and the package
  listings before the first `cargo publish`.

## Release version convention

### What each position means

Semantic Versioning 2.0.0: **major** for incompatible API changes, **minor** for
functionality added in a backward-compatible way, **patch** for backward-compatible
fixes. Semver §4 also says `0.y.z` is initial development, where anything may change.

Cargo treats `0.x` differently from `1.x`: `^0.22.3` resolves to `>=0.22.3, <0.23.0`,
so while this project is pre-`1.0` the **minor** is the compatibility unit. One
consequence follows, and one does **not**.

**It follows that a breaking change cannot ship as a patch.** A `0.21.4` would be
picked up automatically by a `loeres = "0.21"` requirement, so a change that breaks a
caller has to bump the minor. That is why `0.22.0` is `0.22.0` and not `0.21.4`:
`ConstrainedSolveRecord` gained a field and became `#[non_exhaustive]`, so a caller
constructing it by struct literal stopped compiling.

**It does not follow that a minor bump means something broke.** A minor is also where
additive functionality goes — that is its meaning in semver — and this project has
shipped exactly that: **`0.20.0`** was "a verification-only conformance hardening
release. Runtime crate APIs are unchanged."

So never read `0.22.x → 0.23.0` as a signal that a caller must change code. Read the
release's own `CHANGELOG.md` entry, which states what changed and whether anything
broke.

Earlier documents compress this to "under Cargo's `0.x` rules the minor is the breaking
position" — in `rfcs/done/034`, `rfcs/done/036`, the `0.22.0` prep handoff and the
`0.22.0` changelog entry. Read as "a breaking change must be expressed in the minor"
that is correct; read as "a minor bump signifies breakage" it is wrong, and it has
caused that error once. Those are historical records and are not edited; this section
is the current statement.

### A slice that moves the release's position bumps the version before the cut

The released version is normally set in the release's own finalization revision. **A slice whose
content changes which version the next release will carry is the exception**, because `cargo
xtask release-gate --intended-tag <v>` validates the intended tag against the workspace version,
and that gate is required of every slice. The gate cannot run until the workspace version matches
the release the slice's content forces.

What `main` carries between releases is a **placeholder, not a prediction**: the next patch,
set by the post-release commit. Which release it actually becomes follows from the slices:

| The slice | Next release | Who bumps |
| --- | --- | --- |
| fixes a defect, or only measures or documents | the patch already on `main` | nobody; the placeholder was right |
| **adds** functionality a caller can call | a minor | the first such slice |
| **breaks** a caller | a minor (Cargo's `0.x` rules) | the first such slice |

The bump touches four places, every one of them gated:

1. the workspace version, and the five internal `[workspace.dependencies]` pins that must move
   with it or `published-metadata` fails closed;
2. all eight lockfiles;
3. the apex `This tree` field, which `doc-currency` will otherwise demand with three
   `APEX RELEASE` findings;
4. the `## [x.y.z] — unreleased` `CHANGELOG.md` heading, because `release-gate` requires
   **exactly one** `## [<workspace version>]` heading and the placeholder's heading no longer
   matches. This is the gate's *first* preflight check, ahead of the tag and cleanliness
   checks, so `CHANGELOG: expected exactly one ...` is what a half-done bump reports.

Nothing else moves. In particular `Last reconciled repository release` stays where it is: the
release has not happened, and RFC 024's inequality is strict.

RFC 042 was the first slice to hit this, as a breaking change, and the architect's handoff had
required the gate without accounting for it. An earlier form of this section stated it only for
that case, as "a breaking slice bumps the minor before the cut"; the trigger is the release's
position, not breakage, so an additive slice reaches it the same way.

### Bumping after a release

After a release ships, `main`'s workspace version is bumped to the next patch
immediately, in the first ordinary post-release commit — never left at the
released value, which would let a later commit claim to be that release. The
released version itself is set authoritatively only in the release's own
finalization revision, which is what gets tagged; it is never edited into an
already-tagged commit. See RFC 024 for the apex-currency mechanics this
convention keeps green.

Every version bump takes **eight** lockfile updates, not one:

```sh
cargo update --workspace --offline
cargo update --offline --manifest-path examples/cluster-batch-solve/Cargo.toml
cargo update --offline --manifest-path examples/cluster-qp-constrained/Cargo.toml
cargo update --offline --manifest-path examples/cluster-capacity-dispatch/Cargo.toml
cargo update --offline --manifest-path examples/device-box-pfo/Cargo.toml
cargo update --offline --manifest-path examples/device-mpc-step/Cargo.toml
cargo update --offline --manifest-path examples/cluster-counted-work/Cargo.toml
cargo update --offline --manifest-path device-size-reference/Cargo.toml
```

The examples, and the RFC 040 size-budget fixture `device-size-reference`, are excluded
from the workspace (RFC 023 §11.1; RFC 040 §1.1 for the fixture), so `--workspace` cannot
reach any of them, and both the `examples` gate and `size-budget` build their subject
`--locked`. Forget one of the last seven and that gate fails loudly — `EXAMPLE BUILD` plus
`EXAMPLE RESOLVE` findings for an example, or `unavailable: device-size-reference did not
build` for the fixture — rather than silently, which is what `--locked` is for, but the
cause is the bump, not the thing that failed to build.

## Workflow

Development is **design-first**: requirement / RFC → external design → internal
design → implementation → testing. New public-boundary work starts as an RFC
under `rfcs/proposed/` (see `rfcs/done/000-rfc-lifecycle-policy.md`).

### Every path in a tracked document is relative to the project root

RFCs, handoffs, checklists and review records cite files as
`crates/loeres/src/scalar/fixed_point.rs`, never as an absolute path and never
elided with `…`. Two reasons, and the first is the binding one:

- **An absolute path is wrong for every reader but one.** The tree is cloned to
  a different place by each person and each CI job, so a path rooted in one
  machine's home directory does not resolve anywhere else and cannot be pasted
  into an editor, a `grep`, or a later document.
- **A relative path stays true as the document is quoted.** Handoffs cite
  reviews, reviews cite RFCs, and checklists cite all three; a root-relative
  path survives every one of those copies unchanged.

A file that genuinely lives outside the tree — a scratch harness, a temporary
run directory — is not a path a tracked document should hand on at all. State
the **method** instead, so the reader can rebuild it, and say which tracked
path will own it.
