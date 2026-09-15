# RFC 028 - Cross-Environment Archive Identity Anchor

**Status.** Accepted (design frozen 2026-09-15)
**Design approval.** Architect-authored from the `0.21.0` distribution closeout (architect review 050); project owner accepted it and authorized the `accepted/` transition on 2026-09-15.
**Tracks.** Finding recorded at the `0.21.0` distribution closeout (architect review 050 §2).
**Touches.** `xtask/src/checks/release_gate/package.rs` (evidence writer), `docs/src/verification.md`, `docs/src/development.md`.

---

### Extended Metadata

* **Rust Edition Compliance:** Rust 2024; MSRV 1.85.
* **Target Environment:** Host-side release tooling and its evidence.
* **Proposed release:** next minor with a release-surface change; no runtime impact.
* **Relationship to RFC 019:** additive. RFC 019 §11.4/§11.7 require the archive
  SHA-256 and tracked content manifest to be recorded; it never claims the archive
  digest is reproducible across environments. This RFC narrows what that field
  means and adds the anchor that is. RFC 019 is not superseded.

## 1. Summary

The `0.21.0` release gate produced archive SHA-256 `6b056905…b72a` in CI and
`d52329ca…5664` in a local run of the same revision `e930db2`. Investigated:

| Comparison | Result |
|---|---|
| Tracked content manifest | identical |
| **Uncompressed tar stream SHA-256** | identical — `0748304f878fac82815edd341110905222de7d1a01bf235ed686ad0404f3457b` |
| Compressed size | 514,436 vs 517,190 bytes |
| gzip header | identical; mtime already normalised |

The released content is exactly the reviewed content. Only gzip output differs:
the same tar stream compresses to different bytes under a different gzip
implementation or level.

The evidence records only the compressed digest, so a consumer who builds locally
and compares against published evidence sees a mismatch and reasonably suspects
tampering when nothing is wrong. Architect reviews 046, 048, and 049 also called
the packaging "deterministic" on the strength of two local runs agreeing; that
held only within one toolchain.

**For the record, `0.21.0`'s cross-environment identity is the uncompressed tar
SHA-256 above.** The `0.21.0` tag message names `d52329ca…`, the local build's
archive digest; the tag is immutable, and this RFC is the tracked correction of
what that digest does and does not identify.

## 2–10. Crates; API, dependency, `std`/`alloc`, determinism, scalability, error, feature, semver impact

None. Host-only evidence change; no workspace dependency (hashing via the existing
`sha256sum` convention, decompression via `gzip -dc`, already required because the gate extracts with
`tar --gzip`; the archive itself is built by `git archive --format=tar.gz`).

## 11. Design

1. `EVIDENCE.md` gains `**Uncompressed tar SHA-256:**`, computed from the archive
   the gate itself produced, before clean extraction.
2. Both digests carry their meaning in the evidence text:
   - uncompressed tar SHA-256 — **cross-environment identity** of the source
     artifact's content and layout;
   - archive SHA-256 — **integrity of this build's bytes**, for verifying a
     download of this exact file; not expected to reproduce elsewhere.
3. `docs/src/verification.md` tells consumers which to compare: rebuild or
   download, `gzip -dc | sha256sum`, compare against the uncompressed digest.
4. No architect review, changelog, or tag message may describe the compressed
   archive as reproducible across environments.

## 12. Rejected alternatives

| Alternative | Reason rejected |
|---|---|
| Pin gzip implementation and level | Makes the compressed digest reproducible only while the pinned tool exists; binds the release gate to a tool version, the weaker trade |
| Drop the archive digest | It still verifies that downloaded bytes are the bytes CI produced |
| Ship an uncompressed tarball, or switch format | Changes the consumer-facing artifact to fix an evidence-labelling problem |
| Leave it as a note in review 050 | The corpus is untracked private input (RFC 020 §11.1); a finding recorded only there is invisible to the repository |

## 13. Verification gates

- Unit test: the same tar stream compressed at two gzip levels yields equal
  uncompressed digests and different archive digests; the evidence writer emits
  both with their labels.
- `release-gate` on a clean candidate: both digests present in `EVIDENCE.md`.
- Cross-environment check at the next release: CI and local uncompressed digests
  match; archive digests need not.
- Full developer suite; `mdbook build`.

## 14. Implementation sprint plan

S0 design freeze → S1 evidence writer + test → S2 docs → S6 closeout with the release that carries it.

## 15. Exit criteria

1. `EVIDENCE.md` records both digests with their meanings.
2. The unit test in §13 exists and passes.
3. `verification.md` names the uncompressed digest as the comparison consumers use.
4. The next release's CI and local uncompressed digests are observed equal.
