# RFC 039 implementation handoff — What the Kernel Does With a Linear Objective

**To:** implementer (mid-capability model)
**From:** architect
**Authorized by.** The owner accepted RFC 039 on 2026-10-07. The RFC is
`rfcs/accepted/039-what-the-kernel-does-with-a-linear-objective.md`; read it first.
Architect review 091 §1 holds the scoping.
**Base revision:** `main` at or after the RFC 039 acceptance commit.

**Three review requests.** **A** covers S1 (characterise). **B** covers S2+S3 (correct the
claim, and the step-guidance pointer). **C** covers S4 (the algorithm decision), which is a
written decision, not code. `cargo xtask release-gate` is required in every one.

## 0. Values measured by the architect

Reproduce these in S1. If your numbers differ, that is a finding for your review request.

### 0.1 The LP family

`Q = 0` (all zero, `n × n`), `c` random in `[−1, 1]ⁿ`, box `[0, 1]ⁿ`, `A` random in
`[0, 1]^{m×n}`, `b` random in `[0.5, 1.5]^m`, `x₀ = 0`, `α = 0.3`,
`tolerance = 1e-10`, `projection_tolerance = 1e-10`, `max_iterations = 50_000`,
`ValidateAllInputs`. `n = 4`, `m = 3`, 300 instances from a seeded LCG.

### 0.2 Cap sensitivity

| `projection_max_sweeps` | Converged | Converged but **not** optimal |
|---:|---:|---:|
| 500 | 297 / 300 | 0 |
| 5 000 | 299 / 300 | 0 |
| 50 000 | **300 / 300** | 0 |

Worst optimality violation across every converged run: exactly `0.000e0`.

### 0.3 What the three failures at cap 500 look like

```text
status=NotConverged termination=NoProgress iters=7  violation=2.595e-5  caps=2
status=NotConverged termination=NoProgress iters=8  violation=1.930e-7  caps=4
status=NotConverged termination=NoProgress iters=40 violation=8.122e-10 caps=3
```

All three carry **nonzero `projection_cap_hits`**. That is the evidence that the binding
constraint is the Dykstra sweep cap, not curvature, and it is the single most important
measurement in this RFC.

## 1. S1 — characterise, against exact references

Use **exact references, not the architect's oracle.** RFC 037 §5.6's dense active-set
enumeration already handles `Q = 0` — a zero Hessian is a dense Hessian — so no new
numerical code is needed for `n ≤ 8`. Where `n > MAX_N`, say the deviation is not measured,
as RFC 037's chapter already does.

Beyond reproducing §0, the corpus must cover what the architect's sample could not:

- **An unbounded objective direction with a slack box.** Every instance in §0 was bounded by
  its box, so unboundedness never arose and nothing here says what happens. Construct one:
  a `c` direction with no constraint opposing it and an upper bound far away.
- **A degenerate vertex** — more rows active at the optimum than `n`.
- **A tie** — an optimal face rather than a vertex, where the answer is non-unique. The
  check is that the returned point lies on the face, not that it equals a particular vertex.
- **Cap sensitivity reproduced as a measurement**, not quoted from §0.2.

Report per instance: status, termination, `projection_cap_hits`, terminal violation, and
deviation from the exact optimum. Reuse `cargo xtask bench`'s reporting shape and its
measured/derived labelling; this is the same kind of evidence.

**Do not pin LP figures in `bench-baseline`.** Deviations are floats, and review 087's
ruling stands.

## 2. S2 — correct the claim, in thirteen places, and not in six others

### 2.1 Correct these

Each says LP is "not solved". Each must state the measured position with **both** caveats
from RFC 039 §2.2 — that a `Converged` LP result is optimal, and that convergence is not
guaranteed, with the projection cap named as where it binds.

| File | Line | Note |
|---|---:|---|
| `README.md` | 86 | |
| `TERMS_OF_USE.md` | 63-64 | the "no curvature to converge against" clause is the wrong reason, not just the wrong verdict |
| `crates/loeres/README.md` | 7-8 | **published to crates.io** |
| `crates/loeres-cluster/README.md` | 91 | **published** |
| `crates/loeres-device/README.md` | 51 | **published** |
| `crates/loeres/src/problem.rs` | 35 | **reaches docs.rs** |
| `crates/loeres-cluster/src/solve/constrained.rs` | 546 | **docs.rs** |
| `crates/loeres-device/src/solve/constrained.rs` | 742 | **docs.rs** |
| `docs/src/introduction.md` | 25, 40 | |
| `docs/src/cluster-user-guide.md` | 181 | |
| `docs/src/device-user-guide.md` | 154-155 | also carries the wrong reason |
| `ROADMAP.md` | 86 | |
| `examples/cluster-qp-constrained/src/main.rs` | 18 | a doc comment, not code — see §2.3 |

