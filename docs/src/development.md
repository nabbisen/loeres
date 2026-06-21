# Local Development

Loeres is a Cargo **workspace**. Install the toolchain with
[rustup](https://rustup.rs/); the pinned channel, components, and bare-metal
target are declared in `rust-toolchain.toml`.

## Layout

```text
crates/loeres-core             # no_std, no alloc — shared contracts
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
cargo xtask check          # cargo check across the workspace
cargo xtask release-gate   # every gate implemented at the current milestone
```

Further gates (`panic-audit`, `size-budget`, `check-public-api`, …) are
registered as scaffolds and implemented in later milestones per RFC 010.

## Workflow

Development is **design-first**: requirement / RFC → external design → internal
design → implementation → testing. New public-boundary work starts as an RFC
under `rfcs/proposed/` (see `rfcs/done/000-rfc-lifecycle-policy.md`).
