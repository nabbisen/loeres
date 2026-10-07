# RFC 037 implementation handoff — Measuring Effectiveness and Cost

**To:** implementer (mid-capability model)
**From:** architect
**Authorized by.** The owner accepted RFC 037 on 2026-10-07. The RFC is
`rfcs/accepted/037-measuring-effectiveness-and-cost.md`; read it first — this handoff
does not restate its reasoning. Architect review 086 holds the scoping.
**Base revision:** `20eba27` or later on `main`.

**Three review requests.** **A** covers S1+S2 (the harness and the baseline gate),
**B** covers S3+S4 (effectiveness and throughput), **C** covers S5 (the book chapter
and the `README.md` headline). Each slice must leave `cargo xtask check` green on its
own commit, and **every request must include `cargo xtask release-gate`** — see §8.

## 0. Values measured by the architect before this was written

Reproduce these. If your numbers differ, that is a finding and belongs in your review
request — do not adjust the corpus to match.

### 0.1 The corpus family, exactly

A dynamic QP over `DenseMatrix<f64>` / `DenseVector<f64>`:

- `Q` is `n × n`, `Q[i][i] = 2.0`, `Q[i][i+1] = Q[i+1][i] = off`, all else `0.0`
  (tridiagonal, symmetric positive semidefinite for `off ≤ 1.0`);
- `c[i] = -1.0`;
- box `lower[i] = 0.0`, `upper[i] = 10.0`;
- `A` is `m × n`; row `r` has `1.0` at columns `(r + k) mod n` for `k ∈ {0, 1, 2}`,
  all else `0.0` — each row caps a sliding window of three variables;
- `b[i] = 1.0`;
- `x₀[i] = 0.0`;
- `step_scale = 0.3`;
- `ConstrainedProjectedConfig { max_iterations: 20_000, tolerance: 1e-10, projection_max_sweeps: 500, projection_tolerance: 1e-10 }`;
- `ClusterExecutionContext::new(ClusterCancellationToken::new(), 0, ClusterValidationPolicy::ValidateAllInputs)`.

**Why `0.3`.** RFC 032's Gershgorin bound for this family is `U = 2 + 2·off`, so the
admissible step is `α < 2/U`, which is `0.909` at `off = 0.1` and `0.503` at
`off = 0.99`. A single `α = 0.3` is admissible across the whole corpus, which is what
lets one step scale serve both axes. Do not vary `α` with `off`; that would conflate
the two things being measured.

### 0.2 The eleven reference points

**Size axis, `off = 0.50`:**

| `n` | `m` | outer iterations | cap hits | workspace bytes `(3n+2m)·8` |
|---:|---:|---:|---:|---:|
| 4 | 2 | 30 | 0 | 128 |
| 8 | 4 | 43 | 0 | 256 |
| 16 | 8 | 48 | 0 | 512 |
| 32 | 16 | 48 | 0 | 1 024 |
| 64 | 32 | 48 | 0 | 2 048 |
| 128 | 64 | 48 | 0 | 4 096 |
| 256 | 128 | 48 | 0 | 8 192 |

**Conditioning axis, `n = 32`, `m = 16`:**

| `off` | outer iterations | cap hits | `2/U` |
|---:|---:|---:|---:|
| 0.10 | 25 | 0 | 0.909 |
| 0.50 | 48 | 0 | 0.667 |
| 0.90 | 232 | 0 | 0.526 |
| 0.97 | 578 | 0 | 0.508 |
| 0.99 | 989 | 0 | 0.503 |

The `off = 0.50, n = 32, m = 16` row is shared by both axes and must agree.

### 0.3 Three properties the baseline gate depends on, verified

| Property | Measured |
|---|---|
| deterministic across runs | 5 debug runs, byte-identical output |
| independent of optimisation level | debug and release produce the **same** counted work |
| cheap enough to gate | the full 11-point corpus runs in **0.41 s** in a debug build |

That is why §2 can be an enforced gate and not merely a report.

## 1. S1 — `cargo xtask bench`, counted work, reported

A new `xtask/src/checks/bench.rs` (or a `bench/` module directory if it grows past one
file, following `conformance.rs` + `conformance/`).

It builds the §0.1 family over the corpus, solves through
`solve_constrained_projected_first_order_dyn`, and reports **from the solve record**:
`report.iterations_executed()`, `report.status()`, `report.termination()`,
`projection_cap_hits`, `max_constraint_violation`, and the workspace footprint.

