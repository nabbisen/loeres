# Developer Handoff — RFC 035 visitor-facing documentation and real-world examples

**Governing RFC.** `rfcs/accepted/035-visitor-facing-documentation-and-examples.md` (design frozen 2026-09-26).
**Reference.** The project owner's documentation guideline, supplied at `.git-exclude/rules/ref/DOCUMENTATION_GUIDELINES.md` (untracked); S2 adapts it into the repository.
**Assigned to.** Implementer tier.
**Ordering.** **S1 → S2 → S3 → S4**, one review request each. S1 first is not a preference: S2's README snippet and S4's captured blocks must be **extracted from real programs**, not invented.

---

## 1. Purpose

A visitor cannot see a line of Rust, cannot find a problem they recognise, and
reads six sentences of release bookkeeping first. Examples are maintained as
build artifacts rather than documents. Fix both, and adopt a convention so
neither recurs.

**No crate code changes in any slice.** Not one line under `crates/*/src`.

## 2. S1 — two examples that name a problem before they name a type

Two new workspace-excluded examples, one per path, each with its own lockfile and
registered in `xtask/src/checks/examples.rs` beside the existing three.

**All four criteria are required** (RFC 035 §3.1):

1. **The first paragraph states a problem a domain reader recognises, naming no
   `loeres` type.** Types appear only after the problem is stated.
2. **It shows the system shape** — build the model, solve, **act on the result**
   — not a sequence of API calls.
3. **It prints something interpretable without the source open.**
4. It passes the `examples` gate.

**Device — recommended: one model-predictive control step with input bounds.**
Condensed MPC over a short horizon is `min ½uᵀHu + gᵀu` subject to
`u_min ≤ u ≤ u_max`, which is the RFC 006 kernel's own shape; adding rate limits
gives `Au ≤ b` and makes it RFC 027's. State the reduction from the physical
problem to the QP **in the example's own comments** — that reduction is the part
a reader cannot reconstruct.

**Cluster — recommended: allocation under capacity with a demand floor.** Per-unit
capacity is the box; the demand constraint is a row of `Ax ≤ b`. Matches the
"energy/logistics, scheduling" the README already claims.

A different problem is acceptable **with justification in the review request**.
The four criteria are the requirement; the instances are a recommendation.

**Do not** modify, retire or reframe the three existing examples: they are
conformance artifacts and RFC 023's exit criteria depend on them.

## 3. S2 — `README.md` to 3-30-3, and the convention adopted

### 3.1 The five defects, by name

| # | Where | What |
|---|---|---|
| 1 | above the summary | six accumulating `Repository release X shipped and carries RFCs …` sentences |
| 2 | Quick Start | **zero Rust**; `cargo check` plus a dependency block |
| 3 | Quick Start | a `v0.20.0 — Trusted/cache conformance hardening` note, five releases stale |
| 4 | More Detail | `shipped contracts 000–021`, stale by thirteen RFCs |
| 5 | More Detail | links into `rfcs/accepted/` and `rfcs/proposed/`, both empty |

### 3.2 What to do

Structure to the guideline: one-line summary and 3–5 features first; the working
example in Quick Start; links and meta last. Stay within **100–200 lines**.

**The example is extracted, not written.** Delimit a region of an S1 example's
source with marker comments and copy it verbatim. It must be **under 20 lines**
and must be the thing a first-time reader needs, not the most impressive thing.

**Add a `doc-currency` assertion** that the README block still equals the marked
region. This is the only new enforcement in S2, and it exists because items 3 and
4 are what rot looks like when nothing checks.

**Relocate, do not delete** items 1, 3 and 4 — their home is S3's §4.2 passage.
Verified safe: `doc-currency`'s README assertions are **negative only** ("must
not contain stale phrases"), so nothing requires the landing page to carry them.

**Replace** item 5's links with the RFC index (`rfcs/README.md`).

### 3.3 The convention

