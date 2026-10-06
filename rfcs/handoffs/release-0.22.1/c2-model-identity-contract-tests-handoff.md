# C2 — in-tree tests for the model-identity contract

**To:** implementer (mid-capability model)
**From:** architect
**Authorized by.** Architect review 077 §3, which accepted C1 and raised this as a
release blocker for `0.22.1`. The C1 prescription it builds on is architect
review 076 §2.1.
**Blocks:** release `0.22.1`
**Base revision:** `db9f518` (C1, accepted)

## 1. Why this exists

C1 replaced `fetch_update` with a hand-rolled compare-exchange loop. The loop is
correct — the architect verified it against `fetch_update` call-for-call on both
toolchains. What is missing is any in-tree evidence of the contract it now
carries by hand.

Measured, not assumed. On `db9f518` the architect changed the returned value from
the pre-increment value to `current + 1` — an off-by-one on exactly the semantics
the C1 review turned on — and **all 514 tests passed**. Nothing in the tree pins
the previous-value contract, and nothing reaches the saturation branch.

The release notes for `0.22.1` state that the refactor has "identical semantics".
C2 makes that claim checkable in the repository rather than in a reviewer's
scratch directory.

## 2. Scope

Two files. No public surface changes, no behaviour change, no new dependency, no
`#[allow]`, no MSRV or toolchain change.

### 2.1 `crates/loeres-cluster/src/validation_cache.rs`

Split the loop out of `next_model_identity` so it can be driven by a counter a
test owns. The process-global `NEXT_MODEL_ID` cannot reach saturation in a test
without wrecking every other test in the binary; this is the whole reason for the
split, and the doc comment says so.

```rust
fn next_model_identity() -> Result<ModelIdentity, SolverError> {
    next_identity_from(&NEXT_MODEL_ID)
}

/// Takes the next identity from `counter`, returning the value it held before the
/// increment and `InternalInvariantViolation` once it has saturated.
///
/// Separate from [`next_model_identity`] so the previous-value contract and the
/// saturation edge can be tested without disturbing the process-global counter.
///
/// A compare-exchange loop rather than `fetch_update`, which rustc 1.99
/// deprecates and whose replacement, `try_update`, is not available on MSRV 1.85.
/// Saturation is re-checked on every retry: a taker that loses the exchange can
/// come back holding the ceiling, and must refuse rather than wrap.
fn next_identity_from(counter: &AtomicU64) -> Result<ModelIdentity, SolverError> {
    let mut current = counter.load(Ordering::Relaxed);
    loop {
        if current == u64::MAX {
            return Err(SolverError::InternalInvariantViolation);
        }
        match counter.compare_exchange_weak(
            current,
            current + 1,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(previous) => return Ok(ModelIdentity(previous)),
            Err(observed) => current = observed,
        }
    }
}
```

The loop body is **unchanged from C1**. Only the counter it reads is now a
parameter. Both call sites of `next_model_identity` stay as they are.

### 2.2 `crates/loeres-cluster/src/validation_cache/tests.rs`

Append the six tests below. `use super::*;` is already at the top, so
`next_identity_from`, `AtomicU64` and `Ordering` are in scope. The accessor is
`ModelIdentity::value()`.

