use super::Q32;
use crate::error::SolverError;
use crate::scalar::{BaseScalar, DivisibleScalar, FiniteScalar, MetricScalar, OrderedScalar};

type Q = Q32<16>;

#[test]
fn zero_one_and_round_trip_through_f64() {
    assert_eq!(Q::zero().to_f64(), 0.0);
    assert_eq!(Q::one().to_f64(), 1.0);
    assert_eq!(Q::from_f64(0.5).to_f64(), 0.5);
    assert_eq!(Q::from_f64(-2.25).to_f64(), -2.25);
}

#[test]
fn add_sub_mul_neg_match_float_arithmetic_at_this_precision() {
    let a = Q::from_f64(1.5);
    let b = Q::from_f64(0.25);
    assert_eq!(a.add(b).to_f64(), 1.75);
    assert_eq!(a.sub(b).to_f64(), 1.25);
    assert_eq!(a.mul(b).to_f64(), 0.375);
    assert_eq!(a.neg().to_f64(), -1.5);
}

#[test]
fn mul_rounds_toward_negative_infinity_not_to_nearest() {
    // raw 3 at FRAC_BITS=16 is 3/65536; squaring gives 9/2^32, which shifted
    // back down by 16 bits truncates (arithmetic shift) rather than rounding.
    let tiny = Q::from_raw(3);
    let product = tiny.mul(tiny);
    assert_eq!(product.to_raw(), 0, "{product:?}");
}

#[test]
fn add_sub_and_neg_saturate_rather_than_overflow() {
    let max = Q::from_raw(i32::MAX);
    let min = Q::from_raw(i32::MIN);
    assert_eq!(max.add(Q::one()).to_raw(), i32::MAX);
    assert_eq!(min.sub(Q::one()).to_raw(), i32::MIN);
    // The classic two's-complement trap: -i32::MIN overflows i32::MAX by one.
    // `abs`/`neg` must saturate, not panic.
    assert_eq!(min.neg().to_raw(), i32::MAX);
}

#[test]
fn abs_of_the_most_negative_value_saturates_and_does_not_panic() {
    // RFC 041 handoff §1: "MetricScalar::abs is the trap: in two's complement
    // abs(MIN) overflows." Exercised directly, not merely asserted.
    let min = Q::from_raw(i32::MIN);
    let result = min.abs();
    assert_eq!(result.to_raw(), i32::MAX, "{result:?}");
}

#[test]
fn mul_saturates_at_the_extremes_instead_of_wrapping() {
    let max = Q::from_raw(i32::MAX);
    let two = Q::from_f64(2.0);
    let product = max.mul(two);
    assert_eq!(product.to_raw(), i32::MAX, "{product:?}");
}

#[test]
fn finite_scalar_is_vacuously_true_false_false() {
    let values = [
        Q::zero(),
        Q::one(),
        Q::from_raw(i32::MIN),
        Q::from_raw(i32::MAX),
    ];
    for v in values {
        assert!(v.is_finite(), "{v:?}");
        assert!(!v.is_nan(), "{v:?}");
        assert!(!v.is_infinite(), "{v:?}");
    }
}

#[test]
fn checked_div_matches_float_division_at_this_precision() {
    let a = Q::from_f64(1.0);
    let b = Q::from_f64(4.0);
    let q = a.checked_div(b).expect("4 is nonzero");
    assert_eq!(q.to_f64(), 0.25);
}

#[test]
fn checked_div_by_zero_is_numerical_domain() {
    assert_eq!(
        Q::one().checked_div(Q::zero()),
        Err(SolverError::NumericalDomain)
    );
}

#[test]
fn checked_div_reports_overflow_when_the_true_quotient_does_not_fit() {
    // No non-finite value can arise for this family (there is no infinity to
    // produce), but the true quotient's magnitude can still exceed what Q32
    // represents, and that must still be `Overflow`, not a silent saturation
    // reported as `Ok` (RFC 041 handoff §0.3).
    let numerator = Q::from_raw(i32::MAX);
    let tiny_divisor = Q::from_raw(1);
    assert_eq!(
        numerator.checked_div(tiny_divisor),
        Err(SolverError::Overflow)
    );
}

#[test]
fn checked_recip_is_one_over_self_via_checked_div() {
    let four = Q::from_f64(4.0);
    assert_eq!(four.checked_recip().unwrap().to_f64(), 0.25);
}

#[test]
fn ordered_scalar_min_max_and_clamp_are_the_ordinary_total_order_extrema() {
    let a = Q::from_f64(-1.0);
    let b = Q::from_f64(2.0);
    assert_eq!(a.min(b), a);
    assert_eq!(a.max(b), b);
    assert_eq!(Q::from_f64(5.0).clamp(a, b), b);
    assert_eq!(Q::from_f64(-5.0).clamp(a, b), a);
    // The documented panic-avoidance behaviour when the caller violates
    // `lo <= hi`: returns `hi`, not a panic.
    assert_eq!(Q::zero().clamp(b, a), a);
}

#[test]
fn metric_scalar_epsilon_is_the_smallest_representable_positive_step() {
    let eps = Q::epsilon();
    assert_eq!(eps.to_raw(), 1);
    assert!(eps.to_f64() > 0.0);
    assert!(Q::zero().lte_tolerance(eps));
}

#[test]
fn from_f64_saturates_rather_than_wrapping_at_the_representable_edge() {
    assert_eq!(Q::from_f64(1e9).to_raw(), i32::MAX);
    assert_eq!(Q::from_f64(-1e9).to_raw(), i32::MIN);
}

#[test]
fn a_different_frac_bits_is_a_genuinely_different_type_and_scale() {
    type Coarse = Q32<4>;
    let one = Coarse::one();
    assert_eq!(one.to_raw(), 16);
    assert_eq!(one.to_f64(), 1.0);
}

/// `FRAC_BITS` at both documented boundaries compiles and behaves ordinarily
/// — `VALID_FRAC_BITS` is forced on every construction path and does not
/// disturb a valid instantiation (RFC 044).
///
/// **`31` and above is a compile error by construction, not something an
/// ordinary `#[test]` can exercise here** — a failing const assert is
/// `E0080` at compile time, not a runtime panic, so `#[should_panic]` cannot
/// catch it and this project takes no new dependency (`trybuild` or
/// similar, RFC 026) to assert on compile failures. Verified once, directly
/// against this tree rather than taken from RFC 044's own scratch
/// reproduction: `Q32::<32>::VALID_FRAC_BITS` fails with
///
/// ```text
/// error[E0080]: evaluation panicked: Q32's FRAC_BITS must satisfy 1 <= FRAC_BITS <= 30
///    --> crates/loeres/src/scalar/fixed_point.rs:115:37
///     | evaluation of `scalar::fixed_point::Q32::<32>::VALID_FRAC_BITS` failed here
/// ```
#[test]
fn both_documented_boundaries_compile_and_behave() {
    type Lowest = Q32<1>;
    assert_eq!(Lowest::one().to_raw(), 2);
    assert_eq!(Lowest::one().to_f64(), 1.0);
    assert!(Lowest::one() > Lowest::zero());

    type Highest = Q32<30>;
    assert_eq!(Highest::one().to_raw(), 1 << 30);
    assert_eq!(Highest::one().to_f64(), 1.0);
    assert!(Highest::one() > Highest::zero());
}
