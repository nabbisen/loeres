//! A `Q`-format signed fixed-point scalar baseline (RFC 041), behind
//! `fixed-point-hooks`.
//!
//! [`Q32`] is a signed `Q`-format number over an `i32` representation, with the
//! fractional bit count a const parameter (`FRAC_BITS`) so the type is not
//! wired to one precision. The raw integer `r` represents the real value
//! `r / 2^FRAC_BITS`. `FRAC_BITS` must leave room for the sign bit and for
//! [`BaseScalar::one`] to be representable: `1 <= FRAC_BITS <= 30`.
//!
//! **This range is enforced at compile time** (RFC 044), unlike
//! [`OrderedScalar::clamp`]'s `lo <= hi` — that distinction is worth keeping,
//! not erasing: `clamp`'s bounds are ordinary runtime values with no way to
//! check them before the call exists, while `FRAC_BITS` is a const generic
//! the compiler already has in hand at every instantiation. `Q32::<F>`'s
//! `VALID_FRAC_BITS` associated const asserts the range and is forced by
//! every constructor (`zero`, `one`, `from_raw`, `from_f64`); an
//! out-of-range `FRAC_BITS` fails to compile with
//! `evaluation panicked: Q32's FRAC_BITS must satisfy 1 <= FRAC_BITS <= 30`,
//! not a silent wrong answer at runtime. `Q32<64>` and above already failed
//! this way regardless (the scale shift itself overflows); this closes the
//! `31..=63` window that previously compiled and silently had no
//! multiplicative identity.
//!
//! **Arithmetic discipline: saturating.** RFC 001 §100 permits exactly one
//! device-facing discipline for a type whose ordinary arithmetic cannot be
//! allowed to panic: wrapping silently returns a wrong answer, panicking is
//! forbidden outright, and "range-proven by construction" cannot be claimed
//! for general solver input. [`BaseScalar::add`], [`BaseScalar::sub`],
//! [`BaseScalar::neg`] and [`MetricScalar::abs`] all saturate to
//! `[i32::MIN, i32::MAX]` rather than overflow. [`BaseScalar::mul`] computes
//! the full `i64` product, shifts it back down by `FRAC_BITS` (an arithmetic
//! shift, so it rounds toward negative infinity, not to nearest), and then
//! saturates the result into range.
//!
//! **Tier disposition**, stated per RFC 041 §3.1 and exit criterion 3:
//!
//! | Tier | Disposition |
//! |---|---|
//! | [`BaseScalar`] | **satisfied**, saturating (above) |
//! | [`OrderedScalar`] | **satisfied** — a total order, the ordinary integer extrema (RFC 001 §124) |
//! | [`FiniteScalar`] | **satisfied, vacuously** — see below |
//! | [`DivisibleScalar`] | **satisfied**, with its own overflow disposition (see below) |
//! | [`MetricScalar`] | **satisfied** |
//! | [`AdvancedNumericalScalar`] | **not satisfied** — see below |
//!
//! **`FiniteScalar` is vacuous, and that costs something real.** `Q32` has no
//! NaN and no infinity, so [`FiniteScalar::is_finite`] is the constant `true`,
//! [`FiniteScalar::is_nan`] and [`FiniteScalar::is_infinite`] are the constant
//! `false` — `FiniteScalar`'s own doc comment anticipates exactly this ("for
//! fixed-point / bounded integer-like scalars these may be trivial
//! constants"), so the vacuity itself is not new (RFC 041 Amendment 1). What is
//! genuinely undocumented until now is the **consequence**: the cluster and
//! device constrained kernels call `is_finite()` at **43 sites across 12
//! files** (13 cluster, 11 device) as their only overflow guard, because
//! [`BaseScalar`]'s arithmetic (`add`, `sub`, `mul`, `neg`) has no failure
//! channel of its own — the float protection chain is `overflow → ±inf →
//! is_finite() == false → NumericalDomain`. For `Q32`, every one of those 43
//! calls is a no-op: `is_finite()` always returns `true`, so **the kernels'
//! finiteness guards do not protect this family at all.** `Q32`'s own
//! saturating arithmetic is the only overflow protection a solve over it has;
//! a caller who needs to know whether a value actually saturated must compare
//! against `i32::MAX`/`i32::MIN` directly, which this baseline does not
//! automate (RFC 041 §4: no general library).
//!
//! **`DivisibleScalar::checked_div`'s documented trigger cannot arise here,
//! and `Overflow` is still the honest answer when it would have.** The trait
//! documents `Overflow` for "finite operands whose quotient is non-finite" —
//! a condition that cannot occur for a type with no non-finite values. What
//! *can* occur is the true quotient's magnitude exceeding what `Q32` can
//! represent (e.g. a small divisor driving the result far outside
//! `[i32::MIN, i32::MAX]`). `checked_div` returns `Overflow` for that case
//! too, deliberately not silently saturating a division and reporting `Ok`
//! (RFC 041 §0.3): a caller already treats `Overflow` as "the answer does not
//! fit", which remains true even though the documented float-shaped trigger
//! never applies.
//!
//! **`AdvancedNumericalScalar` is not implemented.** `checked_sqrt`,
//! `checked_ln` and `checked_exp` need real numerical algorithms for a
//! fixed-point representation (a lookup table, CORDIC, or similar) — that is
//! "a general fixed-point arithmetic library", explicitly out of scope for
//! this baseline (RFC 041 §4), and S2's box kernel demonstration needs none of
//! them. S3 decides whether a future checked-arithmetic tier is warranted;
//! this baseline commits to nothing beyond what is implemented here.
//!
//! **A narrower, measured claim: the constrained kernel's usable band is
//! `7..=20`, not the type's full `1..=30` (RFC 043 S6).** This is a
//! *different kind of claim* from the compile-time-enforced range above —
//! `1..=30` is an algebraic invariant of `Q32` itself, true for every
//! instantiation and every kernel; `7..=20` is a **measurement of one
//! corpus against the constrained kernel specifically** (`n = 4`,
//! `m ∈ {1, 2, 3}`, `O(1)` data; `xtask/src/checks/fixed_point_constrained.rs`),
//! not a theorem, and RFC 044 deliberately declines to assert it for exactly
//! that reason — a box-kernel solve at `FRAC_BITS = 24` is sound (RFC 041
//! S2), so the narrower band is a property of this one kernel's
//! Dykstra-projection arithmetic over this one corpus, not of `Q32`. Outside
//! `7..=20` on that corpus, the constrained kernel's `infeasibility_evidence`
//! disagrees with an `f64` solve of the same problem:
//!
//! | `FRAC_BITS` | what disagrees |
//! |---|---|
//! | `1..=3` | missed evidence only — for some instances at the coarse end, constraint rows quantize to all-zero and the solve reports `InvalidInput` instead of a comparison at all |
//! | `4, 5` | fabricated evidence only — one instance each |
//! | `6` | missed evidence only |
//! | `7..=20` | **neither direction (the usable band)** |
//! | `21..=23` | missed evidence only, `4 → 7 → 7` of `16` |
//! | `24..=30` | missed evidence, `15` of `16` |
//!
//! No single precision in `1..=6` disagrees in both directions — the region as a whole
//! touches both, but `1..=3` and `6` only ever miss, and `4, 5` only ever fabricate.
//!
//! A caller who picks `FRAC_BITS = 24` — well inside the type's documented
//! range, and a reasonable choice for data in `[-1, 1]` — silently loses
//! infeasibility detection on 15 of 16 genuinely infeasible problems on this
//! corpus, and nothing in the public API tells them. The `[0.23.1]`
//! `CHANGELOG.md` entry for RFC 043 S2 carries the full before/after table.