**Do not read `loeres_cluster::observe`.** It buckets and redacts by design
(`IterationsBucket`, `ElapsedBucket`) under RFC 009 and the threat model. The records
are the measurement surface; telemetry is for operators. Say so in the module doc.

### 1.1 Registration — three sites, not four

`bench` is the first xtask command that is **not** a gate. Register it in:

| File | What to add |
|---|---|
| `xtask/src/checks.rs` | `pub mod bench;` |
| `xtask/src/main.rs` (the `IMPLEMENTED` list) | `"bench",` |
| `xtask/src/main.rs` (the dispatch match) | `Some("bench") => checks::bench::run(),` |

**Do not** add it to the gate tuple list in `xtask/src/checks/release_gate.rs`.
`cargo xtask check` must not run `bench`: it reports figures, and figures are not
gates. The module doc must state that, so the next reader does not "fix" the omission.

### 1.2 Every figure is labelled measured or derived

The records give iterations, sweeps, cap hits and violation. They do **not** give
per-iteration arithmetic, and you must **not** instrument the kernels to count it —
RFC 037 §4.2 and §6; T4 measures rather than modifies.

Per-iteration cost is therefore **derived**: a dense `Q·x` is `n²` multiply-adds and
the projection is `O(m)` per sweep. Print `measured` or `derived` against every figure,
as a column or an explicit marker. A derived figure that reads as a measurement is the
failure this RFC exists to prevent, so make the distinction visible in the output, not
merely true in the code.

### 1.3 Keep the logic in pure cores with unit tests

Because `bench` is outside `cargo xtask check`, nothing else will exercise it. Put the
corpus construction, the table formatting and the measured/derived labelling in pure
functions with unit tests, so `cargo test --workspace` keeps them alive.

## 2. S2 — the `bench-baseline` gate, the device path, and a threshold

### 2.1 The gate

RFC 037 exit criterion 3 requires counted-work baselines to be **asserted**, and §1.1
keeps `bench` out of `check`. Resolve it with a second, narrow command: a
`bench-baseline` gate that runs only the deterministic counted-work corpus and compares
it against a pinned table.

- Registered at **four** sites, including the gate tuple list in
  `xtask/src/checks/release_gate.rs`, after `doc-build`. `Enforced`. It becomes the
  **twentieth** gate.
- The pinned table is a `const` in the gate, holding the eleven `(n, m, off)` rows of
  §0.2 with their expected iterations and cap hits.
- **Host target only** (RFC 037 §4.3). `TERMS_OF_USE` already states determinism is
  target-scoped, so the device target is reported by `bench`, never asserted here.
- No wall time in this gate, ever.
- A mismatch must name the row, the expected value and the measured one, in the style
  `published-metadata` already uses.

A changed count failing this gate is the intended behaviour, not a nuisance: it forces
a conscious decision, exactly as RFC 022's citation registry and RFC 030's differential
registry do. Say that in the module doc, with the remedy — re-measure, and update the
table in the same commit as the change that moved it.

### 2.2 The device path

The device entry point is const-generic:
`solve_constrained_projected_first_order<P, S, const N: usize, const M: usize>`, so its
corpus **cannot** be a runtime loop. Use a `const` table of `(N, M)` instantiations
driven by a macro, in the registry idiom RFC 030 and RFC 036 already use.

Report the footprint through
`loeres_backend_static::workspace::WorkspaceFootprint::footprint_bytes()` and check it
against RFC 027's documented `(3N + 2M)·size_of::<S>()` plus a 16-byte header. If the
two disagree, that is a finding about the documentation or the implementation — report
it, do not paper over it.

### 2.3 The threshold `size-budget` has owed since RFC 010

`cargo xtask size-budget` reports the device artifact at **25 282 bytes** with
"threshold pending owner RFC", in a **debug** build.

Measure a **release** build of `-p loeres-device --no-default-features` for
`thumbv7em-none-eabihf`, report both numbers, and **propose** a threshold with stated
headroom and the reasoning for the headroom. **Do not set it.** The owner sets it; the
RFC deliberately declines to guess, and so should you. Put the proposal in your review
request, not in the gate.

## 3. S3 — effectiveness at scale

Extend the deviation-from-exact-optimum measurement beyond the smoke corpus, reusing
RFC 030's exact solvers rather than writing new ones. RFC 031's difficulty reporting
already prints, on the smoke suite:

```text
deviation from the exact optimum: largest 2.1070367672848533e-12;
  paths deviating by more than 1e-6: 0 of 10
```

Report the same quantity over the §0 corpus, and report how often `Converged` is
truthful against its three legs — feasible within `projection_tolerance` (RFC 027
Amendment 5), stationary **at the final iteration** (RFC 029), and produced by an
**uncapped** projection (RFC 033). A `Converged` that fails any leg is a finding, not a
statistic.

## 4. S4 — throughput, advisory

Single solve, batch, and `parallel-rayon` speedup. The parallel path is reachable:
`crates/loeres-cluster/src/runtime.rs:206` selects `BatchExecutionPolicy::Parallel` when
the feature is on, and `crates/loeres-cluster/src/solve.rs:116` is the parallel branch.

Every wall-time figure must carry its host and toolchain, and must say in the output
that it is **not reproducible**. Nothing in `cargo xtask check` or
`cargo xtask release-gate` may depend on a wall-time figure. `std::time::Instant` only
— no new dependency, and **no criterion** (RFC 037 §4.5).

## 5. S5 — the answer, and the headline

### 5.1 The chapter

A book chapter that answers "how effective or powerful" with the harness's figures and
their caveats. Counted-work figures are checked against a real run using RFC 035's
`example-output` machinery, which already exists in `xtask/src/checks/examples.rs`.
Wall-time figures must be marked unchecked, because they cannot be checked — and the
marking must be visible to a reader, not only to the gate.

### 5.2 The headline

`README.md:5` reads "high-throughput server solving" while four tracked documents say
the project has no throughput evidence: `docs/src/architecture.md:20`,
`docs/src/cluster-user-guide.md:248`, `crates/loeres-cluster/README.md:112`, and
`docs/src/threat-model.md:75` and `:161`.

**Do not decide this yourself.** Prepare both forms in your review request — the
substantiated version with its environment stated, and the version that leads with what
`zero-bleed` and `no-std` prove on every commit — and say which you would pick and why.
The architect recommends the second; the owner decides. Whichever is chosen, the four
disclaimers and the headline must agree afterwards, and your request must show that
they do.

## 6. Explicit non-scope

- **No tuning.** If a measurement makes a kernel change attractive, report it; do not
  make it.
- **No cross-solver comparison** (OSQP, Clarabel, or any other).
- **No kernel instrumentation**, no public API change, no new dependency, no criterion.
- **No wall-time gate.**
- No change to the conformance corpora or to RFC 031's reporting.
- No `#[allow(…)]`. Do not tag; do not run `cargo publish`.

## 7. What to do if the figures disagree with §0.2

Report it. Four of the architect's values were wrong across RFC 036 and you found them
by following the instruction and measuring; the same standard applies here. The corpus
in §0.1 is specified precisely so that a disagreement is informative rather than
ambiguous — if your iteration counts differ, the cause is either a real behavioural
difference or an ambiguity in §0.1, and both are worth knowing.

## 8. Required evidence

For every request:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-features --all-targets -- -D warnings
cargo test --workspace --all-features
cargo +1.85 check --workspace --all-features
cargo xtask check                                   # 19 gates; 20 from S2
cargo xtask bench                                   # standalone, from S1
cargo xtask bench-baseline                          # standalone, from S2
cargo xtask release-gate --intended-tag 0.22.2      # must PASS
```

`release-gate` is **required in every request**. Three RFC 036 handoffs omitted it and
that is the only reason C8's clean-extraction defect reached a release cut (record 083
§3). Redirect its whole stream to a file; never pipe it through `head` or `tail`. To
repeat a run on an unchanged revision, first remove only
`.git-exclude/release-evidence/<revision>-v0.22.2/`.

State in each request:

1. **Examples affected** — expected `none` until S5, which adds a book chapter.
2. The eleven reference rows as **you** measured them, beside §0.2.
3. For S2, the break-and-restore record for the baseline gate: change one pinned row,
   record the exact failure, restore, confirm it passes. A baseline gate nobody has
   seen fail is not known to work.
4. For S2, both device artifact sizes (debug and release) and your proposed threshold
   with its headroom reasoning.
5. Which figures your output labels `derived` rather than `measured`, and how a reader
   sees the difference.
6. Anything in this handoff that does not match the tree.
