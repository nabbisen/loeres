# RFC 025 - In-Place Amendment of Accepted RFCs

**Status.** Implemented (v0.21.0). Amendment 1 was made while Accepted, under RFC 000's in-place-amendment rule — this RFC's own rule, applied to itself; it is historical record, not a post-release edit.
**Design approval.** Architect review (author-performed adversarial pass, tier `architect`); project owner authorized the `accepted/` transition on 2026-09-12.
**Tracks.** Register item I-14; consolidation step C.1 of the 2026-09-12 roadmap decision.
**Touches.** `rfcs/done/000-rfc-lifecycle-policy.md` (one narrow section, following the RFC 021 precedent), `rfcs/README.md`.

---

### Extended Metadata

* **Rust Edition Compliance:** No code change.
* **Target Environment:** Repository governance.
* **Proposed release:** `0.21.0` consolidation release.

## 0. Amendment 1 — 2026-09-12 (architect review 047)

**Defect.** §13 required "a `check-rfcs` assertion that a `done/` RFC contains no
`## 0.N Amendment` heading **added after its Implemented status**." The developer
handoff I wrote dropped the final clause and asked for absence; the implementer
built absence; I reviewed it and accepted it. The assertion therefore forbids the
*presence* of an amendment heading where the rule forbids its *addition after
shipping* — so every RFC that legitimately used this RFC's own rule while
Accepted is permanently barred from ever reaching `done/`. It surfaced the first
time such a transition was attempted: the `0.21.0` finalization revision, where
RFCs 022, 023, and 024 could not move.

**Correction.** The assertion becomes **accountability, not absence**. A `done/`
RFC may carry `0.N Amendment` headings **only if its Status line names them**.
That is §11's own condition (3) made mechanical: it works from tracked bytes
alone, holds in a clean extraction, needs no dates and no git history, permits
the legitimate pre-shipping record, and catches the realistic failure — appending
an amendment to a shipped RFC and leaving the Status line untouched.

**Residual, stated rather than implied.** It does not catch an editor who amends
a shipped RFC *and* updates its Status line to match. Nothing readable from
tracked bytes can: §11 condition (4) already places that in the architect's
recorded review, and this amendment does not pretend otherwise.

**Rejected: dating amendments against the release.** Comparing each heading's
date to the `CHANGELOG.md` date of the version in the Status line is mechanically
stronger for a *future* post-ship amendment, and it was considered seriously. It
is rejected for now because an amendment dated the same day as the finalization
revision — which is every amendment in `0.21.0` — is indistinguishable either
way, so it buys nothing at the point of need; it adds a cross-file dependency and
date arithmetic to a governance rule; and the residual above survives it. It
remains available as a strengthening if a genuine post-ship amendment is ever
attempted.

## 1. Summary

RFC 000 defines states and folders; it does not say whether an Accepted RFC's
text may change before implementation completes. Twice since `0.20.2` the
project needed exactly that — RFC 024 Amendment 1 (a design defect found by
review after partial implementation) and RFC 022 Amendments 1–3 (defects found
against an accepted but unimplemented design). Each recorded its own
justification in a `§0` and each noted the rule existed nowhere. Independent
review (037) recommended codifying it. This RFC does so, narrowly.

## 2–10. Affected crates; API, dependency, `std`/`alloc`, determinism, scalability, error, feature, semver impact

None. Governance only.

## 11. Design

Add to RFC 000, beside the existing RFC 021 exception section:

> **In-place amendment of Accepted RFCs.** An RFC in `accepted/` may be
> corrected in place when all of the following hold: (1) it has not reached
> `done/`; (2) the correction is a numbered, dated `## 0.N Amendment` section
> that records the defect, the change, and the reason, with the original body
> updated in place rather than rewritten; (3) the Status line names the
> amendment and states whether implementation of the amended provisions is
> authorized; (4) the architect's review is recorded and, where the amendment
> touches a shipped constraint, monotonicity (roadmap §1.5) is argued
> explicitly. An RFC in `done/` is never amended in place — it is superseded by
> a new RFC.

Rationale: nothing outside an Accepted RFC's own file depends on its frozen text
until `done/`; a shipped contract does. The boundary is `done/`, not
implementation state, which resolves the two conditions RFC 024 §0.4 and RFC 022
§0.3 recorded separately.

## 12. Rejected alternatives

| Alternative | Reason rejected |
|---|---|
| Always supersede with a new RFC | Correct for shipped contracts; for an unimplemented one it mints a number per typo and fragments the record |
| Leave it as case-by-case `§0` notes | Two cases already diverged on the condition; a third would be invented again |
| A `finalizing/` or `amending/` folder | Broadens RFC 000 globally; RFC 021 §15 rejected the same shape |

## 13. Verification gates

`check-rfcs`, `link-audit`, `doc-currency`; a `check-rfcs` assertion that a `done/`
RFC's `0.N Amendment` headings are each named by its Status line (§0, Amendment
1). Absence is *not* asserted: an RFC amended while Accepted carries that record
into `done/` legitimately, and a `done/` RFC whose Status line does not account
for a heading it carries fails closed.

## 14. Sprint plan

S0 design freeze → S1 RFC 000 section + `check-rfcs` assertion → S6 closeout with `0.21.0`.

## 15. Exit criteria

RFC 000 carries the section; the Status-line accountability assertion exists and
is tested — including an amended RFC reaching `done/` with its Status line naming
the amendments (pass) and not naming them (fail); RFC 024 §0.4 and RFC 022 §0.3
reference it instead of describing the condition themselves.
