# Architecture Review Evidence Index

Tracked integrity register for the architecture reviews cited by this
repository's normative documents. Each row fixes the identity of one review
document by SHA-256 so that a citation such as "architecture review 024" in
`rfcs/done/`, `docs/specs/`, `ROADMAP.md`, or `CHANGELOG.md` resolves to a
named, hash-pinned artifact rather than to an unreachable file.

**This index binds evidence identity, not authority.** RFC 020 §11.1
classifies archived review bundles as historical/private review input, not a
public normative source; this index does not change that. A review is
evidence that a decision was considered, never the source of its authority —
design authority rests with the architect role, and approval with the project
owner. Citing prose must not state that a review *accepted*, *approved*, or
*authorized* anything unless that row's **author tier** is `architect`,
`owner`, or `external`. A row tiered `implementer` or `unrecorded` may only be
cited as having *found*, *reported*, or *recommended* — the checker validates
that every row carries a tier from the closed set, but the authority-verb rule
itself is human review's to apply; a bounded checker cannot read surrounding
prose for intent.

**This index is not a lifecycle.** It records evidence identity and
provenance only. RFC status remains governed exclusively by RFC 000's
folder-as-source-of-truth rule, and nothing here grants, withdraws, or
modifies an RFC state. It must never acquire status columns, approval
semantics, or its own state folders — RFC 000's handoff anti-pattern, one
layer over.

**Review bodies are not tracked.** They are maintainer-held working
documents. This repository tracks their identity, not their contents, so a
released artifact can prove *which* evidence a decision rested on without
carrying the deliberation into the distribution.

**Cited reviews are immutable.** Once a review is cited by a tracked
normative document, its file must never be edited — not even a typographical
correction. A correction is issued as a new review document with its own
reference number. If a cited review could be edited, this index would record
a hash that silently stopped matching, and the project would be unable to
distinguish an innocent fix from evidence tampering.

**Author tier is captured at request time, never reconstructed.** The closed
set is `owner`, `architect`, `implementer`, `external`, `unrecorded`.
`unrecorded` is the correct, honest value wherever provenance was not
captured when the review was requested — it must never be guessed from a
filename, a title, a tone, or a review's own self-description. The incident
that motivated this column is exactly that: two reviews requested and cited
as independent architecture review, and later identified as implementer-tier
output only by direct project-owner statement, well after one of their
recommendations had already been adopted and had to be reversed. New reviews
must record their tier in the request document itself, at request time.

**Verification.** `cargo xtask check` enforces two assertions from tracked
bytes alone, fail-closed: every review reference appearing in a tracked
normative document resolves to a row below, and every row carries a tier from
the closed set.

Two further assertions need the maintainer-held corpus under
`.git-exclude/reviewed/`, and are enforced when it is present: each row's
SHA-256 is recomputed and a mismatch fails, and **coverage is symmetric** —
every corpus file must be registered by some row, so forgetting to register a
newly written review fails the gate rather than drifting silently (Amendment 2,
§0.4). When the corpus is absent — as in a clean extraction — both are reported
**unavailable**, never as passed.

A citation shaped like a review reference but not matching the recognized
grammar is reported as a near-miss finding rather than silently ignored, so an
unrecognized citation format cannot escape enforcement quietly. Near misses do
not fail the gate; the count appears in the summary and verdict lines.

Not enforced: the **Cited in release** column below is hand-maintained. The
checker derives the true cited set at run time and reports its size, but does
not compare that set against these ticks.

**Disposition is deliberately absent from v1.** Recording an outcome per
review requires reading each document; transcribing it from the prose that
cites the review would be circular. Populating a disposition column is a
separate, reviewed pass.

