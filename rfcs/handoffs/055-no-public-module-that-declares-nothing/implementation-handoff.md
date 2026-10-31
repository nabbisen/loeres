# RFC 055 — implementation handoff

**Author tier.** `architect`. **Date:** 2026-10-31.
**Governing text.** `rfcs/accepted/055-no-public-module-that-declares-nothing.md`, Accepted
(design frozen 2026-10-31), **including `## 0.1 Amendment 1`**, which supersedes S2 where they
differ. Where this file and the RFC disagree, **the RFC is the contract**; report it rather than
choosing.
**Release.** `0.25.0`. **This slice is breaking and it performs the version bump** — see §4.
**Scope record.** `rfcs/handoffs/release-0.25.0/scope-and-decision-points.md` §1.3.

**Order: S1, S2, then S3.** S3 asserts what S1 establishes; written first it would fail on a tree
that is still correct.

## 0. Before anything else

### 0.1 Re-derive the figure, with this command

RFC 055 §2.1 states **30 `pub mod` declarations resolved, 4 declaring no public item**. Reproduce
it, and **if it is not 4, stop and report the discrepancy with its cause** rather than adopting
either number (architect review 121 §1).

```sh
git ls-files 'crates/*/src/*.rs' 'crates/*/src/**/*.rs' \
  | grep -v 'tests\.rs$' \
  | xargs grep -nE '^\s*pub mod [a-z_][a-z0-9_]*;'
```

That lists the declarations; the emptiness half is resolving each to `<name>.rs` or
`<name>/mod.rs` **relative to the declaring file's own module directory** — `lib.rs` resolves
against `src/`, and `runtime.rs` against `src/runtime/` — and checking for a line matching
`^\s*pub(\s|\()`. The four expected failures are
`loeres-backend-std/src/lib.rs:19,20,27` (`adapter`, `batch`, `view`) and
`loeres-device/src/lib.rs:31` (`diagnostic`).

### 0.2 Amendment 1 changes two of S2's instructions

RFC 055 S2 said to delete `loeres-external-design-v1.md:295` and `:1114`. **Amendment 1 keeps
both**, reworded. §2.2 and §2.3 below are the replacement texts. This is the only place the
handoff and the frozen S2 differ, and the amendment is why.

## 1. S1 — remove the four modules

### 1.1 The files and the declarations

Delete, with `git rm`:

```text
crates/loeres-backend-std/src/adapter.rs
crates/loeres-backend-std/src/batch.rs
crates/loeres-backend-std/src/view.rs
crates/loeres-device/src/diagnostic.rs
```

and the four declarations: `crates/loeres-backend-std/src/lib.rs:19` (`pub mod adapter;`), `:20`
(`pub mod batch;`), `:27` (`pub mod view;`), and `crates/loeres-device/src/lib.rs:31`
(`pub mod diagnostic;`).

**Nothing else in either `lib.rs` module list changes.** `dense` and `sparse` keep their
`#[cfg(feature = …)]` and `#[cfg_attr(docsrs, doc(cfg(…)))]` attributes exactly as they are, and
`pub(crate) mod internal;` stays (RFC 055 §5 item 1).

### 1.2 `crates/loeres-backend-std/src/lib.rs` — the crate rustdoc

Old, exactly:

```text
//! Public module topography (external design §1.5):
//! `dense`, `sparse`, `view`, `batch`, `adapter`.
```

New:

```text
//! Public module topography (external design §1.5): `dense`, `sparse`.
```

Old:

```text
//! checks and finite-scan helpers. `view`, `batch`, and `adapter` remain
//! placeholders.
```

New:

```text
//! checks and finite-scan helpers.
```

