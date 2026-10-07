# RFC 042 implementation handoff — The Batch Seam Carries What the Solve Found

**To:** implementer (mid-capability model)
**From:** architect
**Authorized by.** The owner accepted RFC 042 on 2026-10-07. Read
`rfcs/accepted/042-the-batch-seam-carries-what-the-solve-found.md` **including Amendment 1**,
which corrects "two fields" to three.
**Base revision:** `main` at or after the acceptance commit.
**Target release:** `0.23.0` — this is a breaking change, so it cannot be a patch (§2).

**One review request.** `cargo xtask release-gate --intended-tag 0.23.0` is required.

## 0. Verified before this was written

### 0.1 The one construction site, and no trait change

`ClusterJob::run_boxed(&self, ctx) -> BatchItemOutcome<S>` returns the outcome itself, so the
**job** builds it with the record in hand. At
`crates/loeres-cluster/src/solve/constrained.rs:830`:

```rust
Ok(record) => BatchItemOutcome::Solved {
    solution: …,
    report: record.report,
},
```

So **no change to `ClusterJob`** is needed, and that single site is where the fields are
dropped. Check whether any other `ClusterJob` implementor constructs `Solved` — the box-only
job does — and handle it per §1.2.

### 0.2 Three fields, not two

`record.report` is taken; `max_constraint_violation`, `infeasibility_evidence` **and
`projection_cap_hits`** are discarded. The third is the one Amendment 1 added, and it is not
optional: RFC 039 ships the sentence *"distinguishable by `projection_cap_hits`: nonzero for
the former, zero for the latter"* in `TERMS_OF_USE.md`, both user guides and three published
crate READMEs. Carry the first two and a batch caller cannot apply the project's own published
guidance.

### 0.3 `BatchItemOutcome` is not `#[non_exhaustive]`

It carries only `#[derive(Clone, Debug)]`. The `#[non_exhaustive]` two declarations above it in
the same file belongs to **`ClusterSolution`** — do not misread that as covering the outcome, as
the architect nearly did on RFC 034.

So adding fields to `Solved` breaks a pattern without a trailing `..` (`error[E0027]`) and
construction by struct literal (`error[E0063]`). That is why the target is `0.23.0`.

## 1. The change

### 1.1 The fields

`BatchItemOutcome::Solved` gains the three. Keep the names and the types they have in
`ConstrainedSolveRecord`, so a reader moving between the direct and batch paths does not have to
translate.

**`infeasibility_evidence` must not be laundered by crossing the seam.** RFC 034 spent three
amendments establishing that it is a *heuristic observation, wrong in both directions*, never a
status and never for control flow, with measured error rates. The seam's documentation carries
the same warning, in substance, not a softened paraphrase. If you shorten it, keep "wrong in
both directions" and "never for control flow" literally.

### 1.2 A box-only item has no polyhedron

An item with no linear inequalities has no violation to report and no evidence to compute, and
its cap-hit count is meaningless rather than zero. **"No constraints" must be distinguishable
from "constraints, all satisfied"**, because a reader who sees a zero and concludes "feasible"
has been misled by the representation rather than by the solver.

How to represent that is yours to choose and to justify in the request. `Option` of a small
struct holding the three is the obvious candidate; a separate variant is another. Say why you
chose what you chose — the criterion is that a reader cannot mistake absence for zero.

### 1.3 `#[non_exhaustive]` versus accessors — decide it in writing

RFC 042 §2.2 makes this an explicit decision, not a reflex. The two options:

- **`#[non_exhaustive]` on the variant**, so the *next* field addition breaks nobody, at the
  cost that callers can never match it exhaustively again;
- **RFC 014 §311's pattern** — private fields with public accessors, which that RFC adopted for
  `SolveReport` precisely so "adding a private field in a later version is not a breaking
  change", and which also records that "adding public fields to these structs is not a
  compatible change and is forbidden".

RFC 014's precedent is the stronger one on this project's own record, and `SolveReport` — which
sits *inside* this very variant — already follows it. The architect's recommendation is
therefore accessors over `#[non_exhaustive]`, but **the decision is yours to make and to argue**;
if you reach the other answer with a reason, say it and the architect will rule.

### 1.4 The example

`examples/cluster-qp-constrained` says "what the batch seam does and does not carry: **status
only**" in its module comment, and demonstrates it in its output. Both change. Its captured
output is checked byte-for-byte by the `examples` gate, so update the book block in the same
commit or the gate will tell you.

It is an **API artifact** under RFC 038's registry, so its README is written rather than
extracted — but check the README too, since it describes what the example shows.

## 2. The CHANGELOG entry

`0.23.0` is a minor because this breaks callers. The entry must name **both** error codes and
the fix for each:

- `error[E0027]` — a pattern match on `Solved` without a trailing `..`: add `..`;
- `error[E0063]` — construction by struct literal: stop constructing it (it is an *output*), or
  use whatever constructor §1.3's decision provides.

That is the shape the `0.22.0` entry used for `ConstrainedSolveRecord`, and it is the reason a
reader can act on a minor bump instead of guessing. `docs/src/development.md`'s version
convention explains why a breaking change cannot be a patch; do not restate it, and do not
repeat the phrase "the minor is the breaking position", which that section records as having
misled a reader once.

## 3. Non-scope

- **No kernel change.** The fields exist and are computed; this moves them across a seam.
- No change to `ClusterJob` (§0.1), to `ConstrainedSolveRecord`, or to
  `infeasibility_evidence`'s meaning or strength.
- Not the other three T6 items — metadata-only observability, the mock-only `ffi-gateway`
  seam, the process-local validation cache. They stay closed until a consumer exists.
- No new dependency, no `#[allow(…)]`. Do not tag; do not publish.

## 4. Required evidence

```text
cargo fmt --all -- --check          # run last, immediately before committing
cargo clippy --workspace --all-features --all-targets -- -D warnings
cargo test --workspace --all-features
cargo +1.85 check --workspace --all-features
cargo xtask check                   # 20 gates
cargo xtask examples                # the example's captured output must match its new run
cargo xtask release-gate --intended-tag 0.23.0      # must PASS
```

State in the request:

1. **Examples affected** — `cluster-qp-constrained`, with its new output quoted.
2. The three fields as they now appear in `Solved`, and the §1.2 representation with your
   reason for it.
3. Your §1.3 decision and its argument, addressing RFC 014 §311.
4. **Proof that the two error codes are what a caller actually sees.** Write a throwaway snippet
   that matches `Solved` without `..` and one that constructs it by literal, compile both, and
   quote the errors. The CHANGELOG claims them; do not claim them unverified.
5. That a batch caller can now distinguish RFC 039's two non-convergence mechanisms from the
   outcome alone (exit criterion 1a).
6. Anything in this handoff that does not match the tree.
