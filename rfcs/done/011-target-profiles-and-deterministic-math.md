# RFC 011 — Target Profiles and Deterministic Math Policy

**Status.** Implemented (v0.17.0) — `xtask/target-profiles.toml` is the machine-readable target-profile manifest and `cargo xtask target-profiles` is manifest-driven, with mandatory, advisory-installed, and documented-only evidence classes.
**Tracks.** Cross-cutting target, floating-point, reproducibility, and device release policy
**Touches.** `xtask/target-profiles.toml`, `xtask/src/checks/target_profiles.rs`, `.cargo/config.toml`, workspace profiles, `loeres-device` build profiles, `loeres-backend-static` target assumptions, documentation for supported targets

---

### Extended Metadata
* **Rust Edition Compliance:** Rust 2024 Baseline
* **Target Environment:** `loeres-device`, `loeres-backend-static`, `loeres`, `loeres-cluster`, and cluster/device conformance comparison

## 1. Executive Summary & Problem Statement

Loeres cannot honestly promise universal bit-for-bit numerical behavior across every CPU, FPU, compiler profile, and math backend. The correct contract is profile-scoped:

* device execution must be bounded, panic-averse, and reproducible within a declared target profile;
* cross-target and cross-backend comparisons must be tolerance-based, not bitwise identity by default;
* target triples, panic strategy, floating-point assumptions, scalar family, feature set, and conformance group must be explicit release inputs.

Earlier RFC 011 drafts framed this as required before implementing the first device solver. The repository has moved beyond that: RFC 006 shipped the device projected-first-order kernel, RFC 016 shipped the std-side cluster analog, and RFC 010 now owns the aggregate verification command namespace. This RFC therefore freezes the implementation-era target-profile vocabulary required before making stronger target/profile release claims and before RFC 013 promotes profile-aware conformance fixtures.

## 2. Architectural Context & Dependency Alignment

This RFC does not add runtime dependencies. It constrains host-side tooling, CI target claims, deterministic-language policy, and public documentation.

Dependency alignment:

| Crate | Relationship to this RFC | Dependency impact |
|---|---|---|
| `loeres` | Must compile under no-std device profiles | No new dependencies |
| `loeres-backend-static` | Must compile under reference device profiles | No `std`, no `alloc` |
| `loeres-device` | Must follow target-profile release gates | No `std`, no `alloc` |
| `loeres-backend-std` | Participates in cluster counterpart profiles and RFC 013 parity | No edge impact |
| `loeres-cluster` | Provides host/cluster profile path and std-side comparison path | `std` allowed |
| `xtask` | Parses the target-profile manifest and verifies profiles | Host-only; may use `std` and TOML parsing |

`xtask/target-profiles.toml` is host-only tooling data. It must not be parsed by `loeres`, `loeres-backend-static`, or `loeres-device`.

## 3. Concrete Technical Specification

### 3.1 Enforcement classes

Every target profile must have one enforcement class:

| Class | Aggregate behavior | Meaning |
|---|---|---|
| `mandatory` | Missing target/tool or failed command fails `cargo xtask check` | Required release gate for the current repository release. |
| `advisory-installed` | If the target/tool is installed, failed command fails; if missing, report `advisory unavailable` and continue | Portability signal without making standard CI depend on optional targets. |
| `documented-only` | Report profile metadata only; do not compile | A named profile exists for review and future promotion but is not built by this release. |

Advisory and documented-only profiles must never be summarized as enforced passes. This follows the RFC 010 rule that advisory checks and owner-RFC hooks cannot look like mandatory verification.

### 3.2 v0.17.0 target profile set

Loeres target support must be expressed as named profiles, not scattered target-triple mentions.

The v0.17.0 mandatory profile set is intentionally small:

| Profile name | Target | Class | Required behavior |
|---|---|---|---|
| `cluster-linux-host` | `host` | `mandatory` | Run `cargo check -p loeres-cluster --all-features`; command output records the effective host triple from `rustc -vV`. |
| `device-thumbv7em-hardfloat` | `thumbv7em-none-eabihf` | `mandatory` | Build `loeres-device --no-default-features` for the reference no-std hard-float MCU target using the declared device panic policy. |

The v0.17.0 advisory and documented profiles are:

| Profile name | Target | Class | Required behavior |
|---|---|---|---|
| `device-thumbv7em-softfloat` | `thumbv7em-none-eabi` | `advisory-installed` | If the target is installed, run `cargo check -p loeres-device --target thumbv7em-none-eabi --no-default-features`; if missing, report advisory unavailable. |
| `device-riscv32-advisory` | `riscv32imac-unknown-none-elf` | `advisory-installed` | If the target is installed, run a portability check; if missing, report advisory unavailable. |
| `wasm32-no-threads` | `wasm32-unknown-unknown` | `documented-only` | Document no-threads portability intent only; no v0.17.0 build gate or conformance path. |
| `cluster-linux-aarch64` | `aarch64-unknown-linux-gnu` | `documented-only` | Document deployment target only; local `xtask` does not require a cross-runner or linker. |

Only named profiles may be used as release gates. Adding or promoting a release-gated profile requires updating this RFC or a successor RFC.

The soft-float counterpart deliberately stays on the `thumbv7em` Cortex-M4/M7 class rather than `thumbv7m`, so soft-float verification does not change the reference ISA or mislead developers about FPU expectations. RISC-V is an advisory portability smoke target, not a v0.x release gate.

### 3.3 Deterministic-claim vocabulary

Loeres must avoid unqualified "deterministic" claims. Use these terms instead:

| Term | Meaning | Required for |
|---|---|---|
| Bounded execution | Memory use, iteration count, and failure exits are bounded by public configuration and workspace shape | All device solvers |
| Same-profile reproducibility | Same profile, scalar family, solver config, validation policy, and fixture produce compatible status and tolerance results | Device release evidence |
| Cross-profile parity | Different profiles or backends are compared by RFC 013 tolerance categories | Conformance evidence |
| Bitwise identity | Exact binary-identical scalar output | Not a baseline v0.x promise |

Device documentation may say "target-profile-scoped reproducibility" or "same-profile reproducibility" only when the profile is named. Any claim of bitwise identity must be rejected unless a later RFC scopes it to a specific target/profile/scalar/solver combination.

### 3.4 Floating-point policy

For floating-point scalar implementations:

1. The default device release profile must prohibit compiler settings that intentionally relax IEEE-like operation ordering for speed.
2. Fast-math-like behavior must not be silently enabled by Loeres features.
3. The target profile must document whether hardware FPU, software floating point, no FPU, or host floating point is expected.
4. Algorithms must not rely on NaN payload preservation, signed-zero distinctions, or target-specific exception flags unless explicitly stated by a solver RFC.
5. Primitive float implementations must validate finite inputs before solve hot paths unless a validated/trusted state is explicitly supplied under RFC 012.

### 3.5 Cargo feature and profile naming

A Cargo feature named `deterministic-math` may exist later, but it must not be treated as sufficient by itself. The release gate is the combination of:

* named target profile;
* target triple or host profile;
* Rust toolchain version and host triple emitted by `xtask`;
* Cargo command and feature set;
* panic strategy / applied rustflags;
* scalar family;
* solver configuration;
* validation state;
* conformance corpus group.

The feature may select stricter Loeres code paths, but it cannot control all compiler and hardware behavior alone.

### 3.6 Panic strategy

Device release profiles must use `panic = "abort"` or equivalent no-unwind target rustflags. This is not a substitute for panic-path auditing and it is not proof of panic absence. It is a containment policy if a panic path survives unexpectedly.

For v0.17.0, `device-thumbv7em-hardfloat` uses manifest-driven rustflags:

```toml
rustflags = ["-C", "panic=abort"]
```

`cargo xtask target-profiles` must own the build invocation and print the exact applied rustflags. If it cannot mechanically observe Cargo's final panic strategy, it may only claim that it controlled the invocation and printed the applied flags. It must not claim formal panic absence or verified final panic strategy.

Cluster profiles may use normal unwind behavior unless a later deployment profile requires otherwise.

### 3.7 Constant-iteration mode

`loeres-device` may expose a constant-iteration timing mode. This mode means:

* the solver executes the configured number of iterations even if convergence is detected early;
* the final public outcome may still report convergence, non-convergence, or invalid input;
* timing stabilization is best-effort and scoped to the configured target profile;
* it is not a cryptographic constant-time guarantee.

The mode must be selected by runtime configuration, not by const-generic policy values.

### 3.8 Target profile manifest

The repository must include a machine-readable manifest consumed by `cargo xtask target-profiles`:

```text
xtask/target-profiles.toml
```

The manifest schema is versioned from the first implementation:

