# Release `0.22.1` — finalization checklist

**Status:** cut in progress; publication awaiting owner authorization.
**Authorized by.** Architect review 082 §6 recommends GO and lists these steps;
architect review 081 accepted RFC 036 C1-C6; architect review 080 accepted RFC 036
S1-S4; architect review 078 accepted C2 and first recommended GO for `0.22.1`;
architect review 076 accepted RFC 035 S1-S4. The owner authorizes publication.

**Who runs this.** The architect and the owner. Mechanical preparation may be
delegated; **the cut and the publication may not**.

## 1. Scope

`0.22.1` ships two RFCs.

- **RFC 035** — visitor-facing documentation and real-world examples. No crate
  behaviour changed.
- **RFC 036** — the published artifact: dependency requirements that state the
  exact workspace version, the Apache-2.0 text inside every package, `keywords`
  and `categories`, docs.rs configuration with feature banners, absolute links and
  a `## Features` section in every packaged README, `published` separated from
  RFC 021's `distributed`, and two new gates taking `cargo xtask check` to
  nineteen.

Plus one lint-driven refactor with identical semantics (C1 of the `0.22.1` cycle),
evidenced in-tree by C2. **No public API changed in this release.**

## 2. History of this tag

Tag `0.22.1` was created once, at `3cbbd74`, and **dropped** — locally and on
`origin` — on 2026-10-07, before any publication. The attempt to publish that tree
found that nothing had been published since `0.20.2` and that the artifact's
metadata was wrong in six ways; architect review 079 records the investigation and
RFC 036 is its remedy. The owner authorized the drop: "We can drop and remake
0.22.1 this time."

Nothing was ever published from `3cbbd74`. No other tag was moved.

## 3. The cut

1. **RFC 036 to done.** `rfcs/accepted/` → `rfcs/done/`, status
   `Implemented (v0.22.1)`, index row repointed. RFC 035 moved in the earlier cut.
2. **Apex scope `RFCs 001-036`** in all three apex documents, derived from
   `rfcs/done/`.
3. **`Last reconciled repository release` stays `0.22.0`**; `This tree` stays
   `0.22.1`. RFC 024's inequality is **strict** and equality is never valid, at any
   point, including mid-cut.
4. **`CHANGELOG.md`** heading dated `2026-10-07`, titled for both RFCs, with
   `**Release status:** unreleased` until the post-release commit.
5. `cargo xtask check` → nineteen gates PASS.
6. `cargo xtask release-gate --intended-tag 0.22.1` → PASS. Redirect the whole
   stream to a file; never pipe it through `head` or `tail`. To repeat a run on an
   unchanged revision, first remove only
   `.git-exclude/release-evidence/<revision>-v0.22.1/`.
7. Commit, push `main`.
8. **Recompute the RFC 028 anchor by hand** at the tagged revision:
   `git archive --format=tar <revision> | sha256sum`. It will not equal the
   `3cbbd74` value `5c7963af…`.
9. Tag `0.22.1`, confirm it peels to `HEAD`, push the tag.

## 4. Publication — the step that has never been run

Authorized separately from distribution (RFC 021 §7) and by the owner explicitly.
This is the first publication of these crates since `0.20.2` on 2026-07-30.

1. `cargo xtask published-metadata` → PASS, and `cargo package -p <crate> --list`
   for each of the five.
2. Publish **in dependency order**: `loeres`, `loeres-backend-static`,
   `loeres-backend-std`, `loeres-device`, `loeres-cluster`.
3. `cargo publish --dry-run` cannot verify a dependent before its siblings are
   published, and `[patch.crates-io]` does not change that. Each crate's own
   publish-time verify build resolves the sibling just published from the real
   registry; that is the real whole-set verification.
4. **Irreversible.** A published version can be yanked, never replaced. If crate
   *N* fails verification, crates *1..N-1* are already public.
5. Afterwards, confirm all five versions appear in the crates.io index before
   writing any `published` claim.

## 5. Post-release commit

1. `**Release status:**` → `released (tagged 2026-10-07, distributed <date>); published to crates.io`.
2. Apex `Last reconciled repository release` → `0.22.1`.
3. Bump the workspace version to the next patch so `This tree` > `Last reconciled`
   holds strictly again, and set apex `This tree` to match. **The internal
   dependency requirements in `[workspace.dependencies]` must be bumped with it** —
   `published-metadata` asserts they equal the workspace version and will fail
   closed otherwise. This is the first release where that coupling exists.
4. `docs/src/specifications.md`'s "What is on crates.io" paragraph must be updated:
   `0.22.1` becomes the newest published version, and the sentence directing users
   to `0.20.2` is removed.
5. `cargo xtask check` → PASS. Commit, push.

## 6. Not in this release

- No backfill of `0.21.0`-`0.22.0` to crates.io; they stay tagged, distributed and
  honestly marked unpublished.
- No yanking of `0.20.2`.
- No publishing automation or credentials in CI (RFC 021 §7, RFC 036 §4).
- The benchmark harness (theme T4) — the only honest answer to the owner's fourth
  visitor question, "how effective or powerful" — remains unscheduled and is the
  architect's next recommendation. Themes T5 (LP), T6 (server-side maturity) and
  T7 (fixed-point scalars) follow it.
