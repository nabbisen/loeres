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

The two crates under `examples/` are **not** workspace members. They are listed
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
```

## Verification gates (`xtask`)

```sh
cargo xtask zero-bleed       # no forbidden server <-> edge dependency edge exists
cargo xtask no-std           # edge crates build for thumbv7em-none-eabihf (no std/alloc)
cargo xtask doc-currency     # bounded RFC 020/024 apex metadata/lifecycle/navigation assertions
cargo xtask review-evidence  # RFC 022 architecture-review citation and provenance integrity
cargo xtask supply-chain     # RFC 026 dependency advisories, licenses, bans, sources
cargo xtask examples         # RFC 023 per-example build and resolved-graph isolation
cargo xtask check            # canonical developer architecture aggregate
cargo xtask release-gate     # complete non-publishing RFC 019 candidate evidence
```

RFC 010 implements the stable command namespace: `check-rfcs`, `zero-bleed`,
`check-public-api`, `feature-matrix`, `target-profiles`, `panic-audit`,
`size-budget`, `unsafe-audit`, `conformance`, `doc-currency`, `review-evidence`,
`supply-chain`, `examples`, and `link-audit`.
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
Local runs record that no tag assertion occurred; tagged CI additionally proves
that the peeled canonical unprefixed SemVer tag equals `HEAD`. Evidence and the
archive remain in gate-owned ignored workspace state; temporary extraction
state is removed only after an ownership check. A passing local dry run is not
tagged-release evidence and neither mode publishes crates or creates a release.
Pre-existing `docs/book/` output or tracked changes fail closed. The candidate
gate expects the canonical Linux workflow tools `git`, GNU `tar`, and
`sha256sum`; the release workflow supplies the reviewed execution environment.

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

## Release version convention

After a release ships, `main`'s workspace version is bumped to the next patch
immediately, in the first ordinary post-release commit — never left at the
released value, which would let a later commit claim to be that release. The
released version itself is set authoritatively only in the release's own
finalization revision, which is what gets tagged; it is never edited into an
already-tagged commit. See RFC 024 for the apex-currency mechanics this
convention keeps green.

## Workflow

Development is **design-first**: requirement / RFC → external design → internal
design → implementation → testing. New public-boundary work starts as an RFC
under `rfcs/proposed/` (see `rfcs/done/000-rfc-lifecycle-policy.md`).
