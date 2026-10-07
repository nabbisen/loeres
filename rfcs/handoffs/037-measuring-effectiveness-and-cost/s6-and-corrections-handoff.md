# RFC 037 S6 and corrections C1-C3

**To:** implementer (mid-capability model)
**From:** architect
**Authorized by.** Architect review 087, which accepted requests A, B and C, and
RFC 037 Amendment 1 (2026-10-07), which added §5.6 after your finding that §5.3's
premise was false.
**Base revision:** `41f417f`, or later on `main` once review 087 is committed.

**None of this blocks `0.22.2`.** The release is held only on the owner's headline
decision (review 087 §7). Everything here is work the measurements earned.

Requests A, B and C were the best sequence of this project so far. You found five
errors in the architect's own RFC and handoff, measured every reference point exactly,
and refused twice to assert something you could not see. Three of the five are now
corrected in RFC 037 itself.

## S6 — an exact reference for a dense `Q`, `n ≤ 8`

Your request B §1.3 offered three options and recommended the first. **Option 1 is
accepted**, with the cross-validation you proposed, and as its **own slice** rather than
an addendum — it is new numerical code, and RFC 030's standard applies to new numerical
code without exception.

RFC 037 §5.6 carries the design. In short:

- An active-set enumeration for a **dense** `Q`, in `xtask`, for `n ≤ 8` only. Your
  bound is right and is now in the RFC: the pool is `m + 2n` rows with subsets up to
  size `n`, which is 20 rows at `n = 8` and out of reach at `n = 16`.
- **Cross-validate against `conformance/reference.rs`** on the fixtures where both
  apply — a diagonal `Q` is a special case of a dense one, so the two must agree to
  tolerance on every separable fixture. That agreement is the evidence that the
  generalisation is right, and it is the slice's main deliverable, not a side check.
- Report the deviation for `n ∈ {4, 8}` of the corpus, and replace the chapter's
  `NOT REPORTED` paragraph with the figures **plus** the statement that nothing larger
  is exactly checked and that larger instances rest on the three legs and the
  conformance suites.
- Do **not** pin a deviation figure in `bench-baseline`. A float deviation is not an
  integer count; §4.3's reproducibility argument does not extend to it.

If the cross-validation disagrees anywhere, stop and report it. A disagreement means
either the new routine or the old one is wrong, and which is wrong matters more than
shipping the slice.

## C1 — close the loop between `bench` and the example

Your request C §1.1 names this gap yourself: `examples/cluster-counted-work` carries a
second copy of the family, the two agree today at six points (30, 48, 48, 25, 232, 989),
and nothing compares them. The example's block is pinned to the example's own run, and
`bench-baseline` pins `bench`, so the two can drift apart while both gates pass.

Close it in `bench_baseline.rs`: assert the example's printed rows against the **same
pinned table** the gate already holds. That makes the chain pinned table → `bench` →
example checked end to end, and a drift in either copy fails.

Run the example and parse its output, as `examples.rs` already does — do not re-implement
the family a third time to compare against.

If the parse turns out to be brittle enough to be a liability, say so and propose the
alternative instead of building something fragile; a shared fixture consumed by both is
the fallback, and RFC 037 §4.1 as amended does not forbid it for this purpose.

## C2 — the derived-column units, and one comment

1. **`bench`'s legend already states "per outer iteration"** for both derived columns,
   and the architect confirmed it before raising it. No change.
2. **`xtask/Cargo.toml` gains a one-line comment** on
   `loeres-cluster = { workspace = true, features = ["parallel-rayon"] }` saying that the
   feature is what makes `cargo xtask throughput`'s parallel batch reach the parallel
   path, and that removing it makes the measurement sequential. Your
   `policy used: …` line already prevents a silent false speedup — that was good
   defensive design — but the comment stops someone removing the feature as unused.

## C3 — the speedup, reported only as a range, with its granularity

Your reading in request B §6.3 is right and the architect is making it a requirement:
**the parallel speedup never appears as a single number**, in the chapter or in
`throughput`'s output. It appears as the observed range with the number of runs behind
it, as the chapter already does (9.8× to 13.3× over three runs).

Add one thing the figures imply and neither document says: the batch is **64 items on 32
logical threads**, so an ideal speedup would be near 32× and the observed 10-13× is
roughly a third of that. State the batch size and the thread count beside the range, and
say plainly that the gap is scheduling and work granularity, not a defect. A reader who
sees "11.8×" without the denominator cannot tell whether that is good.

## Non-scope

- No kernel instrumentation. The sweep count stays unobservable; RFC 037 §0.1 records it
  as a candidate for a later opt-in counting RFC.
- No change to the three legs of §5.3, to `bench`'s corpus, or to the pinned table's
  eleven rows.
- No new dependency; no criterion; no wall-time gate.
- Do not change `README.md:5`. The owner decides the headline (review 087 §7), and the
  four disclaimers move with it in the same commit, whoever applies it.
- No `#[allow(…)]`. Do not tag; do not run `cargo publish`.

## Required evidence

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-features --all-targets -- -D warnings
cargo test --workspace --all-features
cargo +1.85 check --workspace --all-features
cargo xtask check                                   # 20 gates
cargo xtask bench
cargo xtask bench-baseline
cargo xtask throughput
cargo xtask release-gate --intended-tag 0.22.2      # must PASS
```

State in the request:

1. **Examples affected** — `cluster-counted-work` if C1 changes its output; otherwise
   `none`.
2. For S6, the cross-validation result on **every** separable fixture where both
   references apply, with the largest disagreement, and the deviation figures for
   `n ∈ {4, 8}`.
3. For C1, the break-and-restore record: change one example row, confirm
   `bench-baseline` fails and names it, restore.
4. **Run `cargo fmt --all -- --check` after your last edit, before committing.** Request
   C §4 records a candidate that failed `release-gate` on formatting alone. The gate
   caught it, which is what it is for — but the cheaper check is yours and costs a
   second.
5. Anything in this handoff that does not match the tree.
