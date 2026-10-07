# RFC 042 - The Batch Seam Carries What the Solve Found

**Status.** Proposed (2026-10-07).

**Author tier.** `architect`.

**Governing history.** Architect review 056 L1 raised this and the owner settled its shape as
option (a). Review 065 listed it as L10 under theme T6 and noted it is "a small API-shape
question the owner already settled". Architect review 093 §7 and the ROADMAP record it as the
one T6 item needing no consumer.

## 1. Summary

Solve a constrained problem directly and the caller receives `ConstrainedSolveRecord`: the
report, `projection_cap_hits`, `max_constraint_violation`, and the heuristic
`infeasibility_evidence`. Solve the *same problem* through `solve_batch` and the caller
receives

```rust
pub enum BatchItemOutcome<S> {
    Solved { solution: ClusterSolution<S>, report: SolveReport },
    …
}
```

**status only.** The batch path computes the violation and the hint — `ClusterConstrainedJob`
implements `ClusterJob`, so a constrained problem really does run through the batch — and then
**discards them at the seam**.

The consequence is a caller who cannot tell apart, in a batch, a `NotConverged` whose violation
is `1e-9` from one whose violation is `1e-1`, and who loses RFC 034's infeasibility hint
entirely. `examples/cluster-qp-constrained` already documents it: "what the batch seam does and
does not carry: **status only**."

This is not a missing feature. It is one code path returning less than another, silently,
after having computed the difference — the owner's second principle, and the same shape as the
LP claim RFC 039 corrected.

## 2. Design

### 2.1 The two honest fields cross the seam

`BatchItemOutcome::Solved` carries the terminal constraint violation and the infeasibility
evidence alongside the report, for items whose job produced them.

Shape per review 056's settled option (a). The fields must keep the character they have in
`ConstrainedSolveRecord`, which RFC 034 Amendment 2 was explicit about:
`infeasibility_evidence` is a **heuristic observation, wrong in both directions**, never a
status and never for control flow. Crossing the seam must not launder it into something
stronger, and the documentation at the seam must carry the same warning the record's does.

A box-only item has no polyhedron, so its violation is zero by construction and it has no
evidence to report. The representation must make "this item had no constraints" distinct from
"this item had constraints and they were satisfied", because conflating them is how a reader
draws the wrong conclusion from a zero.

### 2.2 This is a breaking change, and the version follows from that

`BatchItemOutcome` carries only `#[derive(Clone, Debug)]` — it is **not**
`#[non_exhaustive]`, and the `#[non_exhaustive]` two declarations above it in the same file
belongs to `ClusterSolution`. So adding fields to the `Solved` variant breaks:

- a pattern match without a trailing `..` — `error[E0027]`, missing fields in pattern;
- construction by struct literal — `error[E0063]`.

Under Cargo's `0.x` resolution the minor is the compatibility unit, so a breaking change
cannot ship as a patch; this release is a minor. That is the same reasoning RFC 034
Amendment 3 recorded for `0.22.0`, and `docs/src/development.md`'s version-convention section
now states it without the phrasing that misled a reader once.

**Whether to make the variant `#[non_exhaustive]` in the same change is part of this RFC's
decision**, not an afterthought: doing so prevents the next field addition from breaking
anyone, at the cost of preventing callers from matching exhaustively ever again. RFC 014 §311
records the project's existing position for `SolveReport` — private fields with public
accessors, so that adding one is not breaking — and that pattern is the alternative worth
weighing against `#[non_exhaustive]` here.

### 2.3 What the examples must show

`examples/cluster-qp-constrained` documents the limitation this RFC removes, in its module
comment and in its printed output. Both must change, and its captured output is checked
byte-for-byte by the `examples` gate, so the change is visible and gated rather than
incidental.

## 3. Explicit non-scope

- **No kernel change.** The fields already exist and are already computed; this moves them
  across a seam.
- No change to `infeasibility_evidence`'s meaning, strength, or RFC 034 Amendment 2's
  warnings.
- Not the other three T6 items — metadata-only observability, the mock-only `ffi-gateway`
  seam, and the process-local validation cache — which stay closed until a consumer exists.
- No new dependency.

## 4. Risks

| Risk | Mitigation |
|---|---|
| The heuristic is laundered into something stronger by crossing the seam | §2.1 requires the record's own warning to travel with it. RFC 034 spent three amendments establishing that wording; it is not to be re-weakened here. |
| A zero violation read as "feasible" when the item had no constraints | §2.1 requires the two cases to be distinguishable in the representation. |
| The breaking change surprises a caller | It is a minor bump, which is where a breaking change must go, and the CHANGELOG entry states the two error codes a caller will see and the fix for each. |
| `#[non_exhaustive]` decided by reflex | §2.2 makes it an explicit decision with RFC 014 §311's alternative named. |

## 5. Exit criteria

1. `BatchItemOutcome::Solved` carries the terminal violation and the infeasibility evidence
   for items whose job produced them.
2. An item with no linear inequalities is distinguishable from one whose constraints were
   satisfied.
3. `infeasibility_evidence` carries RFC 034 Amendment 2's warning at the seam, unweakened.
4. The `#[non_exhaustive]`-versus-accessors question is decided in writing, with RFC 014
   §311's precedent addressed.
5. `examples/cluster-qp-constrained` no longer says the seam carries status only, and its
   captured output is updated and gate-checked.
6. The CHANGELOG entry names `E0027` and `E0063` and the fix for each.
7. `cargo xtask check` passes; `cargo xtask release-gate` passes.
