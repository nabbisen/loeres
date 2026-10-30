# RFC 051 implementation handoff — one import model, one requirement

**From:** architect
**To:** implementer
**Date:** 2026-10-30
**Governing text:** `rfcs/accepted/051-one-import-model-one-requirement.md`, accepted by the owner
on 2026-10-30 **as written**.
**Base:** `main` at the revision that accepted RFC 051.

**Scope:** `docs/specs/loeres-external-design-v1.md` §1.4 only, one new assertion and one constant
unification in `xtask/src/checks/doc_currency.rs`, and `CHANGELOG.md`. **No new gate, no library
change, no version change, no tag.**

**Order:** take this slice **before** RFC 052. Both touch `doc_currency.rs`; this one unifies a
constant that RFC 052's slice should find already unified. RFC 050 is disjoint from both.

## 0. The open question is decided: treatment B

RFC 051 §7 put three treatments to the owner and recommended **B**, naming **C** as equivalent to
withdrawing the RFC. The owner accepted the RFC as written, which decides §7 in favour of its
recommendation.

**Treatment B: §1.4's fences carry no version requirement, and `README.md` is the one home.** The
assertion is the *absence* of a requirement site in that file, so it needs no value and can never
itself go stale — RFC 048's move, applied once more.

**If you think this reading is wrong, stop and say so before editing anything.** It is an
inference from "accepted as written" plus a recommendation, not a sentence the owner wrote, and it
is cheap to correct now and expensive to correct after a normative spec has changed. Treatment A —
pinning the fences to `"0.24"` with the predicate extended — is the alternative, and RFC 051 §5
specifies both.

## 1. What is there now, measured

`docs/specs/loeres-external-design-v1.md` §1.4 **Public Import Model**. The whole file contains
**exactly two** ` ```toml ` fences and **exactly four** `version =` lines, and all four are the
defect:

```
232  The intended downstream import model is:
234  #### Cluster user
236  ```toml
237  [dependencies]
238  loeres-cluster = { version = "0.x", features = ["parallel-rayon"] }
239  loeres-backend-std = { version = "0.x", features = ["dense"] }
240  ```
242  Cluster users normally import:
244-246  (three bullets)
248  #### Device user
250  ```toml
251  [dependencies]
252  loeres-device = { version = "0.x", default-features = false }
253  loeres-backend-static = { version = "0.x", default-features = false, features = ["owned-arrays"] }
254  ```
256  Device users normally import:
258-260  (three bullets)
262  Device users must not need to import `loeres-cluster` or `loeres-backend-std` …
```

So §2's assertion has **no collateral**: there is no other `toml` fence in the file whose legitimate
contents it could reject. Verify that yourself before editing — if a third fence has appeared,
report it.

## 2. S1 — the fences stop specifying a denied form

The four lines become, keeping every crate name and every feature byte-identical and changing
nothing else in either fence:

```toml
[dependencies]
loeres-cluster = { features = ["parallel-rayon"] }      # version: see README.md
loeres-backend-std = { features = ["dense"] }           # version: see README.md
```

```toml
[dependencies]
loeres-device = { default-features = false }                                        # version: see README.md
loeres-backend-static = { default-features = false, features = ["owned-arrays"] }   # version: see README.md
```

**The trailing comment is my call within B, not RFC 051's text.** Without it the fence is a
manifest that cargo rejects for a reason the reader has to work out; with it, the one thing they
must add is named at the site. Align the comments within each fence; the two fences need not align
with each other. If you would write the comment differently — shorter, or once per fence rather
than once per line — do, and say what you chose.

**And one sentence after `:232`**, before the `#### Cluster user` heading: this model fixes the
crates and the features, and the version requirement each user writes is in the repository's
`README.md`, where `doc-currency` holds it against the last reconciled release. Name `README.md`
in backticks, **not as a link** — a path in this file is read in the repository, and a link adds a
target to maintain for no gain.

**Untouched:** the `[dependencies]` headers, the bullet lists at `:242-246` and `:256-260`, the
`#### Cluster user` and `#### Device user` headings, `:262`, §1.3's diagram, and the file's apex
currency block.

## 3. S2 — the assertion

### 3.1 Shape

A new function in `xtask/src/checks/doc_currency.rs`, called from `validate_repository` alongside
`check_readme_requirements`. **No new gate**, and **no change to RFC 049's existing clauses** —
clause 1 stays the root README's, clause 2 stays the six READMEs'.

```rust
const IMPORT_MODEL_PATH: &str = "docs/specs/loeres-external-design-v1.md";

fn check_import_model_requirements(errors: &mut Vec<String>)
fn import_model_findings(path: &str, source: &str) -> Vec<String>   // pure, for the tests
```

**Reuse `requirement_sites`** — the scanner RFC 049 added, which returns
`(1-based line, identifier before the line's first '=', quoted value)` for every `version = "…"`
line inside a ` ```toml ` fence. Do not write a second scanner and do not change that one.

The predicate is one sentence: **`requirement_sites` over that file must be empty.** Every site
found is a finding, naming the file, the 1-based line, and the identifier — with the reason, since
this message is the only place a future author will learn why the absence is deliberate:

```
  IMPORT MODEL: docs/specs/loeres-external-design-v1.md:238 `loeres-cluster` names a version
  requirement; the import model fixes crates and features only, and the requirement's one home
  is README.md (RFC 051)
