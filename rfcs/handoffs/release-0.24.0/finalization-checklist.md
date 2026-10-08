# Release 0.24.0 — finalization checklist

**Author tier.** `architect`. **Date:** 2026-10-08.
**Authorized by.** The owner authorized the cut on 2026-10-08. Scope and the decision record:
`rfcs/handoffs/release-0.24.0/scope-and-decision-points.md`.

## 1. Scope

**RFC 043 — the constrained kernel over a bounded scalar.** RFC 034 Amendment 1's
infeasibility-evidence predicate cross-multiplied by integers to avoid a division; over a
saturating scalar both operands clamp and the comparison reads `MAX >= MAX`, which is **`true`**
— the direction that asserts divergence. S1 measured the kernel at one precision and found
nothing; A1 swept the type's documented range and found it in **both** directions. S2
reformulated the predicate by division, with the factors built by dividing `one()` down rather
than multiplying a large integer up — because `scalar_from(99)` had already clamped within the
type's own documented range, silently replacing RFC 034's `0.99` with `1.0` and, at
`FRAC_BITS = 27`, its `1.9` with **`1.6`**, inside the band RFC 034 proved unsound. S3 answered
RFC 041's deferred tier question; S4 expressed the converged-but-wrong threshold in quantization
steps in both harnesses; S5 added inherent `checked_*`; S6 documented the measured usable band.

**RFC 044 — a `FRAC_BITS` a caller cannot get wrong.** `Q32<31>` had `one() == -1.0` and
`Q32<32>` had `one() == zero()`, silently, while `Q32<64>` was already a compile error. A
private associated const closes `31..=63` at compile time, forced in all four constructors.

**Five architect reviews**: 097 (S1), 098 (A1), 099 (S2 + A2), 100 (S6 + RFC 044), 101 (G1),
102 (C/D/E + the bump). Registered in `rfcs/review-evidence-index.md` rows `097`–`102`.

## 2. The version is already bumped

The workspace is **already at `0.24.0`**: S5's slice bumped it, because
`release-gate --intended-tag 0.24.0` validates the intended tag against the workspace version
and the gate is required of every slice. `docs/src/development.md`'s "A slice that moves the
release's position bumps the version before the cut" records this as the rule. So step 5 below
is a **check**, not an edit.

`Last reconciled repository release` is at `0.23.0` and must stay there until the post-release
commit. RFC 024's inequality is strict.

## 3. The cut

1. **RFCs 043 and 044 to done.** `accepted/` → `done/`, status `Implemented (v0.24.0)`, and
   **both must name their amendments in the Status line** — `check-rfcs` requires it of a
   `done/` RFC and `doc-currency` forbids the clause while the RFC is Accepted. RFC 043 carries
   **four** amendments, RFC 044 one.
2. RFC index rows repointed.
3. **Apex scope to `RFCs 001-044`**, derived from `rfcs/done/`.
4. **`CHANGELOG.md`**: date the `## [0.24.0]` heading and add the subtitle; keep
   `**Release status:** unreleased`. The entry already opens with the one thing a downstream
   user must act on: **a dependent pinned at `loeres = "0.23"` resolves to
   `>=0.23.1, <0.24.0` and will not receive S2's correctness fix from `cargo update` alone.**
5. **Check** `This tree: 0.24.0` in all three apex documents and the workspace version —
   already set (§2), so confirm rather than edit.
6. `cargo xtask check` → 20 gates PASS, on the gate's own exit status, never through a pipe.
7. `cargo xtask release-gate --intended-tag 0.24.0` → PASS. Whole stream to a file. The gate
   refuses on a dirty tree, so commit first, and its evidence directory is created with
   `fs::create_dir` — once per revision, no cleanup.
8. Commit, push `main`.
9. **Recompute the RFC 028 anchor by hand** at the revision to be tagged, and verify it three
   ways including that the stored archive decompresses to it.
10. Tag `0.24.0`, confirm it peels to `HEAD`, push. **Wait for the tagged CI `release-gate` job**
    with `gh run watch` before claiming `distributed` (RFC 021 §7); a polling loop without a
    delay does not wait.

## 4. Publication

Authorized separately (RFC 021 §7). `published-metadata` and `cargo package -p <crate> --list`
first, then publish in dependency order — `loeres`, `loeres-backend-static`,
`loeres-backend-std`, `loeres-device`, `loeres-cluster` — and confirm all five appear in the
crates.io index before writing any `published` claim. Irreversible: a version can be yanked,
never replaced.

**This release breaks no caller.** The `f64` path's comparisons are bit-identical — the
divide-down factors reproduce `1.9` and `0.99` to the bit — and the whole pinned conformance
corpus is unchanged. The minor is for S5's three added methods, not for a break.

## 5. Post-release commit — the four couplings

1. the five internal requirements in `[workspace.dependencies]`, or `published-metadata` fails
   closed;
2. **every workspace-excluded crate's own lockfile** — a rule, not a count: eight files, each
   updated **with `--workspace`**, or `cargo update` relocks every third-party dependency in it.
   Unscoped it moved **thirteen** packages including `syn 2.0.118 → 3.0.6` (architect review
   102). Then verify the diff: every changed line must be a version string on one of this
   workspace's own five crates;
3. the next version's `## [x.y.z] — unreleased` CHANGELOG section, or `release-gate`'s preflight
   refuses for want of exactly one matching heading — it is the gate's **first** check;
4. `Last reconciled repository release` → `0.24.0` in all three apex documents.

Then `docs/src/specifications.md`: add the row and make it the newest published version.

## 6. Open, not in this release

- **The `f64`-only disagreement direction at `FRAC_BITS = 24`.** S2's fix removed the fabricated
  diagnosis and widened the missed-evidence loss from one precision to ten. The mechanism for
  the missing direction is **not established** and needs the inner Dykstra snapshots, which no
  public accessor exposes. Reviews 099 §3 and 101 §2 bracket `max|λ|` per sweep cap instead,
  from the onset precision — an inference with its assumption named, not a measurement.
- **Whether `check-public-api` should carry an API-shape baseline.** It is a forbidden-token
  sweep, so RFC 044's `pub` associated const and S5's three new methods both cleared every
  gate with nothing to say (review 100 §8). Raised, not decided; its own RFC if taken up.
- **T6's three remaining items** — metadata-only observability, the mock-only `ffi-gateway`
  seam, the process-local validation cache. Closed until a consumer exists.
- **A distance-aware default step for the curvature-free case** (RFC 039's open item).