| Ref | Date | Subject | SHA-256 | Author tier | Cited in release |
|---|---|---|---|---|:--:|
| `001` | 2026-07-15 | prepare architecture review | `5160b5476c722c901b172f0c065fffa8a7722b9cc763e2a4400aee77969652da` | `unrecorded` |  |
| `001` | 2026-07-17 | prepare architecture review | `a076e16d419082f19f99bca9838a8bc4237e7de88cd99b7e5c557b8a14d65a2c` | `unrecorded` |  |
| `002` | 2026-07-15 | rfc019 rfc020 design review | `33772bc0cf270c71efa57116bcc184758ab86317d6c4e8000c26aa8b67f5f2eb` | `unrecorded` |  |
| `003` | 2026-07-15 | rfc019 rfc020 r0 rereview | `9cf71e52a361899a711d393d5affb92ebfbe99c022e61657cf831dc1dec6ed6b` | `unrecorded` |  |
| `004` | 2026-07-15 | rfc019 rfc020 r05 lifecycle review | `fa4673fdbe0ca76a1e4cae0183c9d98768ca329d051d1b15f55d62087b67d241` | `unrecorded` |  |
| `005` | 2026-07-15 | rfc019 rfc020 r05 lifecycle rereview | `e9ace59d782a47c19aeb5f17a4b4ca783a7a858b6c955352f41d64451e118450` | `unrecorded` |  |
| `006` | 2026-07-15 | rfc019 r1 s1 s3 architecture review | `eb39b320bf90ed1fcd98018f2eee0995cdf05d450c1ed24ded887bc1feded2d1` | `unrecorded` |  |
| `007` | 2026-07-15 | rfc020 r1 s1 traceability review | `149218aa7647e3aa34969f53565aaa5af50c8acf7305de939731fb0c606940d4` | `unrecorded` |  |
| `008` | 2026-07-15 | rfc020 r1 s1 traceability rereview | `dcd1bb66ba2773c88442597c0cde1463913c0885329117c912da0c0ca53d54cf` | `unrecorded` |  |
| `009` | 2026-07-15 | rfc020 r1 s2 apex reconciliation review | `b845b248baba1141a3ba75b912aa43936309a34c7f9931c24a9827aa591e2229` | `unrecorded` |  |
| `010` | 2026-07-15 | rfc020 r1 s3 supporting documentation review | `59179e1964f74607ca0e9983a84e9642bec976297624ae6816cc421dbccfb130` | `unrecorded` |  |
| `011` | 2026-07-15 | rfc020 r1 s3 supporting documentation rereview | `6fcecef698a5382351ccff467739a584df2176585e4cf668904066405b2ed277` | `unrecorded` |  |
| `012` | 2026-07-15 | rfc020 r1 s4 doc currency tooling review | `9c0ef374efa3fc59b5e18912b27ec762556cd7c7776529c913381ab1665ea960` | `unrecorded` |  |
| `013` | 2026-07-16 | rfc020 r1 s4 doc currency tooling rereview | `4a8bd05896b2937c19d7021637c21127aa5c27990af1477c2fe98805e2055c6d` | `unrecorded` |  |
| `014` | 2026-07-16 | rfc020 r1 s5 integrated documentation review | `97c76eb4dea1b449d73af86556e01646b118cd7e1b992a381715f99d111653d5` | `unrecorded` |  |
| `015` | 2026-07-16 | rfc020 r1 s5 integrated documentation rereview | `fa8a7dcd39a2beb48f25d0274cbb9b194f553d525c6be10de0c113879dbdfd8f` | `unrecorded` |  |
| `016` | 2026-07-16 | rfc019 r1 s4 s5 package gate review | `327c604fced02465f8fa8c78f52aee910fc6672af170ee99cbfc61c63c56a274` | `unrecorded` |  |
| `017` | 2026-07-17 | rfc019 r1 s4 s5 package gate rereview | `cd429ae8631847d3b6efe63ce40d20b2bf4bac240bf1fa005a6e8c2336edf119` | `unrecorded` |  |
| `018` | 2026-07-17 | rfc019 r1 s4 s5 validation doc rereview | `8c72f8a73993551dae212440c377abe5078f7eaffdbda3edea63541d4325d229` | `unrecorded` |  |
| `019` | 2026-07-17 | rfc019 r1 s4 s5 private path rereview | `5a3fde6cf7e5975fde133c7ae4685659a8b76b482491a27f2cdd73620bfbd573` | `unrecorded` |  |
| `020` | 2026-07-17 | rfc019 r1 s4 s5 private path final rereview | `dd0da904f761b095f4fb755112e1d14fb99e6aaf806df987a2bd319f637a8b02` | `unrecorded` |  |
| `021` | 2026-07-17 | rfc019 rfc020 r2 s6 joint closeout evidence review | `4282e799dff562b9db09be252b5e819c18222e79e569398a8185ef88bda68a02` | `unrecorded` | ✓ |
| `022` | 2026-07-17 | rfc019 rfc020 r2 v0.20.1 local evidence review | `fb0963787f74e27d4f363c1dbde482230a59209dbd10020cc1dd593900ce43ae` | `unrecorded` | ✓ |
| `023` | 2026-07-17 | rfc019 rfc020 r2 0.20.1 b5 rereview | `1bebfc0a15f9e52ead09adc5d74b5c64325313a2b544cc040f745a4d384e13d9` | `unrecorded` |  |
| `024` | 2026-07-17 | rfc019 rfc020 r2 0.20.1 tagged evidence review | `e6c822c092dc483282699ed691555cfccc952f1a43f3dcbd8025daad4215cf2e` | `unrecorded` | ✓ |
| `025` | 2026-07-17 | rfc019 rfc020 r2 s6 post tag closeout review | `72f70d10168cc6e0b04b736b9c2e38597974115c8d63807d681ca2e0e210e3d8` | `unrecorded` | ✓ |
| `026` | 2026-07-17 | rfc021 conditional release finalization design review | `a586264178faca2fe0d3a966968a5e3e618750e381f7a3c67e277adf3c599b10` | `unrecorded` |  |
| `027` | 2026-07-18 | rfc021 conditional release finalization design rereview | `362f8a4c3ede464985bed9cae4583209c3a31b26a836f873364da16df54c8dc4` | `unrecorded` | ✓ |
| `028` | 2026-07-18 | rfc021 q0.5 lifecycle activation review | `4e804ca9d67a4e4e8e7e7662ea140d15a13195ded086c552721e8073b1f42ccb` | `unrecorded` |  |
| `029` | 2026-07-18 | rfc021 q0.5 lifecycle activation rereview | `c348464201d6baabb2ef3ec7d10ce20b00d1d386fd5415193511588f6697eefa` | `unrecorded` |  |
| `030` | 2026-07-22 | rfc021 s1 conditional finalization tooling review | `b4ca244fb08cf2609467d0f6b9ca38f544caf78aba3bd5d6a89b60fc6ff96385` | `unrecorded` | ✓ |
| `031` | 2026-07-22 | rfc021 s1 conditional finalization tooling rereview | `55ec96178c3491d23bea717d95836ffa998a16940cf084ce3d876483126247ac` | `unrecorded` | ✓ |
| `032` | 2026-07-22 | rfc021 s2 conditional preparation review | `5b5a211592c1ad3eaf15215baf62198d588ed8e8f467ccee6fceed35ee6257d3` | `unrecorded` | ✓ |
| `033` | 2026-07-22 | rfc021 q2 exact finalization local evidence review | `992c9b0dff378f59f851f6854fc4e524fc7076881b3bf1e99166289ae2e0aee2` | `unrecorded` | ✓ |
| `034` | 2026-07-22 | rfc021 q2 b13 exact finalization rereview | `498b3c7a283a0615b882f8d46b530231be99bba63e8af8071887172304be2045` | `unrecorded` |  |
| `035` | 2026-07-22 | rfc021 q3 tag bound evidence review | `042b52ea6304e2304b627e72a9859514b86e3f0cc447c078ea85ec979b5bd63d` | `unrecorded` |  |
| `036` | 2026-07-31 | rfc024 implementation architecture review | `949e3356e6685fd35328fa70b07637f48bcde6dffeae60e16ee5c489428496fb` | `implementer` | ✓ |
| `037` | 2026-07-31 | rfc024 amendment1 rereview | `caaeda6ab04d8c17f898eea2ffa5ae54a3445e02c56925fc30a417135a2a7ae9` | `implementer` | ✓ |
| `038` | 2026-09-09 | rfc024 implementation architect review | `bfb95d2773463ca8f3321e8868b2337f659212b153607e962e302d4f09afac62` | `unrecorded` | ✓ |
| `039` | 2026-09-09 | rfc022 implementation architect review | `9383c42eb538c59ea9b7c1f034427e4ecbe8bf04d40cbed8509946e9638e6591` | `unrecorded` | ✓ |
| `040` | 2026-09-12 | rfc022 followups architect review | `d6c723ae87ec95913a7baead5c086432ea5c06005a6f23dbc981b5a9667c19d7` | `unrecorded` |  |
| `041` | 2026-09-12 | rfc022 amendment2 architect review | `c677e43f3d02be37abea06d01e418a626b22ca0a4bde87387aae256b84c4d0e1` | `unrecorded` |  |
| `—` | — | loeres rfc009 architect design review v1 | `d80d14533252eae46b7198abcaeceeb1e9dec1389cb0becc61118b03f027bedc` | `unrecorded` |  |
| `—` | — | loeres rfc009 impl decision and patch review v1 | `80908d7680c8d5ce14be6553b7c25e0e7ab51f96c5817a54591ef908073f16ca` | `unrecorded` |  |
| `—` | — | loeres rfc010 xtask verification governance review v0.1 | `d017955a2f268118a1d2457c89178d1400f6061ba15d06f52c95f2ac081cdc20` | `unrecorded` |  |
| `—` | — | loeres rfc011 design freeze review v0.1 | `123d0c51ff8ed29fc0cfbab10af69ea0227ca2a80ca7bc1ecc29166d31a9e551` | `unrecorded` |  |
| `—` | — | loeres rfc013 design freeze review v0.1 | `7636d43b31f04749be20bfc309b3059cad12e0a60f719825d76ad94e5a83911e` | `unrecorded` |  |
| `—` | — | loeres rfc015 design freeze review v0.1 | `904eaa331478a0d7281d912240bb8762f2c94ad0f8c4d298032c8519f9888625` | `unrecorded` |  |
| `—` | — | loeres rfc015 patched design freeze review v0.1 | `6867bc46115912adb9c30e1f3e70b42d8f0b7a1fdb893e333e1359bb51ccb2a6` | `unrecorded` |  |
| `—` | — | loeres rfc017 design draft review v0.1 | `13ca5086e124ebc6afc21ab06ee0c42250e7d633dbedc30ffd82909a3f0e9aab` | `unrecorded` |  |
| `—` | — | loeres rfc017 trusted cache conformance fixtures | `9f4578d29b1eae8bec3590b29fbf616393be336f01c3895fcb15a3cf3eff4cd2` | `unrecorded` |  |
| `—` | — | loeres rfc018 architect review v1 | `854fb09ac472fcb568b751fbed24327529a1c382d62f9482596b77ab976089a1` | `unrecorded` |  |
| `—` | — | loeres v0.19.0 rfc015 implementation review v0.1 | `75c76ec613935338b5b1c32bfe2889f1576cd7bec4b114ea86fbfc4813045708` | `unrecorded` |  |
| `—` | — | loeres v0.20.0 rfc017 implementation review v0.1 | `1e966e1e586040c365c189fa80db98264fc7cf2d1695e5b55d1fe8216a8f0f33` | `unrecorded` |  |