```

This subsumes the wildcard rule for this file rather than restating it: with no site permitted,
a wildcard site cannot exist.

### 3.2 Tests

In `#[cfg(test)] mod tests`, on literals, because the committed tree satisfies the assertion
vacuously — there will be nothing in the real file for it to find, which is exactly how RFC 046's
F5 defect looked:

1. the two fences as S1 leaves them, comments included — **no findings**;
2. a fence containing `loeres-cluster = { version = "0.24", features = [...] }` — **one finding**,
   naming line and crate. A *correct* version is still a finding here: that is the point of B, and
   a test that only rejects `"0.x"` would not prove it;
3. a fence containing `version = "0.x"` — one finding, so the historical defect is covered;
4. prose outside any fence containing the words `version = "0.24"` — no finding;
5. the S1 comment text alone on a dependency line (`{ features = [...] } # version: see
   README.md`) — **no finding**, which is the one false positive this slice could plausibly ship.

Test 5 matters: `requirement_sites` finds a site by the literal `version` followed by `=` and a
quoted string, so the comment's bare `version:` is not a site — confirm that by test rather than
by reading the scanner.

## 4. S3 — the constant unification (RFC 051 §8)

`doc_currency.rs` holds the five `crates/*/README.md` paths twice:

- `CRATE_README_PATHS`, added by RFC 049 S4;
- `check_stale_ledger`'s local `whole_file_targets` array, which is `README_PATH`, those same five,
  then `docs/src/threat-model.md` and `docs/src/specifications.md`.

Make `CRATE_README_PATHS` the single owner and have `check_stale_ledger` build its list from
`README_PATH`, that constant, and its two remaining paths. **Nothing else in
`check_stale_ledger` changes** — not the APEX loop above it, not `stale_phrases_in`, not the
forty-line window.

Two things to know:

- The two lists order `loeres-backend-static` and `loeres-backend-std` differently. Unifying
  changes the **order findings are printed in** when several fire at once, and nothing else. That
  is acceptable; say in the review request that you saw it.
- `published_metadata.rs:111`'s `PUBLISHED_CRATES` is a third copy and is **out of scope**
  (RFC 051 §8, §9). Different gate, different subject. Do not touch it.

## 5. S4 — the record

`CHANGELOG.md` gains one paragraph: which treatment was chosen and why, that §1.4 now specifies
crates and features while the requirement's one home is the README, and that the eight sites
RFC 049 Amendment 1 measured are now **all** held.

**That last clause is the first completeness claim this pair of RFCs may make**, because it is the
first point at which it is true. State it exactly that narrowly — eight sites, held — and do not
extend it into a claim about every version a user might read anywhere.

No `docs/src/documentation-convention.md` row: under B the property is the absence of a value in a
specification, which belongs in the RFC, not in a convention about how documents are written
(RFC 051 §6).

## 6. What must not change

- **`README.md`, every `crates/*/README.md`, and RFC 049's clauses 1-3.**
- **`docs/specs/loeres-external-design-v1.md` outside §1.4's two fences and the one sentence at
  `:232`** — not the apex block, not §1.3, not the bullet lists.
- **`docs/src/specifications.md`**, including its `Q32` migration advice at `:46-50`, which names
  `"0.24"` deliberately and is a statement about where a fix landed, not a currency claim
  (RFC 051 §3). RFC 052 covers that section.
- **`requirement_sites`, `requirement_value`, `is_wildcard_requirement`, `major_minor`,
  `check_readme_requirements`** — reused, not edited.
- **`PUBLISHED_CRATES`** (§4).
- **No new gate. No version changed. No tag.**

## 7. Evidence required in the review request

1. The usual five logs, plus `release-gate --intended-tag 0.24.1` exit status checked directly
   rather than through a pipe.
2. **§1's measurement, re-run by you**: two `toml` fences and four `version =` lines in that file
   before the slice, zero after.
3. The CHANGELOG entry, quoted in full, with §5's completeness clause stated at exactly that width.
4. **The five unit tests of §3.2**, named, with tests 2 and 5 called out — the first proves B is
   asserted rather than the wildcard, the second proves S1's own comment is not a false positive.
5. **Four exercises against the committed tree**, each reverted, each with the gate's own message
   quoted:

   | # | break | expect |
   | --- | --- | --- |
   | 1 | restore `version = "0.x"` on `docs/specs/loeres-external-design-v1.md:238` | `IMPORT MODEL`, naming that line and `loeres-cluster` |
   | 2 | add `version = "0.24"` to the same line instead | `IMPORT MODEL` again — a correct version is still a finding |
   | 3 | add a wildcard requirement to `crates/loeres-cluster/README.md` | `README VERSION` from RFC 049's clause 2, **still firing after the constant unification** |
   | 4 | reintroduce a stale ledger phrase in one `crates/*/README.md` | the stale-phrase finding, **still firing after the unification** |

   Exercises 3 and 4 are the ones not to skip: S3 rewires the list both of those checks walk, and
   they are the only proof the rewiring kept both of them reaching all five crate READMEs.
6. Anything in §1-§5 that does not match the tree, **reported rather than fixed** — including §2's
   comment wording, which is mine and not the RFC's.
7. If `supply-chain` fails on an advisory unrelated to this slice, report it; change no dependency.

## 8. Push

Per `docs/src/development.md`'s *"An accepted slice is pushed; a tag is not the implementer's"*:
once the review request is accepted, push without asking again. No tag.
