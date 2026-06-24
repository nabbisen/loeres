# RFC 010 — xtask Verification Governance

**Status.** Proposed
**Tracks.** Cross-cutting verification infrastructure for all Loeres phases and milestones
**Touches.** `xtask/`, workspace `Cargo.toml`, CI workflows, `rfcs/README.md`, dependency-boundary checks, size-budget checks, conformance test orchestration

---

### Extended Metadata
* **Rust Edition Compliance:** Rust 2024 Baseline
* **Target Environment:** Verification tooling; not linked by `loeres`, `loeres-backend-static`, `loeres-device`, `loeres-backend-std`, or `loeres-cluster`

## 1. Executive Summary & Problem Statement

Loeres relies on mechanical gates to preserve its central architecture: `loeres`, `loeres-backend-static`, and `loeres-device` must remain free of `std`, `alloc`, hidden dependency bleed, unchecked panic paths, oversized diagnostics, broken RFC references, and accidental binary-size growth.

Many earlier RFCs refer to `xtask` checks as mandatory enforcement points. Without a first-class RFC for `xtask`, those checks would become informal promises rather than part of the public project governance model.

This RFC defines the public verification contract for `xtask`. It does not implement any numerical algorithm. It defines which checks exist, what they must prove, how failures are reported, and how the tool remains outside the runtime dependency graph.

## 2. Architectural Context & Dependency Alignment

`xtask` is a workspace tooling crate. It may use `std`, filesystem APIs, process spawning, `cargo_metadata`, TOML parsing, JSON parsing, and host-only diagnostic libraries because it is never linked into Loeres runtime crates.

Dependency alignment:

| Component | Relationship to this RFC | Runtime dependency impact |
|---|---|---|
| `xtask` | Owns verification commands | Host-only; may use `std` |
| `loeres` | Subject of checks | Must not depend on `xtask` |
| `loeres-backend-static` | Subject of zero-bleed and size checks | Must not depend on `xtask` |
| `loeres-device` | Subject of zero-bleed, panic-path, size, and target checks | Must not depend on `xtask` |
| `loeres-backend-std` | Subject of cluster-side dependency and feature checks | Must not depend on `xtask` |
| `loeres-cluster` | Subject of orchestration, feature, and size checks | Must not depend on `xtask` |

The governance rule is strict: `xtask` may inspect the workspace but must never become a dependency of production crates.

## 3. Concrete Technical Specification

### 3.1 Required command namespace

`xtask` must expose a stable command namespace:

```text
cargo xtask check
cargo xtask check-rfcs
cargo xtask zero-bleed
cargo xtask check-public-api
cargo xtask feature-matrix
cargo xtask target-profiles
cargo xtask panic-audit
cargo xtask size-budget
cargo xtask unsafe-audit
cargo xtask conformance
cargo xtask link-audit
```

`cargo xtask check` is the aggregate command and the canonical release gate. It must run all applicable checks for the current workspace state. A `cargo xtask release-gate` alias may be exposed for CI clarity; it must not be a separate primary command. A command-alias policy keeps future renames from fragmenting CI scripts.

### 3.2 RFC hygiene checks

`cargo xtask check-rfcs` must validate:

1. Every RFC file under `rfcs/proposed/`, `rfcs/done/`, and `rfcs/archive/` follows the `NNN-slug.md` naming pattern, except optional `.gitkeep` files.
2. RFC numbers are unique across all folders.
3. Each RFC top-level `Status.` field matches the folder-derived state.
4. `rfcs/README.md` lists every RFC exactly once.
5. Relative Markdown links between RFCs resolve.
6. A moved RFC must not leave stale inbound links to its prior folder.

This command implements the lifecycle hygiene expected by RFC 000.

### 3.3 Zero-bleed dependency checks

`cargo xtask zero-bleed` must inspect the resolved dependency graph for baseline device/core feature sets.

The check must fail if any of the following crates transitively pull in `std` or `alloc` through normal library dependencies:

* `loeres`
* `loeres-backend-static`
* `loeres-device`

The check must separately report:

* direct dependency violations;
* transitive dependency violations;
* feature-induced violations;
* dev-dependency exceptions used only in host-side tests.

Dev-dependencies may use `std` only when the target being compiled is a host-side test target and not a device artifact.

### 3.4 Feature-matrix checks

`cargo xtask feature-matrix` must compile the workspace under the canonical feature profiles defined by the external design and roadmap:

| Profile | Required behavior |
|---|---|
| `core-min` | `loeres` builds as `#![no_std]` with no `alloc` |
| `static-min` | `loeres-backend-static` builds without `std` or `alloc` |
| `device-min` | `loeres-device` builds without `std`, `alloc`, threads, logging frameworks, or heap-backed collections |
| `cluster-default` | `loeres-cluster` builds with approved `std` integrations |
| `cluster-ffi` | FFI gateway builds only when explicitly enabled |

The command must fail on mutually exclusive feature combinations rather than silently choosing precedence.

### 3.5 Target-profile checks

`cargo xtask target-profiles` must build or check the target profiles defined by RFC 011. At minimum, it must include one host cluster target and one reference device target.

The command must record:

