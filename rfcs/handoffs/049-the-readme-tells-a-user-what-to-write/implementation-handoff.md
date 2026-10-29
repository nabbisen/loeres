# RFC 049 implementation handoff — the README tells a user what to write

**From:** architect
**To:** implementer
**Date:** 2026-10-30
**Governing text:** `rfcs/accepted/049-the-readme-tells-a-user-what-to-write.md`, accepted by the
owner on 2026-10-30, as amended in place the same day by its own §0.1 (Amendment 1), which
corrects the measurement and adds no provision.
**Base:** `main` at the revision that accepted RFC 049.

**Scope:** `README.md`, `docs/src/getting-started.md`, `docs/src/documentation-convention.md`,
and one new assertion in `xtask/src/checks/doc_currency.rs`. **No new gate, no library change, no
version change, no tag.**

## 0. Read this first

Three things decide whether this slice is correct.

**0.1 Four lines are deliberately not yours.** `docs/specs/loeres-external-design-v1.md:238,239,252,253`
carry the identical `version = "0.x"` wildcard in the `toml` fences of §1.4 **Public Import
Model**. They are measured, recorded in RFC 049 §0.1 and §2.1, and **deferred to the owner**, who
is choosing between three treatments. **Do not touch those four lines, and do not extend the new
assertion to that file.** If you believe the slice is wrong without them, say so in the review
request — that is a live question, not a settled one.

**0.2 Two existing assertions can be broken by S2, silently.** `check_readme_landing` in
`xtask/src/checks/doc_currency.rs` holds two properties of `README.md` that S2's reordering sits
on top of:

- `README_LINE_RANGE` is `(100, 200)` and `README.md` is **102 lines** today. The floor is two
  lines away. S2 and S3 add lines, so this should move the wrong way only if you delete more than
  you add — check the count, do not assume it.
- `quick_start_rust_block` takes the **first** ` ```rust ` block after the literal
  `\n## Quick Start\n` and compares it byte-for-byte with the region marked in
  `examples/device-mpc-step/src/main.rs`. **Invariant: after S2, the `## Quick Start` heading must
  still be followed — eventually, and before any other ` ```rust ` block — by that same block,
  byte-identical.** Reordering within the section is fine; moving the Rust block above the heading,
  or introducing another Rust block between the heading and it, is not. The ` ```toml ` and
  ` ```sh ` fences are invisible to that function.

**0.3 One instruction of mine was unsatisfiable last time.** RFC 048's handoff told you to place a
block before a document's first `##` heading in a file that has none (architect review 115 F2).
I checked this slice's placements against the tree: `docs/src/getting-started.md` has a lead
paragraph at `:3-6` and `## 1. Install` at `:8`, so S3's placement exists. Everything in §1-§4
below was read in the tree at `3386761` before being written. Report anything that still does not
match rather than working around it.

## 1. S1 — pin the README's four sites to the last released minor

`README.md:53-61` is the only ` ```toml ` fence in the file. Inside it, four lines:

| line | crate | now | after |
| --- | --- | --- | --- |
| 55 | `loeres-cluster` | `version = "0.x"` | `version = "0.24"` |
| 56 | `loeres-backend-std` | `version = "0.x"` | `version = "0.24"` |
| 59 | `loeres-device` | `version = "0.x"` | `version = "0.24"` |
| 60 | `loeres-backend-static` | `version = "0.x"` | `version = "0.24"` |

Change the requirement only. **The column alignment in that fence is deliberate** — the four crate
names are padded so the `=` signs line up; `"0.24"` is the same width as `"0.x"` plus one
character, so re-pad nothing and the alignment survives unchanged. Leave `features`,
`default-features` and the two comment lines byte-identical.

**Then add one sentence** under the fence saying why the requirement names a minor: that at `0.y`
the minor is the compatibility unit, so this names one deliberately rather than accepting the
next. Write it in the README's register, in one sentence, and do not explain semver. Do not state
which release is current and do not name a date — `docs/src/specifications.md` is the one home for
that, and the README links to it at `:96`.

`"0.24"` is not a guess: it is `MAJOR.MINOR` of the apex block's **`Last reconciled repository
release`**, which reads `0.24.0` in all seven apex documents today. S4 is the assertion that it
stays that.

## 2. S2 — the user's instruction goes first

**Target shape of `## Quick Start`,** in order:

