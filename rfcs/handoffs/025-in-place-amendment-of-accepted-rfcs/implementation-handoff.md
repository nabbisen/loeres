# Developer Handoff — RFC 025 in-place amendment of Accepted RFCs

**Governing RFC.** `rfcs/accepted/025-in-place-amendment-of-accepted-rfcs.md`. **Assigned to.** Implementer tier. **Release.** `0.21.0`.

## Change scope
1. Add the §11 section verbatim to `rfcs/done/000-rfc-lifecycle-policy.md`, immediately after the existing "Loeres conditional release-finalization exception" section (the RFC 021 precedent). Editing RFC 000 in place for a narrow exception **is** that precedent; it is not an amendment of a shipped design decision.
2. `check-rfcs`: assert that every `## 0.N Amendment` / `### 0.N Amendment` heading in a file under `rfcs/done/` **is named by that file's Status line** (RFC 025 §0, Amendment 1). Fail-closed. Do **not** assert absence — this handoff previously did, dropping §13's "added after its Implemented status", which barred every legitimately amended RFC from shipping. Tests: a `done/` RFC whose Status line names its amendments passes; one that does not fails; an `accepted/` file is never inspected.
3. Replace the self-described conditions in RFC 024 §0.4 and RFC 022 §0.3 with a one-sentence reference to RFC 000's new section. Do not delete the history they record.

## Non-change scope
No code outside `xtask/src/checks/check_rfcs.rs`. No other RFC text.

## Evidence
fmt, clippy `-D warnings`, tests, MSRV, `cargo xtask check`, `mdbook build docs`. Standard review request; record your tier.
