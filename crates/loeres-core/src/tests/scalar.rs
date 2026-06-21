//! Tests for the RFC 001 stratified scalar tiers.
//!
//! These validate the **design specification**: the base algebra and identities
//! (§3.2), the NaN-propagating extrema and clamp contract (§3.3/§6.4), the
//! guarded-division policy (§3.5/§6.3), the metric/tolerance contract (§3.6),
//! and the scalar laws (§6.5). Type-agnostic laws run generically over both
//! primitives; NaN behavior is checked on concrete `f32`/`f64`.

use crate::error::SolverError;
use crate::scalar::{BaseScalar, DivisibleScalar, FiniteScalar, MetricScalar, OrderedScalar};

// ---------------------------------------------------------------- base algebra

/// Generic base-algebra laws (§3.2, §6.5), exercised for each primitive.
fn base_laws<S: BaseScalar + core::fmt::Debug>() {
    assert!(S::zero().is_zero());
    assert!(!S::one().is_zero());
    assert_eq!(S::zero().add(S::one()), S::one()); // additive identity
    assert_eq!(S::one().mul(S::one()), S::one()); // multiplicative identity
    let two = S::one().add(S::one());
    assert_eq!(two.sub(S::one()), S::one());
    assert_eq!(S::one().neg().neg(), S::one()); // double negation
    assert!(S::one().add(S::one().neg()).is_zero()); // x + (-x) == 0
}

#[test]
fn base_algebra_holds_for_primitives() {
    base_laws::<f32>();
    base_laws::<f64>();
}

#[test]
fn negative_zero_is_zero() {
    // §3.10: is_zero uses PartialEq, and -0.0 == 0.0 for primitive floats.
    assert!((-0.0f32).is_zero());
    assert!((-0.0f64).is_zero());
}

// -------------------------------------------------------------- ordering / NaN

/// Generic ordering laws for finite, non-NaN operands (§6.5).
fn ordering_laws<S: OrderedScalar + core::fmt::Debug>(a: S, b: S) {
    assert_eq!(a.min(b), b.min(a)); // commutative
    assert_eq!(a.max(b), b.max(a));
    assert_eq!(a.min(a), a); // idempotent
    assert_eq!(a.max(a), a);
    // min/max agree with the total order.
    let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
    assert_eq!(a.min(b), lo);
    assert_eq!(a.max(b), hi);
}

#[test]
fn ordering_matches_total_order() {
    ordering_laws::<f32>(2.0, 5.0);
    ordering_laws::<f32>(5.0, 2.0);
    ordering_laws::<f64>(-3.0, 7.5);
}

#[test]
fn extrema_propagate_nan() {
    // §3.3/§6.4: NaN with any operand yields NaN (unlike `f32::min`/`f64::min`).
    // UFCS is required: on a concrete float, `x.min(y)` would call the inherent
    // NaN-ignoring method, not the trait method exercised here.
    assert!(OrderedScalar::min(f64::NAN, 1.0).is_nan());
    assert!(OrderedScalar::min(1.0_f64, f64::NAN).is_nan());
    assert!(OrderedScalar::max(f64::NAN, 1.0).is_nan());
    assert!(OrderedScalar::max(1.0_f64, f64::NAN).is_nan());
    assert!(OrderedScalar::min(f32::NAN, 1.0).is_nan());
    assert!(OrderedScalar::max(1.0_f32, f32::NAN).is_nan());
}

#[test]
fn clamp_within_bounds_and_panic_free() {
    // §6.4: clamp returns a value in [lo, hi] for lo <= hi (UFCS hits the trait).
    assert_eq!(OrderedScalar::clamp(5.0_f64, 0.0, 10.0), 5.0);
    assert_eq!(OrderedScalar::clamp(-2.0_f64, 0.0, 10.0), 0.0);
    assert_eq!(OrderedScalar::clamp(99.0_f64, 0.0, 10.0), 10.0);
    // Box projection of an interior / exterior point.
    assert_eq!(OrderedScalar::clamp(3.0_f32, 1.0, 4.0), 3.0);
    assert_eq!(OrderedScalar::clamp(0.0_f32, 1.0, 4.0), 1.0);
}

#[test]
fn clamp_with_inverted_bounds_returns_hi_without_panic() {
    // §6.4: lo > hi returns hi (panic-avoidance, not valid projection). The
    // trait clamp never panics, unlike the inherent `f64::clamp`.
    assert_eq!(OrderedScalar::clamp(5.0_f64, 10.0, 0.0), 0.0);
}

