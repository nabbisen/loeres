# Contributing to Loeres

Thanks for your interest in Loeres. This project is developed **design-first**:
the design is the source of truth, and code follows accepted design. Please read
this before opening an RFC, a pull request, or a substantial issue.

## Where the design lives

- **Specifications** — `docs/specs/`: the requirements, external design, and
  roadmap & milestones. These are the authoritative design documents.
- **RFCs** — `rfcs/`: the per-contract design records. Their lifecycle (states,
  folders, numbering) is defined by `rfcs/done/000-rfc-lifecycle-policy.md`.
- **Book** — `docs/src/` (mdbook): a curated summary of the above for readers.

The book *summarizes*; `docs/specs/` and `rfcs/` are what you edit when the
design changes.

## Development workflow

Changes move through stages, design before code:

```text
Requirement (Planning / RFC) -> External Design -> Internal Design
  -> Program Design -> Implementation -> Testing
```

To propose a design change:

1. Open an RFC under `rfcs/proposed/` following
   `rfcs/done/000-rfc-lifecycle-policy.md` (flat `NNN-slug.md` numbering,
   `proposed/done/archive` folders).
2. Get it reviewed and accepted before implementing it.
3. Implement against the accepted RFC; move the RFC to `done/` when it ships.

## Environment

- Install the toolchain with [rustup](https://rustup.rs/).
- Loeres targets the **Rust 2024** edition and 2018+ module style
  (`foo.rs` + `foo/` may coexist; no `mod.rs` needed).

## Code conventions

- Separate `.rs` files by logical boundaries. Consider splitting a file past
  **300 effective lines of code**; strongly consider it past **500**.
- Keep edge-facing crates (`loeres`, `loeres-backend-static`,
  `loeres-device`) `#![no_std]` and free of `alloc`; never let server-facing
  types or features reach them.
- When implementation is complete: run `cargo fmt` **once** (do not hand-review
  the formatted output), then run the full test and check suite.

## Tests

- Tests validate the **design specifications**, not merely the written code.
- Keep `#[test]` code out of the module file. Place a module's unit tests in a
  colocated `tests.rs` beside it — `src/some_module.rs` pairs with
  `src/some_module/tests.rs` (Rust 2018 lets the file and directory coexist) —
  and declare `#[cfg(test)] mod tests;` in the module. If a `tests.rs` grows
  large, split it into `src/some_module/tests/(group).rs`, applying the same
  line-count splitting as production code. Do not centralize tests in a single
  top-level `src/tests/` tree.

## Release-candidate verification

`cargo xtask check` is the development aggregate. Project-owner-requested
release preparation uses `cargo xtask release-gate` from a clean tracked
revision. The latter runs the complete non-publishing RFC 019 §11.3 suite,
constructs and validates the tracked-input source archive, and repeats the
applicable gates in a clean extraction. A local run records that no tag
assertion was performed and cannot substitute for tagged-revision evidence.

Do not create a tag, upload outside the reviewed workflow, create a GitHub
release, or publish a crate without separate explicit project-owner authority.
Generated local evidence remains gate-owned ignored workspace state and must
not be committed as repository content.

### Post-release version convention

Once a release ships, `main`'s workspace version is bumped to the next patch
in the first ordinary commit that follows — never left at the released value,
which would let a later commit claim to be that release. The released version
itself is set only in the release's own finalization revision, the one that
gets tagged, and is never edited into an already-tagged commit afterward. See
`docs/src/development.md` and RFC 024 for the apex-currency mechanics this
convention keeps satisfied.

## License

By contributing, you agree that your contributions are licensed under the
Apache License, Version 2.0. See `LICENSE` and `NOTICE`.
