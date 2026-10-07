# Release `0.23.0` — finalization checklist

**Status:** ready to cut, awaiting the owner's authorization.
**Authorized by.** Architect review 095 §10 recommends GO; architect review 094 scoped T7;
architect reviews 092 and 093 closed RFCs 039 and 040. The owner authorizes the release.

**Who runs this.** The architect and the owner. The cut and the publication may not be
delegated.

## 1. Scope — and why this one is a minor

`0.23.0` is the project's first **breaking** release since `0.22.0`.

**RFC 042 — the batch seam carries what the solve found.** A constrained problem solved directly
yielded the terminal violation, the projection cap-hit count and the heuristic infeasibility
hint; solved through `solve_batch` it yielded status only, the batch having computed all three
and discarded them. `BatchItemOutcome::Solved` now carries them as
`Option<ConstrainedBatchDetail<S>>` behind `constrained_detail()`, where `None` means the item
had no inequalities and `Some` with a zero violation means it had them and they were satisfied.

The variant is now `#[non_exhaustive]`, which is **the only mechanism Rust offers** here:
RFC 014 §311's private-field-plus-accessor pattern cannot be applied to an enum variant's own
fields (`error[E0449]`, visibility qualifiers not permitted), as review 095 §2 records.

**RFC 041 — a fixed-point scalar baseline.** `Q32<const FRAC_BITS: u32>`, saturating, behind
`fixed-point-hooks`, which stops gating nothing. `FiniteScalar` is satisfied **vacuously** and
the type documents what that costs: the kernels' 43 `is_finite()` guards do not protect this
family. The box kernel is demonstrated over it against an exact reference — 300 instances, 300
converged, zero converged-but-wrong, and `Q32`'s deviation matching the `f64` solve's own to five
significant figures.

## 2. The version is already bumped

Unlike every previous cut, the workspace is **already at `0.23.0`**: RFC 042's slice bumped it,
because `release-gate --intended-tag 0.23.0` validates the intended tag against the workspace
version and the gate is required of every slice.
`docs/src/development.md`'s "A breaking slice bumps the minor before the cut" records this as the
rule rather than an exception. So step 5 below is a **check**, not an edit.

`Last reconciled repository release` is at `0.22.3` and must stay there until the post-release
commit. RFC 024's inequality is strict.

## 3. The cut

1. **RFCs 041 and 042 to done.** `accepted/` → `done/`, status `Implemented (v0.23.0)`, and
   **both must name Amendment 1 in their Status lines** — `check-rfcs` requires it of a `done/`
   RFC and `doc-currency` forbids it while the RFC is Accepted.
2. RFC index rows repointed.
3. **Apex scope to `RFCs 001-042`**, derived from `rfcs/done/`.
4. **`CHANGELOG.md`**: date the `## [0.23.0]` heading, keep `**Release status:** unreleased`.
   The entry must name the **verified** error codes a caller will see and the fix for each:
   `error[E0638]` and `error[E0027]` together for a pattern on `Solved` without a trailing `..`
   (add `..`), and `error[E0639]` for construction by struct literal (stop constructing it — it
   is an output). **Not `E0063`**, which `#[non_exhaustive]` makes unreachable from outside the
   defining crate; review 095 §1 has the proof.
5. **Check** `This tree: 0.23.0` in all three apex documents and the workspace version — already
   set (§2), so confirm rather than edit.
6. `cargo xtask check` → 20 gates PASS.
7. `cargo xtask release-gate --intended-tag 0.23.0` → PASS. Whole stream to a file, never piped.
   The gate refuses on a dirty tree, so commit first.
8. Commit, push `main`.
9. **Recompute the RFC 028 anchor by hand** at the revision to be tagged.
10. Tag `0.23.0`, confirm it peels to `HEAD`, push. **Wait for the tagged CI `release-gate` job**
    with `gh run watch` before claiming `distributed` (RFC 021 §7); a polling loop without a
    delay does not wait.

## 4. Publication

As before: authorized separately (RFC 021 §7), `published-metadata` and
`cargo package -p <crate> --list` first, then publish in dependency order — `loeres`,
`loeres-backend-static`, `loeres-backend-std`, `loeres-device`, `loeres-cluster` — and confirm
all five appear in the crates.io index before writing any `published` claim. Irreversible: a
version can be yanked, never replaced.

**This release breaks callers.** The CHANGELOG entry is the only thing a downstream user has to
act on, which is why §3 item 4 requires the verified codes rather than the plausible ones.

## 5. Post-release commit — the three couplings

1. the five internal requirements in `[workspace.dependencies]`, or `published-metadata` fails
   closed;
2. **every workspace-excluded crate's own lockfile** — a rule, not a count, which has now gone
   stale three times: five at `0.22.1`, six after RFC 037, seven after RFC 040, and
   `docs/src/development.md` said seven until RFC 042's slice corrected it to **eight**.
   Enumerate them:
   `for d in examples/*/ device-size-reference/; do cargo update --manifest-path "$d/Cargo.toml" --offline; done`;
3. the next version's `## [x.y.z] — unreleased` CHANGELOG section, or `release-gate`'s preflight
   refuses for want of exactly one matching heading.

Then `docs/src/specifications.md`: add the row and make it the newest published version.

## 6. Open, not in this release

- **A coarser-precision measurement for `Q32`, with a step-relative threshold.** Review 095 §7:
  S2's `1e-3` converged-but-wrong threshold is absolute, and at `FRAC_BITS = 20` the measured
  worst deviation of `4.5e-5` is about **47 quantization steps**. At `FRAC_BITS = 12` the step is
  `≈2.4e-4`, so the same 47 steps would be `≈1.1e-2` — eleven times past the absolute threshold,
  reading as a wrong answer when it is merely a coarser one. Repeat the measurement at a second
  precision with the threshold expressed in **steps**, or the experiment answers the wrong
  question.
- **Inherent `checked_add`/`checked_sub`/`checked_mul` on `Q32`**, the narrower alternative to a
  checked-arithmetic tier (review 095 §8). Commits no other `BaseScalar` implementor to anything.
- **The constrained kernel over `Q32`.** RFC 041 §4's non-scope, and the named trigger that would
  reopen the tier question: the Dykstra projection does far more arithmetic per outer iteration,
  across more rows, than the box kernel's single clamp. Whoever takes it up measures it as S2 did.
- **T6's other three items** — metadata-only observability, the mock-only `ffi-gateway` seam, the
  process-local validation cache — closed until a consumer exists.
- **A distance-aware default step for the curvature-free case** (RFC 039's open item).
