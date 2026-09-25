# RFC 035 - Visitor-Facing Documentation and Real-World Examples

**Status.** Proposed (2026-09-26)
**Design approval.** Architect-authored from the project owner's 2026-09-25 request and reference guideline; assessment in architect review 075.
**Tracks.** The landing page, the book's entry path, and the example set. Adopts a documentation convention so the regression this RFC fixes does not recur.
**Touches.** `README.md`, `docs/src/` (new chapters, `SUMMARY.md`), `examples/`, `Cargo.toml` (workspace `exclude`), `xtask/src/checks/doc_currency.rs`, `CONTRIBUTING.md`.

---

## 1. Summary

A visitor arriving at this repository cannot see a line of Rust, cannot find a
problem they recognise, and reads six sentences of release bookkeeping before the
one-line summary. Fix the landing page, add two examples that state a real
problem before they name a type, and adopt a convention so it stays fixed.

**Examples are documents, and this RFC treats them as such.** Today they are
maintained as build artifacts: the `examples` gate compiles each one and checks
its resolved dependency graph, and **never runs it**. Nine lines of captured
example output are quoted verbatim in the two user guides, and nothing checks
them. `doc-currency` does not mention examples at all.

## 2. What is wrong, measured

`README.md` is **94 lines** — inside the reference guideline's 100–200 band. The
defect is not length (architect review 075):

1. **The first screen is release metadata** — six accumulating "Repository
   release `X` shipped and carries RFCs …" sentences above the summary, one added
   every release.
2. **Zero Rust.** No minimal working example; "Quick Start" is `cargo check` and
   a dependency block.
3. **A five-release-stale note inside Quick Start** (`v0.20.0 — Trusted/cache
   conformance hardening`).
4. **"shipped contracts `000`–`021`"** — stale by thirteen RFCs.
5. **Links into `rfcs/accepted/` and `rfcs/proposed/`**, both empty.

Findings 1, 3 and 4 are one failure: release bookkeeping leaking onto a landing
page, added by the architect nine surfaces at a time.

The three existing examples each open with *"What this example demonstrates
(RFC NNN §…)"* and exercise an API surface. The README claims robotics, MPC,
scheduling and energy; **no example is any of them**.

**And nothing keeps them honest.** Measured:

| Asset | Guarded by |
|---|---|
| example compiles | `examples` gate |
| example's resolved dependency graph | `examples` gate (forbidden crates) |
| **example actually runs** | **nothing — the gate never runs one** |
| **9 captured-output lines quoted in the user guides** | **nothing** |
| **example prose** (what it claims to demonstrate) | **nothing** |

RFC 034 S2 updated one of those captured blocks when the printed line gained a
field — because the implementer remembered, not because anything would have
caught it.

## 3. Design

### 3.1 S1 — two examples that name a problem before they name a type

One per execution path, each satisfying **all** of:

- **The first paragraph states a problem a domain reader recognises**, with no
  `loeres` type named. Types appear only once the problem is stated.
- **It shows the system shape** — build the model, solve, *act on the result* —
  not a sequence of API calls.
- **It prints something a reader can interpret** without the source open.
- It is workspace-excluded with its own lockfile and passes the `examples` gate,
  as the three existing examples do.

**Device — recommended: a model-predictive control step with input bounds.**
Condensed MPC over a horizon yields exactly `min ½uᵀHu + gᵀu` subject to
`u_min ≤ u ≤ u_max`, which is the RFC 006 kernel's own shape; rate limits add
`Au ≤ b` and make it RFC 027's. This is what the device path was built for and
the README already claims it.

**Cluster — recommended: allocation under capacity and a demand floor.** A
dispatch/assignment problem over several units with per-unit capacity (box) and a
linear demand constraint (`Ax ≤ b`) is recognisable, matches the claimed
"energy/logistics, scheduling" domains, and needs the constrained cluster kernel.

The implementer may choose different problems **with justification**; the four
criteria are the requirement, the instances are a recommendation.

**Not in scope for S1:** retiring or rewriting the three existing examples. They
are correct conformance artifacts and RFC 023's exit criteria depend on them.

### 3.2 S2 — `README.md` to the 3-30-3 rule, and the convention adopted

**Structure:** one-line summary and 3–5 features on the first screen; a
**minimal working example under 20 lines** in the Quick Start; links and meta
last. Target the guideline's 100–200 lines.

**The example is extracted, not written.** It is a verbatim region of an S1
example's source, delimited in that source by marker comments, and
`doc-currency` gains an assertion that the README block still matches the marked
region byte-for-byte. A snippet that rots is worse than no snippet, and findings
3 and 4 are what rot looks like here.

