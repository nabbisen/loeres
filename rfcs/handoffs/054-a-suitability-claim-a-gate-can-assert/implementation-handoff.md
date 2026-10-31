# RFC 054 — implementation handoff

**Author tier.** `architect`. **Date:** 2026-10-31.
**Governing text.** `rfcs/accepted/054-a-suitability-claim-a-gate-can-assert.md`, Accepted
(design frozen 2026-10-31). Where this file and the RFC disagree, **the RFC is the contract**;
report the disagreement rather than choosing. Findings are architect review 125.
**Release.** `0.25.0`. **This slice performs no version bump** — it adds no callable surface. The
bump belongs to the slice that lands the linear-objective step
(`rfcs/handoffs/release-0.25.0/scope-and-decision-points.md` §1.1), and until that lands the
workspace stays at `0.24.3`, so every slice here runs
`cargo xtask release-gate --intended-tag 0.24.3`. **If the step slice lands first, use its
version instead and say so in your request.**

**Order: S1, then S2, then S3, then S4.** S4 asserts what S1-S3 establish; written first it would
fail on a tree that is still correct.

**All normative prose below is written out in full.** It is user-facing text in an apex
specification; writing it is the architect's job, not yours. Use it verbatim. If a sentence in it
is wrong, say so — do not improve it silently.

## 0. Two claims I found while writing this handoff

Neither is in RFC 054 §2's tables. Both are inside sites the RFC already names, so correcting them
is within the frozen scope — but they are flagged rather than slipped in.

1. **`real-time-safe`** is in the same sentence as `safety-relevant` at
   `docs/specs/loeres-requirements-v1.md:36` and `docs/src/introduction.md:16`. It is the same
   class of claim: no gate asserts a deadline guarantee, `NG-010` deliberately narrows even the
   timing vocabulary, and RFC 054 §2.2 deletes *"Stable timing behavior suitable for WCET review"*
   as unheld. **It goes with the sentence**, and the replacement text in §1.1 and §1.4 omits it.
2. **`crates/loeres-backend-std/src/batch.rs:5`** says *"Lands in Milestone 3; this Phase 0
   placeholder declares no public items yet."* Milestone 3 is complete. **Do not touch line 5 and
   do not remove the module.** That module is part of a larger finding I am taking to the owner
   separately — it is one of four empty `pub mod`s in published crates, and removing a public path
   is a breaking change that is not RFC 054's to make. Correct **only** line 3, per §1.6.

A third datum, for the record and no action: `STALE_PHRASES` at
`xtask/src/checks/doc_currency.rs:56` contains `"phase 0 skeleton"` and does **not** catch
`"Phase 0 placeholder"`. That is RFC 047 §1.1's criticism of incident-grown lists, demonstrated on
this project's own ledger. S4's list is not that shape (RFC 054 S4), and it is worth knowing why
the difference matters.

## 1. S1 and S2 — the statement, and the deletions

### 1.1 `docs/specs/loeres-requirements-v1.md:35-36` — the two execution worlds

Old, exactly:

```text
1. **Cluster / Server optimization**: high-throughput, dynamically allocated, parallel, cloud-native computation.
2. **Device / Edge optimization**: deterministic, allocation-free, panic-averse, real-time-safe computation for embedded, control-loop, and safety-relevant environments.
```

New:

```text
1. **Cluster / Server optimization**: dynamically allocated, parallel, server-side computation.
2. **Device / Edge optimization**: deterministic, allocation-free, panic-averse computation for code that cannot allocate.
```

The compile-time boundary this sentence introduces is **not** being changed (RFC 054 §4 item 3) —
only the adjectives no gate asserts.

### 1.2 `docs/specs/loeres-requirements-v1.md` — insert the statement at the head of §2

Immediately after the `## 2. Target Users and Use Cases` heading and before `### 2.1`, insert:

```text
**Who this is for.** Loeres is for engineers writing Rust who already have a model and need it
solved under constraints they can check. Using it means implementing this library's traits: there
is no file format and no command-line entry point, so a user who does not write Rust cannot use it
today.

Both paths solve exactly one class:

> Minimize `f(x) = ½xᵀQx + cᵀx` subject to `lo ≤ x ≤ hi` and `Ax ≤ b`, with all data finite and
> `Q` symmetric positive semidefinite as an unverified caller precondition. There are no equality
> constraints and no free variables.

Equality constraints and unbounded variables are **absent from the design**, not unimplemented.

**The device path's engineering properties, each asserted by a gate in `cargo xtask check`:**
`#![no_std]`, no `alloc`, no panic in public solve paths, bounded iteration, determinism for a
given target, zero external dependencies, and a caller-owned workspace whose dimensions are fixed
at compile time or at initialization. They are stated here because a gate asserts them, and that
is the only reason this document states any of them.