1. The one-line lead-in, rewritten for a user rather than a builder.
2. The *"Choose the crates by where the solve runs"* table, now at `README.md:44-49`.
3. The dependency fence, now at `:51-61`, with S1 applied and its new sentence.
4. The device-side control step: the prose line now at `:20`, the ` ```rust ` block now at
   `:22-40`, and the closing line now at `:42` — **moved as a unit, byte-identical.** The Rust
   block is the one `doc-currency` compares (§0.2).

**The three contributor commands** now at `README.md:12-18` — the lead-in sentence *"Build and
check from source…"* and the ` ```sh ` fence — move **below** the Quick Start section, under a new
`##` heading that says what they are. `## Building from source` is a reasonable name; pick it or a
better one. Keep all three commands and the `rust-toolchain.toml` sentence; they are correct and
they stay in the README.

**Do not change** the opening bullets at `:1-8`, the `## Reading a result` section at `:63-68`, the
`## Examples` table at `:70-81`, `## Design Notes`, `## More Detail` or `## License`, beyond the one
line S3 changes at `:92`.

### 2.1 S2 makes one tracked document false — fix it in the same slice

`docs/src/documentation-convention.md` §2 is the convention that governs `README.md`, and its
3-30-3 table says the Quick Start part is *"One build command; one minimal working example"*, with
the *Setup* row of **Where documentation belongs** keeping *"One build command"* in `README.md`.
After S2 the Quick Start holds **no** build command.

Leaving that is the defect RFC 048 just removed and the one RFC 047 property **G** forbids: a
current-facing document stating something the tree contradicts. So, in the same slice:

- the Quick Start row of the 3-30-3 table becomes the dependency requirement and the one minimal
  working example;
- the build command is where the convention puts it: name the `## Building from source` section as
  its home, in the *Setup* row or a row of its own;
- change **no other row**, and keep the *"No release sentences"* rule on the first screen exactly
  as it is.

**This is my call, not RFC 049's text.** §3 S2 does not mention this file. I judge it a required
consequence rather than a widening, because the slice must not leave a governing document
describing a README that no longer exists. If you disagree, implement S1-S4, leave this, and say
so in the review request.

## 3. S3 — the book chapter says what it is

### 3.1 `docs/src/getting-started.md`

Insert **one paragraph** between the lead paragraph that ends at `:6` and `## 1. Install` at `:8`.
It says: this chapter is a from-source tour, and a reader who wants to use the library in their own
project should take the README's dependency snippet instead. Link the README the way this chapter
links its two sibling guides at `:5-6`.

**Lines `:3-6` and everything from `## 1. Install` onward are untouched.** §2-§6 of that chapter
are explicitly out of scope (RFC 049 §3 S3, §4).

### 3.2 `README.md:92`

Currently: *"Start here: Getting Started — install, solve one problem, read the result."*

The word *install* is the whole defect: a library user reads it as *add the dependency*, and the
chapter means *clone the repository*. Rewrite the trailing description so it says what the chapter
does — a from-source tour ending in a solved problem — and keep the link target
`docs/src/getting-started.md` exactly as it is. Keep the bullet to one line; the surrounding
bullets at `:93-98` are untouched.

## 4. S4 — the assertion

**Home:** `xtask/src/checks/doc_currency.rs`. **Not** `published_metadata.rs` — the root
`README.md` is deliberately exempt from that file's link rule at `published_metadata.rs:66-70`
because it ships with no crate, and RFC 049 §3 S4 and §4 both forbid changing it.

### 4.1 The value, and where it comes from

