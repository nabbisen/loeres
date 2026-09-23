# Developer Handoff — RFC 030 randomized differential testing for numerical kernels

**Governing RFC.** `rfcs/accepted/030-differential-testing-for-numerical-kernels.md` (design frozen 2026-09-24).
**Assigned to.** Implementer tier.
**Ordering.** No dependency on other work. `0.21.1` is released; this targets the `0.21.2` tree.

---

## 1. Purpose

Make the discipline that found this cycle's two worst defects a rule rather than
a reviewer's habit: every numerical kernel carries a randomized differential test
against an independently constructed exact reference, and a gate asserts it.

## 2. Slices

Submit **one review request per slice**. S1 is the gate and registry; S2 and S3
are the two retrofits; they may be done in any order after S1.

**S1 — the `differential` gate and its registry.** *Landed at `1e71ece`, accepted in architect review 064.*
**S2 — retrofit `loeres-device::solve::solve_projected_first_order` (RFC 006).** *Landed at `71164d2`, accepted in architect review 064.*
**S3 — retrofit `loeres-cluster::solve::projected_first_order::solve_projected_first_order_dyn` (RFC 016).** *Landed at `ba16c17`, accepted in architect review 064.*

**All three slices are complete and RFC 030's six exit criteria are met.** Two
corrections to this handoff came out of the work and are recorded rather than
silently absorbed: §3.1's seed list of "all six entrypoints" **missed
`solve_batch_async`**, which is `pub async fn` and which the specified
`pub fn solve_` scan would not have matched — the implementer extended the scan
and added the row; and §3.2.2's `fn <test>(` check was too weak, since a
same-named helper satisfies it, so the gate additionally requires a `#[test]`
attribute.

## 3. S1 — gate and registry

### 3.1 Registry

New `xtask/src/checks/differential.rs`, following the **`examples` gate's
const-table idiom** (`xtask/src/checks/examples.rs`) rather than inventing a file
format:

