# Release 0.24.1 — cut plan

**Author tier.** `architect`. **Date:** 2026-10-31.
**Status.** Plan. The cut is **not** authorized by this file; §3 is what the owner is asked.

**Whose file this is: the architect's.** It is a pre-flight for an irreversible, multi-step
procedure, written before being executed rather than recalled. It is also this release's scope and
decision record — one file rather than two, because the one thing this release exists to stop is a
claim maintained by hand in more than one place.

**It states what is *different* at this cut and the steps in order. It does not restate the
release-version convention, the gate list, or the publication mechanics** — those live in
`docs/src/development.md` and RFC 021 §7, and duplicating them here is the defect RFC 048 removed.

## 1. The position is `0.24.1`, not `0.25.0`

Measured against `docs/src/development.md`'s "A slice that moves the release's position bumps the
version before the cut" and its table: a slice that *fixes a defect, or only measures or documents*
releases as **the patch already on `main`**, bumped by nobody, because the placeholder was right.

```
$ git diff --stat 0.24.0..HEAD -- crates/
(no output)
```

**Not one byte under `crates/` has changed since the `0.24.0` tag.** No public API was added and
none broke, so nothing forces a minor. The workspace is at `0.24.1`, the apex block says
`This tree: 0.24.1`, `CHANGELOG.md` carries one `## [0.24.1] — unreleased` heading, and every slice
in this sequence ran `release-gate --intended-tag 0.24.1`. The tree has been telling us the
position all along.

**I said `0.25.0` in architect review 121 §5 and in reporting it.** That was asserted from the
size of the sequence rather than read from the convention, and it would have mis-scheduled the
release — the eighth figure in five slices I stated without deriving it the way the tree does.
Review 121 is corrected and its hash re-registered.

One consequence worth naming now, because it is the first live test of RFC 049: the README's
advertised requirement keys on `MAJOR.MINOR` of `Last reconciled repository release`. `0.24.0` and
`0.24.1` share a minor, so **the README does not change at this cut**. That is the cost RFC 049 §3.1
chose deliberately, and this is the first cut at which it costs nothing.

## 2. Scope — seven RFCs, and one that stays

| RFC | moves to `done/` | why |
| --- | --- | --- |
| 045 gates that fire where they matter | yes | S1 implemented; S2 withdrawn by its own Amendment 1 |
| 046 pinned tools installed by identity | yes | slices A-F implemented, reviews 105-113 |
| 047 the state worth aiming at | **no** | a destination. It prescribes nothing to implement, and Phases II-VI are unscheduled |
| 048 one release list that cannot rot | yes | implemented |
| 049 the README tells a user what to write | yes | implemented, Amendment 1 |
| 050 a link the book can follow | yes | implemented |
| 051 one import model, one requirement | yes | implemented |
| 052 a release table that cannot omit a release | yes | implemented, Amendment 1, plus its corrections slice |

Plus the `tokio 1.54.1` bump for RUSTSEC-2026-0386, already in the `0.24.1` entry.

### 2.1 The implemented scope becomes a **gapped** range, and that is the mechanism working

With RFC 047 left in `accepted/`, `rfcs/done/` holds 001-046 and 048-052, so `implemented_scope`
renders:

```
Implemented scope: **RFCs 001-046, 048-052**.
```

**This is the first gapped scope string this project has carried.** It is correct, and it is what
the mechanism was built for: `implemented_scope_at`'s own comment records that *"a maximum is not a
set: this binds every number actually present, so a withdrawn or accepted-but-not-done RFC below
the highest implemented number cannot be certified as implemented (review 036 blocker B2)"*.

Three things already handle it, none of which needs changing:

- `render_scope` emits the comma-separated form.
- `check_apex_currency` compares the field to that exact string in all seven `APEX_DOCS`.
- RFC 052's `max_scope_number` reads every digit run, so the bound in its clause 4 sees `52`. Its
  test `release_table_scope_with_a_gap_parses_to_its_true_maximum` exists for precisely this.

**The release table's new row will carry the same gapped cell**, `001-046, 048-052`, which is the
first non-`001-NNN` cell in that table. Clause 4 bounds and orders it; nothing requires it to be
contiguous.

