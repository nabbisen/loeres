# RFC 044 implementation handoff — A `FRAC_BITS` A Caller Cannot Get Wrong

**To:** implementer (mid-capability model)
**From:** architect
**Authorized by.** The owner accepted RFC 044 on 2026-10-08. Read
`rfcs/accepted/044-a-frac-bits-a-caller-cannot-get-wrong.md` in full. Architect reviews
097 §6 and 099 §4 hold the measurements.
**Base revision:** `main` at or after the acceptance commit.
**Target release:** whichever RFC 043 S5 sets — see §4. **RFC 044 does not bump anything.**

**One review request: G.** `cargo xtask release-gate --intended-tag <v>` is required; §4 says
which `<v>`.

## 0. Verified before this was written, so you can check rather than discover

The architect built the mechanism in a scratch crate before proposing it. Both results are
yours to reproduce, not to trust:

1. **A const assert on a const generic parameter fires at compile time**, with the message
   intact, and a valid instantiation still builds:

   ```text
   error[E0080]: evaluation panicked: Q32's FRAC_BITS must satisfy 1 <= FRAC_BITS <= 30
     evaluation of `Q::<32>::VALID_FRAC_BITS` failed here
   ```
2. **`panic-audit` already exempts `assert!`** for exactly this use, and says so in its own
   source: *"`assert!` is not listed: the const-assert dimension invariants are compile-time
   checks"* (`xtask/src/checks/panic_audit.rs:18-19`). So the gate should not object. **If it
   does, stop and report it** rather than adding an exception.

The failure being closed, measured on the public `Q32` API across five precisions:

| | `one()` raw | `one()` as a value | `one() > zero()` | `one() + one()` |
| ---: | ---: | ---: | ---: | ---: |
| `Q32<30>` | `1073741824` | `1.0` | `true` | `2.0` |
| **`Q32<31>`** | `-2147483648` | **`-1.0`** | **`false`** | **`-1.0`** |
| **`Q32<32>`** | `0` | **`0.0`** | **`false`** | `0.0` |

`Q32<64>` and above is already `E0080` from `1i64 << FRAC_BITS`. **31 to 63 is the silent
window** and the only thing this slice closes.

## 1. The assert, and the part that is easy to get wrong

An associated const on `impl<const FRAC_BITS: u32> Q32<FRAC_BITS>`
(`crates/loeres/src/scalar/fixed_point.rs:86`), with the range in the message.

**A const assert fires only where it is forced.** Declaring it does nothing on its own —
`Q32<32>` would still compile. Bind it in **every** constructor:

- `BaseScalar::zero` (`fixed_point.rs:140`)
- `BaseScalar::one` (`:145`)
- `Q32::from_raw` (`:95`)
- `Q32::from_f64` (`:112`)

A value of this type cannot come into existence by any other route — the architect checked:
`to_raw` and `to_f64` are the only other `pub` items and both consume a `Self` rather than
producing one, so those four close the set. **Confirm it against the current file anyway**;
if a fifth constructor has appeared, it needs the binding too and finding one is a finding.

`from_raw` and `zero` are `const fn`; confirm the binding is legal in a `const fn` body and
say so in the request if it needs a different form.

## 2. The module doc stops saying nothing enforces it

`fixed_point.rs:7-12` says the range is *"a caller precondition, the same shape as
`OrderedScalar::clamp`'s `lo <= hi`"* and that *"nothing here enforces that at compile
time"*. After §1 that sentence is false.

Rewrite it: the range **is** enforced at compile time, quote what the error says so a caller
who hits it knows it is a precondition and not a bug, and **keep the distinction rather than
erasing it** — `clamp`'s `lo <= hi` is a runtime value and genuinely cannot be checked this
way. That contrast is the useful part; do not just delete the comparison.

## 3. Tests: two, and deliberately not three

`1` and `30` compile and behave. **`31` and above cannot be tested by an ordinary `#[test]`** —
a failing const assert is a compile error, not a panic, so `should_panic` does not catch it.

**Do not reach for a compile-fail harness** (`trybuild` or similar). This project takes no new
dependency for this (RFC 026), and the assert is one comparison. Record in the test's doc
comment that `31` is a compile error by construction, quoting the verified `E0080` message
from §0.

## 4. Release position — RFC 044 bumps nothing

Documentation and a compile-time assert. No callable API, no behaviour change for any valid
instantiation, so this is a **patch**.

- If RFC 043 S5 has **already** landed when you take this, the workspace is at `0.24.0`:
  run `release-gate --intended-tag 0.24.0`.
- If it has **not**, the workspace is still at `0.23.1`: run
  `release-gate --intended-tag 0.23.1` and **do not bump**. S5's slice carries the bump, per
  `docs/src/development.md`'s "A slice that moves the release's position bumps the version
  before the cut".

Check the workspace version rather than assuming which order the slices landed in.

## 5. Non-scope

- **RFC 043 S6's measured `7..=20` usable band.** Do **not** assert it. `1..=30` is an
  algebraic invariant of the type; `7..=20` is a measurement of one corpus for one kernel,
  and the box kernel is sound at precisions the constrained kernel is not. RFC 044 §4 draws
  that line deliberately — the two are different kinds of claim and conflating them would
  forbid sound instantiations.
- Any change to `Q32`'s saturating discipline, tier disposition, or any kernel.
- A workspace-wide audit of const-generic preconditions.

## 6. Required evidence

```text
cargo fmt --all -- --check          # run last, immediately before committing
cargo clippy --workspace --all-features --all-targets -- -D warnings
cargo test --workspace --all-features
cargo +1.85 check --workspace --all-features
cargo xtask check                   # 20 gates
cargo xtask release-gate --intended-tag <v>      # §4 gives <v>
```

**Check `cargo xtask check`'s own exit status.** Never through a pipe: a pipeline's status is
the last command's.

State in the request:

1. **Examples affected** — expected `none`.
2. **The `CHANGELOG.md` entry you added**, quoted.
3. Which constructors you bound the assert in, and **whether §1's list was complete**.
4. What `check-public-api` and `published-metadata` said about a new associated const behind
   a feature. Expect an opinion; report it either way.
5. That `panic-audit` passed — and if it did not, what it said, without adding an exception.
6. The `E0080` message your tree actually produces for `Q32<32>`, pasted. Not the one in §0:
   the architect's came from a scratch reproduction, not from this type.
7. Anything in this handoff that does not match the tree — the line numbers in §1 especially.