use core::cmp::Ordering;

use crate::error::SolverError;
use crate::scalar::{BaseScalar, DivisibleScalar, FiniteScalar, MetricScalar, OrderedScalar};

/// A signed `Q`-format fixed-point number: the raw `i32` `r` represents the
/// real value `r / 2^FRAC_BITS`. See the module doc for the full tier
/// disposition and the saturating arithmetic discipline.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Q32<const FRAC_BITS: u32>(i32);

impl<const FRAC_BITS: u32> Q32<FRAC_BITS> {
    /// `2^FRAC_BITS`, the scaling factor between the raw representation and
    /// the real value it denotes.
    const SCALE: i64 = 1i64 << FRAC_BITS;

    /// Compile-time proof that `FRAC_BITS` is in the documented range
    /// (module doc): `1 <= FRAC_BITS <= 30`. **Declaring this enforces
    /// nothing by itself — only evaluating it does**, which is why every
    /// constructor below forces it with `let () = Self::VALID_FRAC_BITS;`.
    /// `zero`, `one`, `from_raw` and `from_f64` are the only four `pub`
    /// routes that produce a `Self` (`to_raw`/`to_f64` consume one instead),
    /// so forcing it at all four closes the set (RFC 044).
    ///
    /// `Q32<64>` and above already fails earlier and unconditionally, at
    /// `SCALE`'s own shift (`1i64 << FRAC_BITS` overflows the shift amount,
    /// `E0080`, regardless of whether anything reads `SCALE`). This assert
    /// closes the silent window this project found between `31` and `63`,
    /// where `one()` is negative or zero and every arithmetic invariant
    /// quietly fails without it.
    const VALID_FRAC_BITS: () = assert!(
        FRAC_BITS >= 1 && FRAC_BITS <= 30,
        "Q32's FRAC_BITS must satisfy 1 <= FRAC_BITS <= 30"
    );