**A question, not a decision for this cut:** if RFC 047 is never "implemented" as a unit, that gap
is permanent, and every future scope string and table cell carries it. The honest options are to
leave it (it is true), or to extend `implemented_scope`'s RFC 000 exclusion to destination RFCs,
which needs its own RFC because it changes what the scope field means. **Leave it at this cut.** I
will not propose the second until the gap has actually cost something.

## 3. What the owner is asked

**Two decisions, with their costs.**

### 3.1 Authorize the cut

Everything else in this file is mechanical once this is given.

### 3.2 Publish to crates.io, or distribute only — and I recommend **distribute only**

RFC 021 §7 makes publication separately authorized and it is irreversible: a version can be yanked,
never replaced.

- **Distribute only** (`released (…); not published to crates.io`).
  *Cost:* the newest published version stays `0.24.0`, so a user installing from the registry gets
  `0.24.0` — which is **identical library code**, since nothing under `crates/` changed. They lose
  nothing. `docs/src/specifications.md`'s `What is on crates.io.` paragraph needs **no edit**, and
  RFC 052 clause 5 asserts that for us. Precedent: `0.21.0` through `0.22.0`.
  *Gain:* no five-crate publication of byte-identical source, and no version churn in anyone's
  lockfile for a release with no user-facing change.
- **Publish.**
  *Cost:* five crates published whose source is unchanged from `0.24.0`; a dependent on
  `loeres = "0.24"` picks it up from `cargo update` and gets nothing. The crates.io paragraph and
  the table row then both move, which RFC 052 holds.
  *Gain:* the registry's newest version matches the newest release, which is one less thing to
  explain.

**Recommendation: distribute only.** This release's whole content is gates, documentation and CI.

**If distribute-only is chosen, one hand-maintained claim needs a clause and is not gate-held:**
`docs/src/specifications.md`'s paragraph at `:52-56` says *"`0.21.0` through `0.22.0` were tagged
and distributed but never published, and they stay that way"*. With `0.24.1` also unpublished that
sentence understates the set. RFC 052 §4 deliberately left that paragraph out of the assertion, so
nothing will catch it — **add the clause in the post-release commit, and do not let it become the
next thing that rots.**

## 4. What is different at this cut

Three gates now move values a human used to move. This is the first cut that tests them, and the
first cut to move an amended RFC at all.

1. **RFC 048 — the apex block is in seven documents, not three.** `APEX_DOCS` is
   `docs/specs/loeres-requirements-v1.md`, `docs/specs/loeres-external-design-v1.md`,
   `docs/specs/loeres-roadmap-milestones-v1.md`, `ROADMAP.md`, `docs/src/introduction.md`,
   `docs/src/specifications.md`, `docs/src/threat-model.md`. Every scope and
   last-reconciled edit below is **seven edits, byte-identical**, or `check_apex_currency` reports
   `APEX MISMATCH`.
2. **RFC 052 — the release table joins the atomic set.** The `0.24.0` checklist ended with *"Then
   `docs/src/specifications.md`: add the row"*, a trailing step. It can no longer trail:
   - clause 1 requires a row for `0.24.1` **the moment** `CHANGELOG.md` marks it `released`;
   - clause 4 requires the table's **last row** to equal the apex `Last reconciled`.

   So marking released, adding the row, and moving `Last reconciled` to `0.24.1` **must be one
   commit**. Any two of the three without the third fails `doc-currency`. Clause 3 also requires
   the new row's `Shipped` cell to be the **distributed** date from the status line, not the tagged
   date.
3. **Four RFCs carry an amendment whose Status line must name it.** `check-rfcs` enforces this for
   `done/` only, so it fails *at* the cut, and `doc-currency` forbids the clause while the RFC is
   Accepted — the two rules are complementary and the transition is the only moment both apply.

   | RFC | heading | Status line at the move |
   | --- | --- | --- |
   | 045 | `## 0.1 Amendment 1 (2026-10-29)` | `Implemented (v0.24.1). Amendment 1 was made while Accepted, under RFC 000's in-place-amendment rule.` |
   | 046 | `## 0.1 Amendment 1 (2026-10-29)` | same form |
   | 049 | `## 0.1 Amendment 1 (2026-10-30)` | same form |
   | 052 | `## 0.1 Amendment 1 (2026-10-30)` | same form |

   The form is `rfcs/done/043-the-constrained-kernel-over-a-bounded-scalar.md`'s and
   `rfcs/done/027-qp-contract-and-constrained-kernel.md`'s. 048, 050 and 051 take the plain
   `Implemented (v0.24.1).`

