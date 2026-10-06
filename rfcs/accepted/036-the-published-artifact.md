# RFC 036 - The Published Artifact

**Status.** Accepted (design frozen 2026-10-07)

**Author tier.** `architect`.

**Governing review.** Architect review 079 records the investigation and every
measured value quoted here.

## 1. Summary

`0.22.1` is tagged but unpublished, and the attempt to publish it uncovered that
**the published artifact has never been treated as a reviewed deliverable.** The
repository is held to seventeen gates; the thing a user actually installs is held
to none.

The newest version of every crate on crates.io is `0.20.2` (2026-07-22). Five
tagged releases — `0.21.0`, `0.21.1`, `0.21.2`, `0.21.3`, `0.22.0` — were never
published. This is **not** a records failure: RFC 021 §7 states that "GitHub
release creation, registry publication, and certification are not part of
`D_release` and remain separately authorized optional actions", and review 063
recorded in plain words that "these crates are not on crates.io, and
`distributed` here means the tag plus retained tagged-CI evidence". The project
has been internally consistent throughout.

It has also been invisible. A user reading `CHANGELOG.md` sees
`released (tagged 2026-09-24, distributed 2026-09-24)` against `0.22.0` and will
conclude `0.22.0` is installable. The word is defined in a governance RFC the
user never opens. That is the owner's second principle breached by vocabulary
alone, and it is the smallest of the problems found.

This RFC makes the published artifact a reviewed deliverable: correct metadata,
an API reference that shows the whole API, a vocabulary that cannot be misread,
and gates that hold all three.

## 2. What is wrong, measured

Every value below was measured on `3cbbd74`, the tagged `0.22.1` tree.

### 2.1 Internal dependency requirements advertise a combination that cannot compile

`Cargo.toml` `[workspace.dependencies]` declares all five crates as
`version = "0"`. Cargo strips the `path` on publish, so the published
`loeres-cluster 0.22.1` manifest reads exactly:

```toml
[dependencies.loeres]
version = "0"
```

`"0"` admits every `0.x`, so `loeres-cluster 0.22.1` advertises compatibility
with `loeres 0.13.3`. It does not compile against anything before RFC 027:

```text
error[E0432]: unresolved import `loeres::QuadraticProgram`
  no `QuadraticProgram` in the root
```

This is not hypothetical — it is what `cargo publish --dry-run` produced, because
the resolver selected the newest *published* sibling, `0.20.2`.

**It also silently invalidated the verification.** Three of the five dry-runs
reported success while compiling against the wrong siblings:

| dry-run | verified against | result |
|---|---|---|
| `loeres` | — | OK |
| `loeres-backend-static 0.22.1` | `loeres 0.20.2` | "OK", meaningless |
| `loeres-backend-std 0.22.1` | `loeres 0.20.2` | "OK", meaningless |
| `loeres-device 0.22.1` | `loeres-backend-static 0.20.2` | "OK", meaningless |
| `loeres-cluster 0.22.1` | `loeres 0.20.2` | failed |

A user who writes `loeres = "0.20"` alongside `loeres-cluster = "0.22"` gets that
same compile error instead of a clean version conflict. With a correct
requirement, Cargo reports the conflict and names it.

The packaged `0.22.1` sources **do** compile together — verified by extracting all
five `.crate` tarballs and building them as a patched workspace with
`--all-features`. The content is sound; only the metadata lies.

### 2.2 The license text is not in any published package

`license = "Apache-2.0"` is declared, and `cargo package --list` contains **zero**
`LICENSE` entries for every crate. Apache-2.0 §4(a) requires that recipients of a
distributed copy receive a copy of the License. The SPDX identifier names the
license; it does not deliver it.

### 2.3 docs.rs would publish an incomplete API

`loeres-cluster` and `loeres-device` both declare `default = []`, and no crate has
a `[package.metadata.docs.rs]` section. docs.rs builds default features, so the
reference a user reads would omit every gated item — `parallel-rayon`,
`async-tokio`, `observability-tracing`, `observability-metrics`, `serde`,
`ffi-gateway`, `owned-arrays`, `constant-iteration`, `diagnostic-snapshot`,
`panic-gate` — with nothing to indicate they exist.