```rust
struct Kernel {
    /// Crate-relative path to the module that defines the entrypoint.
    source: &'static str,
    /// The public entrypoint's name.
    entrypoint: &'static str,
    coverage: Coverage,
}

enum Coverage {
    /// A randomized differential test meeting RFC 030 §4.2.
    Differential {
        /// File containing the test.
        tests: &'static str,
        /// The `#[test]` function's name.
        test: &'static str,
        /// A mutation the author verified this test detects.
        detects: &'static str,
    },
    /// Not a numerical kernel. The reason is mandatory and non-empty.
    Exempt { reason: &'static str },
}
```

Seed it with all six entrypoints:

| Entrypoint | Coverage |
|---|---|
| `solve_projected_first_order` (device, RFC 006) | `Differential`, added by S2 |
| `solve_constrained_projected_first_order` (device, RFC 027) | `Differential`, test `random_feasible_two_constraint_polytopes_match_the_exact_projection`, detects "per-sweep multiplier reset (RFC 027 §0.3.2)" |
| `solve_projected_first_order_dyn` (cluster, RFC 016) | `Differential`, added by S3 |
| `solve_constrained_projected_first_order_dyn` (cluster, RFC 027) | `Differential`, test `random_feasible_polytopes_n3_m3_match_the_exact_projection`, detects "per-sweep multiplier reset (RFC 027 §0.3.2)" |
| `solve_projected_first_order_dyn_cached` (cluster) | `Exempt` — "RFC 017 caching wrapper; delegates the numerical work to `solve_projected_first_order_dyn`" |
| `solve_batch`, `solve_batch_observed` (cluster) | `Exempt` — "batch orchestration; performs no numerical iteration" |

Until S2 and S3 land, their rows are `Exempt { reason: "retrofit pending, RFC 030 S2/S3" }`. **Change them to `Differential` in the same commit as each retrofit** — an exemption that outlives its slice is the failure mode this gate exists to prevent.

### 3.2 Discovery and the symmetry assertion

Discover candidate entrypoints by scanning the two kernel crates' `src/` for
`pub fn solve_` at module scope (including inside `mod owned`, which is where the
RFC 006 entrypoint lives — a scan that misses it is wrong). Then assert, all
fail-closed:

1. **Coverage symmetry**, the RFC 022 pattern: every discovered entrypoint has
   exactly one registry row, and every row names a discovered entrypoint. Report
   both directions separately — "in code, not in registry" and "in registry, not
   in code" — because they have different causes.
2. **The named test exists**: `tests` file contains `fn <test>(`.
3. **Non-empty `reason`** on every `Exempt`, and non-empty `detects` on every
   `Differential`.

Report a one-line-per-kernel summary like the other gates, then `PASS`/`FAIL`.

### 3.3 Wire it in

Add to `cargo xtask check` after `conformance`. **The gate count becomes
seventeen**; update every site that states sixteen — at the time of writing
`CHANGELOG.md` (the `0.21.0` record is historical, leave it), and the two RFC 027
/ 029 handoffs' evidence lines. Sweep **wrap-tolerantly**: `git grep "16 gates"`
misses a line-wrapped "sixteen\ngates".

## 4. S2 — the RFC 006 device retrofit

### 4.1 The exact reference, and why it is independent

For a separable quadratic `f(x) = ½ Σᵢ qᵢ(xᵢ − tᵢ)²` over `lo ≤ x ≤ hi`, the
constrained minimiser is, per coordinate:

```text
xᵢ* = clamp(tᵢ, loᵢ, hiᵢ)          for any qᵢ > 0
```

A closed form with **no iteration in it**, and independent of `q` — so it is a
genuine reference, not a re-expression of the kernel. This satisfies §4.2 item 2.

### 4.2 The test

In `crates/loeres-device/src/solve/tests.rs`. The existing `Quadratic` fixture
hardcodes `q = 1`; add a `ScaledQuadratic` fixture carrying a per-coordinate `q`
(it must implement `gradient_at` **and** `objective_at`).

Verified recipe — the architect ran exactly this and it passes at
**worst error 1.07e-12 over 400 instances**:

- seeded LCG, no dependency; `N = 3`;
- `qᵢ ∈ [0.2, 3.2]`, `tᵢ ∈ [−4, 4]`, `loᵢ ∈ [−4, 0]`, `hiᵢ = loᵢ + [0, 4]`,
  `x0ᵢ ∈ [−3, 3]`;
- `alpha = 1 / max(q)`, which satisfies the convergence condition
  `0 < α < 2/max(q)`;
- `max_iterations 200_000`, `tolerance 1e-13`, `EarlyExitAllowed`;
- assert `Converged`, then per coordinate: agreement with `clamp(tᵢ, loᵢ, hiᵢ)`
  within `1e-6`, **and** feasibility `loᵢ ≤ xᵢ ≤ hiᵢ`.

**Randomized, per-coordinate, non-uniform bounds are the point.** Every existing
fixture uses uniform bounds, and that is what hides the mutation below.

### 4.3 The mutation to record in `detects`

The architect verified this one. In `projected_gradient_step`'s contiguous
branch, clamp every coordinate to **coordinate 0's** bounds
(`lo_slice[0]`, `hi_slice[0]`) instead of its own:

```text
all 19 existing device box tests: PASS
the new differential test:        FAIL
```

That is the case for this RFC in one line, on the kernel it targets. Record it as
`detects: "per-coordinate bound indexing (clamp using coordinate 0's bounds)"`,
and **re-verify it yourself** rather than taking it from this document.

## 5. S3 — the RFC 016 cluster retrofit

The same reference and the same assertions, over `DenseVector` at runtime sizes,
in `crates/loeres-cluster/src/solve/projected_first_order/tests.rs`. Use the
cluster kernel's own problem trait and validation policy
(`ValidateAllInputs`). Vary `n` across instances rather than fixing it, since the
cluster kernel is not const-generic and a runtime-length bug is exactly what this
buys.

Re-verify a mutation it detects and record it.

## 6. Explicit non-change scope

- **No kernel behaviour, signature, public surface, or feature change.**
- **No new dependency**, dev or otherwise. Seeded LCG only; no `proptest`.
- **Do not touch the conformance corpus or RFC 013.** Different question,
  different home (RFC 030 §5).
- Do not weaken or delete any existing test to make room.
- No `#[allow(dead_code)]`.

## 7. Prohibited shortcuts

- **A reference that reuses the kernel's iteration.** It is not a reference and
  the gate cannot detect it — this is the one hole RFC 030 §8 names openly, and
  the review will look for it specifically.
- Recording a `detects` mutation you did not actually run.
- Leaving a retrofit row `Exempt` after its slice lands.
- Asserting only proximity and not feasibility. That single omission is what let
  RFC 027 S2's defect through 18 tests.
- A single minimal shape (`N = 1`, or one constraint). RFC 027's defect was
  invisible at `M = 1`.

## 8. Required evidence

Per slice: fmt, clippy `-D warnings`, `cargo test --workspace --all-features`,
MSRV 1.85, `cargo xtask check` (**seventeen** gates after S1),
`cargo xtask conformance` (24/24, unchanged), `mdbook build docs --dest-dir
target/xtask-book/local`, and for device work the `thumbv7em-none-eabihf` build.

Plus, for each retrofit: the recorded worst error and instance count, and the
mutation run showing the existing suite passing while the new test fails.

## 9. Acceptance criteria

RFC 030 §9 items 1-6.

## 10. Review request

Standard eleven-item format; **record your author tier**; send to the architect.
Do not move RFC 030 to `done/` — it moves with the release that carries it.