### 4.1 RFC 046 §3.3 item 3's figure cannot be recorded where §3.5 says

§3.5 requires the `release-gate` job duration with its per-step breakdown, measured **at a real
cut**, recorded **in §3.3**. The number does not exist until the tagged job has run — by which time
RFC 046 is in `rfcs/done/`, and RFC 000 forbids amending a `done/` RFC in place.

**Resolution, needing no amendment:** at the cut revision, while RFC 046's Status line is being
edited for the move anyway, add **one sentence** to §3.3 pointing at §7 of this file as the home of
item 3's measured figure. A pointer names a file, not a number, so it can be written before the
number exists; and editing the Status line in the move commit is already this project's practice.
The measurement itself is written into §7 below, in the post-release commit.

The prediction it is measured against: `release-gate` **6m 20s → about 3m 18s**, a 48% reduction.

## 5. The cut, in order

Every gate invocation is checked on its own exit status, never through a pipe.

1. **Seven RFCs `accepted/` → `done/`**, with the Status lines of §4 item 3. RFC 047 stays.
2. **RFC index rows repointed** to `done/…` for those seven, status `Implemented (v0.24.1)`.
3. **RFC 046 §3.3 gains the pointer sentence** of §4.1.
4. **Apex `Implemented scope` → `RFCs 001-046, 048-052`** in **all seven** `APEX_DOCS`,
   byte-identically.
5. **Check, do not edit:** `This tree: 0.24.1` in all seven and the workspace version — already
   set. `Last reconciled repository release` **stays `0.24.0`**; RFC 024's inequality is strict and
   the release has not happened.
6. **`CHANGELOG.md`:** date the `## [0.24.1]` heading and add its subtitle. **Keep
   `**Release status:** unreleased`.** The entry already leads with the one thing a reader needs:
   this release changes no library code.
7. `cargo xtask check` → 20 gates PASS.
8. `cargo xtask release-gate --intended-tag 0.24.1` → PASS, whole stream to a file. It refuses on
   a dirty tree, so commit first; its evidence directory is created once per revision and is not
   cleaned up.
9. Commit, push `main`.
10. **Recompute the RFC 028 archive anchor by hand** at the revision to be tagged and verify it
    three ways, including that the stored archive decompresses to it.
11. **Tag `0.24.1`**, confirm it peels to `HEAD`, push. **Wait for the tagged CI `release-gate` job
    with `gh run watch`** before writing any `distributed` claim (RFC 021 §7) — a polling loop
    without a delay does not wait.
12. **Record the measured `release-gate` duration and per-step breakdown in §7**, from the run's own
    step API rather than from a wall-clock impression (architect review 109's method).

## 6. The post-release commit — one commit, and larger than last time

`doc-currency` now binds items 1-4 together. Splitting them fails the gate.

1. **`CHANGELOG.md`:** `## [0.24.1]`'s status → `released (tagged YYYY-MM-DD, distributed
   YYYY-MM-DD); not published to crates.io` — or `published to crates.io` per §3.2. **The final
   clause must be exactly one of those two strings**: RFC 052's corrected `is_published` fails
   closed on a third phrasing.
2. **`docs/src/specifications.md`:** add `| \`0.24.1\` | <distributed date> | 001-046, 048-052 |` as
   the table's last row. The `Shipped` cell is the **distributed** date (clause 3).
3. **Apex `Last reconciled repository release` → `0.24.1`** in all seven `APEX_DOCS`.
4. **Apex `This tree` → `0.24.2`** in all seven, with the version bump: the workspace version, the
   five internal `[workspace.dependencies]` pins, and **all eight lockfiles** — root plus
   `device-size-reference/` and the six `examples/*/`, each updated **with `--workspace`**. Then
   verify the diff: every changed line must be a version string on one of this workspace's own five
   crates. Unscoped, `cargo update` once moved thirteen third-party packages here including a `syn`
   major (architect review 102).
