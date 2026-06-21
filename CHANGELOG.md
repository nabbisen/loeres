# Changelog

All notable changes to Loeres are recorded here. The format is loosely based on
Keep a Changelog, and the project follows semantic versioning. Versions below
`1.0.0` are pre-stability; a `1.0.0` release requires explicit project-owner
sign-off (see RFC 000 and the requirements specification).

## [0.3.0] — 2026-06-21 — Phase 0: Cargo workspace skeleton

First implementation phase (roadmap §12.1; external design §1). This release
instantiates the workspace structure and the verification gates the structure
can already satisfy. It contains **no** solver, scalar, access, or validation
logic — those land in Milestone 1+. Design-before-code is preserved: the
skeleton realizes already-accepted structure and does not pre-empt the open
design rounds.

### Added

- **Cargo workspace** (`resolver = "3"`, edition 2024, MSRV 1.85) with the five
  crates and shared metadata via `[workspace.package]`:
  - `loeres-core` — `#![no_std]`, no `alloc`, `#![forbid(unsafe_code)]`, no deps.
  - `loeres-backend-static` — `#![no_std]`, no `alloc`; depends on `loeres-core`.
  - `loeres-device` — `#![no_std]`, no `alloc`, `#![forbid(unsafe_code)]`;
    depends on `loeres-core` + `loeres-backend-static`.
  - `loeres-backend-std` — `std`; depends on `loeres-core`.
  - `loeres-cluster` — `std`; depends on `loeres-core` + `loeres-backend-std`.
  Each crate carries its public module topography (external design §1.5) as
  documented placeholder modules, each tracing to its owning RFC. The feature
  surface from external design §1.6 is declared (no-op until its RFC wires it).
- **`xtask` automation crate** with the gates the skeleton supports implemented
  for real — `zero-bleed` (forbidden server↔edge dependency edges, roadmap §5.5),
  `no-std` (edge crates build for `thumbv7em-none-eabihf`), `check`, and an
  aggregate `release-gate` — plus the remaining RFC 010 / §5.4 gates registered
  as honest scaffolds. `cargo xtask <cmd>` alias added.
- **`rust-toolchain.toml`** pinning stable + rustfmt/clippy + the bare-metal
  target; **CI workflows** (`ci`, `no-std`, `msrv`, `release`) wired to `xtask`;
  `.github/SECURITY.md` and issue templates.
- **Docs:** per-crate `README.md`; maintainer docs `docs/src/development.md`
  (local dev / xtask) and `docs/src/adr.md` (ADR index), wired into the mdbook
  `SUMMARY.md`.

### Verified

- `cargo check --workspace --all-features` — clean.
- `cargo clippy --workspace --all-features -- -D warnings` — clean.
- `cargo xtask zero-bleed` — **PASS** (no forbidden dependency edge).
- `cargo xtask no-std` — **PASS** (`loeres-core`, `loeres-backend-static`,
  `loeres-device` build `no_std`/no-`alloc` for `thumbv7em-none-eabihf`).
- `cargo fmt --all` applied.

This meets the Phase 0 acceptance criteria (roadmap §12.1): the workspace
compiles with placeholder crates, edge-facing crates have no forbidden
dependency path, and the docs explain the server/edge split.

### Release audit

- **Security.** No executable application logic, data flows, external
  integrations, or auth were introduced — the crates expose no public API and
  the only runtime code (`xtask`) is a local dev tool that shells out to cargo.
  No new attack surface; the design-level threat model and its controls
  (compile-time server/edge isolation, FFI cluster-only/default-off,
  panic-aversion, boundary validation) remain valid. The structural isolation
  control is now **machine-enforced** by `zero-bleed` + `no-std` rather than
  asserted only in prose. `SECURITY.md` added.
- **Docs consistency.** README, ROADMAP, and CHANGELOG reflect the Phase 0
  state; the workspace layout matches external design §1.1/§1.5.

