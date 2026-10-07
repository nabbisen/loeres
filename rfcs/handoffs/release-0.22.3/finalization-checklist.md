# Release `0.22.3` — finalization checklist

**Status:** not scheduled. `0.22.3` currently carries RFC 038 alone.
**Authorized by.** Architect review 090 §8 accepted RFC 038 and recommends it as `0.22.3`
content. The owner authorizes the release and its timing.

**Who runs this.** The architect and the owner. The cut and the publication may not be
delegated.

## 1. Scope so far

**RFC 038** — an entry guide at every example: six per-example `README.md` files, an
`examples/README.md` index, a problem statement bound to its own module comment, and
two-directional symmetry between the example tables and the directories on disk. No crate
changed; `cargo xtask check` stays at twenty gates.

## 2. Is it worth cutting yet?

Not on its own, on the reasoning the owner applied to `0.22.2` on 2026-10-07: a release
whose whole content is documentation and a gate gives a registry user a version whose crates
are byte-identical to the one before it. `git diff 0.22.2..HEAD -- crates` is empty.

RFC 038's value reaches a visitor through the **repository**, which is already updated, not
through the registry. So `0.22.3` should accumulate something a user installs — a kernel
change, an API addition, or one of themes T5-T7 — before being cut.

If the owner prefers to publish documentation promptly, that is a legitimate different
answer, and the cut below is ready to run.

## 3. The cut

1. **RFC 038 to done.** `git mv rfcs/accepted/038-an-entry-guide-at-every-example.md rfcs/done/`,
   status `Implemented (v0.22.3)`. **Amendment 1 must be named in the Status line**, which
   `check-rfcs` enforces for `done/` only and `doc-currency` forbids while the RFC is in
   `accepted/` — the coupling is recorded in the RFC's own §0.1. Follow RFC 037's wording.
2. RFC index row repointed to `done/` with the same status.
3. **Apex scope to `RFCs 001-038`** in all three apex documents, derived from `rfcs/done/`.
4. **`Last reconciled repository release` stays `0.22.2`**; `This tree` stays `0.22.3`.
   RFC 024's inequality is strict; equality is never valid, including mid-cut.
5. **`CHANGELOG.md`**: date the `## [0.22.3]` heading; keep
   `**Release status:** unreleased` until the post-release commit.
6. `cargo xtask check` → 20 gates PASS.
7. `cargo xtask release-gate --intended-tag 0.22.3` → PASS. Whole stream to a file; never
   piped. The gate also refuses on a dirty tree (`release_gate.rs:224-226`), so commit
   first. To repeat on an unchanged revision, remove only
   `.git-exclude/release-evidence/<revision>-v0.22.3/`.
8. Commit, push `main`.
9. **Recompute the RFC 028 anchor by hand** at the revision to be tagged.
10. Tag `0.22.3`, confirm it peels to `HEAD`, push. **Wait for the tagged CI `release-gate`
    job to succeed** before claiming `distributed` (RFC 021 §7) — `gh run watch` blocks
    properly; a polling loop without a delay does not.

## 4. Publication

As `0.22.2`: authorized separately (RFC 021 §7), `published-metadata` and
`cargo package --list` first, then publish in dependency order — `loeres`,
`loeres-backend-static`, `loeres-backend-std`, `loeres-device`, `loeres-cluster` — and
confirm all five appear in the crates.io index before writing any `published` claim.
Irreversible: a version can be yanked, never replaced.

## 5. Post-release commit — the three couplings

Unchanged from `rfcs/handoffs/release-0.22.1/finalization-checklist.md` §5, each found by a
gate failing rather than by foresight. With the next version bump move:

1. the five internal requirements in `[workspace.dependencies]`, or `published-metadata`
   fails closed;
2. **all six** example lockfiles, or the `examples` gate fails with `cannot update the lock
   file … --locked`;
3. the next version's `## [x.y.z] — unreleased` CHANGELOG section, or `release-gate`'s
   preflight refuses for want of exactly one matching heading.

Then `docs/src/specifications.md`: add the row and make it the newest published version.

## 6. Open, not in this release

- **`size-budget`'s device threshold.** Still unset. Interim proposal 32 000 bytes on the
  **release** rlib (25 512 measured), with review 087 §4's caveat that an rlib is a library
  archive, not a linked image, so nothing has been dead-code-eliminated. The owner's number
  to set; a linked-image measure needs a binary target this repository lacks.
- **Themes T5 (LP), T6 (server-side maturity), T7 (fixed-point scalars)**, unscheduled. T5
  is the natural next: LP is already expressible as `Q = 0` and documented as not solved.
- Kernel instrumentation to expose the projection sweep count, which would turn RFC 037's
  bounded `proj ops ≤` into a measured figure. Recorded in RFC 037 §0.1.
- An optional polish from review 090 §5: the six example `README.md` H1 headings end in a
  full stop, because the title is the module comment's first paragraph. The title is not
  gated, so stripping a terminal period is a one-line change touching six files. Typography,
  not accuracy.
