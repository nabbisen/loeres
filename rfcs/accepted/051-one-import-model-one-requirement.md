# RFC 051 - One Import Model, One Requirement

**Status.** Accepted (design frozen 2026-10-30)

**Author tier.** `architect`.

**Governing scoping.** RFC 049 §0.1 (Amendment 1), which measured four wildcard sites it did not
cover and deferred them to the owner; architect review 116 §3, which priced the three treatments;
architect review 117 §7, which records that the choice got more expensive once RFC 049 landed. Every
figure here was measured on the tree at `438c5d3`, `0.24.1`.

RFC 049 fixed what the README tells a user to write. This fixes what the **normative specification**
tells them to write, and ends the disagreement RFC 049 created between the two.

## 1. Summary

`docs/specs/loeres-external-design-v1.md` §1.4 **Public Import Model** states, normatively, what a
downstream user should write. It says `version = "0.x"` for all four crates — the wildcard form
`deny.toml:49` denies in this project's own manifests, and the form RFC 049 removed from the
README.

So the project currently gives two answers to one question: `README.md` says `"0.24"` and its own
normative external-design spec says `"0.x"`. RFC 049 named these four sites and deferred them
rather than widening a frozen RFC; this is that decision, brought back with its options priced.

**§7 carries one open question for the owner** — which of three treatments the spec's fences get —
and it is the only thing in this RFC that is not already decided. §8 adds one small cleanup that
belongs in whichever slice touches this predicate.

## 2. The defect, measured

### 2.1 Eight sites were measured; four were fixed

`git grep -n '= "0\.x"' -- '*.md'`, over tracked Markdown, with the generated and untracked
`docs/book/` excluded, returns exactly four current-facing matches today — all four in one file:

| file | lines | what it is |
| --- | --- | --- |
| `docs/specs/loeres-external-design-v1.md` | 238, 239, 252, 253 | the `toml` fences of §1.4 **Public Import Model** |

Every other match is `rfcs/`-layer text quoting the defect, which no assertion scans.

The four sit under *"The intended downstream import model is:"*, split into a **Cluster user**
fence at `:236-240` and a **Device user** fence at `:250-254`, each preceded by a `[dependencies]`
table header and followed by a bullet list of what that user normally imports. They are
byte-equivalent to the four RFC 049 removed from the README, modulo the README's column padding.

### 2.2 Why a spec's copy is worse than a duplicate

Checked rather than assumed: both snippets first appeared on 2026-06-30 under RFC 008, the
README's one commit before the apex sync (`e511c54`, then `7ab0108`). Neither is derived from the
other.

What makes these four worse than the README's is the document's standing. This file is one of the
three normative apex specs, and its currency block is machine-held by `doc-currency` — the block
sits in the same file, correct, while the instruction twelve screens below it tells a user to
write a denied form. §1.4 does not illustrate a requirement; it **specifies the import model**. So
this is RFC 047 property **B** failing one document higher than RFC 049 found it, and property
**D** — two doors, two answers — failing between a spec and the README that implements it.

### 2.3 The window RFC 049 opened

Before RFC 049, both documents said `"0.x"`: wrong, and consistent. After it, `README.md:25,26,29,30`
say `"0.24"` and the spec still says `"0.x"`. A reader who follows the spec writes the form the
project denies; a reader who compares the two finds a contradiction and cannot tell which is
current. **This was the stated cost of dispatching RFC 049's core first (review 116 §5), and it is
now paid rather than predicted.** Leaving it is the one outcome that makes the tree less consistent
after RFC 049 than before it.

## 3. What is already true, and must not be re-fixed

- **`README.md` and every `crates/*/README.md` are already held.** `doc_currency.rs`'s
  `check_readme_requirements` asserts the root README's requirement equals `MAJOR.MINOR` of the
  apex block's `Last reconciled repository release`, and denies a wildcard in all six READMEs
  (RFC 049 S4). **Do not change that predicate's existing clauses.**
- **The apex block in this very file is correct** and is the value any new assertion reads. Do not
  parse it a second time; `check_apex_currency` already returns it.
- **`docs/src/specifications.md`'s migration advice is deliberately version-named.** *"If you use
  `Q32` with the constrained kernel, move your requirement to `0.24`"* at `:46-50` is a true
  statement about where a fix landed, not a currency claim, and it must not be swept into a
  released-minor assertion. **Out of scope** — and see RFC 052, which holds the claims in that
  section that *are* currency.

## 4. S1 — the spec's fences stop specifying a denied form

The treatment is §7's open question. Both candidate treatments share this much, and the
implementer does only the one the owner authorizes:

- the two fences keep their `[dependencies]` header, their crate names and their features
  exactly; the **only** thing that changes is the version requirement;
- the bullet lists at `:242-246` and `:256-260`, the surrounding prose, and §1.4's heading
  structure are untouched;
- the file's apex currency block is untouched.

## 5. S2 — the assertion that keeps it

One clause added to the predicate `doc_currency.rs` already has, not a new function and not a new
gate. Which clause depends on §7:

- **under treatment A**, the two fences in `docs/specs/loeres-external-design-v1.md` join the root
  README in clause 1: every requirement site in them must equal `MAJOR.MINOR` of the apex block's
  `Last reconciled repository release`, and clause 2's wildcard denial covers them;
- **under treatment B**, the assertion is that those fences contain **no** requirement site at all
  — no `version` key inside a `toml` fence in that file — which needs no value and cannot go
  stale.

In both cases the failure message names the file and the 1-based line, as clause 3 requires, and
the existing `requirement_sites` scanner is reused rather than copied.

## 6. S3 — the record

One sentence in `docs/src/documentation-convention.md` §5's table if treatment A is chosen (the
spec's import model names the released minor), or none if B is chosen, since B's property is the
absence of a value and belongs in the RFC rather than in a convention about documents.

`CHANGELOG.md` states which treatment was chosen and why, in one paragraph, and says that the
eight sites RFC 049 Amendment 1 measured are now all held — **the first completeness claim this
pair of RFCs may make**, because it is the first point at which it is true.

## 7. Open question — the owner's, at acceptance

Which treatment §1.4's fences get. Each cost is measured, not estimated.

**A — pin them as the README is pinned** (`"0.24"`, clause 1 extended to that file).
*Cost:* a normative apex spec carries a value that must move at every reconciled release. The cost
is mechanical rather than human — the value is parsed from the apex block already in that same
file, so the gate carries the maintenance and names the line when it is wrong — but the spec now
has a release-coupled cell where it had none.
*Gain:* both documents give one answer, and a reader who copies the spec's fence gets a manifest
that resolves.

**B — the fences show no version and the README is the one home** (`loeres-cluster = { features =
[...] }`, one sentence naming the README as where the requirement lives).
*Cost:* §1.4's fences stop being complete copy-pasteable manifests; a reader who copies one must
add the requirement. For a spec whose stated subject is *which crates and which features*, I judge
that correct, but it is a real loss for a reader treating the fence as an example.
*Gain:* zero per-release maintenance, one home for a release-coupled fact — RFC 048's own move —
and an assertion that can never itself go stale.

**C — leave them.**
*Cost:* §2.3. The contradiction stays, in a normative document, with a gate reading the same file
for other purposes. This is the only option under which RFC 049 made the tree less consistent.
*Gain:* nothing is edited.

**I recommend B, with A acceptable.** A's only advantage over B is the copyable fence, which a
normative design spec does not need; B composes with RFC 048 instead of adding a seventh
release-coupled value. **C is not recommended and this RFC does not provide for it** — authorizing
C means withdrawing this RFC, which is a legitimate answer and should be said in those words.

## 8. S4 — one cleanup, because this slice is already in that file

`doc_currency.rs` now holds the five `crates/*/README.md` paths **twice**: in
`CRATE_README_PATHS`, added by RFC 049, and in `check_stale_ledger`'s `whole_file_targets`, which
has held the same five plus the root README since earlier. A sixth crate would silently fall
outside the wildcard denial while remaining inside the stale-phrase ledger.

Unify them on one constant. `published_metadata.rs:111`'s `PUBLISHED_CRATES` is a third copy and
is **out of scope** — it is a different gate with a different subject (what is published), and
merging the two would couple a documentation check to a release check for no gain.

This was left undone deliberately: RFC 049's handoff forbade touching any other function in the
file, so the implementer was right not to (architect review 117 F5). The instruction was correct
and its cost is one duplicated constant, collected here rather than left.

## 9. Non-goals

- **No change to the README, any `crates/*/README.md`, or RFC 049's existing clauses.**
- **No change to `docs/specs/loeres-external-design-v1.md` outside §1.4's two fences** — not its
  apex block, not §1.3's diagram, not the bullet lists.
- **No change to `docs/src/specifications.md`** — including its version-named migration advice
  (§3). RFC 052 covers that section's currency claims.
- **No new gate, no new predicate function, no version change, no tag.**
- **No `PUBLISHED_CRATES` merge** (§8).

## 10. Risks

- **Functional — Low.** One document's two fences and one predicate clause.
- **Business — Low, and the slice exists to lower it.** The effect is that the normative spec and
  the README stop disagreeing about the one thing a user must type.
- **Operational — Low under B, Low-with-a-cell under A.** A adds one release-coupled value to a
  normative spec; the gate holds it, but it is one more thing a release reconciliation touches.
- **Security — Low.** No channel, no artifact, no trust root. Worth one sentence only because a
  wildcard requirement is a supply-chain shape this project denies itself, and under C that shape
  keeps being specified in a normative document.

## 11. What I could be wrong about

- **B's reading of what §1.4 is for.** I claim a normative import model specifies crates and
  features, not versions. Someone could reasonably hold that an *external design* spec must state
  a complete, resolvable example, which is the argument for A.
- **That a user reads the spec at all.** I have not measured whether anyone reaches
  `docs/specs/loeres-external-design-v1.md` before the README. If nobody does, the defect is
  internal consistency rather than user confusion — which still matters under property G, but is a
  weaker claim than §2.2 makes.
- **§8's sixth-crate risk is hypothetical.** No sixth crate is planned. The cleanup earns its place
  because the slice is already in that file, not because the risk is live.
