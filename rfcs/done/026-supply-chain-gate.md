# RFC 026 - Supply-Chain Gate

**Status.** Implemented (v0.21.0)
**Design approval.** Architect review (author-performed adversarial pass, tier `architect`); project owner authorized the `accepted/` transition on 2026-09-12.
**Tracks.** Consolidation step C.1; requirements SEC-S-003/REL-005 posture; the "N/A — not configured" entry carried since v0.14.
**Touches.** `deny.toml` (new), `xtask` (one gate), `.github/workflows/ci.yml`, `docs/src/verification.md` / `development.md`.

---

### Extended Metadata

* **Rust Edition Compliance:** Rust 2024; MSRV 1.85.
* **Target Environment:** Host-side verification; CI.
* **Proposed release:** `0.21.0` consolidation release — **before** any capability RFC adds a dependency.

## 1. Summary

The project has four external dependencies (`rayon`, `serde`, `tokio`, `toml`)
and no mechanism that fails when one acquires an advisory, changes license, or
pulls an unexpected transitive crate. The handoff bundle has said "cargo
audit/deny: N/A — not configured yet (advisory follow-up)" since v0.14. RFC 027
will add the first new dependency in a year; the gate lands first.

## 2. Affected crates

None. `deny.toml` at the root; `xtask` gains `supply-chain`; CI runs it.

## 3–10. Public API, `std`/`alloc`, determinism, scalability, error, feature, semver impact

None. Dependency impact: `cargo-deny` is a **tool**, installed in CI and by
developers; it is not a workspace dependency.

## 11. Design

`deny.toml` with four checks: **advisories** (RustSec database; deny
vulnerabilities and unmaintained), **licenses** (allow-list: Apache-2.0, MIT,
BSD-2/3, ISC, Unicode-3.0, Zlib; everything else denied), **bans**
(deny multiple versions where avoidable; explicit skip list with reason;
deny wildcard requirements), **sources** (crates.io only).

`cargo xtask supply-chain` runs `cargo deny check` and is **enforced** in the
developer aggregate and `release-gate`. If `cargo-deny` is not installed the gate
reports `unavailable` and **fails** — unlike RFC 022's corpus, a missing tool is
an environment defect, not a legitimate absence. CI installs a pinned version.

Edge crates must show an empty external dependency set under `deny check bans`
for `--no-default-features`; the gate asserts it, doubling as a second zero-bleed
witness.

## 12. Rejected alternatives

| Alternative | Reason rejected |
|---|---|
| `cargo audit` only | Advisories only; no license or ban policy |
| Advisory (non-failing) gate | The project has one advisory-class result already (size-budget); a security gate that cannot fail is theatre |
| Vendor dependencies | Solves a different problem at large cost |

## 13. Verification gates

Gate fails on a planted advisory fixture and on a disallowed license in a test
manifest; passes on the current tree; missing tool fails.

## 14. Sprint plan

S0 freeze → S1 `deny.toml` + gate → S2 CI pin → S3 docs → S6 closeout with `0.21.0`.

## 14a. Supersession note (implementation review 044)

RFC 020 §12.4 requires the threat model to state that "supply-chain checks
[are] not yet enforced." This RFC makes that clause false. RFC 020 is shipped and
is **not** edited (RFC 025's rule: `done/` RFCs are superseded, never amended);
this section records that RFC 026 supersedes that single clause of RFC 020
§12.4 and nothing else in it. The `0.21.0` changelog record carries a security
note: RUSTSEC-2026-0204 (`crossbeam-epoch 0.9.18`, reached via `parallel-rayon`)
was present and unseen for as long as the gate was "not configured", and was
remediated by the advisory's own fix (`0.9.21`, in-range) on 2026-09-12.

## 15. Exit criteria

Enforced in `check` and `release-gate`; CI green; "N/A — not configured" removed
from every document that carries it.