```toml
schema_version = 1

[[profiles]]
name = "cluster-linux-host"
class = "mandatory"
environment = "cluster"
target = "host"
package = "loeres-cluster"
command = "check"
default_features = true
features = ["all"]
panic_strategy = "target-default"
fpu = "host"
scalar_family = "host"
size_budget_group = "cluster-host"
conformance_group = "cluster-reference-smoke"

[[profiles]]
name = "device-thumbv7em-hardfloat"
class = "mandatory"
environment = "device"
target = "thumbv7em-none-eabihf"
package = "loeres-device"
command = "build"
default_features = false
features = []
panic_strategy = "abort"
fpu = "hardware-single"
scalar_family = "float"
size_budget_group = "device-reference"
conformance_group = "device-reference-smoke"
rustflags = ["-C", "panic=abort"]

[[profiles]]
name = "cluster-linux-aarch64"
class = "documented-only"
environment = "cluster"
target = "aarch64-unknown-linux-gnu"
panic_strategy = "target-default"
fpu = "host"
scalar_family = "host"
size_budget_group = "cluster-aarch64"
conformance_group = "cluster-advisory"
```

Field requirements:

| Field | Required | Notes |
|---|---:|---|
| `schema_version` | yes | Must be `1` for v0.17.0. |
| `name` | yes | Stable profile ID used by RFC 013 and release notes. |
| `class` | yes | `mandatory`, `advisory-installed`, or `documented-only`. |
| `environment` | yes | `cluster`, `device`, or `portability`. |
| `target` | yes | Rust target triple or `host`; `host` is allowed only for cluster/host profiles. |
| `package` | yes for buildable profiles | Required for `mandatory` and `advisory-installed`; omitted for `documented-only` unless a later RFC promotes it. |
| `command` | yes for buildable profiles | `check` or `build`. |
| `default_features` | yes for buildable profiles | Device baseline must be `false`. |
| `features` | yes for buildable profiles | Empty list is explicit; `"all"` means `--all-features`. |
| `panic_strategy` | yes | `abort`, `unwind`, or `target-default`; device release profiles use `abort`. |
| `fpu` | yes | `hardware-single`, `software`, `host`, `none`, or `unspecified`. |
| `scalar_family` | yes | `float`, `fixed-point-future`, `integer-like-future`, or `host`. |
| `size_budget_group` | yes | Metadata consumed by `size-budget`; thresholds remain owner-RFC concerns. |
| `conformance_group` | yes | Metadata consumed by RFC 013; fixtures are not enforced by RFC 011. |
| `rustflags` | optional | Used when a profile requires explicit flags not encoded elsewhere. |

Validation rules:

1. Unknown enum values fail schema validation.
2. Profile names must be unique.
3. `mandatory` and `advisory-installed` profiles must provide `package`, `command`, `default_features`, and `features`.
4. `documented-only` profiles may omit build fields, but must provide all metadata fields.
5. Device profiles must use explicit Rust target triples; they may not use `target = "host"`.
6. The command must not install Rust targets, fetch toolchains, or mutate the developer's toolchain state.

### 3.9 `cargo xtask target-profiles` behavior

`cargo xtask target-profiles` must:

1. parse `xtask/target-profiles.toml`;
2. validate schema version, unique names, enum values, and buildable/documented-only field requirements;
3. print `rustc -vV`, including active host triple;
4. for each profile, print name, class, target, package, command, features, panic strategy, FPU, scalar family, size group, conformance group, and applied rustflags;
5. fail on missing target/tool or command failure for `mandatory` profiles;
6. for `advisory-installed` profiles, run only when the target and required tools are installed; fail on command failure when run; report `advisory unavailable` when missing;
7. list `documented-only` profiles without compiling them;
8. end with an evidence-status summary grouped by enforcement class:
   * mandatory passed / failed;
   * advisory installed passed / failed / unavailable;
   * documented-only listed;
   * conformance groups emitted as metadata only, with fixtures deferred to RFC 013.

Missing optional advisory targets are not aggregate failures. Installed advisory targets that fail their command are real failures for `target-profiles`, because they indicate visible portability drift in the developer or CI environment that chose to install them.

### 3.10 RFC 013 handoff

RFC 011 owns profile names and metadata. RFC 013 owns fixtures, tolerance thresholds, and numerical parity comparisons.

Required handoff fields:

| RFC 011 field | RFC 013 use |
|---|---|
| `name` | Fixture target/profile selector |
| `environment` | Device vs cluster path selection |
| `scalar_family` | Fixture scalar compatibility |
| `conformance_group` | Smoke/extended/advisory suite membership |
| `fpu` | Cross-profile floating-point expectation metadata |
| `panic_strategy` | Release-evidence metadata, not a numerical comparison input |

`cargo xtask conformance` remains a not-enforced RFC 013 hook until RFC 013 adds fixtures. RFC 011 must not make fixture absence a failure.

## 4. Rust Systems-Level Nuances & Memory Safety

Floating-point behavior may change with target CPU features, LLVM code generation, optimization level, and linked math libraries. Loeres must therefore avoid target-independent bitwise claims.

For device builds, this RFC requires avoiding:

* hidden calls into host math libraries;
* formatting paths in hot loops;
* panic-unwind behavior for release-gated device profiles;
* runtime allocation;
* background threads;
* target-feature drift between CI and release builds.

For fixed-point implementations, RFC 001 and solver RFCs must define overflow and rounding behavior. This RFC only requires that target profiles identify when fixed-point is the preferred deterministic scalar family.

## 5. Algorithmic & Numerical Fail-Safe Guardrails

Solver RFCs must state their numerical tolerance policies in terms compatible with this RFC.

Minimum public policy:

1. Same-profile repeated runs must classify outcomes consistently for the same scalar/backend/configuration/corpus fixture.
2. Cross-profile runs are compared with tolerance thresholds defined by RFC 013 and solver-specific RFCs.
3. `InvalidInput`, `NumericalDomain`, `IllConditioned`, and the core status pair `SolveStatus::NotConverged` + `TerminationReason::IterationCap` (RFC 014) must remain stable public outcome categories across profiles.
4. A profile that cannot run finite-input checks must not claim standard device release support.
5. Fast-math-like transformations are out of the safety-oriented default profiles.

## 6. Verification, Validation, and CI Gates

Acceptance gates for the RFC 011 implementation:

1. Enforced: `cargo xtask target-profiles` validates `xtask/target-profiles.toml` schema version, unique names, enum values, and required fields.
2. Enforced: `cluster-linux-host` runs `cargo check -p loeres-cluster --all-features` and records the effective host triple.
3. Enforced: `device-thumbv7em-hardfloat` builds `loeres-device --no-default-features` for `thumbv7em-none-eabihf` with manifest rustflags printed, including `-C panic=abort`.
4. Enforced: the target-profile command summary separates mandatory, advisory-installed, documented-only, and RFC 013 conformance metadata.
5. Advisory: `device-thumbv7em-softfloat` and `device-riscv32-advisory` run when their targets are installed; missing optional targets report advisory unavailable and do not fail the aggregate gate.
6. Documented-only: `wasm32-no-threads` and `cluster-linux-aarch64` are listed with metadata and not compiled by v0.17.0 local `xtask`.
7. Enforced: no command installs targets, fetches toolchains, or mutates toolchain state.
8. Enforced: broad bitwise identity claims are absent unless a later RFC explicitly scopes them to a specific target/profile/scalar/solver combination.
9. Handoff: `cargo xtask conformance` remains not-enforced until RFC 013 supplies fixtures, but target-profile output emits conformance groups as metadata.

## 7. Implementation Closeout

RFC 011 shipped in v0.17.0 as a tooling and documentation release. Runtime crate
APIs are unchanged.

Implementation summary:

1. Added `xtask/target-profiles.toml` with schema version `1`.
2. Replaced the hardcoded RFC 010 interim `target-profiles` command with
   manifest parsing, schema validation, profile execution, rustflags injection,
   and evidence-status summaries.
3. Enforced the v0.17.0 mandatory profiles:
   `cluster-linux-host` and `device-thumbv7em-hardfloat`.
4. Reported `device-thumbv7em-softfloat` and `device-riscv32-advisory` as
   `advisory-installed`; missing optional targets are advisory unavailable, not
   aggregate failures.
5. Reported `wasm32-no-threads` and `cluster-linux-aarch64` as
   `documented-only`.
6. Kept RFC 013 conformance fixtures out of scope; conformance groups are
   metadata only until RFC 013 lands.

The implementation deliberately keeps `xtask` dependency-free in v0.17.0 by
parsing the constrained manifest schema directly. This remains host-only tooling
and creates no runtime dependency impact.
