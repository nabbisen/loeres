# RFC 047 - The State Worth Aiming At

**Status.** Accepted (design frozen 2026-10-29)

**Author tier.** `architect`.

**Governing scoping.** Authorized by the owner on 2026-10-29 ("Authorized."), on the decisions
recorded in architect review 114. Architect strategy 106, requested by the owner on 2026-10-28 — *"let's
establish an overall strategy … let's acknowledge those setbacks as we work together to define
the ideal future state and the ultimate goal we should aim for."* Architect review 114 §1 and §3
corrected 106 §3's goal statement and decided what this RFC may contain; architect review 114 §2
settled the audience question whose answer §2.2 below records. 106's own §5 sequencing and §6
cost estimates are **deliberately not here** (114 §3).

This RFC states a destination and the properties that would constitute arriving. **It prescribes
no implementation**, authorizes no slice, and adds no gate. Each later theme is its own RFC and
is answerable to §2 and §3 of this one.

## 1. Why a destination needs to be tracked

This project's evidence apparatus is unusually complete and points inward. Twenty gates, a
conformance corpus with exact references, an RFC lifecycle, a hand-recomputed source-identity
anchor held across twelve consecutive releases, and a hash-pinned review record per slice — all
of it serves a reader who has already cloned the repository.

`rfcs/review-evidence-index.md` states, from RFC 020 §11.1, that archived review bundles are
historical/private review input and **not a public normative source**. A strategy that lives
only in that corpus is therefore a document the apex documents may never cite as their
destination, however carefully it is written. That is the whole reason this RFC exists: so
`ROADMAP.md` can stop being a list of milestones with nothing stated for them to be milestones
*toward*.

### 1.1 The one diagnosis this destination answers

Every significant defect this project has found in itself lived in a gap no gate spoke to, and
every one was found by a person noticing rather than by the machinery. The gates grew by
incident — sixteen, then nineteen, then twenty — so their coverage traces this project's
**history of failures** rather than its **surface of risk**. The destination below is written as
properties precisely because of that history: an unchecked claim here rots exactly as the
unchecked claims it catalogues did.

## 2. The goal

> **loeres should be the optimization solver whose claims its user can verify rather than
> believe — each one checkable against the artifact they received — and whose first solve, for a
> newcomer, is their own problem rather than our example.**

The two halves are one claim. The first is nearly true inside the repository and unevidenced
outside it. The second is false today. The bridge is to turn the verification apparatus outward:
the same evidence that lets this project assert its properties should let a user, or their
auditor, check them against the artifact they received.

### 2.1 What this goal deliberately does not say

**Not *safety-critical*, not *safety-relevant*, not *certified*.**

`docs/src/threat-model.md` states that this project ships no formal security proof, no safety
certification and no claim of release-readiness, that no dedicated safety-critical
engineering-use disclaimer exists (OQ-012 open), and that Apache-2.0 warranty language is not an
engineering-use or deployment-suitability assessment. It places system-level hazard analysis,
independent validation, safe fallback behaviour, watchdogs, resource limits and regulatory
assurance with the user.

The engineering properties that make loeres **fit to evaluate** for a constrained target are
`no_std`, no `alloc`, no panic, bounded iteration, determinism, and zero external dependencies
on the device path — each asserted by a gate, which is the only reason they may be stated at
all. Fitness for any particular safety-relevant system is the integrator's hazard analysis.
**No document deriving from this RFC may widen that**, and a goal sentence that reads as a
suitability claim is a defect in this RFC, not a stylistic preference (architect review 114 §1.1,
where exactly that defect was found and removed).

### 2.2 The problem class, stated

Settled from the trait set in `crates/loeres/src/problem.rs` rather than from intent (architect
review 114 §2.1):

> **Minimize `f(x) = ½xᵀQx + cᵀx` subject to `lo ≤ x ≤ hi` and `Ax ≤ b`, with all data finite
> and `Q` symmetric positive semidefinite as an unverified caller precondition. There are no
> equality constraints and no free variables.**

`Q` is a general `n × n` matrix, not a diagonal one. `m` may be zero. Equality constraints and
unbounded variables are **absent from the design**, not unimplemented: `LinearInequalities` is
the only constraint trait in the family, and finiteness of `Q`, `c`, `A`, `b` and the bounds is
validated rather than optional.

This sentence is normative for every user-facing document and every front door. It is also the
reason the first user-facing input format is a native one: a reader for a standard exchange
format — MPS for linear programs, QPS for quadratic — would accept, **as a matter of design
rather than of tuning**, a large class of files whose only honest answer is refusal, because
equality rows and free variables are ordinary features of such files. Such a reader remains a
legitimate later deliverable, on top of a settled native format; when it comes, **the scope
rejection is the deliverable and the parser is the easy half** — it must name the out-of-scope
feature in the file's own vocabulary and point at what this solver does solve.

## 3. The properties that constitute arriving

Written as properties, not features, so that each can be asserted rather than claimed.

