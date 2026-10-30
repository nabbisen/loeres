# RFC 048 - One Release List That Cannot Rot

**Status.** Implemented (v0.24.1)

**Author tier.** `architect`.

**Governing scoping.** Accepted by the owner on 2026-10-29 ("RFC 048 is accepted."), as written —
which decides §7's open question in favour of the wider reading: **S2 covers all four documents**,
book chapters included, under one currency mechanism rather than two. RFC 047 property **G** — *no current-facing document may state a version, a
release list or a scope that the workspace contradicts, and that is asserted mechanically rather
than by review discipline.* Architect review 114 §4 found the defect while reading
`docs/src/threat-model.md` for an unrelated purpose, and understated it; §2 below is the
measurement, taken before this RFC was written, and it is five times larger than that review said.

RFC 047 is a destination and prescribes no implementation. This is the first implementing RFC
under it, and it is deliberately the smallest one: it adds **no new mechanism**. Every assertion
it relies on is already written, already reviewed and already running.

## 1. Summary

Seven tracked documents carry the same hand-maintained sentence listing this project's repository
releases. All seven stop at `0.22.0`. `CHANGELOG.md` records `0.22.1`, `0.22.2`, `0.22.3`, `0.23.0`
and `0.24.0` as released since. **Five releases are missing from seven documents**, including all
three normative apex specifications — where the stale prose sits a few lines away from a
machine-checked currency block that is correct.

`cargo xtask check` passes, twenty gates, because nothing compares a document's release claims
against the workspace.

This RFC deletes that sentence everywhere, extends the existing RFC 024 apex currency block to the
four current-facing documents that were making release-scope claims without one, and lets the
`doc-currency` gate hold them — the same gate, the same parser, the same cross-document identity
check that already holds the three specs.

## 2. The measurement

Taken on the tree at `bf72724`, `0.24.1`:

| document | last release its prose names | occurrences of the list sentence |
| --- | --- | --- |
| `ROADMAP.md` | `0.22.0` | 10 |
| `docs/specs/loeres-requirements-v1.md` | `0.22.0` | 6 |
| `docs/specs/loeres-external-design-v1.md` | `0.22.0` | 6 |
| `docs/specs/loeres-roadmap-milestones-v1.md` | `0.22.0` | 6 |
| `docs/src/introduction.md` | `0.22.0` | 6 |
| `docs/src/specifications.md` | `0.22.0` | 6 |
| `docs/src/threat-model.md` | `0.22.0` | 6 |

Released since, per `CHANGELOG.md`: `0.22.1` (2026-10-07), `0.22.2` (2026-10-07), `0.22.3`
(2026-10-07), `0.23.0` (2026-10-08), `0.24.0` (2026-10-08). `0.24.1` is the unreleased candidate.

Two further facts, each verified:

- **`docs/src/threat-model.md:3` opens** *"This chapter describes the implemented v0.20.0 device,
  cluster, validation, telemetry, and gateway boundaries plus RFCs 019/020/021, which shipped in
  repository release `0.20.2` (released 2026-07-30)"* — four releases behind the tree, in the one
  document a cautious integrator reads to decide whether to trust this library.
- **The list is an append-only run-on sentence.** Each release concatenates another clause onto one
  line: *"Repository release X shipped … Repository release Y shipped …"*. It is unreadable by the
  time it is accurate, which is why nobody re-read it.

### 2.1 Why `doc-currency` does not see it

The gate does read all seven documents. What it checks in them is a **blocklist of specific stale
phrases**, each entry added after an earlier incident
(`xtask/src/checks/doc_currency.rs:38-108`, `:60-78`). Nothing in it compares a claimed version
against `workspace.package.version`.

That is RFC 047 §1.1's diagnosis as code: the gates grew by incident, so their coverage traces this
project's history of failures rather than its surface of risk. A blocklist cannot catch a sentence
that was true when written.

## 3. What is already built, and must not be rebuilt

RFC 024's ordinary apex currency block is exactly the mechanism property G asks for. Verified by
reading `xtask/src/checks/doc_currency.rs` rather than from its documentation:

**The block.** A blockquote containing, in order:

```markdown
> **Release currency metadata.**
> This tree: **0.24.1**.
> Last reconciled repository release: **0.24.0**.
> Implemented scope: **RFCs 001-044**.
>
```

`extract_currency_block` takes the marker line and every following blockquote line **up to the
first blank blockquote line**. Everything after that bare `>` is free prose and is *not* compared
across documents — so each document may keep its own explanatory paragraph.

**What it enforces, today, for every path in `APEX_DOCS`:**

1. exactly one `**Release currency metadata.**` marker, and it must be inside a blockquote;
2. `This tree` equals `workspace.package.version` (`APEX RELEASE` on mismatch);
3. `This tree` **strictly exceeds** `Last reconciled` — equality is never valid, so a tracked tree
   can never claim to be a past release;
