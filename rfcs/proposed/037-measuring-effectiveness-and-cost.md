# RFC 037 - Measuring Effectiveness and Cost

**Status.** Proposed (2026-10-07).

**Author tier.** `architect`.

**Governing scoping.** Architect review 086, which the owner authorized on
2026-10-07. Every measurement quoted below was taken by the architect on `ddbf356`
and is reproducible by the harness this RFC specifies.

## 1. Summary

The owner's fourth visitor question — **how effective or powerful is this?** — is the
one RFC 035 §4 deferred. It is still unanswered, and in the meantime the landing page
has been answering it without evidence: `README.md:5` claims "high-throughput server
solving" while four tracked documents say the project has no throughput evidence.

This RFC builds the harness, and resolves that claim.

Its central decision is that **"effective" and "powerful" are different questions with
different reproducibility**, and that a single "benchmark suite" conflating them is how
such suites come to mislead. Three classes, in decreasing reproducibility:

| Class | Reproducible? | May it be asserted? |
|---|---|---|
| Effectiveness — deviation from the exact optimum, truthfulness of `Converged` | yes, deterministic | yes |
| Cost in counted work — iterations, sweeps, cap hits, workspace bytes | yes, **per target** | yes, host target only |
| Throughput in wall time — solves/second, batch, parallel speedup | **no** | never |

## 2. What is already measured, and what the records give

RFC 031's difficulty reporting already computes, on the smoke corpus:

```text
deviation from the exact optimum: largest 2.1070367672848533e-12;
  paths deviating by more than 1e-6: 0 of 10
terminal max_constraint_violation: largest 8.359979375427429e-14
projection cap hits: 0 in 170 outer iterations (0.00%)
```

So **effectiveness is half-built and unpublished.** The solve records carry the exact
quantities: `ConstrainedSolveRecord` has `report` (status, iterations executed,
termination reason), `projection_cap_hits`, `max_constraint_violation` and
`infeasibility_evidence`.

`loeres-cluster::observe` must **not** be used as a source. It buckets and redacts by
design — `IterationsBucket`, `ElapsedBucket`, "Redacted elapsed-time bucket" — under
RFC 009 and the threat model. Telemetry is for operators; records are for measurement.

The device workspace footprint is introspectable through
`loeres-backend-static::workspace::WorkspaceFootprint::footprint_bytes()`, documented
as `(3N + 2M)·size_of::<S>()` plus a 16-byte header.

## 3. The finding that sets the shape: dimension is not the cost axis

The architect built a scalable family — tridiagonal SPD `Q` (2 on the diagonal, `off`
either side), `c = −1`, box `[0, 10]ⁿ`, `m` sliding-window halfspaces each capping a
sum of three variables at 1, `α = 0.3`, `tolerance = 1e-10`,
`projection_max_sweeps = 500` — and measured both axes.

**Size, at fixed conditioning (`off = 0.5`):**

| `n` | `m` | outer iterations | cap hits | workspace bytes |
|---:|---:|---:|---:|---:|
| 4 | 2 | 30 | 0 | 128 |
| 16 | 8 | 48 | 0 | 512 |
| 64 | 32 | 48 | 0 | 2 048 |
| 256 | 128 | 48 | 0 | 8 192 |

**Conditioning, at fixed size (`n = 32`, `m = 16`):**

| off-diagonal | outer iterations | admissible step limit `2/U` |
|---:|---:|---:|
| 0.10 | 25 | 0.909 |
| 0.50 | 48 | 0.667 |
| 0.90 | 232 | 0.526 |
| 0.97 | 578 | 0.508 |
| 0.99 | 989 | 0.503 |

**Iteration count is flat in dimension and grows about fortyfold with conditioning.**
That is the expected behaviour of a projected first-order method, and it means a
harness organised around problem size alone would measure the wrong thing and publish
a flattering, uninformative curve. The `2/U` column is RFC 032's Gershgorin bound
shrinking as the matrix stiffens.

The honest characterisation, which the harness must produce rather than assert:

- **dimension** drives memory, linearly and exactly — `(3n + 2m)` scalars — and
  per-iteration work;
- **conditioning** drives the iteration count, strongly.

## 4. Design

### 4.1 A two-axis corpus

The corpus is parameterised on **size and conditioning**, not size alone, and lives in
tracked fixtures beside the existing `conformance/` corpora. Every reported figure
names the family and both parameters, so no number is quotable without its problem.

### 4.2 Measured against derived, never blurred

The records expose iterations, sweeps, cap hits and violation. They do **not** expose
per-iteration arithmetic, and this RFC does **not** instrument the kernels to count it:
that would be a crate change, and T4 measures rather than modifies.

So per-iteration cost is **derived** from the algorithm — a dense `Q·x` is `n²`
multiply-adds, the projection is `O(m)` per sweep — and every output marks each figure
as *measured* or *derived*. A derived figure presented as a measurement is the failure
mode this RFC exists to avoid; the report must make the distinction visible, not
merely true.

