# RFC 038 implementation handoff — An Entry Guide at Every Example

**To:** implementer (mid-capability model)
**From:** architect
**Authorized by.** The owner accepted RFC 038 on 2026-10-07. The RFC is
`rfcs/accepted/038-an-entry-guide-at-every-example.md`; read it and its **Amendment 1**
first, which withdraws the marker mechanism §2.1 originally specified.
**Base revision:** `main` at or after the RFC 038 acceptance commit.

**One review request** covers the whole RFC; it is small. `cargo xtask release-gate` is
required, as in every slice since record 083 §3.

## 0. What the architect measured before writing this

### 0.1 The two categories, and the exact shape to extract

Every example's `//!` block has a **one-line first paragraph** (its title). The **second**
paragraph is what differs, and it is what sorts the examples into RFC 038 §3's two
categories:

| Example | Second paragraph | Category |
|---|---:|---|
| `device-mpc-step` | 5 lines, "A heater, a valve or a motor drives a process that must track a setpoint…" | worked problem |
| `cluster-capacity-dispatch` | 6 lines, "A grid operator, a logistics planner or a plant scheduler has several units…" | worked problem |
| `cluster-counted-work` | 4 lines, "A planner who builds a constrained model wants to know two things…" | worked problem |
| `device-box-pfo` | 1 line, "What this example demonstrates (RFC 023 §11.3):" | API artifact |
| `cluster-qp-constrained` | 1 line, "What this example demonstrates (RFC 027 Amendment 1):" | API artifact |
| `cluster-batch-solve` | 1 line, "What this example demonstrates (RFC 023 §11.3):" | API artifact |

Extraction rule, which reproduced on all six: read lines from the top while they begin
`//!`; strip `//!` and one following space if present; split into paragraphs on empty
stripped lines. Paragraph 1 is the title, paragraph 2 is the problem statement.

### 0.2 Why not marker comments

Do not reach for `README-EXAMPLE-BEGIN` / `README-EXAMPLE-END`. That pair delimits **code**
and `marked_region` compares raw lines (`doc_currency.rs:871-880`). Inside a `//!` block a
plain `// MARKER` line contributes nothing to rustdoc, so **the paragraphs either side would
join into one** in the rendered documentation, and `doc-build`'s `-D warnings` would not
necessarily catch a silent merge. Amendment 1 records this. Structural extraction needs no
source edit at all.

### 0.3 The omission this gate must catch

The root `README.md` table has **five** rows against **six** directories:
`cluster-counted-work` is missing. It was added by RFC 037, declared in its review request,
accepted by the architect in review 087, and never listed. **Build the gate first and show
it failing on the current tree**, then add the row — see §4 item 2.

## 1. Per-example `README.md` (RFC 038 §2.1, §3)

One `examples/<name>/README.md` per directory, six in all. Keep each short — a signpost,
not a chapter.

**Worked problems** (`device-mpc-step`, `cluster-capacity-dispatch`,
`cluster-counted-work`): the README's problem paragraph is the **extracted second
paragraph**, reflowed to one paragraph of prose. Then the run command and the pointers.

**API artifacts** (`device-box-pfo`, `cluster-qp-constrained`, `cluster-batch-solve`): a
written description, which must say that the example is an **API artifact** rather than a
worked problem, so a visitor who lands there is not left expecting a domain story. The
convention already says their form "is not the standard for new examples"; say it where
they land.

Every README carries:

- a title heading, the module comment's first paragraph;
- the problem statement or the written description, per category;
- `cargo run --manifest-path examples/<name>/Cargo.toml`;
- a link back to the root `README.md` and to the relevant user guide
  (`docs/src/device-user-guide.md` or `docs/src/cluster-user-guide.md`).

**Relative links are correct here** and the RFC 036 prohibition does not apply: these
READMEs are not inside a published crate, and they are read on GitHub where relative paths
resolve. `link-audit` will check them.

## 2. `examples/README.md` (RFC 038 §2.2)

One table — every example, its problem in a clause, its category, and its path. This is what
a visitor browsing `examples/` lands on.

## 3. The gate (RFC 038 §2.3)

All of this goes in **`doc-currency`**, which already owns the README landing assertions.
**No new gate and no new registration**: `cargo xtask check` stays at twenty.

A `const` registry names the category of each example — the six names of §0.1 with their
category — and the assertions are:

1. **Registry symmetry.** Every directory under `examples/` appears in the registry exactly
   once, and every registry name is a directory that exists.
2. **A README everywhere.** Every example directory has a `README.md`.
3. **Extraction, worked problems only.** For each worked problem, the README's problem
   paragraph equals the second paragraph extracted from its `src/main.rs` by §0.1's rule.
   Report the example and both texts on a mismatch.
4. **Table symmetry, both directions, both tables.** Every example directory has a row in
   the root `README.md` table **and** in `examples/README.md`; every row in either table
   names a directory that exists.

Fail closed on all four, with the failing example named, in the style
`published-metadata`'s findings use.

Put the extraction and the comparison in pure functions with unit tests, as
`published_metadata.rs` does — including a test for a `//!` block whose second paragraph is
absent, which must be a finding rather than a panic.

## 4. Non-scope

- **No change to any example's code or printed output.** No `src/` edit at all under this
  RFC — structural extraction needs none, and the `examples` gate's captured-output checks
  must keep passing unchanged.
- No new gate; no change to the `examples` gate, to `published-metadata`, or to
  `bench-baseline`.
- No move of the root README's table.
- **Watch the root README's length.** It is **101 lines** against `doc-currency`'s 100-200
  band. Adding the `cluster-counted-work` row makes it 102. Do not remove anything from it;
  if some change would take it below 100, stop and report rather than padding it.
- No new dependency, no public API change, no `#[allow(…)]`. Do not tag; do not publish.

## 5. Required evidence

```text
cargo fmt --all -- --check          # run last, immediately before committing
cargo clippy --workspace --all-features --all-targets -- -D warnings
cargo test --workspace --all-features
cargo +1.85 check --workspace --all-features
cargo xtask check                   # 20 gates
cargo xtask examples                # unchanged; captured output must still match
cargo xtask release-gate --intended-tag 0.22.3      # must PASS
```

State in the request:

1. **Examples affected** — expected `none` in the sense the convention means: no example's
   code or printed record changes. Say that explicitly, since six new files land under
   `examples/`.
2. **The gate failing on the current tree before the row is added**, with the exact
   finding naming `cluster-counted-work`. This is the one chance to prove the gate against
   a real omission rather than a synthetic one; do not add the row first.
3. A break-and-restore record for **each** of the four assertions in §3.
4. For assertion 3, the extracted paragraph and the README paragraph for one worked problem,
   so the comparison can be checked by eye.
5. The root README's line count after your change.
6. Anything in this handoff that does not match the tree. Six values in the architect's last
   two handoffs were wrong and you found all six; the same standard applies.