| | property |
| --- | --- |
| **A** | A stranger solves **their own** problem without writing Rust and without cloning the repository. |
| **B** | **Every claim a user can read is asserted by a gate** — the install command, the version pin, the usage snippet, the printed output, the supported scope. |
| **C** | The **published artifact is the reviewed deliverable**, and the identity anchor covers what users receive, not only what this project archives. |
| **D** | **Scope is stated in the user's vocabulary and refusal is a first-class answer** — a non-converged solve, an out-of-scope file and an unrepresentable precision each say what cannot be done and why. §2.2 is the scope that must be stated. |
| **E** | **Graduating costs a translation, not a rewrite** — the file a newcomer writes maps onto the traits an integrator implements. |
| **F** | The device path's guarantees **survive the front door** — nothing new reaches `loeres`, `loeres-backend-static` or `loeres-device`. |
| **G** | **This destination and the claims about it are themselves subject to B** — no current-facing document may state a version, a release list or a scope that the workspace contradicts, and that is asserted mechanically rather than by review discipline. |

**G is not symmetry for its own sake.** It was added because a review of this RFC's own source
material found `docs/src/threat-model.md` opening with *"This chapter describes the implemented
v0.20.0 …"* against a `0.24.1` workspace, with a release list two releases short, in the one
document a cautious integrator relies on — and all twenty gates passing, because the only
currency check over that file is a blocklist of phrases each added after an earlier incident
(architect review 114 §4). A destination that cannot detect its own staleness is the defect in
§1.1 reproduced at the top of the document hierarchy.

### 3.1 Property E is satisfiable, measured

Recorded here because E was the property most at risk of being an aspiration. Across all 61
fixtures of `conformance/`, spanning schema versions 1, 2 and 3 and both solver families, the
`[problem]` and `[config]` tables are **uniform with a strictly additive extension**: five
problem keys and three config keys in every fixture, plus a constraint block in the 48 fixtures
that have constraints. No key is renamed, dropped or redefined across the three versions
(architect review 114 §5).

So a declarative description of this library's problems is known to be small, sufficient and
already gated, and its keys map onto `BoxBounds` and `LinearInequalities` directly. **The
corpus's schema is ancestry, not compatibility**: `conformance/README.md` states that runtime
crates do not parse these files and do not depend on the fixture schema, and the schema has
versioned three times to serve test needs. A user-facing format must be its own contract,
related to the corpus by an **asserted** gate rather than by resemblance — two TOML formats that
look alike and differ subtly is the confusion property D forbids, and it would be this project's
own doing.

## 4. Anti-goals

These constrain as much as §3 does. Each is a decision, not a preference.

- **Not a general-purpose solver suite.** The kernels solve the narrow class in §2.2. Breadth
  would dilute the evidence claim, which is the asset. Narrow and defensible beats broad and
  plausible.
- **Not Python-first.** A binding may come; it must never become the only door, or the device
  path becomes a second-class citizen of its own library.
- **Not a hosted service.** It would make this project the trust root for other people's data.
- **No channel without verifiable artifact identity.** Stated on identity, not on connectivity:
  a channel is not disqualified for needing network, and is disqualified for shipping an
  artifact with no version identity to compare against.
- **No claim without a gate.** If a property cannot be asserted mechanically, it is not claimed
  in user-facing text. This is the direct answer to §1.1, and §3's property G applies it to this
  RFC's own descendants.

## 5. What this RFC does not decide

Named explicitly, so that their absence is a decision rather than an omission:

1. **Sequencing and cost.** Which phase comes first, what each costs, and what may be cut remain
   in the review corpus as advisory architect material, revised as each is measured. Freezing an
   unmeasured order under an Accepted RFC's amendment discipline would buy churn (architect
   review 114 §3).
2. **The binary-identity question.** What the release identity of a binary artifact is, in a
   project whose anchor is RFC 028's uncompressed-tar SHA-256 of source. Property C depends on
   its answer; this RFC does not supply one, and no channel that ships a prebuilt artifact can
   be entered honestly before it has one.
3. **The user-facing format's spelling.** In particular whether the diagonal `quadratic_diag`
   form the corpus uses is the right user-facing shape when the trait admits a general `Q`.
4. **Which distribution channel.** Only the anti-goal above constrains it.

## 6. Risks to this destination

**The assurance apparatus is both the asset and the cost.** Twenty gates, a conformance corpus, a
review record per slice and a hand-verified identity anchor are maintained by one owner and a
mid-capability implementer tier. Pursuing properties A, C and B outward adds a front door, an
artifact-identity model, per-platform distribution and an externally runnable evidence suite. That
could plausibly enlarge the surface that must stay green by a large factor — **an impression, not
a measurement** — and a strategy that outgrows its maintenance is how a project acquires the
unchecked gaps §1.1 describes.

Two disciplines bound it by construction, and both are already in §4: **no claim without a
gate**, which stops a phase from shipping a claim it cannot assert; and **narrow before broad**,
which every anti-goal exists to protect.

If this destination must be cut, cut **breadth of distribution** — one ecosystem done well, with
real artifact identity, beats several that lag. Do not cut property **C**: an unidentified
artifact is the one outcome that would contradict the project's first principle outright.