* Rust toolchain channel and version;
* target triple;
* profile flags;
* panic strategy;
* floating-point assumptions;
* binary size output when available.

### 3.6 Panic-path checks

`cargo xtask panic-audit` must verify that device hot-path entrypoints are panic-averse.

The check must combine multiple techniques rather than relying on a single tool:

1. static text scan for banned operations in core/device hot-path modules: `unwrap`, `expect`, unchecked indexing in solver kernels, formatting macros, and `panic!`;
2. compiler or linker-based checks where supported;
3. test profiles that force `panic = "abort"` for device artifacts;
4. explicit allowlist for tests and host-only examples.

The command may not claim formal proof. Its output must say whether the configured release gate passed. Panic-gate findings are reported as a CI/release-gate result owned here, not as a runtime `SolverError` variant; RFC 003 no longer carries `PanicGateViolation`.

### 3.7 Size-budget checks

`cargo xtask size-budget` must measure and compare:

* `.text` size;
* `.rodata` size;
* stack-sensitive public error and diagnostic type sizes;
* device artifact binary size;
* cluster monomorphization growth when multiple scalar/backend combinations are enabled.

The exact byte budgets are owned by RFC 003, RFC 006, RFC 008, and RFC 011. This RFC defines only the existence and common reporting format of the checker.

### 3.8 Unsafe audit checks

`cargo xtask unsafe-audit` must identify every `unsafe` block, `unsafe impl`, `extern` block, FFI wrapper, and raw-pointer conversion in the workspace.

Each occurrence must be classified:

* forbidden in core/device baseline;
* allowed only in `loeres-backend-std` adapter internals;
* allowed only in explicit FFI gateway modules;
* test-only.

Every allowed unsafe occurrence must link to an RFC section that justifies it.

### 3.9 Conformance orchestration

`cargo xtask conformance` must run the shared corpus defined by RFC 013. It must compare equivalent problem instances across device and cluster paths using tolerance-based convergence criteria, not bitwise equality.

The command must support:

* a fast smoke corpus;
* an extended corpus;
* deterministic seed reporting;
* per-instance failure classification.

### 3.10 Public-API surface checks

`cargo xtask check-public-api` must scan public signatures in `loeres`, `loeres-backend-static`, and `loeres-device` and reject, unless explicitly allowlisted by an accepted RFC:

* `Vec`, `String`, `Box`, `Rc`, `Arc`, `HashMap`, `BTreeMap`;
* `std::*` / `alloc::*` types;
* async runtime handles, OS thread handles, logging/tracing/FFI handle types;
* `dyn Trait` in device/core hot-path public interfaces, including `dyn AsCoreReport` (RFC 014 §4.2).

It must additionally reject (RFC 014):

* any `SolverError` variant denoting non-convergence at the iteration cap;
* any device-facing terminal status category not derivable from `loeres::solver::SolveStatus`.

The scanner is source-level at first; it need not be perfect in v0.x, but it must be mandatory and part of the aggregate `check`. An `xtask/public-api-allowlist.toml` (or equivalent) may exempt a signature, but every entry must cite an accepted RFC ID; entries without a valid RFC reference fail the check.

## 4. Rust Systems-Level Nuances & Memory Safety

`xtask` is allowed to be ordinary host-side Rust. However, it must not create false confidence by checking only the host build. The most important checks must operate on the actual target profiles used by device and cluster artifacts.

Special care is required for dependency checks because `cargo metadata` reports dependency graphs, not direct language-level use of `std`. `zero-bleed` must therefore combine dependency graph inspection with target compilation checks for no-std profiles.

Size checks must avoid depending on a single platform-specific tool. The preferred strategy is:

1. use `cargo` artifact metadata where sufficient;
2. use `llvm-size` or equivalent when available;
3. fail with a clear “measurement unavailable” error when the required toolchain component is missing, unless the check is explicitly run in advisory mode.

## 5. Algorithmic & Numerical Fail-Safe Guardrails

This RFC does not define numerical algorithms. Its fail-safe role is procedural:

* If a required verification command cannot run, the result is not treated as success.
* If a check is advisory, it must be labeled advisory in the command output.
* If an unsupported target profile is requested, the command must fail with a structured message naming the missing target or tool.
* If conformance results differ from the expected tolerance policy, the command must preserve per-instance diagnostics for review.

`xtask` must never “fix” source files automatically during mandatory CI checks unless invoked through a separate explicit formatting or repair command.

## 6. Verification, Validation, and CI Gates

This RFC is accepted only when the project agrees on the command namespace and required failure semantics. Implementation acceptance requires:

1. `cargo xtask check-rfcs` validates RFC 000 through all proposed RFCs.
2. `cargo xtask zero-bleed` fails on an intentionally injected illegal dependency edge.
3. `cargo xtask feature-matrix` compiles canonical profile combinations.
4. `cargo xtask target-profiles` compiles or checks at least one cluster and one device target.
5. `cargo xtask panic-audit` detects an intentionally injected `unwrap()` in a device hot path.
6. `cargo xtask size-budget` reports at least one device binary and one public error type size.
7. `cargo xtask conformance` runs the smoke corpus once corpus fixtures exist.
8. CI runs `cargo xtask check` before any RFC is moved from `proposed/` to `done/`.