### Deferred (unchanged from v0.2.0)

- `examples/` (cluster/device) arrive with their solver milestones (M2/M3).
- Requirements §5.1.2 base-scalar wording flag remains open for the architect;
  it gates Milestone 1 scalar code, not this skeleton.

## [0.2.0] — 2026-06-21 — RFC 001 `OrderedScalar` split resolved

Design / governance baseline increment. This release resolves the first open
design round (RFC 001 — `OrderedScalar` scalar-tier split) and reconciles the
design-layer documents with it. No implementation code is included yet; coding
still follows the design-before-code workflow once Phase 0 (workspace skeleton)
lands.

### Changed

- **RFC 001 — Stratified Scalar Capability Model: five tiers → six tiers.**
  Adds `OrderedScalar` as Tier 2 (between `BaseScalar` and `FiniteScalar`):
  - `BaseScalar` now requires only `Copy + Clone + PartialEq + Sized` — it no
    longer requires `PartialOrd` or `core::fmt::Debug`. Ordering, `min`, `max`,
    and `clamp` move to `OrderedScalar`.
  - `OrderedScalar: BaseScalar + PartialOrd` defines Loeres-owned `min` / `max` /
    `clamp` with a **NaN-propagating** contract for floating-point (deliberately
    unlike `f64::min` / `f64::max`); `clamp` is panic-free with a documented
    `lo <= hi` precondition validated at the solve boundary.
  - Supertrait graph: `FiniteScalar: BaseScalar`, `DivisibleScalar: BaseScalar`,
    `MetricScalar: OrderedScalar`, `AdvancedNumericalScalar: DivisibleScalar +
    MetricScalar`. A `MetricScalar` bound therefore implies `OrderedScalar`.
  - `DivisibleScalar::checked_div` must not return `Ok` containing NaN/∞: finite
    operands whose quotient is non-finite return `Err` (`Overflow` / numerical
    domain), keeping near-zero conditioning a solver-level `MetricScalar` concern.
  - `AdvancedNumericalScalar` for primitive floats is **not** baseline core work
    (requires `libm` or a later adapter decision); transcendentals stay out of
    baseline core.
  - `epsilon()` accepted only as a provisional name (candidate
    `algorithmic_epsilon()`); to be re-decided by RFC 006 / RFC 013 before first
    public release.
  - New verification: ordering/NaN tests (§6.4) and scalar-law tests (§6.5).
- **External design reconciled to six tiers.** §2.2 scalar-family table adds the
  `OrderedScalar` row and corrects `BaseScalar` (equality only, no ordering);
  §2.3 adds an `OrderedScalar` opt-in row; §9 open question #2 (whether
  `BaseScalar` requires `PartialOrd`) is marked **resolved**.
- **Roadmap reconciled to six tiers.** §2.3 (RFC 001) capability table adds the
  `OrderedScalar` row; the "must not require division" constraint becomes "must
  not require ordering or division"; the `PartialOrd`-sufficiency and NaN-
  semantics risks are annotated as resolved.
- **ROADMAP.md / README.md** updated: open design round #1 (RFC 001
  `OrderedScalar`) is resolved; the README Design Notes describe the six tiers.

### Known reconciliation flag (deferred to the architect)

