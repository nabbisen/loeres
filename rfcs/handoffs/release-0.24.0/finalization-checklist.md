# Release 0.24.0 — finalization checklist

**Author tier.** `architect`. **Date:** 2026-10-08.
**Authorized by.** The owner authorized the cut on 2026-10-08.

**Whose file this is: the architect's.** It is a pre-flight for the cut — a multi-step,
irreversible procedure whose couplings were each found by a gate failing, which is why the
steps are written down before being executed rather than recalled. The owner's record of this
release is `rfcs/handoffs/release-0.24.0/scope-and-decision-points.md`: what was in scope, what
was decided and at what cost. The dev team's instructions were the two RFC handoffs, and
`rfcs/handoffs/043-the-constrained-kernel-over-a-bounded-scalar/implementation-handoff.md` §6.1
says where their work stops — at this file.

**One decision here is the owner's: §4, publication.** RFC 021 §7 makes it separately
authorized, and it is irreversible.

## 1. Scope

**RFC 043** (the constrained kernel over a bounded scalar, Amendments 1–4) and **RFC 044** (a
`FRAC_BITS` a caller cannot get wrong, Amendment 1), with six architect reviews, `097`–`102`,
registered in `rfcs/review-evidence-index.md`.

Stated once, not twice: the scope narrative and the decision record live in
`rfcs/handoffs/release-0.24.0/scope-and-decision-points.md`. If the two ever disagree, that
file is the intent.

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

In `rfcs/handoffs/release-0.24.0/scope-and-decision-points.md` §6, where the next release's
planning reads them — not here, where only the cut does.
