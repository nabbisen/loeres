# Release 0.25.0 — cut plan

**Author tier.** `architect`. **Date:** 2026-11-24.
**Status.** Planned, not executed. Awaiting the owner's authorization (§3).
**Scope record.** `rfcs/handoffs/release-0.25.0/scope-and-decision-points.md`.
**Evidence.** Architect reviews 125-132. RFC 054 accepted at review 132; RFC 055 at review 126,
with reviews 127-130 correcting the `size-budget` gate the toolchain broke mid-sequence.

**Whose file this is: the architect's.** A pre-flight for an irreversible procedure, written
before execution rather than recalled during it. **Two decisions are the owner's: §3.1 and §3.2,
and §3.2 is a real decision this time** — unlike `0.24.1` and `0.24.2`, this release changes
library code.

## 1. The position is `0.25.0`, and the bump is already done

`docs/src/development.md`'s table: *breaks a caller* → a minor, bumped by the first such slice.
RFC 055's slice performed it. So **§5 contains no bump step** — step 4 is a check.

Verified on `3b03273`: workspace `0.25.0`; the five internal pins `0.25.0`; seven apex
`This tree: 0.25.0`; `Last reconciled: 0.24.2`, which RFC 024's strict inequality requires until
§6; exactly one `## [0.25.0]` CHANGELOG heading.

## 2. Scope — two RFCs, both amended

| RFC | moves to `rfcs/done/` | why |
| --- | --- | --- |
| 054 a suitability claim a gate can assert | **yes** | implemented, Amendment 1, plus the completeness slice (reviews 125, 131, 132) |
| 055 no public module that declares nothing | **yes** | implemented, Amendment 1 (reviews 126, and 127-130 on the gate it exposed) |
| 047 the state worth aiming at | no | a destination; its phases are held in `rfcs/destination-sequence.md`, II-VI `deferred` |
| 056 a release page that states the anchor | no | accepted and deliberately sequenced **after** this release — release machinery must not change in the release that uses it |

**This is the first cut to move two amended RFCs.** Both Status lines, exactly:

```text
**Status.** Implemented (v0.25.0). Amendment 1 was made while Accepted, under RFC 000's in-place-amendment rule.
```

`check-rfcs` requires a `done/` RFC's Status line to name its amendments and `doc-currency`
forbids the clause while Accepted, so each edit is correct **only** as part of the move.

### 2.1 The apex scope, still gapped at 047

`rfcs/done/` then holds 001-046 and 048-055, so `implemented_scope` renders:

```text
Implemented scope: **RFCs 001-046, 048-055**.
```

Byte-identical in all seven `APEX_DOCS`, or `check_apex_currency` reports `APEX MISMATCH`. The
third gapped scope string; 047's gap is now ordinary and still costs nothing.

## 3. What the owner is asked

### 3.1 Authorize the cut

Everything else here is mechanical once this is given.

### 3.2 Publish to crates.io, or distribute only — and I recommend **publish**

RFC 021 §7 makes publication separately authorized, and it is irreversible: a version can be
yanked, never replaced.

**Measured first, because it makes this decision unlike the last two.** `0.24.1` and `0.24.2` were
byte-identical library code; this one is not:

```sh
git diff --name-only 0.24.2..HEAD -- crates/   # → six files
```

Four public paths are **removed** — `loeres_backend_std::{adapter, batch, view}` and
`loeres_device::diagnostic` — and `cargo package --list` is three files shorter for
`loeres-backend-std` and one shorter for `loeres-device`.

- **Publish.**
  *Cost:* the break reaches the registry. A dependent pinned `loeres = "0.25"` picks it up and, if
  it names one of those four modules, stops compiling.
  *Gain:* the newest published version equals the newest release, which RFC 052 clause 5 keeps
  asserting; and **Cargo's own `0.x` rules insulate the users who have not opted in** — a dependent
  on `"0.24"` resolves to `>=0.24, <0.25` and does **not** receive this from `cargo update`. The
  break reaches only someone who deliberately moves their requirement, which is exactly what a
  minor is for.
- **Distribute only.**
  *Cost:* the newest published version stays `0.24.2`, so the registry no longer describes this
  tree — the first release where that statement would be false rather than merely stale. And
  `docs/src/specifications.md:52-56`'s *"`0.21.0` through `0.22.0` were tagged and distributed but
  never published, and they stay that way"* would understate the set, which RFC 052 §4 left out of
  the assertion, so **no gate would catch it**.
  *Gain:* the removed paths never reach a registry user.

