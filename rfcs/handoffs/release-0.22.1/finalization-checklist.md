# Release `0.22.1` — finalization checklist

**Status:** prepared, awaiting owner authorization.
**Authorized by.** Architect review 078 §8 recommends GO for `0.22.1`; architect
review 077 §6 accepted C1 and raised C2; architect review 076 §0 accepted RFC 035
S1-S4. The owner authorizes the release itself.
**Candidate reviewed:** `e9a92d5` (C2) — 17 gates and a complete RFC 019 candidate
run pass; RFC 028 anchor verified end-to-end.

**Who runs this.** The architect and the owner. Per the owner's standing
instruction, mechanical preparation may be delegated but **the actual cut may
not**. Every step below is part of the cut.

## 1. Scope of the release

`0.22.1` ships RFC 035 (visitor-facing documentation and real-world examples) and
one lint-driven refactor with identical semantics (C1), evidenced in-tree by C2.
No crate behaviour changed. No public surface changed.

## 2. Pre-cut state, already true at `e9a92d5`

| Item | State |
|---|---|
| `CHANGELOG.md` `## [0.22.1]` section | present, `**Release status:** unreleased` |
| Workspace version | `0.22.1` |
| Apex trio `This tree:` | `0.22.1` |
| Apex trio `Last reconciled repository release:` | `0.22.0` |
| Apex trio `Implemented scope:` | `RFCs 001-034` |
| RFC 035 | `rfcs/accepted/` |
| Tag `0.22.1` | does not exist, locally or on the remote |

## 3. The cut

1. **Move RFC 035 to done.** `git mv rfcs/accepted/035-visitor-facing-documentation-and-examples.md rfcs/done/`.
   RFC 025: an RFC in `done/` is never amended in place. Any correction to its text
   must happen in this same commit, while it is still Accepted, or not at all.
2. **Apex scope to `RFCs 001-035`** in all three of
   `docs/specs/loeres-requirements-v1.md`, `docs/specs/loeres-external-design-v1.md`,
   `docs/specs/loeres-roadmap-milestones-v1.md`. Derived from `rfcs/done/`, not from
   memory.
3. **Leave `Last reconciled repository release:` at `0.22.0`** and `This tree:` at
   `0.22.1`. RFC 024's invariant is **strict**: `This tree` > `Last reconciled`.
   Equality is never valid at any point, including mid-cut.
4. **Date the `CHANGELOG.md` heading** — `## [0.22.1] — 2026-10-06 — Visitor-facing
   documentation and examples` — and **leave `**Release status:** unreleased`**.
   The status changes only in the post-release commit.
5. `cargo xtask check` → 17 gates PASS.
6. `cargo xtask release-gate --intended-tag 0.22.1` → PASS. Redirect the whole
   stream to a file; do not pipe it through `head` or `tail`. If a run must be
   repeated on an unchanged revision, remove only
   `.git-exclude/release-evidence/<revision>-v0.22.1/` first — see
   *Development* → "A candidate run is once-per-revision".
7. Commit, push `main`.
8. **Recompute the RFC 028 anchor by hand at the tagged revision:**
   `git archive --format=tar <revision> | sha256sum`. It will **not** equal
   `d66b3a7b66…`, which was the C2 candidate; the cut adds commits.
9. Tag `0.22.1` (unprefixed canonical SemVer), push the tag.
10. Publish, in dependency order: `loeres`, `loeres-backend-static`,
    `loeres-backend-std`, `loeres-device`, `loeres-cluster`.

## 4. Post-release commit

1. `**Release status:**` → `released (tagged 2026-10-06, distributed 2026-10-06)`.
2. Apex `Last reconciled repository release:` → `0.22.1`.
3. Bump the workspace version to the next patch so `This tree:` > `Last reconciled`
   holds strictly again, and set apex `This tree:` to match.
4. `cargo xtask check` → PASS. Commit, push.

## 5. What is deliberately not in this release

- The benchmark harness (theme T4). It is the only honest answer to the owner's
  fourth visitor question — "how effective or powerful" — and review 065 scoped it
  as its own cycle. The architect brings it as a separate recommendation.
- Themes T5 (LP), T6 (server-side maturity), T7 (fixed-point scalars), unscheduled.