**Relocate, do not delete:** the six currency sentences, the stale `v0.20.0`
note and the contract range move to their single home (§3.3). `doc-currency`'s
existing README assertions are **negative only** — "must not contain stale
phrases" — so nothing requires the landing page to carry them.

**Remove** the links into the two empty RFC folders; link the RFC index instead.

**Adopt the convention.** The owner's guideline, adapted to this project, becomes
a tracked chapter in the book's maintainer section and is linked from
`CONTRIBUTING.md`. Its mechanically checkable parts — README line ceiling, the
snippet match — become `doc-currency` assertions; the rest is guidance.

**The convention covers examples explicitly**, because they are documents:
§3.1's four criteria are stated there as the standard for any new example; each
example says which problem it solves and for whom; and the workflow rule is
recorded — **a slice that changes a public surface or a printed record states
whether an example is affected, in its review request**, the way slices already
state their evidence.

### 3.3 S3 — an entry path in the book, and one home for currency

- **A getting-started chapter that is a tutorial**, not a reference: install,
  solve one problem end to end, read the result. The two user guides (238 and 246
  lines) stay as references.
- **`SUMMARY.md`** gains it as the first entry after the introduction.
- **One home for release currency.** Every "Repository release `X` shipped and
  carries RFCs …" sentence collapses to a single passage in the book, which the
  README links. The apex trio's RFC 024 blocks are **untouched** — they are the
  normative currency record and are not what this RFC is about.

### 3.4 S4 — examples maintained like documents

**The `examples` gate runs each example**, not only builds it. Compilation proves
a signature; running proves the program still works. Cost is seconds for three
short programs, and it closes the gap that an example can compile while being
broken.

**Captured output in the book is checked.** The nine lines quoted in the two user
guides become verifiable: each captured block is delimited, and the gate compares
it against the output the example actually produces. Same mechanism as §3.2's
README snippet, same reason — the alternative is that it silently rots and a
reader copies a number that is no longer true.

Where output is legitimately unstable (timings, anything environment-dependent),
the block is **not** quoted verbatim; it is described. A block that cannot be
checked must not be presented as captured output.

**Not in scope, deliberately:** an example *coverage registry* on the RFC 030
pattern, asserting that every kernel or capability has an example. That needs a
rule for what deserves one, which is a judgement call that would harden into a
gate. The convention states the obligation (§3.2) and review enforces it; if
coverage later proves to drift, a registry is the right answer then, on evidence.

## 4. Explicit non-scope

- **No performance claim of any kind.** The fourth visitor question — "how
  effective or powerful" — requires measurement that does not exist in this tree
  (review 065 §1; review 075 §4). It is Cycle 4's, and putting a number on the
  landing page before the harness exists would repeat the mistake four releases
  have been spent undoing.
- **No crate code, no public API, no behaviour change.** Not one line under
  `crates/src`.
- No change to the apex trio's currency blocks, to `rfcs/done/`, or to the
  conformance corpus.
- No book hosting or publication decision; no `SECURITY.md` or
  `CODE_OF_CONDUCT.md` (separate, and the owner's call).
- The three existing examples stay.

## 5. Risks

| Risk | Mitigation |
|---|---|
| The README snippet rots, as items 3 and 4 already did | It is extracted from a gated example and `doc-currency` asserts the match — the only new enforcement this RFC adds |
| "Real-world" examples become long tutorials | The four criteria in §3.1 are about *framing*, not size; an example that no longer builds under the `examples` gate has failed |
| Currency prose regrows on the landing page | §3.3 gives it one home; the adopted convention says where documentation of each kind belongs |
| The chosen problems are not actually solvable by these kernels | Both recommendations reduce to the box- or `Ax ≤ b`-constrained QP the kernels already solve; the implementer must state the reduction in the example's own comments |

## 6. Exit criteria

1. Two examples meeting §3.1's four criteria, workspace-excluded, `examples`
   gate PASS.
2. `README.md` within 100–200 lines, opening with summary and features, carrying
   a working Rust example **under 20 lines**.
3. That example is extracted from an S1 example and `doc-currency` fails if the
   two diverge.
4. No release-currency sentence, stale release note, stale contract range or
   empty-folder link remains in `README.md`.
5. A getting-started chapter exists, is in `SUMMARY.md`, and is a tutorial.
6. The documentation convention is tracked in the book and linked from
   `CONTRIBUTING.md`.
7. The `examples` gate **runs** every example, and every captured-output block in
   the book is checked against real output; a deliberately changed number fails
   the gate.
8. The convention states §3.1's criteria as the standard for new examples and
   records the workflow rule that a slice touching a public surface or a printed
   record says whether an example is affected.
9. `cargo xtask check` (17 gates — the `examples` gate gains work, not a new
   gate), conformance 24/24, 6/6, 26 of 31 — all unchanged; no crate code
   touched.