**Recommendation: publish.** The break is expressed in the minor, which is the mechanism's whole
purpose, and the users who did not ask for it are protected by resolution rather than by our
withholding the release. There is still no production use, so the cost is bounded by a
deliberate opt-in.

## 4. What is different at this cut

1. **The README changes, for the first time since RFC 049 shipped.** `check_readme_requirements`
   binds the README's advertised requirement to the `MAJOR.MINOR` of **`Last reconciled`**
   (`doc_currency.rs:379`, `:1543-1573`). §6 moves that to `0.25.0`, so the expected minor becomes
   `0.25` and `README.md:25,26,29,30` must move from `"0.24"` to `"0.25"` **in that same commit**
   or `doc-currency` reports four `README VERSION` findings. The last two releases left the README
   untouched because they shared a minor with their predecessor; this one does not.
   **This is the gate doing what RFC 049 built it for**, and the first time it has moved the value.
2. **Two amended RFCs move at once** (§2) — a form `check-rfcs` enforces only at the move.
3. **`CHANGELOG.md` needs three corrections at the cut, not just a date** (§5 step 3).
4. **The release carries a break and an addition**, and the entry already states them separately.
5. **The `size-budget` gate was rebuilt mid-sequence** by reviews 127-130, after `rustc 1.100.0`
   moved the compiled object and four ambient variables were found able to move or break the
   measurement. The pinned baseline was **not** touched: it still reads `10728` against `10736`
   across a compiler minor bump and five build-flag perturbations. Nothing about the cut depends
   on this beyond knowing the gate is now environment-independent.

## 5. The cut, in order

1. **RFCs 054 and 055 `accepted/` → `done/`**, with §2's Status lines, and their index rows in
   `rfcs/README.md` moved to the Done table with `Implemented (v0.25.0)`. `None currently.` stays
   under `## Proposed RFCs`; the Accepted table keeps **047 and 056**.
2. **Apex `Implemented scope` → `RFCs 001-046, 048-055`** in all seven `APEX_DOCS`,
   byte-identical.
3. **`CHANGELOG.md` — three edits:**
   - date the heading and add the subtitle:

     ```text
     ## [0.25.0] — 2026-11-24 — Who this is for, said once in properties, and four public modules that declared nothing
     ```

   - **the lead paragraph names only RFC 055.** It must name both: this release carries RFC 054 and
     RFC 055, and the minor is forced by RFC 055's source break.
   - **architect review 132 F1:** the new *"What is left, and why it is not a residue"* paragraph
     ends *"will find exactly these six, deliberately kept."* A reader who greps finds **ten**
     occurrences, of which **eight** are deliberate — the six named plus
     `loeres-roadmap-milestones-v1.md:726` and `:1143`, RFC 008's section title and its
     dependency-graph node. Restate it as **these eight, of ten occurrences**, naming the title
     sites. Recorded here because it is the one correction that would otherwise be lost between
     the review and the cut.

   - **architect review 133 F1, a second correction in the same paragraph:** the replacement
     sentence the prep slice landed reads *"will find **ten** occurrences, of which these **eight**
     are deliberately kept"* — and the paragraph names **ten** sites (six non-defects, two
     disclaimers, two historical titles), all of them deliberately kept. The *"eight"* is mine,
     carried from review 132 §2's own partition error. Replace the clause with *"and every one of
     them is deliberately kept"*: a sentence with one number cannot disagree with itself.

   Keep `**Release status:** unreleased` — §6 sets it.
4. **Check, do not edit:** §1's five facts.
5. `cargo xtask check` → **20 gates PASS**, on the gate's own exit status, never through a pipe.
6. `cargo xtask release-gate --intended-tag 0.25.0` → PASS. Whole stream to a file; commit first,
   the gate refuses on a dirty tree.
7. Commit, push `main`.
8. **Recompute the RFC 028 anchor by hand** at the revision to be tagged, three ways: the archive
   bytes, `gunzip -c <archive> | sha256sum`, and `git archive --format=tar <rev> | sha256sum`. The
   last two must agree; that value is the cross-environment identity.
