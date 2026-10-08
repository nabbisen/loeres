# Release 0.24.0 — scope and decision points

**Author tier.** `architect`. **Date:** 2026-10-08.
**Status.** Planning. No slice has landed; the workspace is at `0.23.1`, a placeholder.
**Authorized by.** The owner accepted RFC 043 and the release-position recommendation on
2026-10-08. Scheduling is the architect's; **authorizing the cut is the owner's**.

## 1. Scope — one RFC

| | Slice | What it is | Adds callable API? |
| --- | --- | --- | --- |
| A | RFC 043 S1 | the constrained kernel over `Q32`, **measured** against the exact optimum and the `f64` solve | no |
| B | RFC 043 S2 | the infeasibility-evidence predicate reformulated by `checked_div`, both kernels | no |
| C | RFC 043 S3 | the checked-arithmetic-tier question answered in writing | no |
| D | RFC 043 S4 | the coarser-precision `Q32` measurement, threshold in **steps** | no |
| E | RFC 043 S5 | inherent `checked_add`/`checked_sub`/`checked_mul` on `Q32` | **yes** |
| F | RFC 043 S6 | the measured usable `FRAC_BITS` band documented (review 099 §7) | no |
| G | RFC 044 | a const assert closing `FRAC_BITS` 31–63, where `one()` is negative or zero | no |

RFC 044 is its own contract: `rfcs/accepted/044-a-frac-bits-a-caller-cannot-get-wrong.md`,
accepted 2026-10-08, handoff
`rfcs/handoffs/044-a-frac-bits-a-caller-cannot-get-wrong/implementation-handoff.md`. It bumps
nothing and rides whichever release E sets.

Contract: `rfcs/accepted/043-the-constrained-kernel-over-a-bounded-scalar.md`, **including
Amendment 1**. Handoff:
`rfcs/handoffs/043-the-constrained-kernel-over-a-bounded-scalar/implementation-handoff.md`.
Scoping: `.git-exclude/reviewed/096-theme-the-constrained-kernel-over-q32-scoping-2026-10-08.md`,
registered in `rfcs/review-evidence-index.md` row `096`.

RFC 043 absorbs three of the five items
`rfcs/handoffs/release-0.23.0/finalization-checklist.md` §6 left open. The other two are
**not** in this release:

- **T6's three remaining items** — metadata-only observability, the mock-only `ffi-gateway`
  seam, the process-local validation cache. Closed until a consumer exists. Nothing has
  changed that; no consumer has appeared.
- **A distance-aware default step for the curvature-free case** (RFC 039's open item).

## 2. Why this is a minor, and why that is not a breakage claim

**E adds inherent methods to `Q32`.** New callable functionality is the minor position in
semver — that and nothing more. `docs/src/development.md`'s "What each position means"
records why: a minor bump is **not** a signal that a caller must change code, and this
project shipped `0.20.0` as a minor whose changelog says "Runtime crate APIs are
unchanged". A–D, F and G break nothing and add nothing callable; F is documentation and
G is a compile-time assert, so both ride the release E sets.

**E's slice performs the bump**, as the first slice to move the release's position, under
`docs/src/development.md`'s "A slice that moves the release's position bumps the version
before the cut". The four gated places it touches are listed in the handoff §6. A–D run
`release-gate --intended-tag 0.23.1`; E runs `--intended-tag 0.24.0`.

## 3. Decision points — what the owner is asked, and when

| # | Decision | Trigger | Default if nothing is said |
| ---:| --- | --- | --- |
| 1 | **Is B a correctness fix or hardening?** | A's measurement. If A's corpus fires RFC 043 §2's false positive, B fixes a reachable wrong diagnosis; if it never fires, B is hardening against an unreached defect. | The architect reports which, and B proceeds either way. No authorization needed to proceed — this decision changes what the CHANGELOG and the release notes **claim**, not whether the work happens. |
| 2 | **Does E ride along, or does 0.23.1 ship first?** | A's result, per the recommendation the owner accepted on 2026-10-08. If A shows the false positive reachable, A–D are a defect fix worth shipping on their own as **`0.23.1`**, with E pushing **`0.24.0`** afterwards. | E rides along and the release is `0.24.0`. Splitting is the owner's call, not the architect's, because it is an extra publication. |
| 3 | **Fallback if B's division is not negligible on the device profile.** | B's RFC 037 figure. | The architect decides between the multiplicand form plus documentation and keeping the division (RFC 043 §3.2). The owner is told; this is a design call, not a release call. |
| 4 | **Stop condition.** If B's conformance differential shows **any** `f64` verdict moved. | B. | **Work stops and returns to the architect.** A moved verdict on a comparison the codebase explicitly warns against tidying is a finding, not a merge conflict. |

## 4. What the dev team preps, and what it does not

Per the owner's standing division: the dev team may prepare everything except the cut
itself, which the architect and the owner perform together.

**Prepped by the dev team:** all five slices, their review requests, the `CHANGELOG.md`
entries, and E's version bump with its four couplings.

**Not prepped by the dev team:** the finalization revision, the tag, the push, the RFC 028
anchor computation, and publication. Those are the cut.

## 5. The cut, when it comes

A `rfcs/handoffs/release-0.24.0/finalization-checklist.md` is written at the cut, not now,
following `rfcs/handoffs/release-0.23.0/finalization-checklist.md`. Two things it will
carry that are already known:

- **The version will already be bumped** by E's slice, so its step 5 is a **check**, not an
  edit — the same shape as 0.23.0, where RFC 042's slice did the bump.
- **`Last reconciled repository release` stays at `0.23.0`** until the post-release commit.
  RFC 024's inequality is strict; equality is never valid.

The RFC 028 uncompressed-tar anchor has held **eleven consecutive releases**. It is
recomputed by hand and verified three ways at every cut, and `distributed` is claimed only
after `gh run watch` confirms the tagged `release-gate` job — not after the push.
