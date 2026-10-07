# Release `0.22.2` — finalization checklist

**Status:** blocked on one owner decision — the `README.md` headline form. Everything
else is ready.
**Authorized by.** Architect review 089 §7 recommends GO; architect review 088 accepted
S6 and C1-C3; architect review 087 accepted S1-S5 and carries the headline forms;
architect scoping 086 scoped the theme. The owner authorizes the release and chooses the
headline.

**Who runs this.** The architect and the owner. The cut and the publication may not be
delegated.

## 1. Scope

`0.22.2` ships **RFC 037** — measuring effectiveness and cost — and the two gate
corrections from RFC 036 that `0.22.1` could not carry.

- A counted-work harness over a corpus parameterised on **size and conditioning**, with
  every figure labelled *measured* or *derived*.
- Two new gates: `bench-baseline`, pinning eleven counted-work points on the host
  target, and the `published-metadata` additions from RFC 036 C8/C9. `cargo xtask check`
  runs **20 gates**.
- An exact reference for a dense `Q` at `n ≤ 8`, cross-validated against the separable
  one, with a pinned exact-rational vector and a randomized optimality oracle for the
  coupled case.
- `docs/src/effectiveness.md`, the chapter answering the owner's fourth visitor question,
  with its counted-work figures checked byte-for-byte against a real example run.
- Advisory wall-time throughput, never gated, always a range.

**No public API changed.** No kernel change: `git diff` over `crates/` is empty for every
RFC 037 slice.

## 2. The one blocking decision

RFC 037 exit criterion 7 requires `README.md:5` either to stop asserting a performance
claim the tree cannot support, or to support it with a stated environment. Forms A and B are
drafted in architect review 087 §7. **The architect withdraws the Form B recommendation
made in reviews 087, 088 and 089** and recommends **Form C** below. The owner asked, on
2026-10-07, whether that recommendation matched their stated philosophy. It did not.

**Why A and B both fail.** Form A asserts "high-throughput" on a figure that moved from
`9.8×` to `15.2×` across runs, so it fails the first principle — a claim the tree cannot
defend. Form B is defensible but fails the second: `README.md:7` already says "a hard
compile-time boundary between two execution environments" and `:9` already says "Two
paths, one contract … `no_std`, has no `alloc`", so Form B restates the two lines beneath
it in more abstract words and spends the most prominent line in the project on internal
process ("checked on every commit"). A visitor is not misled by it; they learn nothing
from it, which the second principle also forbids.

**What the first screen never says** is what the software is *for*. Lines 7-13 give the
mathematics and the mechanics; no line names a problem a reader might have.

**Form C:**

> **Constrained quadratic programs — model-predictive control, capacity dispatch — on
> servers and in heap-free embedded builds.**

Every part is backed and additive:

- *constrained quadratic programs* — RFC 027's shipped contract and kernels;
- *model-predictive control* — `examples/device-mpc-step`, a gate-checked example;
- *capacity dispatch* — `examples/cluster-capacity-dispatch`, likewise;
- *heap-free embedded builds* — deliberately a **build** property, not a hardware claim:
  nothing in this tree runs on hardware, only builds for `thumbv7em-none-eabihf` without
  `std` or `alloc`, so "runs on a microcontroller" would be a new unsupported claim;
- no performance claim of any kind, so all four disclaimers stay true exactly as written
  (`docs/src/architecture.md:20`, `docs/src/cluster-user-guide.md:248`,
  `crates/loeres-cluster/README.md:112`, `docs/src/threat-model.md:75` and `:161`), as
  under Form B and unlike Form A.

**Whichever form is chosen, the headline and the four disclaimers change in the same
commit**, and the commit must show they agree.

## 3. The cut

