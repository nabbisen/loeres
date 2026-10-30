# RFC 052 corrections handoff — clause 5 does not hold what it says

**From:** architect
**To:** implementer
**Date:** 2026-10-30
**Governing text:** `rfcs/accepted/052-a-release-table-that-cannot-omit-a-release.md` as amended by
its own §0.1 — **unchanged**. This is an implementation defect, not a design change: the amended
RFC says the right thing and the code does not match it. Findings recorded in architect review 119
§2.
**Base:** `main` at the revision that accepted the RFC 052 implementation.

**Scope:** `xtask/src/checks/doc_currency.rs` only, plus `CHANGELOG.md`. **No document change, no
RFC change, no new gate, no version change, no tag.** Clauses 1-4 are accepted and verified — do
not touch them.

## 0. What is wrong, and what is not

**Four of the five clauses are correct.** I re-ran the release-cut simulation myself and it
produces zero release-table findings, so review 118 F2 is genuinely closed. The one cell, the
blockquote sentence, the convention row and the CHANGELOG entry all stand. **Nothing you wrote is
being reverted.**

**Clause 5's derivation is unsound.** Everything below is four changes inside the functions that
feed it.

## 1. F1 — `newest_published_version` admits unpublished releases

```rust
line.contains("published to crates.io")
```

is true for `"not published to crates.io"`. The two clauses this project writes are

```
released (tagged YYYY-MM-DD, distributed YYYY-MM-DD); published to crates.io
released (tagged YYYY-MM-DD, distributed YYYY-MM-DD); not published to crates.io
```

so the filter selects the newest *released* version carrying either, not the newest published one.

**Measured.** With `CHANGELOG.md`'s `0.24.0` status line set to `; not published to crates.io`,
`cargo run -q -p xtask -- doc-currency` reports **PASS with no clause-5 finding**. The correct
result is one finding, demanding the paragraph name `0.23.0`.

It is correct today only because the newest release happens to be published, which makes the right
and wrong answers the same string. **That is review 118 F3 reintroduced** — the defect the
amendment exists to prevent — and it is the shape of RFC 049's own subject: right until the
circumstance it was written for actually occurs.

### 1.1 The fix

**Read the clause, do not search the line.** Own both literals in one place and require a released
version's status line to carry exactly one of them:

```rust
const CHANGELOG_PUBLISHED: &str = "published to crates.io";
const CHANGELOG_NOT_PUBLISHED: &str = "not published to crates.io";
```

For a version whose status is `released`, take the clause after the final `"; "` of its status
line, trim it, and compare it for **equality** with each. Then:

- equal to `CHANGELOG_PUBLISHED` → published;
- equal to `CHANGELOG_NOT_PUBLISHED` → not published;
- **neither → a finding**, naming the version and what its line says. Fail closed: a third phrasing
  must stop the gate, not be silently treated as unpublished. Every one of the eleven released
  entries carries one of the two today, so this branch is unreachable on the committed tree and
  needs a unit test, not an exercise.

`newest_published_version` then filters on the *published* verdict, and only over versions whose
status is `released`.

A one-token patch — testing `"; published to crates.io"` — would also work and I do not want it:
it leaves the negation one typo away from matching again, and the next reader has to notice the
leading semicolon to see why it is correct.

## 2. F2 — the same construction, one clause to the left

`changelog_distributed_date` reads `line.split_once("distributed ")`. No status line says
`not distributed`, so **there is no live defect** — but it is the identical shape in the same
function family, and it will be wrong the first time a status line distinguishes distributed from
not.

Parse the clause structure instead: inside the parenthesised group, split on `", "` and read the
`tagged ` and `distributed ` fields by their own prefixes, each validated with `valid_date` as it
is now. A field present but unparseable stays a finding, as it is today.

Do not add a tagged-date assertion. Clause 3 is about the distributed date and §0.1 settled that.

## 3. F3 — a renamed or missing table header reports twelve wrong causes

`release_table_rows` returns no rows and no finding when the header and separator are not found.
The assertion does not fail open — clause 1 reports every released version as unrowed — but the
report is wrong about why. Measured, with `Shipped` renamed to `Shipped date`: **twelve findings**,
eleven of them naming rows that are sitting in the file.

