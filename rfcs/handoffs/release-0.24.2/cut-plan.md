# Release 0.24.2 — cut plan

**Author tier.** `architect`. **Date:** 2026-10-31.
**Status.** Planned, not executed. Awaiting the owner's authorization (§3).
**Scope record.** `rfcs/handoffs/release-0.24.2/scope-and-decision-points.md` — the owner's record
of what is in scope and why.
**Evidence.** Architect reviews 122, 123 and 124 accepted RFC 053 and its two corrections slices.

**Whose file this is: the architect's.** It is a pre-flight for a multi-step, irreversible
procedure, written before being executed rather than recalled during it. **One decision in it is
the owner's and one is the owner's alone: §3.2, publication** (RFC 021 §7).

## 1. The position is `0.24.2`, and nobody bumps

`docs/src/development.md`'s release version convention derives it: *only measures or documents* →
**the patch already on `main`**, bumped by nobody, "the placeholder was right".

Measured on `a53cff7`:

```sh
git diff --name-only 0.24.1..HEAD | grep '^crates/'   # → empty
```

No crate source, no crate `README.md`, no `LICENSE`. The workspace, the apex `This tree`, the
single `## [0.24.2] — unreleased` heading and every slice's
`release-gate --intended-tag 0.24.2` all already agree. **There is no bump step in §5.**

This was got right at the start this time. At `0.24.1` I named `0.25.0` from the size of the
sequence rather than from the rule (review 121 §5 item 1), and when I proposed this release's
directory I named it `release-0.25.0` again before deriving the position. Both were corrected
before any gate ran against a guessed version; the directory you are reading is the correction.

## 2. Scope — one RFC

| RFC | moves to `rfcs/done/` | why |
| --- | --- | --- |
| 053 a sequence the repository holds | **yes** | implemented, Amendment 1, plus two corrections slices (reviews 122, 123, 124) |
| 047 the state worth aiming at | no | a destination; it prescribes nothing to implement, and its phases II-VI are `deferred` — now visibly, in `rfcs/destination-sequence.md` |

**RFC 053's Status line, at the move**, exactly:

```text
**Status.** Implemented (v0.24.2). Amendment 1 was made while Accepted, under RFC 000's in-place-amendment rule.
```

`check-rfcs` requires a `done/` RFC's Status line to name its amendments, and `doc-currency`
forbids that clause while the RFC is Accepted — so this edit is **only** correct as part of the
move, never before it.

### 2.1 The second gapped scope string

`rfcs/done/` then holds 001-046 and 048-053, so `implemented_scope` renders:

```text
Implemented scope: **RFCs 001-046, 048-053**.
```

Byte-identical in all **seven** `APEX_DOCS` (`xtask/src/checks/doc_currency.rs:17-25`), or
`check_apex_currency` reports `APEX MISMATCH`. The new release-table cell carries the same gapped
value — the second such cell, after `0.24.1`'s. The 047 gap is now permanent in practice and still
has not cost anything; the `0.24.1` cut plan §2.1 said I would not propose changing what the scope
field means until it did, and it still has not.

## 3. What the owner is asked

**Two decisions, with their costs.**

### 3.1 Authorize the cut

Everything else here is mechanical once this is given.

### 3.2 Publish to crates.io, or distribute only — and I recommend **publish**

RFC 021 §7 makes publication separately authorized, and it is irreversible: a version can be
yanked, never replaced.

**Measured first, because it decides the shape of both options.** The only packaged paths that
changed since `0.24.1` are `Cargo.toml` and `Cargo.lock`, and only by the post-release version
bump:

```sh
git diff --name-only 0.24.1..HEAD | grep -E '^(crates/|Cargo)'
# → Cargo.lock, Cargo.toml
```

So a published `0.24.2` would carry **library code byte-identical to `0.24.1`**, differing in its
own version number and nothing else.