No crate uses `doc(cfg)` anywhere, so even with all features enabled a reader
cannot tell which feature a given item requires.

### 2.4 rustdoc does not build clean, and nothing checks it

`cargo doc --workspace --all-features` under `-D warnings` fails. Five
diagnostics, with locations:

| Location | Diagnostic |
|---|---|
| `crates/loeres-backend-static/src/lib.rs:12:9` | `` `array` is both a module and a primitive type `` |
| `crates/loeres-cluster/src/runtime.rs:6:59` | public documentation for `runtime` links to **private** item `executor` |
| `crates/loeres-cluster/src/lib.rs:20:29` | redundant explicit link target |
| `crates/loeres-cluster/src/lib.rs:25:63` | redundant explicit link target |
| `crates/loeres-cluster/src/lib.rs:26:47` | redundant explicit link target |

`cargo doc` and `RUSTDOCFLAGS` appear in **no** gate and **no** CI workflow. The
book is built by `release-gate`; the API reference is not built at all.

### 2.5 Every crate's crates.io page has three dead links

All five per-crate READMEs link `../../README.md`,
`../../docs/src/architecture.md` and `../../rfcs/README.md`. None of those paths
is in the tarball, so all three 404 on crates.io — fifteen dead links across the
five pages.

`link-audit` passes because the links resolve *inside the repository*. The gate
validates the repository; nothing validates the package.

### 2.6 crates.io presents a library without its headline feature

The per-crate READMEs on crates.io are clean and well written — the stale
governance paragraph about an "external predicate `P`" lives in the **root**
README, which is not packaged, so no user ever saw it. But `0.20.2` predates
RFC 027, and its published `crates/loeres/README.md` states:

> The `problem` namespace is reserved and ships no generic public LP/QP/SOCP/problem-family contract.

Constrained quadratic programming is now the library's headline capability.
Anyone evaluating loeres through crates.io is told it does not have one.

### 2.7 No keywords, no categories

No crate sets `keywords` or `categories`. These are how crates.io search and
category browsing find a library. A user looking for a `no_std` optimization
crate cannot find this one.

### 2.8 Publication exists in no procedure and no gate

`cargo publish` appears **nowhere** in the tracked repository — not in an RFC, not
in a handoff, not in `CONTRIBUTING.md`, not in `docs/src/development.md`. There is
no step that publishes and no check that notices publication did not happen. The
first time it was written down was my own `0.22.1` finalization checklist, which
is how this was found.

## 3. Design

Four slices. S1-S3 change what is published; S4 changes what the project claims
and how it is held.

### 3.1 S1 — dependency requirements that mean what they say

`[workspace.dependencies]` internal entries carry the workspace version:

```toml
loeres = { version = "0.22.1", path = "crates/loeres" }
```

For a `0.x` crate this means `>=0.22.1, <0.23.0`: patches may mix, a minor may
not, which matches the project's own semver policy where the minor is the
breaking position.

A new gate, **`published-metadata`**, asserts each internal requirement equals
`[workspace.package] version`, so a version bump that forgets one fails closed
rather than shipping a stale requirement. The gate also asserts §3.2's
obligations. It joins `cargo xtask check` as the eighteenth gate.

### 3.2 S2 — a package a user can read

- **License text in every package.** Add `LICENSE` to each crate directory (or
  `include` the workspace file) so `cargo package --list` contains it. Verified by
  the `published-metadata` gate, per crate.
- **`keywords` and `categories`** on all five crates. `categories` must be valid
  crates.io slugs; invalid slugs are rejected at publish time, so the gate checks
  them against a const list.
- **Absolute links in per-crate READMEs.** The three `../../` links become
  `https://github.com/nabbisen/loeres/...` URLs. The gate rejects any
  relative link in a packaged README, which is the rule `link-audit` cannot
  express because it reasons about the repository.

### 3.3 S3 — an API reference that shows the whole API

- `[package.metadata.docs.rs]` on all five crates with `all-features = true` and
  `rustdoc-args = ["--cfg", "docsrs"]`.
- `#[cfg_attr(docsrs, doc(cfg(feature = "…")))]` on every feature-gated public
  item, so a reader sees which feature each one needs.
- The five diagnostics in §2.4 fixed. The `runtime` → private `executor` link is
  the substantive one: either make the target public or stop linking it.
