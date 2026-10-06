# Specifications & RFCs

This book is a curated summary. The **authoritative design source-of-truth**
lives in the repository, not in the rendered book — it is maintained as raw
documents that the RFCs and this book are derived from. Maintainers and
contributors edit those documents directly; readers can browse them here.

> Release-local repository paths below are the normative review locations.
> Default-branch web links are labeled navigation to moving development state;
> they are not the sole source for a tag or extracted release.

## Release currency

This is the one home for release currency. The README links here (RFC 035 §3.3).

> **Publication and recovery status.** Repository state does not establish
> that every workspace crate or its hosted documentation is published or
> current, so external registry/documentation badges are intentionally
> omitted.

Repository release `0.20.2` is released and carries RFCs 001-021. Repository release `0.21.0` shipped 2026-09-12 and carries RFCs 001-026. Repository release `0.21.1` shipped 2026-09-24 and carries RFCs 001-029. Repository release `0.21.2` shipped 2026-09-24 and carries RFCs 001-030. Repository release `0.21.3` shipped 2026-09-24 and carries RFCs 001-033. Repository release `0.22.0` shipped 2026-09-24 and carries RFCs 001-034.

The apex specifications' currency blocks are the normative record; see the
the RFC index (`rfcs/README.md`) for the RFC states.

### Historical release notes

**v0.20.0 — Trusted/cache conformance hardening.** RFC 017 extends the enforced `conformance/smoke/` corpus with validation-cache fixtures for cache hit/miss, insufficient scope, stale/wrong evidence, current-iterate scan retention, hot-loop numerical-domain retention, and reusable-cache insertion rejection. Runtime crate APIs are unchanged.

At the 0.20.x line the shipped contracts were `000`–`021`. The list has grown since; the RFC index (`rfcs/README.md`) is current.

## Specifications (`docs/specs/`)

> **Currency notice.** The apex trio carries RFC 024's ordinary post-release
> block: this tree's identity and the release this documentation was last
> reconciled against, never release status. Repository release `0.20.2`
> shipped 2026-07-30 and carries RFCs 001-021. Repository release `0.21.0` shipped 2026-09-12 and carries RFCs 001-026. Repository release `0.21.1` shipped 2026-09-24 and carries RFCs 001-029. Repository release `0.21.2` shipped 2026-09-24 and carries RFCs 001-030. Repository release `0.21.3` shipped 2026-09-24 and carries RFCs 001-033. Repository release `0.22.0` shipped 2026-09-24 and carries RFCs 001-034. On conflict, stop affected
> public-boundary work rather than silently choosing code or prose. See the
> [Architecture Recovery Roadmap](recovery-roadmap.md).

- Requirements (release-local): `docs/specs/loeres-requirements-v1.md`
  ([moving-branch navigation](https://github.com/nabbisen/loeres/blob/main/docs/specs/loeres-requirements-v1.md)).
- External design (release-local): `docs/specs/loeres-external-design-v1.md`
  ([moving-branch navigation](https://github.com/nabbisen/loeres/blob/main/docs/specs/loeres-external-design-v1.md)).
- Detailed roadmap (release-local):
  `docs/specs/loeres-roadmap-milestones-v1.md`
  ([moving-branch navigation](https://github.com/nabbisen/loeres/blob/main/docs/specs/loeres-roadmap-milestones-v1.md)).

## RFCs (`rfcs/`)

- RFC index (release-local): `rfcs/README.md`
  ([moving-branch navigation](https://github.com/nabbisen/loeres/blob/main/rfcs/README.md)).
- RFC lifecycle policy (release-local):
  `rfcs/done/000-rfc-lifecycle-policy.md`
  ([moving-branch navigation](https://github.com/nabbisen/loeres/blob/main/rfcs/done/000-rfc-lifecycle-policy.md)).
- Recovery and release-finalization RFCs (release-local): `rfcs/done/` — RFC
  019, RFC 020, and RFC 021 are implemented and shipped in repository release
  `0.20.2`.
- [Accepted RFCs on the moving development branch](https://github.com/nabbisen/loeres/tree/main/rfcs/accepted)
  — navigation only; use the release-local path for normative review.
- [Proposed RFCs](https://github.com/nabbisen/loeres/tree/main/rfcs/proposed) — review-active designs; none currently.

## Contributing

If you want to propose or implement a design change, start with
[`CONTRIBUTING.md`](https://github.com/nabbisen/loeres/blob/main/CONTRIBUTING.md),
which explains the design-first workflow and the RFC process.
