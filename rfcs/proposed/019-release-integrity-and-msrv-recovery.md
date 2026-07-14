# RFC 019 - Release Integrity and MSRV Recovery

**Status.** Proposed
**Tracks.** Architecture recovery milestones R0-R2; audit blockers B1-B3 and documentation-build evidence N5.
**Touches.** `xtask/`, `.github/workflows/`, release documentation, packaging verification, MSRV evidence, and corrective-release records.

---

### Extended Metadata

* **Rust Edition Compliance:** Rust 2024
* **Declared MSRV:** Rust 1.85.0
* **Target Environment:** Host-side governance and release tooling; runtime behavior is out of scope.
* **Proposed release:** Corrective release after v0.20.0; exact version and tag require project-owner approval.

## 1. Summary

The v0.20.0 architecture audit established three release-control failures:

1. the declared Rust 1.85 MSRV build fails in `xtask`;
2. the release workflow selects `v*` tags while repository releases use
   unprefixed tags such as `0.20.0`;
3. the tag workflow runs only the RFC 010 aggregate architecture checks and
   omits required formatting, all-target/all-feature clippy, all-feature tests,
   MSRV verification, documentation build, and clean-extraction evidence.

RFC 019 restores release integrity without changing runtime public APIs or
solver behavior. It freezes one tag convention, separates the fast developer
aggregate from the complete release-candidate gate, and requires the same
evidence for the source tree and packaged clean extraction.

## 2. Affected crates and repository areas

| Area | Impact |
|---|---|
| `xtask` | Repair Rust 1.85 syntax; implement the complete release-candidate orchestration and package validation. |
| `.github/workflows/ci.yml` | Align ordinary CI commands with canonical profiles where practical. |
| `.github/workflows/msrv.yml` | Preserve the exact Rust 1.85 all-feature check. |
| `.github/workflows/release.yml` | Select canonical tags and execute the complete gate for the tagged revision. |
| `docs/src/development.md`, `CONTRIBUTING.md` | Document developer versus release-candidate commands. |
| `CHANGELOG.md` | Record the corrective release and observed gate scope at closeout. |
| runtime crates | Built and tested, but no source or public API change is authorized by this RFC. |

## 3. Public API boundary impact

No runtime public API may change. No solver type, storage type, feature flag,
error category, validation contract, or crate dependency direction changes.

The host-only `xtask` command-line contract changes:

- `cargo xtask check` remains the routine aggregate architecture/developer gate;
- `cargo xtask release-gate` becomes the canonical complete non-publishing
  release-candidate gate and is no longer merely an alias for `check`;
- optional focused subcommands may remain, but release policy must reference
  the canonical command rather than duplicate a list that can drift.

The exact internal function layout is not public API.

## 4. Dependency impact

Runtime dependency changes are forbidden.

`xtask` should reuse the standard library, Cargo/rustup commands, and existing
serde/TOML support. A new host-only dependency requires justification in the
implementation review. Network-dependent validation must not be added to the
baseline release gate by this RFC.

## 5. `std` / `alloc` impact statement

`xtask` and CI use `std` and allocation. The edge crates remain `no_std` and
no-`alloc`; the existing zero-bleed and bare-metal checks remain enforced.

## 6. Device determinism impact statement

Device solver behavior is unchanged. The release gate must continue building
the mandatory `device-thumbv7em-hardfloat` profile with its RFC 011
`-C panic=abort` configuration. RFC 019 makes no new timing, numerical, panic
freedom, or cross-target reproducibility claim.

## 7. Cluster scalability impact statement

Cluster runtime behavior and scalability are unchanged. Tests compile all
cluster features, but this RFC adds no performance or large-model claim.

## 8. Error and diagnostic impact

No `SolverError` or runtime diagnostic changes.

Release-gate failures must identify:

- the failed gate name;
- the exact command or profile;
- whether the failure occurred in the source tree or clean extraction;
- the artifact path when packaging was reached.

The gate must never print environment secrets, registry credentials, or complete
environment dumps.

## 9. Feature flag impact

No feature changes. Release evidence must include:

- workspace all-features checking and testing;
- all-target/all-feature clippy;
- canonical no-default-feature edge profiles;
- existing cluster optional-feature profiles.

## 10. Semver and tag policy

Repository release tags are canonical **unprefixed SemVer**, matching existing
history: `MAJOR.MINOR.PATCH`, for example `0.20.1`. A leading `v` is not part of
the canonical format.

Before the workflow runs release gates, it must validate the ref name as a
supported SemVer release tag. The workflow selector and validation must agree.
Malformed lookalike tags must not publish or create artifacts.

This RFC itself is corrective and has no runtime API impact. The project owner
chooses whether it ships as a patch or is batched with another release; this RFC
does not authorize creation of a tag.

## 11. Concrete technical specification

### 11.1 Rust 1.85 compatibility repair

The current `if let` chain in `xtask/src/checks/target_profiles.rs` must be
rewritten using syntax stable in Rust 1.85. The semantic rule remains:

```text
if command exists and command is neither "check" nor "build": reject profile
```

No MSRV bump is allowed as a shortcut. Any other Rust-1.85 failure discovered
by the exact workspace command is in scope when caused by project source or
manifest configuration. Dependency incompatibility requires a separately
reviewed lock/dependency decision, not silent version churn.

### 11.2 Gate classes

The project has two canonical aggregates:

| Command | Purpose | Packaging | Expected use |
|---|---|---:|---|
| `cargo xtask check` | Fast comprehensive architecture/developer gate | no | ordinary development and pull requests |
| `cargo xtask release-gate` | Complete non-publishing release-candidate evidence | yes | tagged revision and owner-requested release preparation |

`release-gate` must include, in this order unless an implementation review
records a safe reason to reorder:

1. repository/ref preconditions and canonical version consistency;
2. `cargo fmt --all --check`;
3. stable `cargo clippy --workspace --all-features --all-targets -- -D warnings`;
4. stable `cargo test --workspace --all-features` including doc-tests;
5. `cargo +1.85.0 check --workspace --all-features`;
6. the RFC 010 aggregate architecture checks owned by `cargo xtask check`;
7. `mdbook build docs`;
8. package construction;
9. package layout and exclusion validation;
10. the complete applicable gate suite against a clean extraction.

The implementation must avoid recursive invocation of `release-gate` inside
the extracted tree. It may use an internal mode or pass a documented flag so
the extracted run repeats steps 2-7 without packaging again.

### 11.3 Package contract

The release artifact must:

- be named `loeres-v<version>.tar.gz` unless the owner approves a later naming
  RFC; this filename convention is independent of the unprefixed Git tag;
- contain repository files directly at archive root, with no intermediate
  parent directory;
- include `Cargo.lock` and all release-required source, RFC, documentation,
  workflow, conformance, and license files;
- exclude `.git/`, `target/`, `.git-exclude/`, editor caches, local evidence,
  and previously generated archives;
- extract without overwriting the source tree;
- be tested from a fresh directory controlled by the release gate.

Temporary release-gate state should live under `.git-exclude/tmp/release-gate/`
or an equivalent ignored workspace-local path so restricted environments do not
depend on `/tmp`. Cleanup must be scoped to the gate-owned directory.

### 11.4 Version consistency

Before packaging, the gate must verify that the candidate version agrees across:

- the root workspace package version;
- the candidate Git tag when running on a tag;
- the intended archive filename;
- the current changelog entry;
- any release metadata introduced by implementation.

Local non-tagged dry runs may derive the candidate version from `Cargo.toml`
and report that no tag assertion was performed. They must not claim tagged
release evidence.

### 11.5 Workflow composition

The release workflow must:

1. select the canonical unprefixed tag family;
2. check out the tagged revision, not a moving branch;
3. install stable Rust with rustfmt, clippy, and mandatory target support;
4. install Rust 1.85.0;
5. install mdBook using a version/policy accepted by implementation review;
6. execute `cargo xtask release-gate` once as the canonical orchestration;
7. upload evidence/artifacts only after the gate succeeds;
8. perform no crates.io publication in this RFC.