Also in that paragraph's opening line, `Provides dense/sparse storage adapters and optional
third-party numerical backends` — **delete `and optional third-party numerical backends`**. It is
the `adapter` promise in the crate's own one-line description, and it is the same unheld claim
RFC 054 §1.5 removes from `loeres-requirements-v1.md:445`. **Flagged, not slipped in:** RFC 055 S1
names the topography lines and this is a third line in the same doc comment. If you read the RFC
as leaving it, say so.

### 1.3 `crates/loeres-device/src/lib.rs` — the reserved-diagnostics sentence

Old:

```text
//! [`loeres::QuadraticProgram`]. `diagnostic` is reserved for future richer
//! diagnostics.
```

New:

```text
//! [`loeres::QuadraticProgram`].
```

If richer device diagnostics are wanted later, they arrive with a module that declares something.

## 2. S2 — the two apex specifications

### 2.1 `docs/specs/loeres-external-design-v1.md:288-294` — the module maps

In the `loeres-backend-std` block, delete these three rows:

```text
loeres_backend_std::view        // borrowed views over dynamic storage
loeres_backend_std::batch       // server-side batched storage containers
loeres_backend_std::adapter     // optional third-party numerical crate adapters
```

In the `loeres-device` block at `:335`, delete this row:

```text
loeres_device::diagnostic   // compact no_std diagnostics, no logging framework
```

The `dense`/`sparse` rows and `loeres-device`'s other four rows are untouched, as is
`:337`'s *"`loeres-device` must remain intentionally small."* — which this change serves.

### 2.2 `:295` — reworded, not deleted (Amendment 1)

Old:

```text
Adapter modules must be feature-gated.
```

New:

```text
Any third-party numerical adapter added later must be feature-gated, so the baseline public surface never requires a third-party type.
```

The constraint survives because it governs the reserved `adapter-*` features, not the module being
removed, and because it is the rule RFC 008 applied to Rayon and Tokio.

### 2.3 §4.7's first sentence — rewritten, not deleted (Amendment 1)

Old:

```text
`loeres-device::diagnostic` may expose compact diagnostics through result or state types.
```

New:

```text
The device path's diagnostics are the core categories in `loeres::diagnostic` together with the device report and workspace-diagnostic types. They may expose compact diagnostics through result or state types.
```

**The section heading, the seven allowed examples, the five forbidden examples and the closing
sentence about an optional no-`std` sink are untouched.** That policy is the reason §4.7 survives.

Before writing it, **verify the two type names** — the device report and workspace-diagnostic types
as they are actually spelled in `crates/loeres-device/src` — and use the real spellings or report
that they differ from this description. Do not take mine on trust; that is where eight of my
errors in this sequence have come from.

## 3. S3 — the assertion

In `xtask/src/checks/public_api.rs`, a new function beside `audit_public_file`, called from `run`.

### 3.1 Wider than `EDGE_SRC`, and that is the point

`EDGE_SRC` at `:8-12` is the three edge crates, which is why nothing saw three of these four
(RFC 055 §2.5). The new scan covers **all five published crates' `src` trees**:

```text
crates/loeres/src, crates/loeres-backend-static/src, crates/loeres-backend-std/src,
crates/loeres-device/src, crates/loeres-cluster/src
```

Do **not** widen `EDGE_SRC` itself — it is the zero-bleed token sweep's set and must stay the edge
three. Add a separate const.

### 3.2 The predicate

For every `pub mod <name>;` in those trees, outside `tests.rs`: resolve to `<name>.rs` or
`<name>/mod.rs` relative to the declaring file's module directory, and require at least one line
matching `pub` followed by a space or `(` at the start of a trimmed line. Reuse
`util::collect_ext` and `util::strip_comment`; do not write a second file walker.

**Fail closed:** a declaration whose file cannot be found is a finding, not a skip.

Register **`EMPTY PUBLIC MODULE`**, naming the declaring file, its line, and the module. **One
finding per empty module**, not per crate (architect review 119 F3).

**The limits, stated and not discovered.** The simple form does not handle an inline
`pub mod x { … }` block, and it would report a module whose only public surface is a `pub use`
re-export. Neither shape exists in this project today — all 30 declarations are
`pub mod <name>;` against a file. **Implement the simple form and record both limits in the
function's doc comment**, with the measurement that neither shape is present. Do not add handling
for a shape the project does not have.

### 3.3 Tests — four

1. an empty module → one finding naming it;
2. a module with one `pub fn` → no finding;
3. the **nested** shape, `runtime.rs` declaring `pub mod cancel;` resolved to `runtime/cancel.rs`
   → no finding when that file has a `pub` item;
4. a declaration whose file does not exist → one finding, proving it fails closed.

## 4. The version bump — this slice performs it

**This is the first slice to move the release's position**, so
`docs/src/development.md`'s rule applies: the bump happens here, before the cut, because
`release-gate --intended-tag` validates the intended tag against the workspace version.

`0.24.3` → **`0.25.0`**, in the four gated places that section enumerates:

1. the workspace version and the **five** internal `[workspace.dependencies]` pins
   (`Cargo.toml:35,44-48`), or `published-metadata` fails closed;
2. **all eight lockfiles**, each with `--workspace`. Unscoped, `cargo update` relocks every
   third-party dependency the ranges permit; at `0.24.2` the correct result was **27 changed lines,
   every one a version string on this workspace's own crates**. Verify the diff that way.
3. the apex `This tree` field in all **seven** `APEX_DOCS`, byte-identical, or `doc-currency`
   reports `APEX RELEASE`;
4. the `## [0.24.3] — unreleased` `CHANGELOG.md` heading → `## [0.25.0] — unreleased`, because
   `release-gate` requires **exactly one** heading matching the workspace version and that is its
   *first* preflight check.

`Last reconciled repository release` stays at `0.24.2`. The release has not happened and RFC 024's
inequality is strict.

**If RFC 054's slice or the linear-objective step slice has already bumped to `0.25.0`, this is a
check rather than an edit.** Confirm, do not re-edit, and say which it was in your request.

## 5. The CHANGELOG entry — two changes, stated separately

This release carries a break and an addition. **The entry must not blur them.**

For the break, follow the form the `0.23.0` entry took for RFC 042 — it is the project's model for
this:

- **what** vanished: the paths `loeres_backend_std::{adapter, batch, view}` and
  `loeres_device::diagnostic`;
- **the error** a caller sees: `error[E0432]` on `use loeres_backend_std::batch;`, or
  `error[E0433]` on a path mention;
- **who is affected**: only code that imports one of these modules and uses nothing from it — they
  declared no items, so nothing could ever be imported *from* them, and the compiler already warns
  about such an import;
- **the fix**: delete the import;
- **and that no capability is lost**: device diagnostics are `loeres::diagnostic` and always were
  (RFC 055 §7's third risk). Say it in the same breath, or the entry reads as a removal of
  function.

## 6. Verification, and what to hand back

1. `cargo xtask check` → **PASS, 20 gates**, on the gate's own exit status, never through a pipe.
2. `cargo xtask release-gate --intended-tag 0.25.0` → PASS. Whole stream to a file.
3. `cargo xtask published-metadata` → PASS, and `cargo package -p loeres-backend-std --list` and
   `-p loeres-device --list` → **each one file shorter** than before. Quote both counts; that is
   the measurement that the removal reached the published artifact and not only the tree.
4. `cargo xtask feature-matrix` → PASS, and **build `loeres-backend-std --no-default-features`**:
   its public surface was three empty modules (RFC 055 §2.2) and is now empty of modules
   altogether. Confirm it still compiles.
5. **Prove S3 catches the real thing:** after S1, add back a single empty
   `crates/loeres-device/src/diagnostic.rs` and its `pub mod`, record the `EMPTY PUBLIC MODULE`
   finding, revert, and confirm `git diff` is empty afterwards — not only `git status`.
6. §0.1's figure, and the two spellings §2.3 asks you to verify.
7. The review request at
   `.git-exclude/review-request/RFC-055-no-empty-public-module/REVIEW_REQUEST.md`.

**Not yours:** moving RFC 055 to `rfcs/done/`, its Status line naming Amendment 1, the apex
`Implemented scope` field, the release table, the tag, the push, and publication.
