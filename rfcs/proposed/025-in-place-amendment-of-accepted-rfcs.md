# RFC 025 - In-Place Amendment of Accepted RFCs

**Status.** Proposed
**Tracks.** Register item I-14; consolidation step C.1 of the 2026-09-12 roadmap decision.
**Touches.** `rfcs/done/000-rfc-lifecycle-policy.md` (one narrow section, following the RFC 021 precedent), `rfcs/README.md`.

---

### Extended Metadata

* **Rust Edition Compliance:** No code change.
* **Target Environment:** Repository governance.
* **Proposed release:** `0.21.0` consolidation release.

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
RFC contains no `## 0.N Amendment` heading added after its Implemented status.

## 14. Sprint plan

S0 design freeze → S1 RFC 000 section + `check-rfcs` assertion → S6 closeout with `0.21.0`.

## 15. Exit criteria

RFC 000 carries the section; the assertion exists and is tested; RFC 024 §0.4 and
RFC 022 §0.3 reference it instead of describing the condition themselves.