An opt-in counting feature in the kernels is a reasonable future RFC. It is not this
one.

### 4.3 Counted work is reproducible per target only

`TERMS_OF_USE` already states that determinism is target-scoped. An iteration count can
differ on `thumbv7em-none-eabihf`, whose libm paths differ. Therefore any **pinned
baseline is host-target only**, and the device target is reported, never asserted.

The device path is const-generic over `N` and `M`, so its corpus cannot be a runtime
loop: it is a `const` table of instantiations, in the registry idiom RFC 030 and
RFC 036 already use.

### 4.4 Wall time is advisory, always, and carries its host

Throughput figures record the host and the toolchain, state that they are not
reproducible, and are never gated. Precedent: `size-budget` is "advisory baseline
reported" and RFC 031's difficulty is "reported, not enforced".

### 4.5 No new dependency

**Criterion is rejected.** It buys statistical rigour for §4.4 — the least reproducible
and least assertable class — and pays a large transitive tree against RFC 026's
allow-listed, `yanked = "deny"` posture. §4.1-§4.3 need no dependency; §4.4 needs only
`std::time::Instant`, already used in `loeres-cluster`.

## 5. Slices

### 5.1 S1 — `cargo xtask bench`, counted work, reported

The command, the two-axis corpus, and the counted-work measures from the records for
both paths. Reported, not enforced. No wall time.

### 5.2 S2 — growth, and the threshold `size-budget` has owed since RFC 010

Counted work and workspace bytes against both axes, host and device. `size-budget`
currently reports the device artifact at 25 282 bytes with "threshold pending owner
RFC". S2 measures a **release** build, proposes a threshold with stated headroom, and
the owner sets it; the RFC does not guess a number here.

### 5.3 S3 — effectiveness at scale

Deviation from the exact optimum beyond the smoke corpus, reusing RFC 030's exact
solvers, and how often `Converged` is truthful against RFC 027 Amendment 5, RFC 029 and
RFC 033's three legs — feasible, stationary at the final iteration, and produced by an
uncapped projection.

### 5.4 S4 — throughput, advisory

Single solve, batch, and `parallel-rayon` speedup. The parallel path is reachable:
`runtime.rs:206` selects `BatchExecutionPolicy::Parallel` when the feature is on, and
`solve.rs:116` is the parallel branch. Host and toolchain recorded; never gated.

### 5.5 S5 — the answer, and the headline

A book chapter that answers the fourth question with these figures and their caveats.
Counted-work figures are checked against a real run by RFC 035's `example-output`
machinery, which already exists; wall-time figures are marked unchecked, because they
cannot be checked.

**And the headline is resolved.** Either "high-throughput server solving" is
substantiated by S4 with its environment stated, or it leaves `README.md:5`. The
architect's recommendation, for the owner to overrule: lead with what is enforced on
every commit — one contract, two worlds, separated at compile time, which `zero-bleed`
and `no-std` prove — and let the throughput figures speak for themselves in the
chapter, where their host can travel with them.

## 6. Explicit non-scope

- **No tuning.** T4 measures. A kernel change the measurements make attractive is a
  separate RFC.
- **No cross-solver comparison.** A fair comparison against OSQP or Clarabel needs
  tuning parity this project cannot yet demonstrate, and an unfair one is worse than
  silence.
- **No kernel instrumentation** (§4.2), no public API change, no new dependency.
- **No wall-time gate**, ever.
- No change to the conformance corpora or to RFC 031's reporting.

## 7. Risks

| Risk | Mitigation |
|---|---|
| A figure becomes a claim the project cannot defend | Only §4.1-§4.3 may be stated without a host. The headline change in §5.5 is part of the work. |
| A derived figure read as measured | Every output labels each figure; §4.2 makes this the RFC's own failure mode. |
| Pinned counted-work baselines churn | Intended: a changed count forces a conscious decision, as RFC 022's and RFC 030's registries do. Host only. |
| The corpus flatters the solver | §3 is why the corpus has a conditioning axis. The adversarial corpus (RFC 031) stays the hard case. |
| Wall-time figures go stale in the book | Marked unchecked and dated; only counted work is gate-checked. |
| Measuring reveals poor behaviour | Then the project learns something true. Publishing it is the point; RFC 034's heuristic is documented as wrong in both directions for the same reason. |

## 8. Exit criteria

1. `cargo xtask bench` reports counted work for both paths over a corpus
   parameterised on size **and** conditioning.
2. Every reported figure is labelled *measured* or *derived*, and names its family and
   parameters.
3. Counted-work baselines are pinned for the host target and asserted; the device
   target is reported.
4. Effectiveness is reported beyond the smoke corpus, against exact optima.
5. Wall-time throughput is reported with its host, and no gate depends on it.
6. The book answers "how effective or powerful" with figures the harness produced,
   counted-work ones gate-checked against a real run.
7. `README.md:5` no longer asserts a performance claim the tree cannot support, or
   supports it with a stated environment.
8. `cargo xtask check` passes, and `cargo xtask release-gate` passes in every slice.