**What is not claimed.** Wall-clock throughput: it depends on the host, and no gate asserts it.
Solver breadth: there is one projected-first-order family, and no LP, SOCP, large-`n` or
adversarial-parity claim. **Fitness for any application domain:** this document names none, and
`TERMS_OF_USE.md` and `docs/src/threat-model.md` state what the project does and does not
establish.
```

The problem-class block is RFC 047 §2.2 **quoted, not paraphrased** (RFC 054 §4 item 6). Reproduce
it character for character, including `½xᵀQx + cᵀx`.

Reference `TERMS_OF_USE.md` and `docs/src/threat-model.md` as **code spans, not links**, exactly as
written. A relative link from `docs/specs/` would resolve, but code spans keep this insertion out
of `link-audit`'s graph entirely, and nothing here needs to be a link.

### 1.3 `docs/specs/loeres-requirements-v1.md` §2.1 — the server paragraph and four unheld rows

Old:

```text
Server users need high-throughput optimization over large or variable-sized models. They may run Loeres inside SaaS systems, batch jobs, distributed services, logistics engines, scheduling platforms, energy grid analysis, or analytics pipelines.
```

New:

```text
Server users solve many problems of the same class inside a long-running process or a batch job, over models whose dimensions are known only at run time.
```

**No sector is named in the replacement**, including the ones that look harmless. RFC 054 S2's rule
is that a corrected domain list is still a claim no gate can assert.

Then delete these four rows from the list below it, and **only** these four:

```text
- Optional FFI adapters to mature numerical engines.
- Support for large sparse or dense matrices.
- Support for solver families that are not deterministic enough for edge use.
- Isolation policies for multi-tenant service operation.
```

The five that remain — dynamic dimensions, batch solving, multi-threaded execution, async service
integration, and logging/metrics/tracing/cancellation/time budgeting — are all shipped and stay.
Change `- Batch solving.` to `- Batch solving with per-item outcomes.`, which is what RFC 008
actually delivers and is asserted by the batch tests.

### 1.4 `docs/specs/loeres-requirements-v1.md` §2.2 — the device paragraph and the WCET row

Old:

```text
Device users need small, predictable optimization kernels for real-time or near-real-time environments. Example domains include robotics, model predictive control, industrial controllers, autonomous systems, medical IoT, smart-grid devices, and inference-adjacent decision policies.
```

New:

```text
Device users solve small, fixed-size problems inside code that cannot allocate and must not panic.
```

Then delete this row, and only this row:

```text
- Stable timing behavior suitable for WCET review.
```

The other nine rows are each gate-asserted or RFC-defined and stay exactly as they are.

### 1.5 `docs/specs/loeres-requirements-v1.md:445` and `:516` — two crate purposes

| old | new |
| --- | --- |
| `loeres-backend-std` provides dynamic storage and high-throughput numerical backend integrations for server use. | `loeres-backend-std` provides dynamic storage for server use. |
| `loeres-cluster` provides server-side solver engines and orchestration APIs for dynamic, high-throughput workloads. | `loeres-cluster` provides server-side solver engines and orchestration APIs for dynamic workloads. |

The first also drops *"numerical backend integrations"*: that is the adapter capability RFC 054 §2.2
deletes two sections earlier, and leaving it in a crate's own purpose line would be the half-fix.
**Flagged, not slipped in** — if you read the RFC as keeping it, say so.

### 1.6 `crates/loeres-backend-std/src/batch.rs:3`

| old | new |
| --- | --- |
| `//! Grouped model storage supporting high-throughput batch construction.` | `//! Grouped model storage for batch construction.` |

**Line 5 is not touched** and the module is not removed (§0 item 2).

### 1.7 `docs/src/introduction.md:12-17` — the bullets, and the pointer

Old:

```text
- **Cluster / server optimization** — high-throughput, dynamically allocated,
  parallel, cloud-native computation.
- **Device / edge optimization** — deterministic, allocation-free, panic-averse,
  real-time-safe computation for embedded, control-loop, and safety-relevant
  environments.
```

New:

