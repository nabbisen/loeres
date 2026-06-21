# RFC 011 — Target Profiles and Deterministic Math Policy

**Status.** Proposed
**Tracks.** Cross-cutting target, floating-point, reproducibility, and device release policy
**Touches.** `.cargo/config.toml`, workspace profiles, `xtask/target_profiles.rs`, `loeres-device` build profiles, `loeres-backend-static` target assumptions, documentation for supported targets

---

### Extended Metadata
* **Rust Edition Compliance:** Rust 2024 Baseline
* **Target Environment:** `loeres-device`, `loeres-backend-static`, `loeres-core`, and cluster/device conformance comparison

## 1. Executive Summary & Problem Statement

Loeres cannot honestly promise universal bit-for-bit deterministic numerical behavior across every CPU, FPU, compiler profile, and math backend. The correct design contract is more precise:

* device execution must be bounded, panic-averse, and reproducible within a declared target profile;
* cross-target convergence must be compared by tolerance, not bitwise equality;
* target triples, panic strategy, floating-point assumptions, and compiler flags must be explicit release inputs.

This RFC defines the target-profile and deterministic-math policy required before implementing the device solver and conformance suite.

## 2. Architectural Context & Dependency Alignment

This RFC does not add runtime dependencies. It constrains build profiles, CI targets, and public documentation.

Dependency alignment:

| Crate | Relationship to this RFC | Dependency impact |
|---|---|---|
| `loeres-core` | Must compile under no-std target profiles | No new dependencies |
| `loeres-backend-static` | Must compile under reference device profiles | No `std`, no `alloc` |
| `loeres-device` | Must follow deterministic-profile release gates | No `std`, no `alloc` |
| `loeres-backend-std` | Participates in parity testing as cluster counterpart | `std` allowed |
| `loeres-cluster` | Provides high-throughput comparison path | `std` allowed |
| `xtask` | Verifies profile compliance | Host-only |

## 3. Concrete Technical Specification

### 3.1 Target profile taxonomy

Loeres target support must be expressed as named profiles, not scattered target-triple mentions.

| Profile name | Purpose | Example target triple | Status meaning |
|---|---|---|---|
| `cluster-linux-x86_64` | Primary cluster host CI | `x86_64-unknown-linux-gnu` | Must build cluster defaults |
| `cluster-linux-aarch64` | Secondary cluster host CI | `aarch64-unknown-linux-gnu` | Should build cluster defaults |
| `device-thumbv7em-hardfloat` | Reference no-std device profile (f32-first: the M4F/M7F FPU is single-precision, so `f64` is not the reference performance path) | `thumbv7em-none-eabihf` | Must build device baseline |
| `device-thumbv7em-softfloat` | Soft-float ABI counterpart of the reference profile (same Cortex-M4/M7 class, no hardware FPU) | `thumbv7em-none-eabi` | Advisory; build-checked |
| `device-riscv32-advisory` | RISC-V portability smoke target | `riscv32imac-unknown-none-elf` | Advisory / non-blocking |
| `wasm32-no-threads` | Optional portability profile | `wasm32-unknown-unknown` | Advisory unless promoted |

Only named profiles may be used as release gates. Adding a release-gated profile requires updating this RFC or a successor RFC.

The soft-float counterpart deliberately stays on the `thumbv7em` (Cortex-M4/M7) class rather than `thumbv7m` (Cortex-M3), so that soft-float verification does not change the reference ISA or mislead developers about FPU expectations. RISC-V (`riscv32imac-unknown-none-elf`) is an advisory portability smoke target, not a v0.x release gate.

### 3.2 Determinism levels

Loeres must distinguish four levels of determinism:

| Level | Meaning | Required for |
|---|---|---|
| Bounded execution | Iteration count, memory use, and failure exits are bounded | All device solvers |
| Same-profile reproducibility | Same target/profile/input produces equivalent result classification and tolerance-converged values | Device release gate |
| Cross-profile numerical parity | Different targets/backends converge within declared tolerance | Conformance gate |
| Bitwise identity | Exact binary-identical scalar output | Not a baseline Loeres promise |

The design must avoid the phrase “deterministic” without naming which level is meant.

### 3.3 Floating-point policy

For floating-point scalar implementations:

1. The default device release profile must prohibit compiler settings that intentionally relax IEEE-like operation ordering for speed.
2. “Fast math” behavior must not be silently enabled by Loeres features.
3. The target profile must document whether hardware FPU, software floating point, or fixed-point scalar implementations are expected.
4. Algorithms must not rely on NaN payload preservation, signed-zero distinctions, or target-specific exception flags unless explicitly stated by a solver RFC.
5. Primitive float implementations must validate finite inputs before solve hot paths unless a validated/trusted state is explicitly supplied under RFC 012.

### 3.4 Cargo feature and profile naming

A Cargo feature named `deterministic-math` may exist, but it must not be treated as sufficient by itself. The release gate is the combination of:

* target triple;
* Rust toolchain version;
* Cargo profile;
* panic strategy;
* scalar implementation;
* solver configuration;
* validation state;
* conformance corpus version.

The feature may select stricter Loeres code paths, but it cannot control all compiler and hardware behavior alone.

### 3.5 Panic strategy

Device release profiles must use `panic = "abort"` or an equivalent no-unwind target configuration. This is not a substitute for panic-path auditing. It is a containment policy if a panic path survives unexpectedly.

Cluster profiles may use normal unwind behavior unless a deployment profile requires otherwise.

### 3.6 Constant-iteration mode

`loeres-device` may expose a constant-iteration timing mode. This mode means:

* the solver executes the configured number of iterations even if convergence is detected early;
* the final public outcome may still report convergence, non-convergence, or invalid input;
* timing stabilization is best-effort and scoped to the configured target profile;
* it is not a cryptographic constant-time guarantee.

The mode must be selected by runtime configuration, not by const-generic policy values.

### 3.7 Target profile manifest

The repository must include a machine-readable target profile manifest consumed by `cargo xtask target-profiles`.

The manifest must record:

* profile name;
* target triple;
* build command;
* required Rust components;
* panic strategy;
* required feature set;
* whether the profile is mandatory or advisory;
* expected size-budget group;
* expected conformance corpus group.

The exact file format is deferred to implementation, but it must be stable enough for CI review.

## 4. Rust Systems-Level Nuances & Memory Safety

Floating-point behavior may change with target CPU features, LLVM code generation, optimization level, and linked math libraries. The Loeres design must therefore avoid making target-independent bitwise claims.

For device builds, this RFC requires avoiding:

* hidden calls into host math libraries;
* formatting paths in hot loops;
* panic-unwind behavior;
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

## 6. Verification, Validation, and CI Gates

Acceptance gates:

1. `cargo xtask target-profiles` validates the machine-readable target profile manifest.
2. `loeres-core`, `loeres-backend-static`, and `loeres-device` compile for `device-thumbv7em-hardfloat` baseline once the crates exist.
3. Device builds use the required panic strategy.
4. The build log records target triple and Rust toolchain version.
5. `cargo xtask conformance` reports same-profile and cross-profile results separately.
6. Any claim of bitwise identity is rejected unless a later RFC explicitly scopes it to a specific target/profile/scalar combination.
