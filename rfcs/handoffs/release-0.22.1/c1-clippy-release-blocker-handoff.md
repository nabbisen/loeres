# Developer Handoff — C1: unblock `release-gate` (deprecated `fetch_update`)

**Authorized by.** Architect review 076 §2.1. **Outside RFC 035**, which forbids crate changes in every slice.
**Assigned to.** Implementer tier.
**Priority.** **Blocking.** `cargo xtask release-gate` fails today, so no release can be cut until this lands.

---

## 1. Why this is blocking, which the earlier reports missed

`cargo xtask check` **does not run clippy** — it appears only in
`xtask/src/checks/release_gate.rs:77`, which runs
`cargo +stable clippy --workspace --all-features --all-targets -- -D warnings`.
That is exactly the failing command. So "17 gates PASS" and "clippy fails" are
both true and the second one blocks releases.

**Root cause, not a repository change.** `rust-toolchain.toml` pins
`channel = "stable"`, which floats. rustc 1.99.0 deprecated
`AtomicU64::fetch_update`, and `crates/loeres-cluster/src/validation_cache.rs:57`
uses it.

## 2. Do not use the lint's suggestion

It proposes `try_update`. **That method does not exist on MSRV 1.85:**

```text
rustc +1.85 → error[E0599]: no method named `try_update` found for struct `AtomicU64`
```

MSRV 1.85 is declared in the requirements. Do not raise it, and do not add
`#[allow(deprecated)]`.

## 3. The change — verified by the architect on both toolchains

In `next_model_identity()`, replace the `fetch_update` call with an explicit
compare-exchange loop:

```rust
let mut current = NEXT_MODEL_ID.load(Ordering::Relaxed);
loop {
    if current == u64::MAX {
        return Err(SolverError::InternalInvariantViolation);
    }
    match NEXT_MODEL_ID.compare_exchange_weak(
        current,
        current + 1,
        Ordering::Relaxed,
        Ordering::Relaxed,
    ) {
        Ok(previous) => return Ok(ModelIdentity(previous)),
        Err(observed) => current = observed,
    }
}
```

**Preserve the previous-value semantics.** `fetch_update` returns the value
*before* the update, and the existing code wraps that as the identity. An
off-by-one here silently renumbers every model. The architect's check asserted
`Ok(0), Ok(1), Ok(2)` on successive calls and `Err` at `u64::MAX`, on rustc 1.85
and 1.99 with `-D deprecated`.

Keep both orderings `Relaxed`, as now. Re-derive the loop rather than pasting it,
and **re-verify it on 1.85 yourself**.

## 4. Also in this slice

**A `CHANGELOG.md` entry for RFC 035** under the unreleased record. The RFC 035
handoff omitted it — the architect's omission, not yours (review 076 §4.5). It
covers: the restructured landing page with an extracted and checked Quick Start
example; the two new examples and the problems they solve; the getting-started
tutorial; the documentation convention; and that the `examples` gate now runs
each example and checks captured output in the book. Note that no crate behaviour
changed in RFC 035, and record this C1 fix separately as a lint-driven refactor
with identical semantics.

## 5. Explicit non-change scope

- **One call site only.** No other crate file, no other `fetch_update` or atomic
  usage, no ordering change.
- **No MSRV change.** No `rust-toolchain.toml` change — the floating-channel
  exposure is recorded in review 076 §4.4 and is deliberately not addressed here.
- No `#[allow(deprecated)]` anywhere.
- No behaviour change: identities must be issued in the same sequence as before.
- Nothing under RFC 035's slices is reopened.

## 6. Required evidence

- `cargo clippy --workspace --all-features --all-targets -- -D warnings` → **clean**
  (this is the point of the slice);
- **`cargo xtask release-gate --intended-tag 0.22.1`** → PASS. This is the gate
  that was failing; show it passing. Do **not** tag, do not push a tag, and do not
  write outside `.git-exclude/release-evidence/`'s own candidate directory;
- `cargo +1.85 check --workspace --all-features` → Finished;
- `cargo test --workspace --all-features`, `cargo xtask check` (17 gates),
  `cargo xtask conformance` plus `--suite extended` and `--suite adversarial`
  (24/24, 6/6, 26 of 31), `cargo xtask examples`, `mdbook build docs`;
- your own re-verification that the loop returns the previous value and `Err`s at
  saturation, on **1.85** and on current stable.

## 7. Review request

Standard eleven-item format; **record your author tier**; send to the architect.
State whether any example is affected (review 076's workflow rule) — it should
not be.