```text
- **Cluster / server optimization** — dynamically allocated, parallel,
  server-side computation.
- **Device / edge optimization** — deterministic, allocation-free,
  panic-averse computation for code that cannot allocate.

**Who this is for, and what is not claimed.** Loeres is for engineers writing
Rust who already have a model. The device path's properties — `#![no_std]`, no
`alloc`, no panic in public solve paths, bounded iteration, determinism for a
given target, and zero external dependencies — are each asserted by a gate in
`cargo xtask check`. Wall-clock throughput is not claimed; solver breadth is one
projected-first-order family; and no application domain is named as one this
library is fit for. `docs/specs/loeres-requirements-v1.md` §2 is the normative
statement, and `TERMS_OF_USE.md` and the threat model record what the project
does and does not establish.
```

**The currency block at `:3-7` is not touched.** It is machine-held and byte-identical across seven
documents; a stray edit there reports `APEX MISMATCH`.

### 1.8 S3 — `TERMS_OF_USE.md:8-13`

Old:

```text
Loeres describes itself as suitable for edge and control-adjacent domains,
including robotics, model-predictive control, industrial controllers, and
medical IoT. Those domains carry qualification obligations. This document
records precisely what the project does and does not establish, so that an
integrator can tell the difference.
```

New:

```text
An integrator evaluating Loeres may be considering edge or control-adjacent
work — robotics, model-predictive control, industrial controllers, medical IoT.
Those domains carry qualification obligations, and **Loeres makes no claim of
fitness for any of them.** This document records precisely what the project does
and does not establish, so that an integrator can tell the difference.
```

The domains stay named **because they are the integrator's concern, which is why this document
exists** — what changes is that they are no longer presented as the project's own description of
itself. **Nothing else in `TERMS_OF_USE.md` changes** (RFC 054 §4 item 1): the
no-safety-certification section, the panic-averse section and the standards list are the project's
most carefully written user-facing text.

## 2. S4 — the assertion

In `xtask/src/checks/doc_currency.rs`. **No new gate, no new command.**

### 2.1 Shape

- A const term list, **`SUITABILITY_CLAIM_TERMS`**, exactly these seven entries, lowercase:

  ```text
  safety-relevant, safety relevant, safety-critical, safety critical,
  medical iot, medical device, autonomous system
  ```

- A const document set, **`SUITABILITY_CLAIM_DOCS`**: `README.md`, the five crate `README.md`s,
  `docs/src/introduction.md`, `docs/specs/loeres-requirements-v1.md`,
  `docs/specs/loeres-external-design-v1.md`. Reuse `check_stale_ledger`'s existing crate-README
  list rather than retyping the five paths — RFC 051 unified `CRATE_README_PATHS` for exactly this
  reason, and a second hand-written copy is how two lists drift.
- A pure function over one document's text, so every branch is testable on literals, in the shape
  of `stale_phrases_in`; and `check_suitability_claims(errors)` called from `validate_repository`.
- Register **`SUITABILITY CLAIM`**, naming the document, the line, and the term found.

### 2.2 One difference from `stale_phrases_in`, and it is the opposite behaviour

`stale_phrases_in` **strips** decoration — `source.to_ascii_lowercase().replace(['`', '*'], "")` —
so a backticked phrase still matches. **S4 must mask instead**, with
`check_rfcs::mask_code_spans`, because RFC 054 S4's third required test is that *a term inside an
inline code span is masked, per RFC 050 S2*.

**Do not copy `stale_phrases_in`'s body.** Its decoration-stripping is correct for its own purpose
and wrong for this one. Fenced code blocks are **not** masked, which is the conservative direction
and is what RFC 053's clause 1 already does; a writer who needs to name a term in a covered
document backticks it inline.

### 2.3 The exclusion is a rule

`TERMS_OF_USE.md` and `docs/src/threat-model.md` are **absent from `SUITABILITY_CLAIM_DOCS`**, and
the const's doc comment must say why in one sentence: a document whose purpose is to name a
standard and disclaim it must be able to name it — `TERMS_OF_USE.md` names IEC 61508, ISO 26262,
IEC 62304, EN 50128 and DO-178C in order to say the project has been assessed against none of
them. Written as a rule, not as an exemption, so the next reader does not add a third exclusion for
convenience.

### 2.4 Tests — four

1. a covered document containing `safety-relevant` → one finding naming the term and the line;
2. the same word in a document outside the covered set → no finding;
3. the term inside a **single-backtick** span in a covered document → no finding, and the same
   term unbackticked on the next line → one finding (the negative-control pair);
4. `Safety-Critical`, mixed case → one finding, pinning case-insensitivity.

### 2.5 Measure it, and report the figure

After S1-S3, **count the occurrences of each listed term across the covered set**. RFC 054 §6
predicts **zero**. State the command you used, per architect review 121 §1 — and if it is not zero,
**stop and report** rather than deleting whatever remains: a surviving occurrence may be a sentence
the RFC meant to keep.

## 3. Verification, and what to hand back

1. `cargo xtask doc-currency` → PASS.
2. `cargo xtask check` → **PASS, 20 gates**, on the gate's own exit status, never through a pipe.
3. `cargo xtask release-gate --intended-tag 0.24.3` → PASS, whole stream to a file (see the header
   if the step slice has landed first).
4. `cargo xtask published-metadata` → PASS. §1.6 edits a published crate's rustdoc, and this gate
   is what holds the packaged READMEs and sources.
5. A `CHANGELOG.md` entry under the current unreleased heading. **It must not say the project is
   now suitable for anything.** It says which claims were deleted and what replaced them.
6. **Prove S4 catches the real thing:** restore `safety-relevant` to
   `docs/src/introduction.md`'s device bullet, record the finding text, revert, and confirm
   `git diff docs/src/introduction.md` is empty afterwards — not only `git status`.
7. The review request at
   `.git-exclude/review-request/RFC-054-a-suitability-claim/REVIEW_REQUEST.md`, stating §2.5's
   figure and anything in §1 you believe is wrong.

**Not yours:** moving RFC 054 to `rfcs/done/`, the apex `Implemented scope` field, the release
table, the four empty `pub mod`s of §0 item 2, the tag, the push, and publication.