The expected requirement is `MAJOR.MINOR` of the apex block's `Last reconciled repository
release` — `0.24` while that field reads `0.24.0`. **Do not re-parse the apex block.**
`parse_ordinary_apex_currency` already stores it: `ApexCurrency.last_reconciled`, raw and without
a `v` prefix, while `ApexCurrency.release` carries the `v` prefix.

`check_apex_currency` parses it and drops it. Change its signature to return the value it already
holds — the one the cross-document identity check uses as its base:

```rust
fn check_apex_currency(expected_release: Option<&str>, errors: &mut Vec<String>) -> Option<String>
```

returning `found.first().map(|(_, currency)| currency.last_reconciled.clone())`, and thread it
through `validate_repository`:

```rust
let last_reconciled = check_apex_currency(expected_release.as_deref(), &mut errors);
...
check_readme_requirements(last_reconciled.as_deref(), &mut errors);
```

**When apex parsing has already failed**, `found` is empty and the value is `None`. In that case
clause 1 is **skipped** — the apex failure is already a reported error and a second, derived
complaint about the README would be noise — and **clause 2 still runs**, because it needs no
version. Say that in a comment where the `None` arm is handled.

This is the only change to an existing function, and it adds no parsing. RFC 049 §3 S4's *"no new
gate"* means no new entry in the gate list; a new function inside `doc-currency` is the point.

### 4.2 The predicate

A **requirement site** is a line inside a ` ```toml ` fence that contains `version` followed by
`=` and a double-quoted string. A fence opens on a line whose trimmed text is exactly
` ```toml ` and closes on the next line whose trimmed text is exactly ` ``` `.

**Clause 1 — the root README names the released minor.** For every requirement site in
`README.md`: the requirement must equal `MAJOR.MINOR` of the apex `Last reconciled repository
release`. `0.24` passes; `0.24.0`, `0.25`, `0.x`, `>=0.24` all fail.

**Clause 2 — no README advertises a wildcard.** For every requirement site in `README.md` **and in
each `crates/*/README.md`** — the five are `crates/loeres`, `crates/loeres-backend-static`,
`crates/loeres-backend-std`, `crates/loeres-cluster`, `crates/loeres-device` — the requirement must
not be a wildcard: it contains no `*`, and no dot-separated component of it is `x` or `X`. This is
`deny.toml:49`'s own rule, applied to what we tell users rather than only to what we compile.

**Clause 3 — the message names the crate and the site.** `README VERSION:` prefix, matching the
file's existing `README LENGTH:` and `README EXAMPLE:` forms, then the path and the **1-based line
number**, then the crate, then what was found and what was expected. A message that names only the
file makes the reader search a fence; name the line.

**Fail closed on a shape you do not understand.** A requirement site's crate is the identifier
before the line's first `=`. If that identifier is `version` itself — the
` [dependencies.<crate>] ` form, which no README uses today — **emit a finding saying the site's
shape is not one this assertion reads**, and do not skip the line. Silent skipping is how an
assertion becomes vacuous.

**Scan nothing under `rfcs/`.** RFC 049 §2.1 quotes the defective fence verbatim, and architect
review 116 §1 and this handoff quote the lines too. A predicate that scanned RFCs would fail on
its own governing text.

### 4.3 Shape it so it can be tested without a filesystem

Follow `readme_length_findings` and `readme_example_findings`: a pure function over strings,
returning `Vec<String>`, with the I/O in the caller. For example

```rust
fn readme_requirement_findings(path: &str, readme: &str, expected_minor: Option<&str>) -> Vec<String>
```

so `#[cfg(test)] mod tests` can exercise every branch on literals.

**Required unit tests**, because two of these branches are satisfied vacuously by today's tree and
an unexercised branch is the defect architect review 113 F5 found in RFC 046:

1. the four real README lines with `"0.24"` and `expected_minor = Some("0.24")` — no findings;
2. one site at `"0.x"` — a wildcard finding naming the crate and the line;
3. one site at `"0.23"` with `expected_minor = Some("0.24")` — a clause 1 finding; **and the same
   input with `Some("0.23")` — no findings**, which is what proves the function reads the value
   rather than a literal;
4. a synthetic per-crate README with a wildcard site — a clause 2 finding. **Clause 2 over
   `crates/*/README.md` matches nothing in the tree today** (no per-crate README contains
   `version =` at all), so this test is the only thing standing between that half of the predicate
   and never having run;
5. `expected_minor = None` with a wildcard site present — the wildcard finding, and no clause 1
   finding;
6. a `version = "…"` line **outside** any `toml` fence — no finding, so prose that mentions a
   requirement is not a site;
7. a site in the ` [dependencies.<crate>] ` shape — the fail-closed finding of §4.2.

### 4.4 The convention's own table

`docs/src/documentation-convention.md` §5 *"What is checked and what is guidance"* lists the
README properties `doc-currency` holds. Add one row for this assertion, in that table's register,
beside the three README rows already there.

