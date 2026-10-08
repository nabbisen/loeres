# RFC 044 - A `FRAC_BITS` A Caller Cannot Get Wrong

**Status.** Proposed.

**Author tier.** `architect`.

**Governing scoping.** Architect reviews 097 §6 and 099 §4. Every figure below was verified
by the architect on the `0.23.1` development tree.

## 1. Summary

`Q32<const FRAC_BITS: u32>` carries no bound on `FRAC_BITS`. Its module doc states
`1 <= FRAC_BITS <= 30` as a caller precondition and says outright that nothing enforces it
(`crates/loeres/src/scalar/fixed_point.rs:7-12`), citing `OrderedScalar::clamp`'s `lo <= hi`
as precedent. Outside that range the type's algebra silently fails:

| | `one()` raw | `one()` as a value | `one() > zero()` | `one() + one()` | `one() * one()` |
| ---: | ---: | ---: | ---: | ---: | ---: |
| `Q32<30>` | `1073741824` | `1.0` | `true` | `2.0` | `1.0` |
| **`Q32<31>`** | `-2147483648` | **`-1.0`** | **`false`** | **`-1.0`** | `1.0` |
| **`Q32<32>`** | `0` | **`0.0`** | **`false`** | `0.0` | **`0.0`** |
| **`Q32<33>`** | `0` | **`0.0`** | **`false`** | `0.0` | **`0.0`** |

`Q32<64>` and above is **already** a compile error — `const SCALE: i64 = 1i64 << FRAC_BITS`
overflows the shift (`E0080`). So the enforcement boundary today is inconsistent: 64 and
above is caught at compile time, **31 to 63 is silent**, and a kernel instantiated over
`Q32<32>` has `one() == zero()`, so `build_monotonic` returns `None` at the first step, every
tolerance comparison degenerates, and every `is_finite()` guard still returns `true`.

This RFC closes 31–63 with a compile-time assert, and makes no other change.

## 2. Why now, and why not a runtime check

**This is a published type.** `0.23.0` shipped `Q32` behind `fixed-point-hooks`. Each further
release is another release in which `Q32<32>` compiles and quietly has no multiplicative
identity. The window before a consumer exists is open and will not reopen.

**A runtime check is the wrong shape.** `FRAC_BITS` is a compile-time constant, so a runtime
test would burn a branch on a device target to re-discover something the compiler already
knows, and `BaseScalar`'s arithmetic has no failure channel to report it through (RFC 001).

**The codebase already does this.** `panic-audit`'s own documentation records the exception:
*"`assert!` is not listed: the const-assert dimension invariants are compile-time checks"*
(`xtask/src/checks/panic_audit.rs:18-19`). So the gate accommodates the construct and the
project has the precedent; this is consistency, not a new mechanism.

**Verified before proposing.** A const assert on a const generic parameter does fire at
compile time, with a readable message, and does not disturb valid instantiations:

```text
error[E0080]: evaluation panicked: Q32's FRAC_BITS must satisfy 1 <= FRAC_BITS <= 30
  evaluation of `Q::<32>::VALID_FRAC_BITS` failed here
```

## 3. Obligations

### 3.1 The assert, forced on every construction path

An associated `const VALID_FRAC_BITS: () = assert!(...)` on `impl<const FRAC_BITS: u32> Q32<FRAC_BITS>`,
with the message naming the valid range.

**A const assert fires only where it is forced.** Declaring it is not enough — something must
evaluate it. So bind `let () = Self::VALID_FRAC_BITS;` in **every** constructor:
`BaseScalar::zero`, `BaseScalar::one`, `Q32::from_raw` and `Q32::from_f64`. A value of this
type cannot come into existence by any other route, so forcing it at all four closes the set.

State in the doc comment that the check is compile-time and what the message says, so a
caller who hits it knows it is a precondition and not a bug.

### 3.2 The module doc stops saying nothing enforces it

`fixed_point.rs:7-12` currently says the range is *"a caller precondition, the same shape as
`OrderedScalar::clamp`'s `lo <= hi`"* and that *"nothing here enforces that at compile time"*.
After §3.1 that is false. Rewrite it: the range is enforced at compile time, and the `clamp`
comparison no longer applies — `clamp`'s precondition is a runtime value and genuinely cannot
be checked this way, which is the distinction worth drawing rather than erasing.

### 3.3 One test per boundary, and no more

`1`, `30` compile and behave; the `31`-and-above cases **cannot** be tested by an ordinary
`#[test]`, because a failing const assert is a compile error, not a panic — `should_panic`
does not catch it. Do **not** reach for a compile-fail harness (`trybuild` or similar): this
project takes no new dependency for this (RFC 026), and the assert's correctness is one
comparison. Test `1` and `30`, and record in the test's doc comment that `31` is a compile
error by construction, with the verified `E0080` message quoted.

## 4. Non-goals

- The `7..=20` **usable band** for the constrained kernel (RFC 043 §3.6 / S6). That band is
  narrower than this RFC's `1..=30` and is a *measured property of one corpus*, not a type
  invariant; asserting it would forbid instantiations that are perfectly sound for the box
  kernel, which RFC 041 S2 demonstrated at `FRAC_BITS = 20` and S4 will measure coarser.
  **The two ranges are different kinds of claim and this RFC enforces only the algebraic one.**
- Any change to `Q32`'s saturating discipline, to its tier disposition, or to any kernel.
- A general audit of const-generic preconditions elsewhere in the workspace.

## 5. Release position

Documentation and a compile-time assert. No callable API is added and no behaviour changes
for any valid instantiation, so this is a patch and rides whichever release RFC 043 S5 sets
(`rfcs/handoffs/release-0.24.0/scope-and-decision-points.md` §2).

## 6. Risks

- **Functional — Low.** One comparison, forced at four constructors, verified to fire.
- **Business — Low.** A caller already relying on `Q32<31..=63>` is relying on a type whose
  `one()` is negative or zero; breaking their build is the correct outcome and the message
  says why. No such caller is known, and the feature is default-off.
- **Operational — Low.** `panic-audit` already exempts `assert!` for exactly this use.
  `check-public-api` will see a new associated const; expect it to have an opinion and report
  what it said.
- **Security — Low.** Removes a silent-wrong-answer path; adds none.