9. **Tag `0.25.0`** annotated, confirm it peels to the intended revision, push it.
10. **Wait for the tagged CI `release-gate` job** with `gh run watch` before any `distributed`
    claim (RFC 021 §7). A polling loop without a delay does not wait.

## 6. The post-release commit — one commit, and larger than the last two

1. `CHANGELOG.md` `0.25.0` → `released (tagged 2026-11-24, distributed 2026-11-24)`, plus
   `; published to crates.io` **only if §3.2 chose publish** — the phrase must equal one of the two
   literals `doc_currency.rs:1347-1348` compares, or `is_published` fails closed.
2. A new `## [0.25.1] — unreleased` section, or `release-gate`'s **first** preflight refuses.
3. **`README.md:25,26,29,30`: `"0.24"` → `"0.25"`** (§4 item 1). Four lines, and the gate enforces
   them the moment item 5 lands.
4. `docs/src/specifications.md`: the row `| \`0.25.0\` | 2026-11-24 | 001-046, 048-055 |`, and the
   `What is on crates.io.` paragraph moved to `0.25.0` **if published** — RFC 052 clause 5 binds it
   to the newest *published* version. **Its current last sentence must change either way:** it says
   the five crates' sources are byte-identical across `0.24.0`-`0.24.2` and *"a dependent on
   `loeres = "0.24"` needs no action"*. For `0.25.0` the truthful statement is that four public
   paths are removed and a dependent on `"0.24"` is unaffected **because Cargo will not resolve
   them to it**.
5. All seven apex documents: `Last reconciled` → `0.25.0`, `This tree` → `0.25.1`.
6. The workspace version and the **five** internal `[workspace.dependencies]` pins, or
   `published-metadata` fails closed.
7. **All eight lockfiles, each with `--workspace`.** Unscoped, `cargo update` relocks every
   third-party dependency the ranges permit — thirteen packages including a `syn` major, once
   (architect review 102). Then verify: every changed line a version string on this workspace's own
   five crates. `0.24.2` and `0.25.0`'s bump each came out at **27 lines**.

**Advisory drift at cut time is normal maintenance, not an emergency** (RFC 045 Amendment 1); the
fix pattern is a **scoped** `cargo update -p <crate>`, never a bare one.

## 7. Risks

- **Functional — Low.** No solve path, no numerical behaviour, no error mapping changed. The only
  source change is the removal of four modules that declared nothing.
- **Business — Low.** The break cannot reach a user who has not changed their requirement
  (§3.2), and the README's move to `"0.25"` is what tells a new reader which minor to ask for.
- **Operational — Medium, and lower than it was.** The post-release commit is the largest of the
  three recent ones — it now moves the README too — and every part of it is gate-held, so the
  failure mode is a red gate on `main` rather than a bad release. `size-budget` is
  environment-independent as of `fa3e0c3`, which removes the hazard that would otherwise have made
  a cut on a fresh shell unpredictable.
- **Security — Low.** No trust-root, channel or artifact-identity change. Publication, if
  authorized, ships source with four fewer public paths.

## 8. Open after `0.25.0`

- **RFC 056** as `0.25.1`: the GitHub Release page that states the RFC 028 anchor, with review
  130 §3's duplicated-list nit and review 127 E2's `cargo::unused_dependencies` question.
- **The reproducibility question, owner-facing and still unowned** (reviews 128 §2.1, 129 §4,
  130 §4): `rust-toolchain.toml` pins a moving `channel = "stable"` in a project whose central
  claim is evidence a stranger can reproduce. Four data points from this sequence alone.
- **RFC 047's Phases II-VI** stay `deferred`; **Phase II's audience question** is the owner's, and
  Phase III waits on it. RFC 054 closed Phase II's *"and say it"* half.
- **The themes `T4`-`T7`** still have no definitional home (RFC 053 §2.3); **`Carries RFCs`** is
  bounded and ordered, not verified (RFC 052 §0.1); and the `0.25.0` scope file's §1.1
  linear-objective step is **unimplemented** — this release carries neither of that scope's two
  original items' capability work, only the audience correction and the module removal.
- Carried from `rfcs/handoffs/release-0.24.0/scope-and-decision-points.md` §6: the `f64`-only
  disagreement direction at `FRAC_BITS = 24`; whether `check-public-api` should carry an API-shape
  baseline.
