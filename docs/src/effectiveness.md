# How Effective and How Powerful

The question a visitor brings is simple: *how effective or powerful is this?* The answer
has three parts, and they are not equally certain. This chapter gives each part with the
evidence behind it, and says what the evidence does not show.

| Part | What it is | Reproducible? | Checked? |
|---|---|---|---|
| Counted work | iterations, projection cap hits, workspace bytes | yes, on a given target | yes, against a real run (below) |
| Truthfulness of `Converged` | whether a converged result is feasible, stationary and uncapped | yes | yes, against the solve records |
| Throughput | solves per second, batch time, parallel speedup | **no**: wall time depends on the host | **no**: unchecked, and marked as such |

## Counted work

The program below solves a family of constrained problems and prints the work each solve
did. Its objective is tridiagonal, with `off` on the off-diagonals, and its `m` constraints
each cap three neighbouring variables. The first table grows the problem; the second makes
it more tightly coupled.

<!-- example-output: cluster-counted-work -->
```text
Size, at moderate coupling (off = 0.50):
     n      m    off   iterations   cap hits   accepted
     4      2   0.50           30          0        yes
    16      8   0.50           48          0        yes
    64     32   0.50           48          0        yes

Coupling, at fixed size (n = 32, m = 16):
     n      m    off   iterations   cap hits   accepted
    32     16   0.10           25          0        yes
    32     16   0.90          232          0        yes
    32     16   0.99          989          0        yes

A result is used only when accepted = yes: converged, feasible within 1e-10,
and with no projection stopped by its sweep cap.
```

Two things follow from the table.

- **Work does not grow much with size.** From four variables to sixty-four the iteration
  count stays between 30 and 48.
- **Work does grow with coupling.** Raising `off` from `0.10` to `0.99` takes the count from
  25 to 989 at fixed size.

A result is used only when it is accepted, which the last column records. The rule is in the
program itself: the status is `Converged`, the constraint violation is within `1e-10`, and no
projection stopped at its sweep cap. A solve that does not meet all three is reported, not
used.

Reproduce it with:

```sh
cargo run --manifest-path examples/cluster-counted-work/Cargo.toml
```

## Memory on the device path

The device path reports its workspace size exactly. For `N` variables and `M` constraints the
workspace is `(3N + 2M)·8 + 16` bytes on a 64-bit host. The figure is measured with `size_of`
and matches that formula at every size in the corpus. For example, `N = 32` and `M = 16` takes
1040 bytes. This figure is reported by `cargo xtask bench`; the book's checked figures are the
counted-work table above.

## Effectiveness

Over the twelve-point corpus, every solve converged, and every converged result is truthful on
all three conditions the solver promises: it is feasible within tolerance, it stopped on its
convergence criterion rather than on its iteration cap, and no projection was cut short. This
is reported by `cargo xtask bench`, which checks the solve records directly.

**The deviation from the exact optimum is checked for `n = 4` and `n = 8`.** The repository's
original exact reference solves only a separable objective, and this family is not separable, so
a second, independent reference was built: a dense active-set enumeration, scoped to `n ≤ 8`
because its search space grows too large past that. It is cross-validated against the original
reference on every fixture where both apply (43 fixtures, largest disagreement `1.8e-15`, which is
floating-point noise), so the two agreeing is evidence the new one is correct, not merely an
assertion that it is.

| `n` | `m` | `off` | largest `|solver iterate − exact optimum|` |
|---:|---:|---:|---:|
| 4 | 2 | 0.50 | `1.4e-10` |
| 8 | 4 | 0.50 | `1.6e-10` |

Both are near the solver's own tolerance (`1e-10`), which is what a correctly converged solve
should show. **Nothing larger than `n = 8` is checked this way.** For `n = 16` and above, this
chapter's evidence is the three legs above and the conformance suites, not a direct comparison to
an exact optimum.

## Throughput (wall time, unchecked)

The figures below are wall-clock times on one host. **They are not checked by any gate, and they
cannot be reproduced exactly:** a second run on the same machine will differ, and a run on
another machine will differ more. They are shown so that the order of magnitude is visible, with
the host beside them.

Measured 2026-10-07 on: x86_64 Linux, 32 logical threads, rustc 1.99.0. The problem is `n = 32`,
`m = 16`, `off = 0.50`.

| Measure | Median of repeats | Range observed | Status |
|---|---:|---:|---|
| One solve (50 repeats) | 3.1 ms | 3.1 to 3.2 ms | unchecked |
| Batch of 64, sequential (5 repeats) | 201 ms | 200 to 201 ms | unchecked |
| Batch of 64, parallel (5 repeats) | 14 ms | 13 to 16 ms | unchecked |

**Parallel speedup is never one number.** It is reported as the range across 5 paired
sequential/parallel runs of the same batch: **13.1× to 15.2×**, on this run. The batch is 64 items
on 32 logical threads, so an *ideal* speedup is near 32×; the observed range is roughly 41% to 48%
of that. The gap is scheduling and work granularity at this batch size, not a defect — a batch
this small does not give 32 threads enough to divide evenly. The batch figures use the host's
logical thread count as the worker count.

Reproduce the table with `cargo xtask throughput`, which prints the host and the toolchain
beside every figure.

## What this does not show

- **Other problem classes.** Every figure here comes from one family, chosen to vary size and
  coupling. Other problems may behave differently.
- **Larger problems than the corpus.** The largest instance measured has 256 variables.
- **Other hosts.** Counted work is reproducible on a given target. Wall time is not reproducible,
  and the figures above are from one machine.
- **Other solvers.** No comparison with other solvers is made, and none is planned until both sides
  can be tuned to the same standard.
- **The exact optimum for this family.** As above.
