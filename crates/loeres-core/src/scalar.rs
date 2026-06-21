//! Stratified scalar capability tiers for `loeres-core` (RFC 001).
//!
//! Rather than one monolithic `Scalar` trait, Loeres splits scalar capabilities
//! into six tiers so an algorithm states the *smallest* numerical contract it
//! actually needs. Ordering, division, metric comparison, and transcendental
//! functions are opt-in tiers, not baseline requirements — which keeps
//! order-free, fixed-point, and integer-like numeric types valid at the base.
//!
//! ```text
//! BaseScalar             : Copy + Clone + PartialEq + Sized
//! OrderedScalar          : BaseScalar + PartialOrd
//! FiniteScalar           : BaseScalar
//! DivisibleScalar        : BaseScalar
//! MetricScalar           : OrderedScalar
//! AdvancedNumericalScalar: DivisibleScalar + MetricScalar
//! ```
//!
//! These traits are for monomorphized, static-dispatch use; they must not be
//! used behind `dyn` in core or device kernels. No tier references `f32`/`f64`,
//! `std`, `alloc`, formatting, or any backend type.

use crate::error::SolverError;

mod primitive;

/// Tier 1 — the minimum algebraic vocabulary to represent optimization data.
///
/// Requires only equality (for zero-testing), **not** `PartialOrd` and **not**
/// `Debug`. Arithmetic is method-based (so the public contract stays under
/// Loeres control) and is assumed panic-free and total over the implementation's
/// documented operating range.
pub trait BaseScalar: Copy + Clone + PartialEq + Sized {
    /// The additive identity.
    fn zero() -> Self;
    /// The multiplicative identity.
    fn one() -> Self;
    /// `self + rhs`.
    fn add(self, rhs: Self) -> Self;
    /// `self - rhs`.
    fn sub(self, rhs: Self) -> Self;
    /// `self * rhs`.
    fn mul(self, rhs: Self) -> Self;
    /// Additive negation.
    fn neg(self) -> Self;

    /// `true` iff `self` equals [`BaseScalar::zero`]. Relies only on `PartialEq`;
    /// for primitive floats `-0.0 == 0.0`, so `(-0.0).is_zero()` is `true`.
    #[inline]
    fn is_zero(self) -> bool {
        self == Self::zero()
    }
}

/// Tier 2 — ordering plus Loeres-defined extrema and clamp.
///
/// The bound a solver requires for projection, comparison, and box constraints.
/// Kept separate from [`BaseScalar`] so order-free numeric types stay valid at
/// the base tier and so Loeres controls floating-point extrema semantics.
pub trait OrderedScalar: BaseScalar + PartialOrd {
    /// The lesser of `self` and `rhs`. No default body, so each implementation
    /// pins its NaN behavior. For floating-point scalars this is
    /// **NaN-propagating**: if either operand is NaN, the result is NaN
    /// (unlike `f32::min` / `f64::min`, which ignore NaN).
    fn min(self, rhs: Self) -> Self;

    /// The greater of `self` and `rhs`, under the same NaN contract as
    /// [`OrderedScalar::min`].
    fn max(self, rhs: Self) -> Self;

    /// Clamp `self` into `[lo, hi]`. The caller must guarantee `lo <= hi`
    /// (validated at the solve boundary). Never panics: if the precondition is
    /// violated this returns `hi`, which is panic-avoidance, not a projection
    /// semantics callers may rely on.
    #[inline]
    fn clamp(self, lo: Self, hi: Self) -> Self {
        self.max(lo).min(hi)
    }
}

/// Tier 3 — boundary validation for non-finite values.
///
/// Implemented for any scalar used by public solve entrypoints that reject
/// non-finite inputs. For fixed-point / bounded integer-like scalars these may
/// be trivial constants (`true`, `false`, `false`).
pub trait FiniteScalar: BaseScalar {
    /// `true` iff `self` is finite (neither NaN nor infinite).
    fn is_finite(self) -> bool;
    /// `true` iff `self` is NaN.
    fn is_nan(self) -> bool;
    /// `true` iff `self` is positive or negative infinity.
    fn is_infinite(self) -> bool;
}

/// Tier 4 — guarded division.
///
/// Division is never an unchecked baseline operation: every public division path
/// returns a structured error rather than panicking or producing a silent
/// undefined value. Guards the exact-zero denominator and non-finite results;
/// near-zero conditioning is a solver-level [`MetricScalar`] concern, so this
/// tier depends only on [`BaseScalar`].
pub trait DivisibleScalar: BaseScalar {
    /// `self / rhs`, or an error. A zero denominator returns
    /// [`SolverError::NumericalDomain`]; finite operands whose quotient is
    /// non-finite return [`SolverError::Overflow`].
    fn checked_div(self, rhs: Self) -> Result<Self, SolverError>;

    /// `1 / self`, via [`DivisibleScalar::checked_div`].
    #[inline]
    fn checked_recip(self) -> Result<Self, SolverError> {
        Self::one().checked_div(self)
    }
}

/// Tier 5 — magnitude and tolerance comparison for convergence checks.
///
/// Extends [`OrderedScalar`] because tolerance comparison is inherently ordered,
/// so a `MetricScalar` bound implies `OrderedScalar` (and `BaseScalar`).
pub trait MetricScalar: OrderedScalar {
    /// Absolute value. Must be panic-free.
    fn abs(self) -> Self;

    /// The scalar type's default numerical tolerance unit for Loeres algorithms
    /// (not necessarily primitive machine epsilon). Solvers may require explicit
    /// tolerance configuration instead. *Provisional name (RFC 001 §3.6).*
    fn epsilon() -> Self;

    /// `|self| <= tolerance`. The caller must pass a finite, nonnegative
    /// `tolerance` (validated by solver configuration, not here).
    #[inline]
    fn lte_tolerance(self, tolerance: Self) -> bool {
        self.abs() <= tolerance
    }
}

/// Tier 6 — expensive, solver-specific functions (barrier methods, norms).
///
/// **Forbidden as a baseline bound** for core access traits, problem
/// representations, or device entrypoints unless a concrete algorithm requires
/// it. Not implemented for primitive floats in baseline core (`no_std` targets
/// have no built-in transcendental functions); such impls require the `libm`
/// feature or a later adapter RFC.
pub trait AdvancedNumericalScalar: DivisibleScalar + MetricScalar {
    /// `sqrt(self)`, or [`SolverError::NumericalDomain`] for `self < 0`.
    fn checked_sqrt(self) -> Result<Self, SolverError>;
    /// `ln(self)`, or [`SolverError::NumericalDomain`] for `self <= 0`.
    fn checked_ln(self) -> Result<Self, SolverError>;
    /// `exp(self)`, or [`SolverError::Overflow`] on overflow.
    fn checked_exp(self) -> Result<Self, SolverError>;
}