    /// Builds a value from its raw underlying representation (an integer
    /// count of `2^-FRAC_BITS` units), with no scaling applied.
    #[inline]
    #[must_use]
    pub const fn from_raw(raw: i32) -> Self {
        let () = Self::VALID_FRAC_BITS;
        Self(raw)
    }

    /// The raw underlying representation.
    #[inline]
    #[must_use]
    pub const fn to_raw(self) -> i32 {
        self.0
    }

    /// Builds the value nearest `value`, saturating to the representable
    /// range. Not a [`BaseScalar`] operation and not a general `Q`-format
    /// conversion suite (RFC 041 §4) — a minimal adapter for building test
    /// and demonstration fixtures from ordinary numeric literals, which is
    /// what S2's comparison against an `f64` solve of the same problem needs.
    #[must_use]
    pub fn from_f64(value: f64) -> Self {
        let () = Self::VALID_FRAC_BITS;
        let scaled = value * (Self::SCALE as f64);
        if scaled >= i32::MAX as f64 {
            Self(i32::MAX)
        } else if scaled <= i32::MIN as f64 {
            Self(i32::MIN)
        } else {
            // `f64::round` needs `std`/`libm`; round half away from zero with
            // only the arithmetic `core` already has, since `as` truncates
            // toward zero.
            let nudged = if scaled >= 0.0 {
                scaled + 0.5
            } else {
                scaled - 0.5
            };
            Self(nudged as i32)
        }
    }

    /// The real value this represents, as an `f64`. See [`Q32::from_f64`].
    #[must_use]
    pub fn to_f64(self) -> f64 {
        (self.0 as f64) / (Self::SCALE as f64)
    }
}

impl<const FRAC_BITS: u32> BaseScalar for Q32<FRAC_BITS> {
    #[inline]
    fn zero() -> Self {
        let () = Self::VALID_FRAC_BITS;
        Self(0)
    }

    #[inline]
    fn one() -> Self {
        let () = Self::VALID_FRAC_BITS;
        Self(Self::SCALE as i32)
    }

    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self(self.0.saturating_add(rhs.0))
    }

    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self(self.0.saturating_sub(rhs.0))
    }

    #[inline]
    fn mul(self, rhs: Self) -> Self {
        let product = (self.0 as i64) * (rhs.0 as i64);
        let scaled = product >> FRAC_BITS;
        Self(scaled.clamp(i32::MIN as i64, i32::MAX as i64) as i32)
    }

    #[inline]
    fn neg(self) -> Self {
        Self(self.0.saturating_neg())
    }
}

impl<const FRAC_BITS: u32> PartialOrd for Q32<FRAC_BITS> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl<const FRAC_BITS: u32> OrderedScalar for Q32<FRAC_BITS> {
    #[inline]
    fn min(self, rhs: Self) -> Self {
        Self(self.0.min(rhs.0))
    }

    #[inline]
    fn max(self, rhs: Self) -> Self {
        Self(self.0.max(rhs.0))
    }
}

impl<const FRAC_BITS: u32> FiniteScalar for Q32<FRAC_BITS> {
    #[inline]
    fn is_finite(self) -> bool {
        true
    }

    #[inline]
    fn is_nan(self) -> bool {
        false
    }

    #[inline]
    fn is_infinite(self) -> bool {
        false
    }
}

impl<const FRAC_BITS: u32> DivisibleScalar for Q32<FRAC_BITS> {
    fn checked_div(self, rhs: Self) -> Result<Self, SolverError> {
        if rhs.is_zero() {
            return Err(SolverError::NumericalDomain);
        }
        let scaled_numerator = (self.0 as i64) << FRAC_BITS;
        let quotient = scaled_numerator / (rhs.0 as i64);
        if quotient > i32::MAX as i64 || quotient < i32::MIN as i64 {
            return Err(SolverError::Overflow);
        }
        Ok(Self(quotient as i32))
    }
}

impl<const FRAC_BITS: u32> MetricScalar for Q32<FRAC_BITS> {
    #[inline]
    fn abs(self) -> Self {
        Self(self.0.saturating_abs())
    }

    #[inline]
    fn epsilon() -> Self {
        Self(1)
    }
}

#[cfg(test)]
mod tests;
