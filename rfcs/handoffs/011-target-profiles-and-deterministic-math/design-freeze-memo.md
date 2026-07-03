# RFC 011 - Design-Freeze Memo

**Artifact.** [`rfcs/done/011-target-profiles-and-deterministic-math.md`](../../done/011-target-profiles-and-deterministic-math.md)
**Decision pass.** v1, after RFC 010 v0.16.1 verification-governance hardening.
**Target release.** v0.17.0.
**Scope.** Freeze RFC 011 before implementation: target profile taxonomy,
deterministic-math claim language, manifest shape, `xtask target-profiles`
enforcement classes, and RFC 013 integration.

---

## Verdict

RFC 011 is the right next design item, but the current proposed text is not
ready for implementation as-is.

The core policy is sound: Loeres must make target/profile-scoped claims, must
avoid broad bitwise reproducibility promises, and must make target triples,
panic strategy, FPU assumptions, scalar family, feature set, and conformance
group explicit release inputs.

The freeze blocker is currency: the RFC still says it is required before the
device solver exists, while the device kernel, cluster orchestration, std-side
cluster kernel, observability/gateway boundary, and RFC 010 gates are already
implemented. RFC 011 must be patched into an implementation-era contract before
code changes land.

## D1 - Current-repo scope

RFC 011 must be written against the repository as of v0.16.1:

| Area | Current state | RFC 011 implication |
|---|---|---|
| Device kernel | RFC 006 is implemented | RFC 011 profiles govern shipped device behavior; they are not a precondition for inventing the first kernel. |
| Cluster kernel | RFC 016 is implemented | Cluster profile names and deterministic-claim boundaries must cover the std-side projected-first-order path. |
| Verification governance | RFC 010 is implemented | `cargo xtask target-profiles` must graduate from interim hardcoded checks to manifest-driven checks. |
| Conformance | RFC 013 is proposed | RFC 011 must define profile names and conformance groups, but RFC 013 owns fixtures and parity thresholds. |
| Target installation | Only `thumbv7em-none-eabihf` is pinned in `rust-toolchain.toml` today | Mandatory profiles must be enforceable now; advisory profiles must not make the aggregate gate depend on locally missing targets. |

## D2 - Profile classes and enforcement

The implementation must classify every profile as one of:

| Class | Aggregate behavior | Meaning |
|---|---|---|
| `mandatory` | Missing target/tool or failed build fails `cargo xtask check` | Required release gate for v0.17.0. |
| `advisory-installed` | If target/tool is installed, failed build fails; if missing, report `advisory unavailable` and continue | Portability signal without making standard CI depend on optional targets. |
| `documented-only` | Report metadata only | A named profile exists for future review, but is not compiled by this release. |

The v0.17.0 mandatory set should be intentionally small:

| Profile | Target | Class | Required command |
|---|---|---|---|
| `cluster-linux-host` | current host Linux triple | `mandatory` | `cargo check -p loeres-cluster --all-features` |
| `device-thumbv7em-hardfloat` | `thumbv7em-none-eabihf` | `mandatory` | `cargo build -p loeres-device --target thumbv7em-none-eabihf --no-default-features` with the device panic policy |

The v0.17.0 advisory set should be present in the manifest but not required to
be installed by standard CI:

| Profile | Target | Class | Rationale |
|---|---|---|---|
| `device-thumbv7em-softfloat` | `thumbv7em-none-eabi` | `advisory-installed` | Same Cortex-M4/M7 ISA class, soft-float ABI portability. |
| `device-riscv32-advisory` | `riscv32imac-unknown-none-elf` | `advisory-installed` | Embedded portability smoke target. |
| `wasm32-no-threads` | `wasm32-unknown-unknown` | `documented-only` | No-threads portability target; not a real-time claim. No v0.17.0 conformance path exists yet. |
| `cluster-linux-aarch64` | `aarch64-unknown-linux-gnu` | `documented-only` initially | Important deployment profile, but cross-compilation/toolchain availability is outside the default local gate. |

RFC 011 may name additional cluster profiles from the external design, but only
profiles with an explicit class may appear in the machine-readable manifest.

## D3 - Manifest file and schema

The target profile manifest should live with tooling, not runtime crates:

```text
xtask/target-profiles.toml
```

It is host-only data consumed by `xtask`; it must not be parsed by `loeres`,
`loeres-backend-static`, or `loeres-device`.

Required schema:

```toml
schema_version = 1

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
```

Field requirements:

| Field | Required | Notes |
|---|---:|---|
| `schema_version` | yes | Versioned before adoption so future schema edits are explicit. |
| `name` | yes | Stable profile ID used by RFC 013 and release notes. |
| `class` | yes | `mandatory`, `advisory-installed`, or `documented-only`. |
| `environment` | yes | `cluster`, `device`, or `portability`. |
| `target` | yes | Rust target triple or `host`. |
| `package` | yes for buildable profiles | Workspace package to build/check. |
| `command` | yes for buildable profiles | `check` or `build`. |
| `default_features` | yes for buildable profiles | Device baseline must be `false`. |
| `features` | yes for buildable profiles | Empty list is explicit. |
| `panic_strategy` | yes | `abort`, `unwind`, or `target-default`, with device release profiles using `abort`. |
| `fpu` | yes | `hardware-single`, `software`, `host`, `none`, or `unspecified`. |
| `scalar_family` | yes | `float`, `fixed-point-future`, `integer-like-future`, or `host`. |
| `size_budget_group` | yes | Consumed by `size-budget`; thresholds may remain advisory until owner RFCs freeze them. |
| `conformance_group` | yes | Consumed by RFC 013; no fixture enforcement in RFC 011. |
| `rustflags` | optional | Used when a profile requires explicit flags not encoded elsewhere. |

