//! RFC 041 S2: the device box kernel demonstrated over `Q32`, measured against
//! both the exact separable reference (`checks::exact::exact_optimum`, reused
//! as-is — no new numerical code, per the handoff) and the `f64` solve of the
//! same problem.
//!
//! **Not a gate, and not a reported command.** This is `#[cfg(test)]`-only,
//! exactly as the handoff says is the right home for the comparison: there is
//! nothing here `cargo xtask check` or `release-gate` should run, and no LP- or
//! bench-style figure is pinned (RFC 041 §3.2 asks for a measurement, not a
//! threshold).
//!
//! **The failure mode under hunt is a wrong answer that converges.** A box
//! kernel clamps to its bounds every iteration, so saturation at the bounds
//! may be benign — that is a hypothesis, and this module measures it rather
//! than asserting it, on a random corpus of separable box QPs.

#![cfg(test)]

use loeres::scalar::Q32;
use loeres::{
    BaseScalar, DivisibleScalar, FiniteScalar, OrderedScalar, SolverError, VectorAccess,
    VectorAccessMut,
};
use loeres_backend_static::array::FixedVector;
use loeres_device::problem::ProjectedFirstOrderProblem;

use super::exact::{DenseQp, exact_optimum};

/// Fractional bits for every `Q32` instance in this module: representable
/// magnitude up to `~2_048` with a step of `2^-20 ≈ 9.5e-7`, comfortably
/// covering this corpus's value range (coefficients and centers within
/// `[-4, 4]`) with headroom for the intermediate products `mul` computes.
const FRAC_BITS: u32 = 20;
type Q = Q32<FRAC_BITS>;
const N: usize = 3;

/// A separable box QP: `f(x) = ½ Σ qᵢ(xᵢ − cᵢ)²`, gradient `qᵢ(xᵢ − cᵢ)`,
/// generic over the scalar so the identical problem can be solved in `Q` and
/// in `f64` (the second reference).
struct BoxQuadratic<S> {
    lower: FixedVector<S, N>,
    upper: FixedVector<S, N>,
    quadratic_diag: [S; N],
    center: [S; N],
    step_scale: S,
}

impl<S: BaseScalar + FiniteScalar + OrderedScalar + DivisibleScalar>
    ProjectedFirstOrderProblem<S, N> for BoxQuadratic<S>
{
    type Bounds = FixedVector<S, N>;

    fn validate_boundary(&self) -> Result<(), SolverError> {
        for i in 0..N {
            let lo = self.lower.get(i)?;
            let hi = self.upper.get(i)?;
            if !lo.is_finite() || !hi.is_finite() {
                return Err(SolverError::NonFiniteInput);
            }
            if lo > hi {
                return Err(SolverError::InvalidInput);
            }
        }
        Ok(())
    }

    fn lower_bound(&self) -> &Self::Bounds {
        &self.lower
    }

    fn upper_bound(&self) -> &Self::Bounds {
        &self.upper
    }

    fn step_scale(&self) -> S {
        self.step_scale
    }

    fn gradient_at(
        &self,
        x: &FixedVector<S, N>,
        grad: &mut FixedVector<S, N>,
    ) -> Result<(), SolverError> {
        for i in 0..N {
            let xi = x.get(i)?;
            let gi = self.quadratic_diag[i].mul(xi.sub(self.center[i]));
            grad.set(i, gi)?;
        }
        Ok(())
    }

    fn objective_at(&self, x: &FixedVector<S, N>) -> Result<S, SolverError> {
        let half = S::one().add(S::one()).checked_recip().unwrap_or(S::one());
        let mut total = S::zero();
        for i in 0..N {
            let d = x.get(i)?.sub(self.center[i]);
            total = total.add(half.mul(self.quadratic_diag[i]).mul(d).mul(d));
        }
        Ok(total)
    }
}

#[cfg(test)]
mod tests;