5. **`CHANGELOG.md`:** a new `## [0.24.2] — unreleased` heading, or `release-gate`'s **first**
   preflight check refuses for want of exactly one matching heading.
6. **If distribute-only:** the `:52-56` never-published paragraph gains its clause (§3.2). **If
   published:** the `What is on crates.io.` paragraph moves to `0.24.1` and its row's cell, which
   RFC 052 clause 5 asserts.
7. **The README is untouched** (§1). RFC 049's predicate reads `0.24` either way.

**Advisory drift at cut time is normal maintenance, not an emergency** (RFC 045 Amendment 1). If
`supply-chain` goes red on a dependency nothing here touched, fix it with a **scoped**
`cargo update -p <crate>`, never a bare one, and verify every changed lockfile line.

## 7. Measured at this cut — RFC 046 §3.3 item 3

**The prediction held and was beaten.** `release.yml`'s `release-gate` job on the `0.24.1` tag,
run `41147305484`, derived from the run's own step API rather than a wall-clock impression
(architect review 109's method):

| | before | predicted | **measured** |
| --- | ---: | ---: | ---: |
| the `release-gate` job | 6m 20s | about 3m 18s | **2m 56s** |
| reduction | — | 48% | **54%** |

Per-step, `2026-10-30T23:55:41Z` → `23:58:37Z`:

| step | duration |
| --- | ---: |
| Set up job | 2s |
| Check out the candidate revision | 1s |
| Install stable Rust | 2s |
| Install the declared MSRV | 7s |
| **Install the pinned documentation tool** (`mdbook`) | **0s** |
| **Install the pinned supply-chain tool** (`cargo-deny`) | **1s** |
| Record tool versions | 0s |
| **Run the canonical non-publishing candidate gate** | **2m 39s** |
| Retain non-publishing candidate evidence | 1s |
| Post checkout, complete job | 1s |

**The two pinned-tool installs together take one second.** They were compiling from source before —
RFC 046 §3.3 measured `cargo-deny`'s install at 1m 56s in `ci` and predicted `mdbook` at 1m 19s
here — so about 3m 15s of compilation became 1s of download and verification.

**What this measurement actually settles** is the architectural claim, not the arithmetic: the job
is now bounded by `cargo xtask check`'s own work at **2m 39s of 2m 56s**, so a release's cost is
now the cost of its twenty gates and nothing else. Tool compilation has stopped being a release's
problem at the one moment a release cannot be cheaply retried — which is what RFC 046 §3.3 item 3
was authorized to buy.

The 22-second margin over the prediction is not an accuracy claim: §3.3's figures were arithmetic
on measured step times from a different runner on a different day, and runner variance is of the
same order. The direction and the magnitude are what hold.

## 8. Risks

- **Functional — Low.** No library code changes; `git diff 0.24.0..HEAD -- crates/` is empty.
- **Business — Low.** Under §3.2's recommendation, no user-visible change reaches anyone.
- **Operational — Medium, and concentrated in §6.** The post-release commit is now gate-coupled
  across seven documents, a table row, eight lockfiles and two CHANGELOG edits. That coupling is
  what RFC 048 and RFC 052 bought — a cut can no longer half-happen — but it is also the first time
  it is being walked, and the failure mode is a red gate on `main` rather than a bad release.
- **Security — Low.** No trust-root, channel or artifact change. The `tokio` bump is a
  fail-closed-advisory response to an unreachable defect, as its CHANGELOG entry states.

## 9. Open after `0.24.1`

- **RFC 047 Phases II-VI**, unscheduled, and §2.1's permanent-gap question.
- **The `Carries RFCs` column is bounded and ordered, not verified** (RFC 052 §0.1): nothing in
  this repository records which RFCs a release carried, so this cut's own new cell is checked for
  monotonicity and its bound, not for truth. I wrote it from §2's table; **read it once more before
  committing step 2 of §6.**