`xtask` may use TOML parsing because it is host-only. Adding a TOML parser to
`xtask` has no runtime dependency impact and stays inside RFC 010's tooling
allowance.

## D4 - Panic strategy enforcement

RFC 011 must distinguish three things:

1. panic-averse source scanning (`panic-audit`, RFC 006/RFC 010);
2. panic containment policy for a compiled target profile;
3. actual proof of panic absence, which Loeres does not claim.

For `device-thumbv7em-hardfloat`, v0.17.0 should enforce the build using
manifest-driven rustflags:

```toml
rustflags = ["-C", "panic=abort"]
```

If the implementation cannot mechanically observe Cargo's final panic strategy,
the command must at minimum own the build invocation and print the exact applied
rustflags. It must not claim to have verified a panic strategy it did not
control or observe.

Cluster profiles may remain `target-default` / unwind-capable unless a future
deployment profile tightens them.

## D5 - Floating-point and deterministic-claim language

RFC 011 should freeze these terms:

| Term | Meaning |
|---|---|
| Bounded execution | Memory use and iteration count are bounded by public configuration and workspace shape. |
| Same-profile reproducibility | Same profile, scalar family, solver config, validation policy, and fixture should produce compatible status and tolerance results. |
| Cross-profile parity | Different profiles or backends are compared by RFC 013 tolerance categories. |
| Bitwise identity | Not a baseline v0.x promise; may only be claimed by a later profile-specific RFC. |

The RFC must avoid unqualified "deterministic" language. Device docs may say
"target-profile-scoped deterministic" only when the profile is named.

Fast-math-like transformations remain out of the safety-oriented defaults.
Loeres features must not silently enable them.

## D6 - Constant-iteration mode

RFC 011 should preserve the current RFC 006 interpretation:

| Mode | Meaning |
|---|---|
| Early exit | Solver may stop when convergence is detected. |
| Constant iteration | Solver executes the configured iteration count even if convergence is detected earlier; it is not cryptographic constant time. |

No new const-generic policy is needed. Runtime configuration remains the correct
selection surface.

## D7 - RFC 013 integration

RFC 011 owns names and profile metadata. RFC 013 owns fixtures, tolerance
thresholds, and parity comparison implementation.

Required handoff fields from RFC 011 to RFC 013:

| RFC 011 field | RFC 013 use |
|---|---|
| `name` | Fixture target/profile selector. |
| `environment` | Device vs cluster path selection. |
| `scalar_family` | Fixture scalar compatibility. |
| `conformance_group` | Smoke/extended/advisory suite membership. |
| `fpu` | Interpretation of cross-profile float expectations. |
| `panic_strategy` | Release-evidence metadata, not numerical comparison. |

`cargo xtask conformance` should remain a not-enforced RFC 013 hook until RFC
013 adds fixtures. RFC 011 must not make fixture absence a failure.

## D8 - `xtask target-profiles` implementation shape

Implementation should be structured as:

1. parse `xtask/target-profiles.toml`;
2. validate schema version, unique profile names, valid classes, and valid
   environment/panic/FPU/scalar/conformance values;
3. print `rustc -vV` and the active host triple;
4. for each profile:
   - print name, class, target, package, command, features, panic strategy, FPU,
     size group, and conformance group;
   - for `mandatory`, fail if the target/tool is unavailable or the command
     fails;
   - for `advisory-installed`, run only when the target is installed, fail on a
     failed command, and report `advisory unavailable` when missing;
   - for `documented-only`, do not compile and report `documented-only`.
5. summarize by class so advisory or documented profiles cannot look like
   enforced passes.

The command must not install Rust targets or fetch toolchains. Toolchain
installation is an operator action, not a release-gate side effect.

## D9 - Required RFC patch before implementation

Before implementation starts, patch RFC 011 to:

1. change stale "before implementing the device solver" language to "before
   making stronger target/profile release claims and before enforcing RFC 013
   profile-aware conformance";
2. replace "exact file format is deferred" with the manifest schema in D3;
3. add the enforcement classes from D2;
4. state the v0.17.0 mandatory/advisory profile set;
5. state panic-strategy honesty rules from D4;
6. state that missing advisory targets are not aggregate failures;
7. explicitly hand off fixture/tolerance ownership to RFC 013;
8. add an acceptance checklist that distinguishes enforced, advisory, and
   documented-only evidence.

## D10 - Implementation checklist after RFC patch

After the RFC text is patched and reviewed:

- Add `xtask/target-profiles.toml`.
- Replace hardcoded `xtask/src/checks/target_profiles.rs` logic with
  manifest-driven execution.
- Keep `device-thumbv7em-hardfloat` mandatory and green in `cargo xtask check`.
- Report advisory profiles without making missing optional targets fail the
  standard aggregate gate.
- Update README, ROADMAP, RFC index, and changelog for the release that ships
  RFC 011.
- Move RFC 011 to `done/` only after `cargo xtask check-rfcs`, `cargo xtask
  target-profiles`, and the aggregate gate pass on the working tree and a clean
  copy.

## Review decisions

The v0.1 design-freeze review settled the open questions:

1. `wasm32-no-threads` remains `documented-only` in v0.17.0.
2. `cluster-linux-aarch64` remains `documented-only` for local `xtask`; a later
   runner-specific CI job may promote it.
3. The v0.17.0 mandatory device profile uses manifest-driven
   `rustflags = ["-C", "panic=abort"]`; `.cargo/config.toml` validation is not
   required before implementation.

## Gates for this memo

This memo is a design-freeze artifact only. It does not move RFC 011 out of
`proposed/` and does not implement the target-profile manifest. The next step is
the RFC 011 text patch listed in D9, followed by review before code changes.