- A new gate, **`doc-build`**, running
  `cargo doc --workspace --all-features --no-deps` with `RUSTDOCFLAGS=-D warnings`.
  Nineteenth gate. This is the first time the API reference is built by anything.

### 3.4 S4 — say "published" when publication is meant

RFC 021 is in `done/` and is **not** amended; RFC 025 forbids it. This RFC
extends its vocabulary:

- **`distributed`** keeps RFC 021 §7's meaning exactly: the `D_release` bundle
  succeeded — canonical tag accepted by the authoritative remote, and the tagged
  CI `release-gate` job reached success.
- **`published`** is new and separate: the version is present in the crates.io
  index for every publishable workspace crate.

`CHANGELOG.md` release-status lines state both, so neither can be read as the
other. For the five affected releases the line becomes, with no change of fact:

```text
**Release status:** released (tagged 2026-09-24, distributed 2026-09-24); not published to crates.io
```

A short `docs/src/specifications.md` note states that `0.20.2` is the newest
published version and what that means for someone installing today. The
getting-started tutorial already instructs a `git clone` and needs no change;
once `0.22.1` is published it gains a registry path.

**A written procedure.** `docs/src/development.md` gains a publication section:
the dependency order (`loeres`, `loeres-backend-static`, `loeres-backend-std`,
`loeres-device`, `loeres-cluster`), that `--dry-run` cannot verify a workspace
whose siblings are unpublished and the patched-workspace check of §2.1 is what
does, and that publication is authorized separately from distribution.

**No network gate.** The `published` claim is not machine-checked.
`cargo xtask check` is hermetic and a registry query would make it fail offline
and in CI sandboxes. The claim is carried by the publication step in the
finalization checklist, which is reviewed. This is a deliberate limit, recorded
rather than hidden.

## 4. Explicit non-scope

- **No API change.** `doc(cfg)` and doc-link repairs are documentation-only; the
  one exception is §3.3's `executor`, where making a type public *is* an API
  change and must therefore be decided in review, not assumed.
- **No backfill of `0.21.0`-`0.22.0` to crates.io.** Their gate evidence is weeks
  old, each would need its own verification, and crates.io does not require
  contiguous versions. They remain tagged and honestly marked unpublished.
- **No yanking of `0.20.2`.** It is the only installable version; yanking it
  would leave none.
- **No publishing automation or credential handling in CI.** RFC 021 kept this out
  of scope and this RFC does not reopen it. Publication stays a human step.
- **No broader API/UX review.** §2 found packaging and reference problems. Whether
  the *API itself* reads clearly to a newcomer is a larger question and is
  proposed separately.

## 5. Risks

| Risk | Mitigation |
|---|---|
| Tightening requirements breaks a downstream user | None exists at `0.22.x`; nothing past `0.20.2` is published. The change is strictly more honest than `"0"`. |
| `doc(cfg)` churn across the public surface | Mechanical and compile-checked. `check-public-api` proves no surface moved. |
| Making `executor` public to fix a doc link enlarges the API | Treated as an API decision in review. The alternative — stop linking a private item — is always available and is the default. |
| Two new gates slow `check` | `published-metadata` is manifest parsing. `doc-build` is one `cargo doc`, already warm in a normal build. |
| `published` and `distributed` still confuse | They are stated together on every line, never alone. |

## 6. Exit criteria

1. `[workspace.dependencies]` internal requirements equal the workspace version,
   asserted by `published-metadata`.
2. `cargo package --list` contains `LICENSE` for all five crates.
3. All five crates carry `keywords`, `categories`, and `[package.metadata.docs.rs]`
   with `all-features = true`.
4. No packaged README contains a relative link.
5. `cargo doc --workspace --all-features --no-deps` passes under `-D warnings`,
   enforced by `doc-build`.
6. Every feature-gated public item is labeled with its feature under `docsrs`.
7. `CHANGELOG.md` states `published` and `distributed` separately on every
   release-status line, and the five unpublished releases say so.
8. `docs/src/development.md` documents the publication procedure and order.
9. `cargo xtask check` passes with nineteen gates.
10. The packaged set is verified to compile together by the patched-workspace
    method before any publish.
11. `0.22.1` is re-cut and **published**, and `cargo add loeres` installs it.
