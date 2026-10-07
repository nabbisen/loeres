# Release `0.22.3` — finalization checklist

**Status:** not scheduled. `0.22.3` currently carries RFC 038 alone.
**Authorized by.** Architect review 090 §8 accepted RFC 038 and recommends it as `0.22.3`
content. The owner authorizes the release and its timing.

**Who runs this.** The architect and the owner. The cut and the publication may not be
delegated.

## 1. Scope

**RFC 038** — an entry guide at every example: six per-example `README.md` files, an
`examples/README.md` index, a problem statement bound to its own module comment, and
two-directional symmetry between the example tables and the directories on disk.

**RFC 039** — what the kernel does with a linear objective. The claim that LP "is not solved"
is corrected in thirteen places, three of them published crate READMEs and three crate doc
comments, with the measurement behind it and both non-convergence mechanisms named. `cargo
xtask lp` characterises it; `exact_lp_optimum` is the new exact reference the old one could not
supply, since it inverts `Q`.

**RFC 040** — the first enforced device size budget since RFC 010 asked for one:
`.text + .rodata` of a reference instantiation that calls the kernel, **10 736 bytes** at
`(N, M) = (8, 4)`, pinned with a bounded delta.

No crate *behaviour* changed in any of the three, and no public API; `cargo xtask check` runs
twenty gates.

## 2. Is it worth cutting yet? — yes, now

**This section's earlier answer is withdrawn.** It argued no, on the reasoning the owner applied
to `0.22.2`: `git diff 0.22.2..HEAD -- crates` was empty, so a registry user would receive
byte-identical crates. **That is no longer true** (architect review 093 §7). RFC 039 changed what
a published crate contains:

```text
crates/loeres/README.md                        |  4 +++-
crates/loeres/src/problem.rs                   | 22 ++++++++++++++++++----
crates/loeres-cluster/README.md, …/constrained.rs
crates/loeres-device/README.md,  …/constrained.rs
```

Three published crate READMEs and three crate doc comments now state that LP **is** solved
soundly, with the measurement, the two non-convergence mechanisms and the remedy — where they
previously told an LP user the kernel could not help them. A crates.io or docs.rs visitor
receives different and materially better information after this release, which is exactly the
test `0.22.2` failed.

So `0.22.3` is worth cutting on its own. It carries:

- **RFC 038** — an entry guide at every example, with two-directional table symmetry gated.
- **RFC 039** — the LP characterisation and correction, and the step guidance.
- **RFC 040** — the first enforced device size budget since RFC 010 asked for one.

The timing remains the owner's.

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
2. **every workspace-excluded crate's own lockfile** — a rule, not a count, because the count
   has gone stale twice: five at `0.22.1`, six once RFC 037 added `cluster-counted-work`,
   seven once RFC 040 added `device-size-reference`, which is not an example at all. Enumerate
   them: `for d in examples/*/ device-size-reference/; do cargo update --manifest-path "$d/Cargo.toml" --offline; done`.
   Miss an example and the `examples` gate fails with `cannot update the lock file …
   --locked`; miss the RFC 040 fixture and `size-budget` fails instead, since no other gate
   builds it;
3. the next version's `## [x.y.z] — unreleased` CHANGELOG section, or `release-gate`'s
   preflight refuses for want of exactly one matching heading.

Then `docs/src/specifications.md`: add the row and make it the newest published version.

## 6. Open, not in this release

- **`size-budget`'s device threshold — closed by RFC 040.** Kept here as the record of what
  it cost to get right. The earlier framing was wrong on two counts (architect review 091
  §0): it was never the owner's number — RFC 010 §3.7 assigns byte budgets to RFCs 003, 006,
  008 and 011, so `size_budget.rs`'s "pending owner RFC" means a *budget-owning RFC* — and the
  32 000-byte proposal was on the wrong measure, since of the release rlib's 25 512 bytes
  **23 845 are `.rmeta`** and **193** are every other section summed. RFC 040 measures
  `.text + .rodata` of a reference instantiation that calls the kernel instead: **10 736
  bytes** at `(N, M) = (8, 4)`, release, `panic = "abort"`, pinned with a **10%** bounded
  delta rather than an absolute ceiling. The fraction is interim — no toolchain-driven drift
  has been observed for a code-size figure in this project, and review 092 §1.4 records that
  the architect checked and could not supply a better basis. RFC 010 §3.7's two remaining
  items, stack sensitivity beyond the existing type assertions and cluster monomorphization
  growth, are still unimplemented.
- **Themes T5 (LP), T6 (server-side maturity), T7 (fixed-point scalars)**, unscheduled. RFC
  039 (S1-S4) measured and documented LP's actual behaviour instead: a converged result is
  sound regardless of curvature, and both non-convergence causes are configuration matters
  (the projection's sweep cap, or the outer iteration cap when a wide box is paired with a
  small step scale) rather than an algorithm gap. RFC 039 S4 declined a dedicated LP
  algorithm on that evidence.
- **A distance-aware default step for the curvature-free case** (RFC 039's b-ruling handoff
  §3). Not whether such a rule is possible — a step scaled to the problem's extent
  demonstrably converges a wide-box LP in eleven iterations — but whether the library should
  **offer one as a default**, rather than leaving the caller to scale `α` themselves. The box
  extent is available from `BoxBounds`; `‖c‖` from the linear term. A question for a future
  RFC with its own measurement, not a commitment here.
- Kernel instrumentation to expose the projection sweep count, which would turn RFC 037's
  bounded `proj ops ≤` into a measured figure. Recorded in RFC 037 §0.1.
- An optional polish from review 090 §5: the six example `README.md` H1 headings end in a
  full stop, because the title is the module comment's first paragraph. The title is not
  gated, so stripping a terminal period is a one-line change touching six files. Typography,
  not accuracy.