- **Publish.**
  *Cost:* five crates published whose source is unchanged; a dependent on `loeres = "0.24"` picks
  it up from `cargo update` and gets nothing but a version number. Five irreversible artifacts
  that add no library content.
  *Gain:* one invariant stays simple — the newest published version equals the newest release — so
  `docs/src/specifications.md`'s crates.io paragraph keeps saying the registry and this tree
  describe the same library, and RFC 052 clause 5 keeps asserting it.
- **Distribute only** (`released (…); not published to crates.io`).
  *Cost:* the newest published version stays `0.24.1`, which is identical library code, so a
  registry user loses nothing — **but** `docs/src/specifications.md:52-56` says *"`0.21.0` through
  `0.22.0` were tagged and distributed but never published, and they stay that way"*, and with
  `0.24.2` also unpublished that sentence understates the set. RFC 052 §4 deliberately left that
  paragraph out of the assertion, so **no gate would catch it** — it would have to be corrected by
  hand, in exactly the class of claim this release exists to stop maintaining by hand.
  *Gain:* no five-crate publication of byte-identical source.

**Recommendation: publish.** It is the same answer as `0.24.1` and for the reason the owner
supplied there: there is no production use yet, so the one real cost of publishing — churn in
someone's lockfile for no benefit — is approximately zero, while distribute-only would add a
hand-maintained sentence that nothing holds. **I am applying that reasoning from the start rather
than opening with distribute-only and being asked again.** If production use begins, this
calculation changes and the recommendation should be re-derived, not recalled.

## 4. What is different at this cut

1. **The three gates that first moved values by hand at `0.24.1` now do it for the second time.**
   The apex scope field in seven documents (RFC 048), the README's advertised minor (RFC 049), and
   the release-table row (RFC 052). **The README does not change again**: RFC 049's predicate keys
   on the `MAJOR.MINOR` of the last *released* version, and `0.24.1 → 0.24.2` leaves it `"0.24"`.
2. **A fourth gate now has an opinion about this cut.** RFC 053's clause 3 binds a `shipped`
   phase's RFCs to `rfcs/done/`. Phase I names 049, 050 and 051, all already in `done/`, so the
   move of RFC 053 itself changes nothing for it — but the file is now part of the cut's surface
   and **`rfcs/destination-sequence.md` must not acquire an apex currency block**. It is not in
   `APEX_DOCS` and makes no release claim; adding it would make the cut edit eight documents and
   claim something the file has no business claiming.
3. **The one-commit coupling holds, as it did at `0.24.1`.** RFC 052 clause 1 requires the new
   table row the moment `CHANGELOG.md` marks the release `released`, and clause 4 requires the last
   row to agree with the apex `Last reconciled` — so marking released, adding the row and moving
   `Last reconciled` must be **one** commit (§6).
4. **No measurement is owed by this release.** `0.24.1` carried RFC 046 §3.3's `release-gate`
   figure; nothing here promises a number.

## 5. The cut, in order

1. **RFC 053 `accepted/` → `done/`**, with §2's Status line, and its index row in `rfcs/README.md`
   moved from the Accepted table to the Done table with `Implemented (v0.24.2)`. `None currently.`
   stays under `## Proposed RFCs`; the Accepted table keeps RFC 047 alone.
2. **Apex `Implemented scope` → `RFCs 001-046, 048-053`** in all seven `APEX_DOCS`, byte-identical.
3. **`CHANGELOG.md`**: date the `## [0.24.2]` heading and add the subtitle. Keep
   `**Release status:** unreleased` — the release has not happened yet, and §6 sets it.
   Proposed heading:

   ```text
   ## [0.24.2] — 2026-10-31 — A sequence the repository holds, and a roadmap that cannot call a shipped release upcoming
   ```

4. **Check, do not edit:** `This tree: 0.24.2` in all seven apex documents and the workspace
   version. `Last reconciled` stays `0.24.1` until §6 — RFC 024's inequality is strict.
5. `cargo xtask check` → **20 gates PASS**, on the gate's own exit status, never through a pipe.
6. `cargo xtask release-gate --intended-tag 0.24.2` → PASS. Whole stream to a file. The gate
   refuses on a dirty tree, so commit first.