Adapt the owner's guideline into a tracked chapter in the book's maintainer
section, linked from `CONTRIBUTING.md`. It must state, beyond the guideline's own
content:

- **§2's four criteria** as the standard for any new example;
- that each example says **which problem it solves and for whom**;
- the workflow rule: **a slice that changes a public surface or a printed record
  states, in its review request, whether an example is affected** — the way
  slices already state their evidence.

Mechanically checkable parts (line ceiling, snippet match) become `doc-currency`
assertions; the rest is guidance, not a gate.

## 4. S3 — an entry path, and one home for currency

**4.1 A getting-started chapter that is a tutorial**: install, solve one problem
end to end, read the result. It is not a third reference — `cluster-user-guide.md`
(246 lines) and `device-user-guide.md` (238) stay as they are. Add it to
`SUMMARY.md` as the first entry after the introduction.

**4.2 One home for release currency.** The six sentences collapse into a single
passage; the README links it. **Do not touch the apex trio's RFC 024 blocks** —
they are the normative currency record and are not what this RFC is about.

## 5. S4 — examples maintained like documents

**5.1 The gate runs each example.** `xtask/src/checks/examples.rs` currently
builds each example and inspects its resolved dependency graph, and **never runs
one**. Add the run. Compilation proves a signature; running proves the program
works.

**5.2 Captured output is checked.** Nine lines are quoted in the two guides, and
**six of them are elided with `...`**:

```text
docs/src/device-user-guide.md:208-210   3 lines, all elided
docs/src/cluster-user-guide.md:84-86    3 lines, all elided
docs/src/cluster-user-guide.md:128-130  3 lines, complete
```

An elided line cannot be compared. For each block choose **one**:

- **make it complete** and check it byte-for-byte against the run, or
- **stop presenting it as captured output** and describe the behaviour in prose.

Do not keep a `...` line inside a fenced block that looks like real output. Where
output is genuinely environment-dependent, prose is the answer, not a partial
quote.

**5.3 Prove the check bites.** Change one digit in a captured block and show the
gate fails; restore it.

**5.4 Not in scope:** an example coverage registry asserting every kernel has an
example (RFC 035 §3.4). The convention states the obligation and review enforces
it.

## 6. Explicit non-change scope

- **No crate code.** No `crates/*/src` change in any slice.
- No change to the apex trio's currency blocks, `rfcs/done/`, the conformance
  corpus, or the three existing examples.
- **No performance claim anywhere** — no timing, no scale, no comparison. The
  measurement that would support one does not exist in this tree; it is Cycle 4's.
- No new gate: the `examples` gate gains work, and the count stays at **17**.
- No book hosting decision, no `SECURITY.md`, no `CODE_OF_CONDUCT.md`.

## 7. Prohibited shortcuts

- **Writing the README snippet by hand** instead of extracting it from S1.
- Leaving an elided `...` line presented as captured output (§5.2).
- Deleting the currency prose rather than relocating it (§4.2).
- Making the getting-started chapter a third reference guide.
- Any performance number.

## 8. Required evidence

Per slice: fmt, clippy `-D warnings`, `cargo test --workspace --all-features`,
MSRV 1.85, `cargo xtask check` (**17 gates**), `cargo xtask conformance` plus
`--suite extended` and `--suite adversarial` (24/24, 6/6, **26 of 31**),
`mdbook build docs --dest-dir target/xtask-book/local`, `cargo xtask examples`.

Plus, per slice: **S1** each new example's actual output; **S2** the README line
count and a demonstration that changing the snippet fails `doc-currency`; **S3**
the rendered `SUMMARY.md` tree; **S4** §5.3's mutation.

## 9. Acceptance criteria

RFC 035 §6 items 1-9.

## 10. Review request

Standard eleven-item format; **record your author tier**; send to the architect.
Do not move RFC 035 to `done/` — it moves with the release that carries it.