```rust
#[test]
fn taking_an_identity_returns_the_value_held_before_the_increment() {
    let counter = AtomicU64::new(41);
    assert_eq!(next_identity_from(&counter).unwrap().value(), 41);
    assert_eq!(counter.load(Ordering::Relaxed), 42);
}

#[test]
fn successive_identities_are_consecutive_and_start_at_the_initial_value() {
    let counter = AtomicU64::new(1);
    let taken: Vec<u64> = (0..4)
        .map(|_| next_identity_from(&counter).unwrap().value())
        .collect();
    assert_eq!(taken, vec![1, 2, 3, 4]);
    assert_eq!(counter.load(Ordering::Relaxed), 5);
}

#[test]
fn the_last_identity_before_saturation_is_issued_and_then_the_counter_refuses() {
    let counter = AtomicU64::new(u64::MAX - 1);
    assert_eq!(next_identity_from(&counter).unwrap().value(), u64::MAX - 1);
    assert_eq!(counter.load(Ordering::Relaxed), u64::MAX);

    for _ in 0..3 {
        assert_eq!(
            next_identity_from(&counter),
            Err(SolverError::InternalInvariantViolation)
        );
        assert_eq!(
            counter.load(Ordering::Relaxed),
            u64::MAX,
            "a saturated counter must not advance or wrap"
        );
    }
}

#[test]
fn a_saturated_counter_never_issues_the_reserved_non_cacheable_identity() {
    let counter = AtomicU64::new(u64::MAX);
    assert_eq!(
        next_identity_from(&counter),
        Err(SolverError::InternalInvariantViolation)
    );
}

#[test]
fn concurrent_takers_share_out_each_identity_exactly_once() {
    use std::sync::Arc;

    let counter = Arc::new(AtomicU64::new(1));
    let handles: Vec<_> = (0..8)
        .map(|_| {
            let counter = Arc::clone(&counter);
            std::thread::spawn(move || {
                (0..1_000)
                    .map(|_| next_identity_from(&counter).unwrap().value())
                    .collect::<Vec<u64>>()
            })
        })
        .collect();

    let mut issued: Vec<u64> = handles
        .into_iter()
        .flat_map(|handle| handle.join().expect("taker thread panicked"))
        .collect();
    issued.sort_unstable();
    let count = issued.len();
    issued.dedup();

    assert_eq!(issued.len(), count, "an identity was issued twice");
    assert_eq!(issued.first().copied(), Some(1));
    assert_eq!(issued.last().copied(), Some(count as u64));
}

#[test]
fn identities_are_shared_out_exactly_once_up_to_the_saturation_boundary() {
    use std::sync::Arc;

    // Saturation is re-checked on every retry, not once before the loop. A taker
    // can load a value below the ceiling, lose the exchange while other takers
    // consume the rest, and come back holding the ceiling; checking only before
    // the loop would then exchange the ceiling for a wrapped value and hand out
    // the reserved non-cacheable identity. Reaching that interleaving needs real
    // contention, so takers ask for twice what remains and the run is repeated.
    const REMAINING: u64 = 16_384;
    const TAKERS: usize = 8;
    const ASKS: usize = (REMAINING as usize / TAKERS) * 2;

    for attempt in 0..8 {
        let counter = Arc::new(AtomicU64::new(u64::MAX - REMAINING));
        let handles: Vec<_> = (0..TAKERS)
            .map(|_| {
                let counter = Arc::clone(&counter);
                std::thread::spawn(move || {
                    (0..ASKS)
                        .filter(|_| next_identity_from(&counter).is_ok())
                        .count()
                })
            })
            .collect();

        let issued: usize = handles
            .into_iter()
            .map(|handle| handle.join().expect("taker thread panicked"))
            .sum();

        assert_eq!(
            counter.load(Ordering::Relaxed),
            u64::MAX,
            "attempt {attempt}: a saturated counter wrapped instead of refusing"
        );
        assert_eq!(
            issued, REMAINING as usize,
            "attempt {attempt}: exactly the remaining identities may be issued"
        );
    }
}
```

## 3. The constants in the last test are measured, not chosen by taste

Do not "simplify" `REMAINING`, `TAKERS` or the repeat count. They were tuned
against the bug they exist to catch: hoisting the saturation check out of the
loop, which wraps the counter to `0`.

Detection rate per attempt, measured over 100–200 trials per row in a debug
build, against that mutation:

| remaining | takers | detection |
|---|---|---|
| 64 | 8 | 0.5% |
| 1 024 | 8 | 2.5% |
| 4 096 | 16 | 22% |
| 16 384 | 2 | 71% |
| 16 384 | 3 | 98% |
| 16 384 | 8 | **100%** |

Low `remaining` is *worse*, not better: the counter saturates at once, so nearly
every taker loads the ceiling and the cheap pre-loop check catches it. The bug
needs takers holding stale sub-ceiling values when saturation lands, which takes
sustained contention.

The architect's first attempt at this test used `remaining = 64` with a single
attempt and **did not detect the mutation at all**; a 256-attempt version caught
it in 6 of 10 runs. The shape above caught it in **10 of 10** `cargo test` runs.
Against the correct code it produced **0 false failures in 200 trials** at a
config heavier than the one shipped, and the suite ran clean 3 times in a row.

If you change these numbers, re-measure against the mutation and put the table in
your review request.

## 4. Non-scope

- No change to `next_model_identity`'s two call sites.
- No `CHANGELOG.md` change. `0.22.1`'s existing C1 paragraph already claims
  identical semantics; C2 is the evidence for that claim, not a new claim.
- No RFC. This is review-driven correction work inside an open release, the same
  class as C1.
- Do not add `#[allow(dead_code)]` to `next_identity_from`. It is reachable from
  `next_model_identity`, so it is live.

## 5. Required evidence

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-features --all-targets -- -D warnings
cargo test --workspace --all-features            # expect 520 passed, 0 failed
cargo +1.85 check --workspace --all-features
cargo xtask check                                # 17 gates
cargo xtask release-gate --intended-tag 0.22.1   # must PASS
```

State in the review request:

1. The six test names and that they are present.
2. **Examples affected: none** (expected — no example and no printed record changes).
3. That you ran the suite at least twice and saw no flake.
4. Whether you changed any constant in §3, and if so the re-measured table.
5. The `release-gate` **preflight** lines, read, not only the tail: the candidate
   identity line, and that clippy ran in the clean-extraction suite.

## 6. One note on reading the gate log

C1's request disclosed that the preflight went unread. For next time: the
source-tree suite's clippy line can read `Finished in 0.04s`, which is a cached
result and proves little on its own. The clean-extraction suite compiles from an
extracted archive, so its clippy line is the one that proves a real run. Both are
in the log; quote the second.