### 2.2 Do **not** touch these

A search-and-replace on "not solved" would damage all of them.

- **`CHANGELOG.md:530` and `:637`** — historical records of what shipped at `0.21.1`.
  RFC 020 makes the CHANGELOG the owner of "what shipped and when"; editing history would
  falsify it.
- **`rfcs/done/027-…` §11.6 and `rfcs/done/031-…`** — shipped RFCs. RFC 025: an RFC in
  `done/` is **never amended in place**, it is superseded. RFC 039 supersedes 027 §11.6's LP
  claim, and saying so in RFC 039 is how that is recorded.
- **`rfcs/handoffs/…`** — historical.
- **`docs/src/getting-started.md:63` and `examples/device-mpc-step/src/main.rs:34`** — these
  say a particular plan "was not solved to tolerance". Nothing to do with LP. A careless
  replacement here would corrupt a tutorial and an example's doc comment.
- **`rfcs/done/015-…:441-443`** — "not solved" about rejected inputs.

### 2.3 The apex specifications are a commitment, not a description

`docs/specs/loeres-requirements-v1.md:430` reads "**PF-001 (LP) — contract-only, not
solved.**", and `docs/specs/loeres-external-design-v1.md:635` and `:781` echo it.

**Leave the requirement disposition alone.** There is a distinction this RFC turns on: a
*description of what the kernel does* is a measurement and is now wrong, while a
*requirement's disposition* is a commitment about what the project supports. RFC 039
corrects descriptions. Committing to LP as a supported requirement is an S4 question and
the owner's call.

If you judge that an apex line reads as a description rather than a commitment, say so in
your review request with the exact wording and let the architect rule. Do not edit an apex
document under this slice without that ruling.

### 2.4 The example's doc comment

`examples/cluster-qp-constrained/src/main.rs:18` is inside the `//!` block. Changing it is
permitted — it is a doc comment, not code, and RFC 039 §3's "no kernel change" is about
solve behaviour. But check whether that example is a **worked problem** under RFC 038's
registry: if its second paragraph is the extracted problem statement, the per-example README
must stay in agreement, and `doc-currency` will tell you. For the record, that example is an
**API artifact**, so its README description is written rather than extracted.

## 3. S3 — point the refusal at what matters

`suggested_step_scale` returns `SolverError::NumericalDomain` at `U = 0`. **The behaviour
stays.** There is no curvature-derived step to offer, and inventing one is out of scope.

What changes is its documentation, and the LP section S2 writes: say that for a linear
objective the step is not the binding parameter — the projection's sweep cap is — and name
`projection_max_sweeps`. That is the actionable thing a caller needs, and on §0.3's evidence
it is also the true thing.

Do not change the error variant, add a new one, or add a new public function.

## 4. S4 — a decision, recorded

A written decision in your review request, with S1's evidence cited: is a dedicated LP
algorithm warranted?

**"No, and here is why" is an acceptable and possibly correct answer.** A simplex or
interior-point method for a `no_std`, allocation-free, panic-averse device path is a large
undertaking, and §0.2 suggests an LP user's first need is for the documentation to stop
turning them away. Do not implement anything under S4.

## 5. Non-scope

- **No kernel change.** Not one line that alters a solve. `check-public-api` stays green.
- No new public API, no new error variant, no change to `suggested_step_scale`'s behaviour.
- No edit to the files in §2.2, and no apex edit without a ruling (§2.3).
- No LP figure pinned in `bench-baseline`.
- No claim that LP is "supported".
- No new dependency, no `#[allow(…)]`. Do not tag; do not publish.

## 6. Required evidence

```text
cargo fmt --all -- --check          # run last, immediately before committing
cargo clippy --workspace --all-features --all-targets -- -D warnings
cargo test --workspace --all-features
cargo +1.85 check --workspace --all-features
cargo xtask check                   # 20 gates
cargo xtask release-gate --intended-tag 0.22.3      # must PASS
```

State in each request:

1. **Examples affected** — `cluster-qp-constrained` if S2 edits its doc comment; otherwise
   `none`.
2. For S1: §0.2's table as **you** measured it, and the unbounded, degenerate and tie cases
   with what the kernel did on each.
3. For S2: the thirteen locations, each with its new wording, and confirmation that none of
   §2.2's six was touched — `git diff --stat` makes that checkable.
4. For S3: the new wording of the `NumericalDomain` documentation.
5. For S4: the decision and the evidence for it.
6. Anything in this handoff that does not match the tree.