- **Requirements §5.1.2** still describes the base scalar as having
  "equality/ordering behavior", which now contradicts the six-tier `BaseScalar`
  (equality only). This apex requirements wording was **left unchanged** pending
  architect confirmation; the suggested amendment is to move "ordering" to the
  `OrderedScalar` capability. (Sibling RFCs 002/004/005/006/007 remain valid:
  `BaseScalar` is still the correct storage bound and `MetricScalar` now implies
  `OrderedScalar`, so RFC 006's box-projection step gains `clamp` for free.)

### Release audit

- **Security.** Documentation/RFC-only change — no executable code, data flows,
  external integrations, or auth logic — so no new attack surface is introduced.
  The design-level threat model (requirements §8; external design §5;
  `docs/src/threat-model.md`) and its controls (compile-time server/edge
  isolation, FFI restricted to the cluster crate and default-off, boundary
  validation, panic-aversion) remain valid and unchanged.
- **Documentation consistency.** The scalar model is now uniform across RFC 001,
  the external design, and the roadmap (no residual "five-tier" wording and no
  `BaseScalar`-with-ordering statements outside the flagged requirements line).

## [0.1.0] — 2026-06-21 — Design baseline

First release. This is a **design / governance baseline**: the public boundary,
crate topology, and contracts are frozen as accepted and proposed RFCs. No
implementation code is included yet; coding follows the design-before-code
workflow once the remaining design rounds land.

### Added

- Governing specifications (`docs/specs/`): requirements, external design, and
  roadmap & milestones (all v1).
- RFC set under `rfcs/`:
  - `done/000` — RFC lifecycle policy.
  - `proposed/001`–`009` — Milestone 1–3 contracts (scalar capabilities;
    storage-agnostic access; allocation-free errors; static storage; typed
    workspace; deterministic device kernel; dynamic/sparse backend; async
    orchestration; observability/FFI).
  - `proposed/010`–`013` — cross-cutting contracts (xtask verification
    governance; target profiles & deterministic math; validation-state policy;
    conformance corpus & numerical parity).
  - `proposed/014` — core solver outcome & status taxonomy.
- Standard project scaffolding: `README.md`, `CONTRIBUTING.md`, `LICENSE`
  (Apache-2.0), `NOTICE`, and `ROADMAP.md`; plus an mdbook skeleton (`docs/src/`)
  with a Maintainers & Contributors section that bridges the rendered book to
  the raw specifications and RFCs.

### Changed (design reconciliation incorporated into this baseline)

- Introduced RFC 014 as the single owner of the `loeres_core::solver` taxonomy;
  non-convergence at the iteration cap is now a **status**
  (`SolveStatus::NotConverged` + `TerminationReason::IterationCap`), not a
  `SolverError`.
- Reconciled RFCs 003, 005, 006, 008, 010, 011, and 013 with RFC 014: the
  canonical 13-variant `SolverError` set (`u32` dimension payloads; no
  `MaxIterationsReached`; no runtime `PanicGateViolation`); device report
  derivation via `AsCoreReport`; per-item batch outcomes carrying a core report;
  `check-public-api` governance; `SolveStatus`-based conformance.
- Cross-document cleanup: unified the RFC folder scheme to `proposed/done/archive`
  (RFC 000), flat-renumbered the roadmap and external-design RFC references,
  inserted RFC 014 into the dependency graphs, and corrected the RFC 011 target
  profiles (`thumbv7em-none-eabi` soft-float; `riscv32imac-unknown-none-elf`
  advisory; f32-first hard-float reference).

### Release audit

- **Security.** This release contains documentation and RFCs only — no
  executable code, no data flows, no external integrations, and no
  authentication logic — so no new attack surface is introduced. The
  design-level threat model (requirements §8; external design §5; consolidated
  in `docs/src/threat-model.md`) remains valid, and its controls (compile-time
  server/edge isolation, FFI restricted to the cluster crate and default-off,
  boundary validation, panic-aversion) are preserved by the current RFC set.
- **Documentation consistency.** The governing docs were verified against the
  reconciled RFC set: no stale `MaxIterationsReached` / `ConvergenceStatus`
  terminology, no milestone-style RFC numbering, and no folder-scheme drift
  outside RFC 014's explanatory prose.

[0.3.0]: https://github.com/nabbisen/loeres/releases/tag/v0.3.0
[0.2.0]: https://github.com/nabbisen/loeres/releases/tag/v0.2.0
[0.1.0]: https://github.com/nabbisen/loeres/releases/tag/v0.1.0