Emit **one** finding when the header is not found — the table's header line was not found in that
file, quoting the header it looked for — and **return early from `release_table_findings`** so the
other clauses do not pile derived complaints on top of it. The same rule the function already
applies to `last_reconciled` being `None`.

This is the owner's standing requirement that a message not mislead. Eleven false causes for one
real one fails it.

## 4. F4 — `implemented_scope` is derived twice and reports twice

`check_apex_currency` calls `implemented_scope(errors)` and `check_release_table` now calls it
again, both pushing into the same `errors`. Measured, with one badly-named file under `rfcs/done/`:
`RFC SCOPE: … is not a valid \`NNN-slug.md\` name` printed **twice** for one cause.

Derive it once in `validate_repository` and pass it to both, the way `last_reconciled` is already
threaded. `check_apex_currency` takes it as a parameter rather than computing it.

**This one is half mine**: the handoff said to take the scope from `implemented_scope()` rather
than re-reading a document, meaning *do not re-parse the apex block*, and it reads as *call it
again*.

## 5. Tests

Add to `#[cfg(test)] mod tests`. **Keep all twelve existing release-table tests passing
unchanged** — if one needs editing, that is a finding, because clauses 1-4 are not changing.

1. **the newest released version marked `not published to crates.io`** — clause 5 demands the
   paragraph name the newest version that *is* published. This is F1, and it is the test the slice
   was missing: it perturbs the **input the derivation reads**, not the paragraph;
2. every released version marked `not published` — the `CHANGELOG.md names no version published to
   crates.io` finding;
3. a released version whose status line's final clause is neither literal — the fail-closed finding
   of §1.1, naming the version;
4. a status line whose `distributed` field is absent — the existing clause-3 finding, unchanged
   (pins F2's refactor against regression);
5. a status line whose `distributed` field is present but not a date — the same finding;
6. **a source whose table header is absent** — exactly **one** finding, about the header, and
   **no** per-version findings (F3);
7. `implemented_scope` passed once and used by both callers — no test needed for F4 beyond the
   existing suite staying green; say in the request that you checked the duplicate is gone by the
   §6 exercise.

## 6. Exercises against the committed tree

Each reverted, each with the gate's own message quoted:

| # | break | expect |
| --- | --- | --- |
| 1 | `CHANGELOG.md`'s `0.24.0` status line → `; not published to crates.io` | clause 5, naming `0.23.0` as the version the paragraph must name — **this is the defect, and before the fix the gate says PASS here** |
| 2 | rename the table's `Shipped` header to `Shipped date` | **exactly one** finding, about the header |
| 3 | one badly-named file under `rfcs/done/` | `RFC SCOPE: …` **once**, not twice |
| 4 | the release-cut window, as in the accepted slice's §7.1 | **zero** release-table findings, unchanged — clauses 1-4 must survive this slice intact |

Exercise 1 is the point of the slice. Exercise 4 is the regression guard: it already passes and
must still pass.

## 7. What must not change

- **Clauses 1-4's predicates and messages**, and all twelve existing tests.
- **Every document.** The `0.20.2` cell, the blockquote sentence, the convention row, the table,
  the crates.io paragraph, both CHANGELOG entries from the accepted slices.
- **RFC 052 and its Amendment 1.** The design is right; this is the code catching up.
- **`parse_changelog_release_status`'s contract** — it returns the bare status token and that stays
  its job. The new literals and the clause reading sit beside it, not inside it.
- **No new gate. No version changed. No tag.**

## 8. Evidence required in the review request

1. The usual five logs, plus `release-gate --intended-tag 0.24.1` exit status checked directly.
2. The CHANGELOG entry, quoted in full. It records that clause 5 was unsound as first implemented —
   a substring test that matched the negation of what it meant — and that no document was wrong as
   a result. **No completeness claim.**
3. **The six new tests of §5**, named, with test 1 and test 6 called out.
4. **The four exercises of §6**, with exercise 1's before-and-after: the gate's PASS at the parent
   revision and its finding after the fix, both quoted.
5. Confirmation that all twelve existing release-table tests pass **unedited**.
6. Anything in §1-§5 that does not match the tree, **reported rather than fixed**.
7. If `supply-chain` fails on an advisory unrelated to this slice, report it; change no dependency.

## 9. Push

Per `docs/src/development.md`'s *"An accepted slice is pushed; a tag is not the implementer's"*:
once the review request is accepted, push without asking again. No tag.