Ordinary CI may retain parallel jobs, but its commands must not be presented as
substitutes for tagged-revision release evidence.

### 11.6 Tag-selector verification

Implementation must provide a non-publishing verification method demonstrating
that representative tags are classified correctly:

| Ref | Expected |
|---|---|
| `0.20.1` | release candidate |
| `1.0.0` | release candidate, but publication still owner-gated |
| `v0.20.1` | not canonical |
| `0.20` | invalid |
| `release-0.20.1` | invalid |

This may be a focused `xtask` unit test plus a manual `workflow_dispatch`
dry-run path. It must not create a real release or publish a crate.

### 11.7 Evidence retention

The release review package must record:

- revision and candidate version;
- toolchain versions;
- each gate result;
- source-tree versus clean-extraction scope;
- archive name and content-layout result;
- advisory/unavailable target evidence without mislabeling it enforced;
- limitations and waivers.

Raw build output may remain local/ignored, but the durable closeout must state
what was actually observed.

## 12. Security and secret handling

- No publishing token is required or requested.
- Commands must not print registry credentials or broad environment dumps.
- Archive validation must reject `.git/`, `.git-exclude/`, and local credential
  files before an artifact is considered acceptable.
- Release workflow actions must be pinned according to the repository's chosen
  action-version policy; supply-chain scanning itself is a later RFC.
- The clean extraction must execute only repository-controlled build commands.

## 13. Rejected alternatives

| Alternative | Reason rejected |
|---|---|
| Raise MSRV to match stable | Hides a declared compatibility regression without an approved compatibility decision. |
| Rename all existing tags to `v*` | Git tags are durable external references; rewriting history is unnecessary and disruptive. |
| Keep `release-gate` as an alias and duplicate commands in YAML | Recreates the drift that caused B3. |
| Assume main-branch CI proves a tag | It does not establish that every required job passed for the exact tagged revision. |
| Package without testing a clean extraction | Violates the project's release-deliverable policy and misses packaging defects. |
| Add publication now | Publication is separately owner-authorized and outside corrective scope. |

## 14. Verification gates

Design-review gates:

1. RFC lifecycle/link check.
2. Review of tag, version, archive, and command semantics.
3. Confirmation that runtime public APIs are untouched.

Implementation gates:

```text
cargo fmt --all --check
cargo clippy --workspace --all-features --all-targets -- -D warnings
cargo test --workspace --all-features
cargo +1.85.0 check --workspace --all-features
cargo xtask check
mdbook build docs
cargo xtask release-gate
```

The last command is accepted only when its output proves both source-tree and
clean-extraction phases. A command may not be claimed passed unless observed for
the reviewed revision.

## 15. Implementation sprint plan

| Sprint | Work | Review point |
|---|---|---|
| S0 Design freeze | Resolve command, tag, archive, and evidence semantics | Architecture approval of RFC 019 |
| S1 MSRV repair | Replace unstable syntax; add focused regression test | Passing exact Rust 1.85 command |
| S2 Gate composition | Separate `check` and complete `release-gate`; add docs/package phases | Tooling review before workflow changes |
| S3 Workflow alignment | Update CI/MSRV/release workflows and dry-run selector evidence | Operational review |
| S4 Package verification | Build root-layout archive and rerun gates in clean extraction | Artifact review |
| S5 Documentation | Update contributor/release instructions and changelog | Documentation consistency review |
| S6 Closeout | Complete gate evidence and move RFC to `done/` with approved release | Go/No-Go review |

## 16. Exit criteria

RFC 019 is complete only when:

1. `cargo +1.85.0 check --workspace --all-features` passes;
2. canonical unprefixed tags are selected and validated by the release workflow;
3. `cargo xtask release-gate` covers every required gate in §11.2;
4. a root-layout archive is produced with required exclusions;
5. the applicable complete gate suite passes in a clean extraction;
6. source-tree and extraction evidence refer to the same revision/content;
7. documentation distinguishes developer checks from release evidence;
8. no runtime API, solver behavior, or edge dependency boundary changed;
9. an architect review accepts the closeout evidence;
10. any tag or release action is separately authorized by the project owner.

