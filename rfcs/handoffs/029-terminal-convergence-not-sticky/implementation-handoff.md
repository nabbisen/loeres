# Developer Handoff — RFC 029 terminal convergence, not sticky convergence

**Governing RFC.** `rfcs/accepted/029-terminal-convergence-not-sticky.md` (design frozen 2026-09-24).
**Assigned to.** Implementer tier.
**Ordering.** **Before RFC 027 S5.** S5 documents what `Converged` means on a user-facing surface; this RFC changes that meaning, so S5 must write it once, correctly. Nothing else is blocked.

---

## 1. Purpose

Under `TimingMode::ConstantIteration` both device kernels decide the reported
status from a `converged` flag that is set the first time an outer step falls
within tolerance and is **never cleared**, while the loop keeps running to
`max_iterations`. Evaluate the criterion at the **final** iteration instead.

## 2. The two sites

Both are in `crates/loeres-device`. Cluster has no `ConstantIteration` and is
not touched.

### 2.1 `src/solve.rs` — the RFC 006 kernel

Currently:

```rust
let mut converged = false;
let mut executed: u32 = 0;
while executed < max_iterations {
    // ... gradient, projected_gradient_step -> change ...
    executed += 1;
    if change.lte_tolerance(tolerance) {
        converged = true;
        if !constant_iteration {
            return Ok(DeviceSolveReport::from_core(SolveReport::converged_early(executed)));
        }
    }
}
```

Change the set-once into an assignment, so `converged` holds the **last**
iteration's result when the loop ends:

```rust
converged = change.lte_tolerance(tolerance);
if converged {
    if !constant_iteration {
        return Ok(DeviceSolveReport::from_core(SolveReport::converged_early(executed)));
    }
}
```

The post-loop `if converged { converged_at_cap } else { not_converged_cap }`
needs **no** change: it now reads a fact about the returned iterate.

### 2.2 `src/solve/constrained.rs` — the RFC 027 kernel

Identical shape, same one-line change:

```rust
converged = change.lte_tolerance(tolerance);
if converged {
    if !constant_iteration {
        // RFC 027 §0.5.1 feasibility gate — unchanged
        ...
    }
}
```

The post-loop return already combines `converged` with Amendment 5's
feasibility gate and needs **no** change. RFC 027 §0.5 and RFC 029 then hold
together: `Converged` requires the final iterate to be both stationary and
feasible.

`EarlyExitAllowed` behaviour is unchanged in both kernels — it returns the
moment the criterion holds, so its claim was always about the returned iterate.

## 3. Measured effect

The architect prototyped and measured this before it was written down. On the
RFC 027 device kernel, `ConstantIteration`, `Q = I`, `c = 0`,
`step_scale = 2.1` (so `x_{k+1} = -1.1·x_k`, divergent), `x₀ = (1e-13, 0)` so
iteration 1's change is `2.1e-13` under `tolerance = 1e-12`, box `±1e6` and
constraints slack, `max_iterations = 400`:

```text
before : status Converged     termination IterationCap  iters 400  final x₀ 3606.401403
after  : status NotConverged  termination IterationCap  iters 400  final x₀ 3606.401403
```

Genuinely converged control (`c = (-1,-1)`, `step_scale 0.5`, same mode and cap):

```text
before and after : status Converged  termination IterationCap  iters 400  final x₀ 1.0
```

Full workspace suite and `cargo xtask conformance` (24/24) pass unchanged under
the prototype. Only the status moves, and only where it was false.

## 4. Required tests

1. **`crates/loeres-device/src/solve/constrained/tests.rs`**, behind
   `#[cfg(feature = "constant-iteration")]`: §3's divergent configuration
   reports `NotConverged`, and assert the final iterate is far from the start
   so the test cannot pass by accident.
2. **The same file:** a genuinely converged `ConstantIteration` run still
   reports `Converged` with `TerminationReason::IterationCap` — this is the
   regression guard for over-correcting.
3. **`crates/loeres-device/src/solve/tests.rs`** (RFC 006 kernel): the
   equivalent pair, box-only, no constraints. A divergent `alpha` from a start
   a hair off the fixed point, with bounds wide enough that clamping does not
   make the iterate stationary again.
4. Keep every existing `constant-iteration` test passing unchanged.

## 5. Documentation

`crates/loeres-device/src/solve.rs`, the `solve_projected_first_order` doc
comment, currently reads:

> Under `ConstantIteration` it always runs the full `max_iterations` and reports
> `converged_at_cap` / `not_converged_cap`, so `iterations_executed == max_iterations`.

State that the criterion is evaluated at the **final** iteration, so
`converged_at_cap` is a claim about the returned iterate (RFC 029 §5.4). Make
the matching statement on the RFC 027 kernel's doc comment beside the existing
§0.5.1 sentence.

`CHANGELOG.md`, under the unreleased `0.21.1` record: a short RFC 029
subsection saying what changed and that it changes a reported status only where
that status was false.

## 6. Explicit non-change scope

- **No signature, workspace, config, or feature change.** No public type moves.
- **`ConstantIteration` still executes exactly `max_iterations` steps.** The
  timing property is the whole point of the mode; do not add an early return,
  a branch on data, or a conditional computation inside the loop.
- **Do not touch the cluster kernels** — no `ConstantIteration` there.
- **Do not change `EarlyExitAllowed`** in either kernel.
- **Do not alter RFC 027 Amendment 5's feasibility gate**; it composes with this
  change and both must hold.
- No new dependency; `#![forbid(unsafe_code)]` and the 300-ELOC soft limit stand.

## 7. Prohibited shortcuts

- Clearing the flag on a later non-converged step instead of assigning it. It
  reaches the same outcome while keeping a variable whose meaning is a history
  rather than a fact — RFC 029 §6 rejected it for that reason.
- Fixing only the RFC 027 kernel and leaving RFC 006's. RFC 029 §6 rejected
  that: two kernels disagreeing about what `Converged` means in the same timing
  mode is worse than either behaviour alone.
- Asserting the divergent test by its status alone without pinning that the
  iterate actually moved.
- `#[allow(dead_code)]` — remove, never suppress.

## 8. Required evidence

fmt, clippy `-D warnings`, `cargo test --workspace --all-features`, MSRV 1.85,
`cargo xtask check` (17 gates), `cargo xtask conformance` (24/24 unchanged),
`mdbook build docs --dest-dir target/xtask-book/local`, and the
`thumbv7em-none-eabihf` build with `owned-arrays`. Show the before/after status
for §3's divergent configuration on **both** kernels.

Mutation-check and report it: revert the assignment to a set-once in each
kernel and confirm the new test in that kernel fails.

## 9. Acceptance criteria

RFC 029 §8 items 1–5.

## 10. Review request

Standard eleven-item format; **record your author tier**; send to the architect.
Do not move RFC 029 to `done/` — it moves with the release that carries it.