7. Commit, push `main`.
8. **Recompute the RFC 028 anchor by hand** at the revision to be tagged, three ways: the archive
   bytes, `gunzip -c <archive> | sha256sum`, and `git archive --format=tar <rev> | sha256sum`. The
   last two must be equal; that value is the cross-environment identity.
9. **Tag `0.24.2`** annotated, confirm it peels to the intended revision, push it.
10. **Wait for the tagged CI `release-gate` job** with `gh run watch` before any `distributed`
    claim (RFC 021 §7). A polling loop without a delay does not wait.

## 6. The post-release commit — one commit

1. `CHANGELOG.md` `0.24.2` → `released (tagged 2026-10-31, distributed 2026-10-31)`, plus
   `; published to crates.io` **only if §3.2 chose publish** — the phrase must equal one of the two
   literals `doc_currency.rs` compares, or `is_published` fails closed.
2. A new `## [0.24.3] — unreleased` section, or `release-gate`'s **first** preflight check refuses
   for want of exactly one heading matching the workspace version.
3. `docs/src/specifications.md`: the new row `| \`0.24.2\` | 2026-10-31 | 001-046, 048-053 |`, and
   the `What is on crates.io.` paragraph moved to `0.24.2` **if published** — RFC 052 clause 5
   binds it to the newest *published* version, so if not published it must stay at `0.24.1` and
   the `0.21.0 through 0.22.0` sentence needs the correction §3.2 names.
4. All seven apex documents: `Last reconciled` → `0.24.2`, `This tree` → `0.24.3`.
5. The workspace version and the **five** internal `[workspace.dependencies]` pins, or
   `published-metadata` fails closed.
6. **All eight lockfiles, each updated with `--workspace`** — unscoped, `cargo update` relocks
   every third-party dependency the ranges permit, which on this tree once moved thirteen packages
   including a `syn` major (architect review 102). Then **verify the diff**: every changed line must
   be a version string on one of this workspace's own five crates. At `0.24.1` that check came out
   at 27 changed lines and nothing else.

**Advisory drift at cut time is normal maintenance, not an emergency** (RFC 045 Amendment 1). The
fix pattern is a **scoped** `cargo update -p <crate>`, never a bare one.

## 7. Risks

- **Functional — Low.** No crate source changed. The only executable change in this release is
  inside `xtask`, which ships to nobody.
- **Business — Low.** Nothing a user can call changed. The README's advertised requirement does not
  move, so a snippet copied before this release stays valid through it.
- **Operational — Low.** The couplings are the same ones walked successfully at `0.24.1`, with one
  more gate watching (§4 item 2). The failure mode is a red gate on `main`, not a bad release.
- **Security — Low.** No trust-root, channel or artifact-identity change. Publication, if
  authorized, republishes byte-identical library source under a new version.

## 8. Open after `0.24.2`

- **RFC 047's Phases II-VI** stay `deferred` — in `rfcs/destination-sequence.md`, where a gate
  reads them, rather than in anyone's memory. That was the owner's question on 2026-10-31 and it is
  now answered by a file.
- **Phase II's audience question.** The owner's. Phase III is unschedulable until it is answered,
  because review 106 §5 puts property **D** behind it.
- **The plural-form residual** (RFC 053 Amendment 1): `Phases II-VI` is not seen by clause 1;
  clause 4 bounds the exposure, and the first time the residual costs anything is the time to widen
  the matcher.
- **The themes `T4`-`T7` have no definitional home** (RFC 053 §2.3) — a third sequence axis, and
  the question of whether it is one axis or two is worth answering only after Phase II's.
- **`Carries RFCs` is bounded and ordered, not verified** (RFC 052 §0.1). This cut writes its own
  cell by hand from §2; **read it once more before committing step 2 of §6.**
- The three items carried unchanged from `rfcs/handoffs/release-0.24.0/scope-and-decision-points.md`
  §6: the `f64`-only disagreement direction at `FRAC_BITS = 24`; whether `check-public-api` should
  carry an API-shape baseline; and a distance-aware default step for the curvature-free case
  (RFC 039).
