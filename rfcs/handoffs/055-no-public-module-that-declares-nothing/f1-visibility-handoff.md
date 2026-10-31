# RFC 055 — F1: `pub(crate)` is not a public item

**Author tier.** `architect`. **Date:** 2026-11-01.
**Governing text.** `rfcs/accepted/055-no-public-module-that-declares-nothing.md` including
`## 0.1 Amendment 1` — **unchanged by this slice.** S3's requirement is *"at least one `pub`
item"*; this correction brings the code to the RFC rather than changing the RFC. Finding is
architect review 126 F1.
**Base.** `f4d52ad4543748855c7dac6a7c8bb35aac7a60a3` — accepted; S1, S2, S3, the bump and the
CHANGELOG entry are not reopened.
**Release.** Still `0.25.0`. **No version bump** — this slice does not move the position, and the
previous slice already performed it.

One item, in one function. One line and one test.

## The defect, measured

A scratch module containing only `pub(crate) fn helper() {}`, declared `pub mod scratchmod;` in
`crates/loeres-device/src/lib.rs`, gives `[check-public-api] PASS`.

`empty_module_finding` accepts any line where `trimmed.starts_with("pub(")`. **`pub(crate)`,
`pub(super)`, `pub(self)` and `pub(in …)` are all crate-internal**, so a module containing only
those is `pub mod` in a published crate, reachable by a downstream user, and exports nothing to
them — the exact defect class RFC 055 exists to close.

**The defect is in my handoff, not in your reading of it.** §3.2 said *"require at least one line
matching `pub` followed by a space or `(`"*. You implemented that sentence precisely.

## The correction

In `empty_module_finding`, accept only `pub ` — drop the `|| trimmed.starts_with("pub(")` arm:

```rust
trimmed.starts_with("pub ")
```

`pub use`, `pub fn`, `pub struct`, `pub enum`, `pub trait`, `pub type`, `pub const` and `pub mod`
all begin that way, so nothing legitimate is lost, and every restricted visibility is excluded by
construction rather than by a list.

**Do not** add an exclusion list of `pub(crate)` / `pub(super)` / `pub(self)` / `pub(in`. A list of
restricted forms is a list that can be incomplete — `pub(in crate::a::b)` is the one most likely to
be forgotten — while `pub ` is exhaustive in the right direction. This is the same reasoning that
put equality rather than `contains` in RFC 053's state check.

Update the function's doc comment: it currently describes the predicate as *"at least one public
item"*, which is now true rather than approximately true, and it should say that `pub(crate)` and
the other restricted visibilities are **deliberately not** public items here.

## The test

One, beside the four from the previous slice, which stay unedited:

- `a_pub_crate_only_module_is_a_finding`: a fixture module whose only item is
  `pub(crate) fn helper() {}` → **exactly one** finding naming the module. `assert_eq!(len, 1)`
  before asserting contents.

Add it to the existing `ModuleFixture`-based test module; the fixture already does what this needs.

## Verification

1. `cargo xtask check-public-api` → PASS on the tree.
2. `cargo xtask check` → **PASS, 20 gates**, on the gate's own exit status, never through a pipe.
3. `cargo xtask release-gate --intended-tag 0.25.0` → PASS. Whole stream to a file.
4. **Prove the correction on the real tree, both directions:** re-create the `pub(crate)`-only
   scratch module with its `pub mod` line, quote the `EMPTY PUBLIC MODULE` finding, then change
   its item to `pub fn` and confirm the finding goes away. Revert both, and confirm
   `git diff crates/loeres-device/src/lib.rs` is empty and the scratch file is gone — not only
   `git status`.
5. **No CHANGELOG change is needed.** The `0.25.0` entry's claim that the assertion *"fails closed
   when the resolved file declares no public item"* becomes accurate rather than approximate; it
   was written as if this correction were already in. If you read it as needing a word changed,
   say which.
6. The review request at
   `.git-exclude/review-request/RFC-055-f1-visibility/REVIEW_REQUEST.md`.

**Not yours**, unchanged: moving RFC 055 to `rfcs/done/`, its Status line naming Amendment 1, the
apex `Implemented scope` field, the release table, the tag, the push, and publication. **RFC 054's
slice is still outstanding and must land before the cut** — it is not part of this correction.
