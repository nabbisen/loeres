# Specifications & RFCs

This book is a curated summary. The **authoritative design source-of-truth**
lives in the repository, not in the rendered book — it is maintained as raw
documents that the RFCs and this book are derived from. Maintainers and
contributors edit those documents directly; readers can browse them here.

> Release-local repository paths below are the normative review locations.
> Default-branch web links are labeled navigation to moving development state;
> they are not the sole source for a tag or extracted release.

## Specifications (`docs/specs/`)

> **Currency warning.** The apex trio now contains the reviewed RFC 020 S2
> v0.20.0 reconciliation, but its shared metadata remains explicitly draft and
> not yet the current marker. Supporting-document reconciliation, bounded
> semantic checks, and joint closeout are still pending. On conflict, stop
> affected public-boundary work rather than silently choosing code or prose. See the
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
- Accepted recovery RFCs (release-local): `rfcs/accepted/` — RFC 019 and RFC
  020 are design-frozen implementation contracts.
- [Accepted RFCs on the moving development branch](https://github.com/nabbisen/loeres/tree/main/rfcs/accepted)
  — navigation only; use the release-local path for normative review.
- [Proposed RFCs](https://github.com/nabbisen/loeres/tree/main/rfcs/proposed) — review-active designs; none currently.

## Contributing

If you want to propose or implement a design change, start with
[`CONTRIBUTING.md`](https://github.com/nabbisen/loeres/blob/main/CONTRIBUTING.md),
which explains the design-first workflow and the RFC process.
