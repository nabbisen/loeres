# RFC 018 - Cluster Solve Test Helper Cleanup

**Status.** Implemented (v0.20.0) - small cleanup RFC.
**Tracks.** Test readability and maintenance hygiene.
**Touches.** `crates/loeres-cluster/src/solve/tests.rs`.

---

### Extended Metadata
* **Rust Edition Compliance:** Rust 2024 Baseline
* **Target Environment:** Test code only. No runtime crate API, solver behavior, or verification policy changes.

## 1. Summary

`crates/loeres-cluster/src/solve/tests.rs` currently contains a local helper:

```rust
fn kinds(outcomes: &[BatchItemOutcome<f64>]) -> Vec<u8> { ... }
```

The helper is still used by the feature-gated `parallel_matches_sequential` and
`async_matches_sync` tests. It is not dead code. However, its encoding (`0`,
`1`, `2`, `3`, `4`) is local, opaque, and small enough that the comparisons can
be clearer when expressed directly near the assertions.

RFC 018 records this as a small test cleanup item: rewrite the two comparisons
without the `kinds()` helper, or replace the helper with a named local enum if
inline assertions become noisy.

## 2. Affected crates

| Crate | Impact |
|---|---|
| `loeres-cluster` | Test-only change in `src/solve/tests.rs`. |
| all other crates | No impact. |

## 3. Public API boundary impact

None. This RFC touches only unit tests.

No public type, function, feature flag, module, error variant, conformance
fixture, or runtime behavior may change as part of this RFC.

## 4. Dependency impact

No dependency changes are allowed.

## 5. `std` / `alloc` impact statement

No impact. The affected code is already `loeres-cluster` test code. No `std` or
`alloc` requirement may be introduced into `loeres`, `loeres-backend-static`, or
`loeres-device`.

## 6. Device determinism impact statement

No impact. Device code and device tests are out of scope.

## 7. Cluster scalability impact statement

No impact. Cluster runtime code is out of scope.

## 8. Error and diagnostic impact

No impact. Test assertions may continue to distinguish solved-converged,
solved-not-converged, failed, cancelled, and panicked outcomes, but no error
mapping may change.

## 9. Feature flag impact

The affected tests are feature-gated:

| Test | Feature |
|---|---|
| `parallel_matches_sequential` | `parallel-rayon` |
| `async_matches_sync` | `async-tokio` |

The cleanup must preserve both feature gates and must pass with
`--all-features`.

## 10. Semver impact

No public API impact. If released alone, this is patch-level test cleanup. It may
also be batched into the next implementation release that already touches
cluster tests.

## 11. Rejected alternatives

| Alternative | Reason |
|---|---|
| Treat `kinds()` as dead and remove it immediately | Incorrect: it is used by two feature-gated tests. |
| Leave the numeric helper forever | Acceptable, but the numeric encoding is opaque and unnecessary for such small comparisons. |
| Add a public outcome-kind classifier | Too large for this problem; RFC 009 already owns public/redacted observability classification. |
| Expand this into conformance work | Not needed. RFC 013 owns the conformance corpus policy, RFC 015 owns trusted/cache semantics, and RFC 017 owns trusted/cache conformance fixture work. |

## 12. Verification gates

Required gates for the implementation:

1. `cargo fmt --all --check`
2. `cargo test -p loeres-cluster --all-features`
3. `cargo clippy --workspace --all-features --all-targets -- -D warnings`
4. `cargo xtask check`

Because this is test-only and cluster-local, a clean extraction gate is optional
unless the cleanup is batched into a release-bearing implementation.

## 13. Implementation sprint plan

| Sprint | Work |
|---|---|
| S0 Design Freeze | Confirm this remains test-local and does not overlap conformance or trusted/cache work owned by RFC 013, RFC 015, and RFC 017. |
| S1 Cleanup | Rewrite `parallel_matches_sequential` and `async_matches_sync` comparisons without opaque numeric codes. |
| S2 Verification | Run focused cluster tests and aggregate checks. |
| S3 Closeout | Move RFC 018 to `done/` if the cleanup lands. |

## 14. Exit criteria

RFC 018 is complete when:

1. `kinds()` is removed or replaced by a clearer test-local assertion helper;
2. `parallel_matches_sequential` still verifies sequential/parallel outcome
   equivalence;
3. `async_matches_sync` still verifies sync/async outcome equivalence;
4. no runtime code changes;
5. the cleanup is warning-clean under default, single-feature, and
   `--all-features` builds, either by inlining the comparisons or by gating any
   retained helper with `cfg(any(feature = "parallel-rayon", feature =
   "async-tokio"))`;
6. required gates pass.

## 15. Implementation closeout

RFC 018 ships in v0.20.0 as a test-only cleanup batched with the RFC 017
conformance release. Runtime crate APIs and solver behavior are unchanged.

The implementation removes the ungated `kinds()` helper from
`crates/loeres-cluster/src/solve/tests.rs` and rewrites the two feature-gated
comparisons inline:

1. `parallel_matches_sequential` compares solved-converged,
   solved-not-converged, and failed outcomes directly between sequential and
   parallel execution.
2. `async_matches_sync` compares solved-converged and solved-not-converged
   outcomes directly between sync and async execution.

The cleanup avoids preserving an ungated feature-only helper and keeps default,
single-feature, and all-feature builds warning-clean for this module.
