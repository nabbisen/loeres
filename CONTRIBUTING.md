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
- Place unit tests in a `tests.rs` within `src/`; if it grows large, move the
  contents into submodules under a `tests/` directory, applying the same
  line-count splitting as production code.

## License

By contributing, you agree that your contributions are licensed under the
Apache License, Version 2.0. See `LICENSE` and `NOTICE`.