## 5. What must not change

- **`docs/specs/loeres-external-design-v1.md`** — all four wildcard sites (§0.1). Deferred, not
  forgotten.
- **`xtask/src/checks/published_metadata.rs`**, and the root README's link exemption at `:66-70`.
- **`crates/*/README.md`** — no install snippet, no version requirement. RFC 049 §2.4 and §4: four
  more claims to hold, no gain. Clause 2 covers them so that a future one cannot be a wildcard;
  it does not ask for one.
- **The README's opening bullets**, `## Reading a result`, the `## Examples` table, `## Design
  Notes`, `## More Detail` except `:92`, `## License`.
- **`docs/src/getting-started.md` §1-§6**, and its lead paragraph.
- **`APEX_DOCS`**, the apex block, and every other function in `doc_currency.rs` beyond the one
  signature change in §4.1. If you find yourself editing `parse_ordinary_apex_currency`,
  `extract_currency_block` or `bounded_value`, stop and report it.
- **Any RFC and any file under `rfcs/`.** RFC 049's index row and its Amendment 1 are written.
- **No gate added. No version changed. No tag.**

## 6. Evidence required in the review request

1. The usual five logs, plus `release-gate --intended-tag 0.24.1` exit status checked directly
   rather than through a pipe.
2. `README.md`'s **line count before and after**, against the floor of 100 (§0.2).
3. **Proof the Quick Start Rust block still matches** the marked region byte-for-byte — by the
   gate, not by eye: quote `doc-currency`'s PASS line after S2, and state that
   `quick_start_rust_block`'s invariant (§0.2) held without the block being edited.
4. The CHANGELOG entry, quoted in full: what the README now tells a user to write, that the
   previous instruction was a wildcard the project denies in its own manifests, that it resolved
   correctly until the first `0.25.0`, and what is now asserted. **No completeness claim** — say
   what is held and stop. Mention that four sites in `docs/specs/loeres-external-design-v1.md`
   are measured and deferred, so the record does not read as though all eight were fixed.
5. **The seven unit tests of §4.3**, named, with the two negative controls (3's second case and 6)
   called out explicitly.
6. **Five targeted exercises against the committed tree**, each reverted, each with the gate's own
   message quoted:

   | # | break | expect |
   | --- | --- | --- |
   | 1 | `README.md:55` → `version = "0.25"` | clause 1, naming `README.md:55` and `loeres-cluster` |
   | 2 | `README.md:56` → `version = "0.x"` | the wildcard finding, naming `README.md:56` and `loeres-backend-std` |
   | 3 | add a `toml` fence with `loeres = { version = "0.x" }` to `crates/loeres/README.md` | the wildcard finding, naming that path and `loeres` |
   | 4 | `Last reconciled repository release: **0.24.0**` → `**0.23.0**` in **all seven** `APEX_DOCS`, identically | clause 1 on **all four** README sites, expecting `0.23` |
   | 5 | add `loeres-device = { version = "0.24" }` in a `toml` fence in `crates/loeres-device/README.md` | **PASS** — a pinned requirement in a per-crate README is legal, and clause 1 is the root README's only |

   **Exercise 4 is the one that matters** and the one not to skip: it is the only exercise that
   distinguishes a predicate keyed to the apex block from one with `"0.24"` written into it. It is
   satisfiable — `0.23.0` is strictly below the workspace `0.24.1` and `CHANGELOG.md:448` marks it
   released — and it must be applied **identically in all seven** documents, or `APEX MISMATCH`
   fires first and masks the result. Exercise 5 is a negative control: a PASS is the expected
   outcome, and a FAIL there means clause 1 leaked outside `README.md`.
7. **The four deferred sites, reported untouched**: quote
   `git grep -n '= "0\.x"' -- '*.md'` at the end and confirm the only current-facing matches left
   are those four in `docs/specs/loeres-external-design-v1.md`.
8. Anything in §1-§4 that does not match the tree, **reported rather than fixed** — including §2.1,
   which is my judgement and not the RFC's text.
9. If `supply-chain` fails on an advisory unrelated to this slice, report it; change no dependency.

## 7. Push

Per `docs/src/development.md`'s *"An accepted slice is pushed; a tag is not the implementer's"*:
once the review request is accepted, push without asking again. No tag.
