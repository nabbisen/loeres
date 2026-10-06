# RFC 036 corrections C1-C6

**To:** implementer (mid-capability model)
**From:** architect
**Authorized by.** Architect review 080, which accepted requests A and B and
raised these six corrections. C1, C2 and C4 block the `0.22.1` cut; C3, C5 and C6
are small and go in the same slice.
**Base revision:** `62f109c` (S4).

Requests A and B were good work. Two of the findings in request B §3 were
**correct against the architect's own handoff**, and C1 below records why the
architect's original verification was itself invalid. Nothing you implemented
needs to be undone.

## C1 — the packaged-set verification (blocking)

**You were right, and the reason is worse than you could see.** Handoff §4.3's
tarball procedure cannot be run after S1, exactly as request B §3.2 reports. The
architect has reproduced it: `cargo package -p loeres-backend-static --no-verify`
stops at `failed to select a version for the requirement loeres = "^0.22.1"`, and
`[patch.crates-io]` in the root manifest does **not** help — cargo resolves a
publishable manifest against the real registry and ignores patches, by design.

Why the architect's RFC 036 §2.1 claim was wrong: that verification ran while
`[workspace.dependencies]` still said `version = "0"`, so the siblings resolved
to `0.20.2` and `--no-verify` produced tarballs. **Those tarballs carried the old
`"0"` manifests.** The architect verified the metadata S1 was about to replace —
the same "verified the wrong artifact" failure the dry-runs had, and the one
RFC 036 §2.1 criticises. It is corrected in review 080 §2.

### What replaces it, measured

`cargo package --list` needs no registry resolution, works `--offline`, works on a
dirty tree without `--allow-dirty`, and all five crates together take **0.14s**.
Using it, the architect established:

- For **all five** crates the packaged `src/**` set is **identical** to the tracked
  `src/**` set (18, 9, 9, 13, 25 files).
- No crate has a `build.rs`; no source uses `include_str!` or `include_bytes!`.
- The root `exclude` list drops only the five `examples/` directories, which are
  workspace-excluded members and belong to no crate's package.

Therefore the published crates' compilable content is byte-identical to the
workspace members', and the published set compiles **iff**
`cargo check --workspace --all-features` passes. That is a stronger statement than
the tarball check gave, because it covers all five rather than only the root.

### C1.1 Make it a gate assertion

Add to `published-metadata`, per publishable crate, using
`cargo package -p <crate> --list --offline`:

- the set of listed paths beginning `src/` equals the set of tracked files under
  `crates/<crate>/src/` (compare as sets, sorted; report the symmetric difference
  by name on failure);
- the listing contains `LICENSE` and `README.md`.

Keep it hermetic: `--offline`, no `--allow-dirty`. Invoking cargo from a gate is
established — `doc_build.rs` and `feature_matrix.rs` both do it.

### C1.2 Rewrite the procedure in `docs/src/development.md`

Replace the tarball paragraph with:

- The static argument above, with the three facts that support it, and the note
  that C1.1 asserts it per commit.
- That `cargo publish --dry-run` **cannot** verify a dependent crate before its
  siblings are published, and that `[patch.crates-io]` does not change this.
- That the real whole-set verification happens **during** publication: publish in
  dependency order, and each crate's own `cargo publish` verify build resolves the
  sibling just published from the real registry.
- The residual risk, stated plainly: if crate *N* fails verification, crates
  *1..N-1* are already permanently published and cannot be replaced, only yanked.
  C1.1 is what makes that outcome unlikely, because the only thing that could
  differ between the workspace build and the published build is a missing file.

## C2 — reserved-but-inert features (blocking)

Request B §3.1 is right that features gate nothing, and the count is **fifteen**,
not ten. You missed two: `loeres` declares `libm` and `fixed-point-hooks`, both
with zero `cfg(feature = …)` sites.

| Crate | Declared, zero `cfg` sites |
|---|---|
| `loeres` | `libm`, `fixed-point-hooks` |
| `loeres-backend-static` | `static-views`, `diagnostic-snapshot` |
| `loeres-backend-std` | `serde`, `parallel-rayon`, `adapter-ndarray`, `adapter-nalgebra`, `native-linalg` |
| `loeres-device` | `diagnostic-snapshot`, `panic-gate` |
| `loeres-cluster` | `observability-tracing`, `observability-metrics`, `serde`, `ffi-gateway` |

**Ruling: do not remove any of them.** They are RFC 009-sanctioned reservations,
and `docs/specs/loeres-external-design-v1.md` already documents the posture
correctly in the prose under each table — "`serde`, `parallel-rayon`,
`adapter-ndarray`, `adapter-nalgebra`, and `native-linalg` are reserved/inert",
"The observability feature names are reserved…", "`libm` and `fixed-point-hooks`
are reserved/inert". `feature_matrix.rs` also builds a conditional `cluster-ffi`
profile from `ffi-gateway`. Removing them would contradict shipped normative text.

**Do not edit the apex specification tables either.** The posture prose beneath
them is accurate; rewriting normative apex documents for emphasis is churn.