## Summary

- Registered review documents: **54**
- Distinct review references cited by tracked normative documents: **13**
  (`021`, `022`, `024`, `025`, `027`, `030`, `031`, `032`, `033`, `036`, `037`,
  `038`, `039`). The two most recent additions are both self-referential in the
  useful sense: RFC 022 Amendment 2 cites review 039 as the finding behind the
  coverage-symmetry requirement, and RFC 024 §0.5 cites review 038 as the
  finding behind its Status-line rationale. The authoritative count is whatever
  `cargo xtask review-evidence` reports; this line is a hand-maintained echo of
  it.
- Unresolved citations: **0**
- Rows tiered `implementer`: **2** (`036`, `037` — direct project-owner
  statement); all other rows are `unrecorded` because provenance was not
  captured at request time and must not be reconstructed by inference.
- Rows awaiting owner confirmation: `038`, `039`, `040`, and `041`. Each carries a
  `Reviewer author tier` field in its own text, and the architect has attested
  first-hand to authoring `038` through `041`. None is recorded from that alone: a tier taken
  from an artifact's own self-description is the inference §0.1 forbids, and it
  is what made `036`/`037` look independent. The `036`/`037` precedent set
  tiers on project-owner statement, so these stay `unrecorded` until the owner
  states otherwise — a one-row edit plus these counts, per row.

`Ref` mirrors the numeric prefix used by in-repository citations. Pre-recovery
reviews predate that numbering and carry `—`; they are registered for
completeness and are not cited by tracked documents. Two preparation reviews
share `Ref 001`; they are disambiguated by date.
