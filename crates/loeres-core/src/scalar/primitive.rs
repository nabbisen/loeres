//! Baseline primitive scalar implementations for `f32` and `f64` (RFC 001 §3.10).
//!
//! These add no dependency and no target-specific behavior. `AdvancedNumericalScalar`
//! is intentionally **not** implemented here: transcendental functions are not
//! baseline core work and require the `libm` feature or a later adapter.
//!
//! Inherent float methods take precedence over the trait methods of the same
//! name, so `self.is_nan()` / `self.abs()` inside these impls call the inherent
//! float methods (no recursion). `min` / `max` are written out explicitly to
//! honor the NaN-propagating contract — they must not delegate to `f32::min` /
//! `f64::min`, which ignore NaN.

use super::{BaseScalar, DivisibleScalar, FiniteScalar, MetricScalar, OrderedScalar};
use crate::error::SolverError;

macro_rules! impl_float_scalar {
    ($t:ty) => {
        impl BaseScalar for $t {
            #[inline]
            fn zero() -> Self {
                0.0
            }
            #[inline]
            fn one() -> Self {
                1.0
            }
            #[inline]
            fn add(self, rhs: Self) -> Self {
                self + rhs
            }
            #[inline]
            fn sub(self, rhs: Self) -> Self {
                self - rhs
            }
            #[inline]
            fn mul(self, rhs: Self) -> Self {
                self * rhs
            }
            #[inline]
            fn neg(self) -> Self {
                -self
            }
        }

        impl OrderedScalar for $t {
            #[inline]
            fn min(self, rhs: Self) -> Self {
                // NaN-propagating: return the NaN operand if either is NaN.
                if self.is_nan() {
                    self
                } else if rhs.is_nan() {
                    rhs
                } else if self < rhs {
                    self
                } else {
                    rhs
                }
            }
            #[inline]
            fn max(self, rhs: Self) -> Self {
                if self.is_nan() {
                    self
                } else if rhs.is_nan() {
                    rhs
                } else if self > rhs {
                    self
                } else {
                    rhs
                }
            }
        }

        impl FiniteScalar for $t {
            #[inline]
            fn is_finite(self) -> bool {
                self.is_finite()
            }
            #[inline]
            fn is_nan(self) -> bool {
                self.is_nan()
            }
            #[inline]
            fn is_infinite(self) -> bool {
                self.is_infinite()
            }
        }

        impl DivisibleScalar for $t {
            #[inline]
            fn checked_div(self, rhs: Self) -> Result<Self, SolverError> {
                if rhs.is_zero() {
                    return Err(SolverError::NumericalDomain);
                }
                let q = self / rhs;
                // Reject a non-finite quotient (finite-operand overflow to
                // infinity, or a non-finite operand that should have been
                // rejected at the boundary) rather than returning Ok(inf/NaN).
                if !q.is_finite() {
                    return Err(SolverError::Overflow);
                }
                Ok(q)
            }
        }

        impl MetricScalar for $t {
            #[inline]
            fn abs(self) -> Self {
                self.abs()
            }
            #[inline]
            fn epsilon() -> Self {
                <$t>::EPSILON
            }
        }
    };
}

impl_float_scalar!(f32);
impl_float_scalar!(f64);