#[test]
fn clamp_propagates_nan() {
    // §6.4: NaN in self, lo, or hi propagates (clamp uses trait min/max).
    assert!(OrderedScalar::clamp(f64::NAN, 0.0, 1.0).is_nan());
    assert!(OrderedScalar::clamp(0.5_f64, f64::NAN, 1.0).is_nan());
    assert!(OrderedScalar::clamp(0.5_f64, 0.0, f64::NAN).is_nan());
}

// --------------------------------------------------------------- finite checks

#[test]
fn finite_checks_are_mutually_exclusive() {
    // §6.5: is_finite is mutually exclusive with is_nan and is_infinite.
    for x in [
        0.0_f64,
        -1.5,
        1e300,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
    ] {
        let f = FiniteScalar::is_finite(x);
        let n = FiniteScalar::is_nan(x);
        let i = FiniteScalar::is_infinite(x);
        assert_eq!([f, n, i].into_iter().filter(|b| *b).count(), 1, "x={x}");
    }
    assert!(FiniteScalar::is_finite(0.0_f32));
    assert!(FiniteScalar::is_nan(f32::NAN));
    assert!(FiniteScalar::is_infinite(f32::INFINITY));
}

// ------------------------------------------------------------ guarded division

#[test]
fn division_by_zero_is_an_error() {
    // §6.3
    assert_eq!(1.0_f64.checked_div(0.0), Err(SolverError::NumericalDomain));
    assert_eq!(1.0_f64.checked_div(-0.0), Err(SolverError::NumericalDomain));
    assert_eq!(1.0_f64.checked_recip().map(|_| ()), Ok(()));
    assert_eq!(0.0_f64.checked_recip(), Err(SolverError::NumericalDomain));
}

#[test]
fn finite_nonzero_division_succeeds() {
    // §6.3
    assert_eq!(6.0_f64.checked_div(2.0), Ok(3.0));
    assert_eq!(1.0_f32.checked_div(4.0), Ok(0.25));
}

#[test]
fn division_overflowing_to_non_finite_is_an_error() {
    // §6.3: finite operands whose quotient is non-finite return Err, not Ok(inf).
    assert_eq!(
        f64::MAX.checked_div(f64::MIN_POSITIVE),
        Err(SolverError::Overflow)
    );
}

#[test]
fn division_with_non_finite_operands_does_not_yield_ok_non_finite() {
    // Architect review §5.1: a non-finite operand that escaped boundary
    // validation must not produce `Ok(NaN/inf)`. The coarse policy classifies a
    // non-finite quotient as `Overflow`; pin it so the behavior is intentional.
    // (Public solve entrypoints reject non-finite inputs earlier via FiniteScalar.)
    assert!(f64::NAN.checked_div(2.0).is_err());
    assert!(2.0_f64.checked_div(f64::NAN).is_err());
    assert!(f64::INFINITY.checked_div(2.0).is_err());
    // 2.0 / inf == 0.0 is finite, so this one legitimately succeeds; assert it
    // explicitly rather than leaving the boundary ambiguous.
    assert_eq!(2.0_f64.checked_div(f64::INFINITY), Ok(0.0));
}

#[test]
fn recip_agrees_with_one_over_x() {
    // §6.5: checked_recip(x) and checked_div(one(), x) agree for nonzero finite x.
    for x in [1.0_f64, -2.0, 0.5, 1234.5] {
        assert_eq!(x.checked_recip(), f64::one().checked_div(x));
    }
}

// ------------------------------------------------------------------ metric tier

#[test]
fn abs_is_nonnegative_for_finite_values() {
    // §6.5
    for x in [0.0_f64, -3.5, 7.0, -1e9] {
        assert!(MetricScalar::abs(x) >= 0.0);
    }
    assert_eq!(MetricScalar::abs(-4.0_f32), 4.0);
}

#[test]
fn lte_tolerance_uses_magnitude() {
    // §3.6: |self| <= tolerance, exercised with finite nonnegative tolerance.
    assert!((0.05_f64).lte_tolerance(0.1));
    assert!((-0.05_f64).lte_tolerance(0.1));
    assert!(!(0.2_f64).lte_tolerance(0.1));
}

#[test]
fn epsilon_is_positive() {
    assert!(<f64 as MetricScalar>::epsilon() > 0.0);
    assert!(<f32 as MetricScalar>::epsilon() > 0.0);
}