4. `Last reconciled` names a `CHANGELOG.md` record marked released;
5. `Implemented scope` equals the **exact set** of `NNN-slug.md` names under `rfcs/done/` excluding
   RFC 000, rendered canonically — a maximum would not do, so an accepted-but-not-done RFC below
   the highest number cannot be certified as implemented;
6. the four-line block is **identical across every apex document** (`APEX MISMATCH`), after
   whitespace normalization;
7. the two retired markers (RFC 020's draft, RFC 021's conditional) must be absent.

**`APEX_DOCS` is currently three paths** (`xtask/src/checks/doc_currency.rs:11-15`), all under
`docs/specs/`. The four documents in §2 that are not in it are the four that rotted. That is the
whole defect: the mechanism exists, and the documents making the claims were never put under it.

## 4. Slices

### S1 — delete the sentence, in all seven documents

Remove the hand-maintained release-list sentence wherever it appears. `CHANGELOG.md` keeps the
record, because it **is** the record: a dated section per release, which the apex block already
cites by reference.

This is the owner's dead-code rule applied to prose. The sentence is duplicated sevenfold, asserted
by nothing, and five releases stale; keeping a corrected copy in seven places would reproduce the
defect with a later start date. **Prefer removing it to maintaining it.**

### S2 — extend `APEX_DOCS` to the four documents that make release-scope claims

`ROADMAP.md`, `docs/src/introduction.md`, `docs/src/specifications.md`,
`docs/src/threat-model.md`. Each gets the four-line block from §3, **byte-identical** to the three
specs' (requirement 6 makes that mandatory, not stylistic), with its own prose after the bare `>`
if the document wants one.

Verified feasible before prescribing: none of the four contains either retired marker, and none
already contains a `Release currency metadata` block, so each addition is clean.

### S3 — correct `docs/src/threat-model.md`'s opening sentence

It must stop naming a version in prose. The block above it states the tree, the last reconciled
release and the implemented scope, all three machine-held; the sentence's job is to say what the
chapter covers, not when it was written. Its disclaimers — no formal proof, no safety
certification, no release-readiness claim — are correct and stay exactly as they are.

### S4 — `ROADMAP.md` cites RFC 047 as its destination

One sentence. This is the only additive item here, and it is the reason RFC 047 was accepted: a
roadmap that lists milestones with nothing stated for them to be milestones *toward* is the same
family of defect as a release list nobody checks. Everything else in this RFC is deletion plus a
constant.

## 5. Non-goals

- **No new gate, and no new predicate.** `doc-currency` already runs `check_apex_currency`; this
  RFC adds paths to a constant and content to four documents. If an implementation finds itself
  editing `parse_ordinary_apex_currency`, it has left this RFC's scope — **stop and report it.**
- **No change to the block's format**, its field names, or the `APEX MISMATCH` comparison.
- **No RFC-layer document is touched.** `rfcs/README.md` and the RFCs under `done/` are governed by
  `check-rfcs` and RFC 000's amendment discipline.
- **No version is changed**, no release is cut, no tag is created.
- **`CHANGELOG.md`'s own release sections are not touched.** They are the source this RFC makes
  everything else defer to.

## 6. Risks

- **Functional — Low.** Documentation and one constant. No library change.
- **Business — Low.** The only user-visible effect is that seven documents stop being wrong.
- **Operational — Low, and reduced after landing.** Each future release currently requires a human
  to append a clause in seven places and nothing complains when they do not; after this, the block
  is three fields in four more documents, each held by a gate that fails closed.
- **Security — Low, and this is the one worth naming.** The defect is in `threat-model.md`, the
  document a security-minded integrator relies on. Staleness there is not a vulnerability, and it
  reads as negligence about exactly the thing the document exists to be careful about.

## 7. What I could be wrong about

- **S2 puts book chapters under a mechanism RFC 024 wrote for the normative apex specs.** That is a
  deliberate widening and it is the substantive design decision in this RFC. The alternative — a
  separate, weaker check for book chapters — would mean two currency mechanisms that look alike and
  differ, which is the confusion RFC 047 property D forbids. I have chosen one mechanism; the
  owner may prefer the narrower reading, in which case S2 covers `ROADMAP.md` only and the two book
  chapters keep prose that no gate holds.
- **S1 deletes prose a reader might want.** Someone looking for "which release carried RFC 027"
  will have to open `CHANGELOG.md`. I judge that correct — one dated record beats seven
  paraphrases — but it is a judgement about readers, not a measurement.
- **The occurrence counts in §2 are of the sentence's leading phrase**, `Repository release
  \`0.2…\``, not of semantically distinct claims. A document may legitimately mention a release for
  another reason, and an implementation must read each occurrence rather than running a blanket
  substitution. `docs/src/recovery-roadmap.md` has one such mention and is **not** in §2's table
  for that reason: it is a historical narrative about a past recovery, not a currency claim.
