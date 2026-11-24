# RFC 054 - A Suitability Claim A Gate Can Assert

**Status.** Accepted (design frozen 2026-10-31)

**Author tier.** `architect`.

**Governing scoping.** RFC 047 §2.1 — *"Not safety-critical, not safety-relevant, not certified"* —
together with property **B** (*no claim without a gate*), property **D** (*scope stated in the
user's vocabulary*) and property **G**, applied to the oldest user-facing claim in the project:
who it is for. This is **Phase II** of `rfcs/destination-sequence.md`, and it is the whole of
Phase II's *"and say it"* half; Phase II's audience question itself is settled (§2.4). Findings are
architect review 125. Every figure here was measured on `da42822`, workspace `0.24.3`.

The owner's second principle governs this RFC more directly than any other in the sequence: *APIs
and UI/UX for users not to be confused or misunderstand*. The thing a reader is currently able to
misunderstand is whether this library may be used in a system that can hurt someone.

## 0.1 Amendment 1 (2026-11-24)

Made while Accepted, under RFC 000's in-place-amendment rule, and recorded in architect review
131. The Status line carries only the design-freeze date while this RFC is in `accepted/`;
**this amendment must be named in the Status line when the file moves to `rfcs/done/`**.

**§2.3's table was built from truncated `grep` output and reads as exhaustive. It is not.**
Measured with no limit (review 131 §2): **twelve** occurrences of the throughput claim exist in
tracked files outside `rfcs/` and `CHANGELOG.md`, where §2.3 judged seven. Two of the five it never
saw are defects of the same shape §2.3 did judge, and one of them is in a **third normative apex
specification this RFC never surveyed at all**.

### Two further defect sites, added to S2

| site | text | why |
| --- | --- | --- |
| `docs/specs/loeres-roadmap-milestones-v1.md:53` | *"**Cluster Layer:** dynamic, cloud-native, high-throughput execution."* | the identical sentence shape S2 corrects at `loeres-requirements-v1.md:35`, in an apex document outside the original survey |
| `docs/specs/loeres-requirements-v1.md:1225` | *"AC-014 \| Server solver design **supports** dynamic allocation and high-throughput integration without contaminating edge crates."* | an **acceptance criterion** — asserted, not aspired — of the same shape as `:445` and `:516` |

The other three unsurveyed occurrences are **not** defects, and are named so the judgement is on
the record rather than re-litigated: `loeres-requirements-v1.md:730` (a requirement about the
*caller's* high-throughput paths), `:1287` (a traceability row naming a requirement theme), and
`loeres-roadmap-milestones-v1.md:658` (a stated objective). `external-design:829`'s *Purpose*
column is likewise not a defect, by the same rule that spared `:723`'s *"may optimize for"*:
a purpose is an intention. `loeres-roadmap-milestones-v1.md:730` and `docs/src/threat-model.md:82`
are **disclaimers** and must survive.

### `loeres-roadmap-milestones-v1.md` joins S4's covered documents

Measured: a `safety-critical` added to that file leaves `doc-currency` **passing** (review 131 §2).
It is a normative apex specification that makes exactly the claims S4 exists to hold, and its
absence from `suitability_claim_docs()` is a consequence of §2's incomplete survey rather than a
decision.

**`docs/specs/loeres-reconciliation-traceability-v020.md` is deliberately *not* added**, by the
same rule that excludes `TERMS_OF_USE.md` and `docs/src/threat-model.md`: its OQ-012 row records
that a safety-critical disclaimer is *missing*, which is naming a term in order to disclaim it.

### The survey method, corrected

§2.3's defect is not its judgements but its completeness. A survey that becomes a table in an RFC
is **run with no `head` limit, and its total count is stated beside the table** — extending review
121 §1's remedy (state the command) with: the command must not be truncated, and the count must be
printed so a reader can see whether the table accounts for it.

## 1. Summary

**Three tracked documents say who Loeres is for. They do not agree, and the most permissive of
them is the normative specification and the book's first chapter.**

`docs/specs/loeres-requirements-v1.md:36` and `docs/src/introduction.md:16` say the device path is
for *"embedded, control-loop, and **safety-relevant** environments"*, and
`loeres-requirements-v1.md:117` lists *"robotics, model predictive control, industrial
controllers, **autonomous systems**, **medical IoT**, smart-grid devices"*. Two chapters later,
`docs/src/threat-model.md:176` records that **no safety-critical engineering-use disclaimer
exists** (OQ-012, open since `v0.20.0`), and `:183` that the library is not certified for
safety-critical control merely because its device path is bounded. `TERMS_OF_USE.md:8-13` opens by
*reporting* the permissive self-description and then spends the document saying what it does not
establish. RFC 047 §2.1 forbids the claim outright.

So *may I use this in a medical device?* has three different answers in this repository, and
**which one a reader gets depends on which file they open first.**

This RFC states who this is for **once**, in properties a gate already asserts and in the §2.2
problem class, with **no application domain named**; deletes the domain and suitability claims
rather than rewriting them; revises `TERMS_OF_USE.md`'s opening, which reports a self-description
that will no longer exist; and adds one assertion over the safety-and-compliance vocabulary, which
is the half of the problem a gate can close. **§6 records the half it cannot.**

**No crate source, no public API, no new gate, no new mechanism.**

## 2. The defect, measured

### 2.1 Who this is for, as the tree currently says it

| site | claim |
| --- | --- |
| `docs/specs/loeres-requirements-v1.md:36` | device path is for *"embedded, control-loop, and safety-relevant environments"* |
| `docs/specs/loeres-requirements-v1.md:111` | *"Server users need high-throughput optimization … SaaS systems, batch jobs, distributed services, logistics engines, scheduling platforms, energy grid analysis, or analytics pipelines"* |
| `docs/specs/loeres-requirements-v1.md:117` | device domains *"robotics, model predictive control, industrial controllers, autonomous systems, medical IoT, smart-grid devices"* |
| `docs/src/introduction.md:16` | the same *"safety-relevant"* sentence, in the book's **first chapter** |
| `docs/src/threat-model.md:176,183` | no safety-critical disclaimer exists; not certified for safety-critical control |
| `TERMS_OF_USE.md:8-13` | reports the permissive description, then disclaims what it does not establish |
| `rfcs/accepted/047-…:53` | **"Not safety-critical, not safety-relevant, not certified."** |

### 2.2 Capabilities listed as requirements that the tree does not have

`loeres-requirements-v1.md` §2.1's *representative requirements* for server users include
*"Optional FFI adapters to mature numerical engines"* (the gateway is mock-only, RFC 009),
*"Support for solver families that are not deterministic enough for edge use"* (there is **one**
family), *"Support for large sparse or dense matrices"* (no large-N claim exists anywhere), and
*"Isolation policies for multi-tenant service operation"*. §2.2's device list includes *"Stable
timing behavior suitable for WCET review"* — a safety-engineering activity, claimed where
`NG-010` deliberately narrows even the timing vocabulary.

### 2.3 `high-throughput`, judged site by site rather than swept

Architect review 086 objected to this claim in `README.md`; it was removed **there only**.
Surviving sites, with a verdict each, because three of them are not defects:

| site | verdict |
| --- | --- |
| `loeres-requirements-v1.md:35` | **defect** — states the server world *is* high-throughput, as a defining requirement |
| `loeres-requirements-v1.md:445` | **defect** — *"provides … high-throughput numerical backend integrations"* |
| `loeres-requirements-v1.md:516` | **defect** — *"provides … APIs for dynamic, high-throughput workloads"* |
| `crates/loeres-backend-std/src/batch.rs:3` | **defect, low severity** — published rustdoc claiming high-throughput batch construction |
| `loeres-requirements-v1.md:111` | **not a defect in itself** — *"server users **need** high-throughput optimization"* is true of the population; the defect beside it is the sector list and the implied fitness |
| `loeres-external-design-v1.md:727` | **not a defect** — *"It **may** optimize for … high throughput"* is a design permission, not an achievement claim |
| `loeres-external-design-v1.md:1374` | **not a defect** — rationale for why isolation matters to such workloads, not a claim about this project |

### 2.4 Phase II's audience question is settled, and this is not it

Recorded so this RFC is not read as deciding something already decided. Architect strategy 105 §2
asked which population the project builds for, because an MPS/QPS reader serves one and a native
format the other. **RFC 047 §2.2 settled the sequencing from the trait set**: `QuadraticObjective`,
`BoxBounds` and `LinearInequalities` are the whole family and there is no equality trait, so a
standard-format reader *"would accept, as a matter of design rather than of tuning, a large class
of files whose only honest answer is refusal"* — native first, whichever population is preferred.
Architect review 125 §5 withdrew the one remaining claim that anything was blocked on it.

**What is left of Phase II is this RFC: say it.**

### 2.5 In fairness to the documents

RFC 047 §2.1 binds documents *deriving from* it, and `loeres-requirements-v1.md` predates it by
most of the project's history. **This is not non-compliance**; it is the sweep §2.1 implies and
that the architect never scheduled. It is also the fourth instance of one pattern — a rule
adopted, the existing instances left in place — after RFC 048 (release lists in seven documents),
RFC 053 S2 (`ROADMAP.md`'s prose) and `Cargo.toml:31`. This instance is the most consequential,
because the claim is about whether the software may be used where people can be hurt.

## 3. What to do

### S1 — one statement, in properties, in one home

`loeres-requirements-v1.md` §2 is the normative home of *Target Users and Use Cases* and stays so;
RFC 020 settled that the specs are apex and the book summarizes. §2 is **rewritten** to say, in
substance:

- **Loeres is for engineers writing Rust who already have a model** and need it solved under
  stated, checkable constraints. Property **A** — a stranger solving their own problem without
  writing Rust — is false today by the project's own statement, so no honest audience statement
  can be wider than this until Phase III ships.
- **The device path** is for code that cannot allocate: `#![no_std]`, no `alloc`, no panic in
  public solve paths, bounded iteration, determinism for a given target, zero external
  dependencies, and a caller-owned workspace with dimensions fixed at compile time or
  initialization. **Each of these is asserted by a gate in `cargo xtask check`, and that is the
  only reason this RFC permits them to be stated.**
- **The cluster path** is for services solving many problems of the same class: dynamic
  dimensions, heap, threads, batching with per-item outcomes, cancellation, time budgets.
- **Both solve exactly one class**, quoting RFC 047 §2.2 verbatim rather than paraphrasing it, with
  its own sentence that equality constraints and free variables are **absent from the design, not
  unimplemented**.
- **What is not claimed**: wall-clock throughput, solver breadth beyond one projected-first-order
  family, and **fitness for any application domain** — pointing at `TERMS_OF_USE.md` and
  `docs/src/threat-model.md` rather than restating them.

`docs/src/introduction.md` carries the same property list — it is short, and every item in it is
gate-asserted, so the two cannot drift on substance — and links §2 as the normative statement. **No
new file.** RFC 048's lesson is fewer homes, not more.

### S2 — delete the domain and suitability claims; do not rewrite them

Every **defect** row of §2.1, §2.2 and §2.3. Delete and point, as RFC 048 S1 and RFC 053 S2 did.

**A corrected domain list is still a claim no gate can assert**, so "robotics, industrial
controllers" is not an acceptable replacement for "robotics, …, medical IoT". The list goes.

Two things are **not** deleted and must be preserved exactly: `loeres-requirements-v1.md:111`'s
statement of what server users *need* (§2.3 — true of the population), and §2.3's three
non-defect rows.

### S3 — `TERMS_OF_USE.md`'s opening, revised in the same slice

`:8-13` opens *"Loeres describes itself as suitable for edge and control-adjacent domains,
including … medical IoT."* After S2 that is no longer true of the project, so the sentence would
describe a self-description that no longer exists. It is revised to name the domains as ones an
**integrator may be considering**, which is why the document exists, rather than as ones the
project claims.

**The rest of `TERMS_OF_USE.md` is careful and correct and must survive untouched** (§4).

### S4 — one assertion, over the vocabulary a gate can hold

In `xtask/src/checks/doc_currency.rs`, in the shape of the existing `stale_phrases_in`:
case-insensitive, decoration-stripped, substring, register **`SUITABILITY CLAIM`**. No new gate, no
new command.

**The term list is the safety-and-compliance vocabulary only**, because those terms cannot be
anything but a suitability claim:

```text
safety-relevant, safety relevant, safety-critical, safety critical,
medical iot, medical device, autonomous system
```

**It applies to the claim-making documents** — `README.md`, the five crate `README.md`s,
`docs/src/introduction.md`, `loeres-requirements-v1.md`, `loeres-external-design-v1.md` — and
**explicitly not** to `TERMS_OF_USE.md` or `docs/src/threat-model.md`. That exclusion is a **rule,
not a convenience**: a document whose purpose is to name a standard and disclaim it must be able to
name it, and `TERMS_OF_USE.md` names IEC 61508, ISO 26262, IEC 62304, EN 50128 and DO-178C in
order to say the project has been assessed against none of them.

**Why a term list, when RFC 047 §1.1 criticises gates that grow by incident.** The criticism is of
a list that *is* an incident log. This list has a **stated membership rule** — safety,
certification and medical-device vocabulary — written beside the statement it protects, so
extending it is editing the same place that defines the scope, not appending to a scar record.
That is a real but bounded difference, and §6 prices what it still cannot do.

**Three tests at minimum:** a claim-making document containing `safety-relevant` fails; the same
word in `TERMS_OF_USE.md` passes; and a term inside an inline code span is masked, per RFC 050 S2.

## 4. What must not be swept in

1. **`TERMS_OF_USE.md` beyond its opening.** Its no-safety-certification section, the
   panic-averse-is-not-panic-free section and the standards list are the project's most carefully
   written user-facing text. S3 touches `:8-13` and nothing else.
2. **`docs/src/threat-model.md`.** It is the document that got this right. Untouched, and excluded
   from S4 by rule.
3. **The two execution worlds.** The compile-time boundary is the project's defining property and
   is gate-asserted (zero-bleed). S2 removes adjectives and domain lists from the sentences that
   describe it, never the boundary.
4. **`NG-001`-`NG-010` and every other normative requirement row.** Not audience text.
5. **`model predictive control` as a technique.** `examples/device-mpc-step` implements one and
   `docs/src/getting-started.md` describes it. Naming a technique an example performs is not a
   suitability claim, which is why that phrase is **absent from S4's term list** even though §2.1
   quotes it from a domain list being deleted.
6. **The problem class.** RFC 047 §2.2 is quoted, never edited.

## 5. Non-goals

- **No new audience.** This RFC narrows the stated audience to the measured one; it does not
  propose serving anyone new.
- **No answer to the verification concern** of review 125 §5.1 — that every problem this solver has
  been tested on was written here. It is a verification-area question with its own options and
  costs, and it is not bundled with a documentation correction.
- **No Phase III content.** No format, no front door, no refusal vocabulary.
- **No change to `crates/`** beyond `batch.rs`'s one rustdoc line (§2.3). No public API, no
  behaviour, no dependency, no feature, no `std`/`alloc` posture, no device determinism, no cluster
  behaviour. **Semver: none.** It targets `0.25.0` because that release is already a minor
  (`rfcs/handoffs/release-0.25.0/scope-and-decision-points.md` §1.1), not because it needs one.

## 6. Risks, and the half the gate cannot close

- **S4 holds the safety vocabulary, not the domain list.** `robotics`, `energy grid`,
  `logistics`, `scheduling platforms` cannot go in the term list without also catching legitimate
  prose — an example's description, a rationale, a comparison. So **a future domain list could
  return and no gate would stop it.** What stops it is S1's single home and the rule written there.
  This is the honest limit, it is the same shape as RFC 053's plural-form residual, and the first
  time it costs something is the time to reconsider.
- **A paraphrase evades the list.** *"Suitable for systems where failure is dangerous"* contains no
  listed term. Nothing detects it.
- **S4 could fire on a legitimate sentence** in a claim-making document — a changelog-style note
  about this very RFC, say. Measured now: after S2 there are **zero** occurrences of any listed term
  in the covered set. If a false positive appears, the fix is backticking, as RFC 053's C3 records,
  not an exemption.
- **Narrowing the stated audience could read as retreat.** It is the opposite, and S1 says so by
  construction: every property it states, a reader can check against the artifact they received.
  A domain name asks them to trust us instead.

## 7. What I could be wrong about

- **That `loeres-requirements-v1.md` §2 is the right home.** A user reads the book, not the
  normative spec. I chose the spec because RFC 020 made it apex and because two normative
  statements is the defect this RFC is fixing; the book carrying the same gate-asserted property
  list is my compromise, and if it drifts, the drift will be in the book.
- **That the term list should not include `WCET`.** §2.2 deletes *"suitable for WCET review"* as an
  unheld claim, but `WCET` is also a legitimate engineering word a device user may need to see
  discussed. I left it out of S4 and deleted the claim in S2; if it returns, that judgement was
  wrong.
- **That this is Phase II's whole remainder.** Phase II is *"decide who we are for, and say it"*.
  I believe §2.4 settles the deciding and S1 does the saying. If the owner reads Phase II as
  requiring a published statement aimed at a *prospective* user rather than a current one, that is
  Phase III's front page and this RFC is only its precondition.