Two things are genuinely missing, and both are about what a *user* receives.

### C2.1 An inert-feature registry with symmetry

In `published-metadata`, a `const RESERVED_INERT_FEATURES: &[(&str, &str)]`
(crate, feature) holding exactly the fifteen pairs above. Assert **both
directions**:

- every pair in the registry names a declared feature with zero
  `cfg(feature = "…")` sites in that crate's `src/`;
- every declared feature with zero `cfg` sites appears in the registry.

So a newly added feature cannot be silently inert, and a reserved feature that
becomes live fails the gate until it is removed from the registry. This is the
same coverage symmetry as RFC 022's citations and RFC 030's differential registry.

### C2.2 A `Features` section in each packaged README

The packaged `crates/<name>/README.md` is the **only** feature documentation a
crates.io or docs.rs user receives; `docs/specs/` is not shipped with any crate.
Today those READMEs mention features between zero and three times, and a registry
visitor sees the feature list with nothing saying which do anything.

Add to each of the five a short `## Features` section listing that crate's
features, marking each either with one line of what it does or as **reserved; no
effect yet**. Use the posture already written in
`docs/specs/loeres-external-design-v1.md` — do not invent new meanings. Keep it to
a table or a short list; this is a landing page, not a specification.

## C3 — shorter intra-doc labels

Answering request B §1.2: the path-prefixed labels are not required.
`ClusterJob`, `ClusterProjectedFirstOrderJob` and
`solve_projected_first_order_dyn` are all re-exported at the crate root
(`crates/loeres-cluster/src/lib.rs:62-66`), so the bare form resolves. The
architect verified that

```text
[`ClusterJob`]   [`ClusterProjectedFirstOrderJob`]   [`solve_projected_first_order_dyn`]
```

passes `RUSTDOCFLAGS="-D warnings" cargo doc -p loeres-cluster --all-features
--no-deps` with no ambiguity warning. Use the bare form: it reads as prose, which
is the point of the sentence it sits in. Re-wrap the affected lines.

## C4 — a CHANGELOG entry for RFC 036 (blocking)

Request B §6.3 asked, and the answer is **yes**. Add to the `0.22.1` section,
after the RFC 035 material and the existing C1 lint paragraph. It must name what a
user can observe:

- internal dependency requirements now state the exact workspace version instead
  of `0`, so a published crate no longer advertises a combination that cannot
  compile;
- the Apache-2.0 text now ships inside every crate;
- `keywords` and `categories`, so the crates can be found;
- `[package.metadata.docs.rs]` with `all-features`, and feature banners on gated
  items, so the published reference shows the whole API;
- the packaged READMEs' links are absolute, so they resolve off GitHub;
- `distributed` and `published` are now separate words, with the release-status
  lines saying which hold;
- two new gates, `published-metadata` and `doc-build`, taking `cargo xtask check`
  to nineteen.

Say explicitly that **no public API changed**.

## C5 — mark `0.20.2` as published

`0.20.2` is the only installable version, and `docs/src/specifications.md` now
tells users to depend on it. Its release-status line should say so positively
rather than leave the reader to infer it:

```text
**Release status:** released (tagged 2026-07-22, distributed 2026-07-22); published to crates.io
```

Use its actual existing dates — do not change them. Leave `0.20.1` and earlier
alone. In the CHANGELOG preamble, change "Release-status lines from `0.21.0`
onward" to "from `0.20.2` onward", so the scope sentence stays true.

## C6 — a typo the architect let through

`docs/src/specifications.md:36-37` reads "see the / the RFC index". It was
introduced by RFC 035 S2 and the architect accepted it in review 076. Drop the
duplicate word.

## Non-scope

- No API change. `check-public-api` must stay green.
- Do not remove any feature declaration (C2).
- Do not edit the apex specification tables (C2).
- No network access in any gate. The `VALID_CATEGORIES` list stays a const; see
  review 080 §5 for the ruling on request A §6.3.
- No `#[allow(…)]`.
- Do not tag, and do not run `cargo publish` without `--dry-run`.

## Required evidence

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-features --all-targets -- -D warnings
cargo test --workspace --all-features
cargo +1.85 check --workspace --all-features
cargo xtask check                      # 19 gates
cargo xtask published-metadata         # standalone
cargo xtask doc-build                  # standalone
```

State in the request:

1. **Examples affected** — expected `none`.
2. For C1.1 and C2.1, the break-and-restore record for **each new assertion and
   each direction of the symmetry**, with the exact failure line. Your earlier
   bite records were exactly the right evidence; keep that standard. The architect
   independently broke three assertions you had not (six keywords, a 21-character
   keyword, a missing `version` key) and all three bit correctly — expect the same
   treatment here.
3. The `src/**` set sizes C1.1 compares, per crate, so they can be checked against
   the architect's measured 18 / 9 / 9 / 13 / 25.
4. Any value in this handoff that does not match the tree. Two of the three
   findings in request B §3 were correct against the architect's handoff; that is
   the standard, not an exception.
