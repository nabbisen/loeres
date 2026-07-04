# Loeres Conformance Corpus

This directory contains host-side conformance fixtures. Runtime crates do not
parse these files and do not depend on the fixture schema.

The enforced suite is `smoke/`, covering the shared projected-first-order box
quadratic path across `loeres-device` and `loeres-cluster`.

RFC 013 fixtures use `schema_version = 1` for baseline device/cluster parity.
RFC 017 fixtures use `schema_version = 2` for validation-cache conformance:
cache hit/miss, insufficient scope, stale/wrong evidence rejection,
current-iterate scan retention, hot-loop numerical-domain retention, and
reusable-cache insertion rejection cases.

The `trusted-cache-smoke` fixture group is a corpus-local review tag, not an RFC
011 target-profile group. `pfo-cache-match-001` covers the reusable-cache hit
path only; the provided-evidence happy path remains covered by RFC 015 unit
tests and is later-work conformance scope.

`extended/` and `adversarial/` are staged placeholders.
