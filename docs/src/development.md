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
xtask/                         # repository automation (never a library dependency)
```

## Everyday commands

```sh
cargo check --workspace --all-features
cargo test  --workspace
cargo fmt --all            # run once after implementation, before checks
cargo clippy --workspace --all-features -- -D warnings
```

## Verification gates (`xtask`)

```sh
cargo xtask zero-bleed     # no forbidden server <-> edge dependency edge exists
cargo xtask no-std         # edge crates build for thumbv7em-none-eabihf (no std/alloc)
cargo xtask check          # canonical developer architecture aggregate
cargo xtask release-gate   # fail-closed RFC 019 candidate gate; not a check alias
```

RFC 010 implements the stable command namespace: `check-rfcs`, `zero-bleed`,
`check-public-api`, `feature-matrix`, `target-profiles`, `panic-audit`,
`size-budget`, `unsafe-audit`, `conformance`, and `link-audit`.
The aggregate summary labels commands as enforced, advisory/reporting, or
owner-RFC hooks; threshold-less baselines and missing future corpora are not
reported as enforced verification passes.

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

## Workflow

Development is **design-first**: requirement / RFC → external design → internal
design → implementation → testing. New public-boundary work starts as an RFC
under `rfcs/proposed/` (see `rfcs/done/000-rfc-lifecycle-policy.md`).