1. **Apply the chosen headline** and reconcile the four disclaimers (§2).
2. **RFC 037 to done.** `git mv rfcs/accepted/037-measuring-effectiveness-and-cost.md rfcs/done/`,
   status `Implemented (v0.22.2)`. **Amendment 1 must be named in the Status line**, which
   `check-rfcs` enforces for `done/` only and `doc-currency` forbids while the RFC is in
   `accepted/` — the coupling is recorded in the RFC's own §0.1. Follow RFC 034's wording:
   `Implemented (v0.22.2). Amendment 1 was made while Accepted, under RFC 000's in-place-amendment rule.`
3. **RFC index row** repointed to `done/` with the same status.
4. **Apex scope `RFCs 001-037`** in all three apex documents, derived from `rfcs/done/`.
5. **`Last reconciled repository release` stays `0.22.1`**; `This tree` stays `0.22.2`.
   RFC 024's inequality is strict and equality is never valid, including mid-cut.
6. **`CHANGELOG.md`**: date the `## [0.22.2]` heading, keep
   `**Release status:** unreleased` until the post-release commit, and add the RFC 037
   entry in the "what a user can observe" form RFC 036 established.
7. `cargo xtask check` → 20 gates PASS.
8. `cargo xtask release-gate --intended-tag 0.22.2` → PASS. Whole stream to a file; never
   pipe through `head` or `tail`. To repeat on an unchanged revision, remove only
   `.git-exclude/release-evidence/<revision>-v0.22.2/`.
9. Commit, push `main`.
10. **Recompute the RFC 028 anchor by hand** at the revision to be tagged:
    `git archive --format=tar <revision> | sha256sum`.
11. Tag `0.22.2`, confirm it peels to `HEAD`, push the tag. Confirm the tagged CI
    `release-gate` job succeeds before claiming `distributed` (RFC 021 §7).

## 4. Publication

Authorized separately from distribution (RFC 021 §7) and by the owner explicitly.

1. `cargo xtask published-metadata` → PASS, and `cargo package -p <crate> --list` for all
   five.
2. Publish **in dependency order**: `loeres`, `loeres-backend-static`,
   `loeres-backend-std`, `loeres-device`, `loeres-cluster`.
3. Irreversible: a version can be yanked, never replaced.
4. Confirm all five versions appear in the crates.io index before writing any `published`
   claim.

## 5. Post-release commit — three couplings, all found the hard way

1. `**Release status:**` → `released (tagged <date>, distributed <date>); published to crates.io`.
2. Apex `Last reconciled repository release` → `0.22.2`.
3. Bump the workspace version to the next patch and set apex `This tree` to match. **Three
   things move with it** (`rfcs/handoffs/release-0.22.1/finalization-checklist.md` §5
   records why each was found by a gate failing rather than by foresight):
   - the five internal requirements in `[workspace.dependencies]`, or
     `published-metadata` fails closed;
   - **all six** example lockfiles — `cluster-counted-work` joined the five in RFC 037 —
     or the `examples` gate fails with `cannot update the lock file … --locked`.
     Regenerate with `cargo update --manifest-path examples/<name>/Cargo.toml --offline`;
   - the next version's `## [x.y.z] — unreleased` CHANGELOG section, or
     `release-gate`'s preflight refuses for want of exactly one matching heading.
4. `docs/src/specifications.md`: add the `0.22.2` row and make it the newest published
   version.
5. `cargo xtask check` → PASS. Commit, push.

## 6. Not in this release

- Themes T5 (LP), T6 (server-side maturity), T7 (fixed-point scalars), unscheduled.
- A linked-image device size measure. `size-budget`'s threshold is still unset; the
  interim proposal is 32 000 bytes on the **release** rlib (25 512 measured), with the
  caveat recorded in review 087 §4 that an rlib is a library archive, not a linked image,
  so nothing has been dead-code-eliminated. The owner sets the number; a linked-image
  measure needs a binary target this repository does not have and is a later RFC.
- Kernel instrumentation to expose the projection sweep count, which would turn the
  bounded `proj ops ≤` figure into a measured one. Recorded as a candidate in RFC 037
  §0.1.
