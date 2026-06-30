use super::*;
use crate::runtime::ClusterValidationPolicy;
use crate::solve::ClusterExecutionContext;
use crate::{ClusterCancellationToken, ClusterSolveConfig, solve_batch};
use loeres::validation::{TrustToken, TrustedByCaller};
use loeres::{VectorAccess, VectorAccessMut};

/// Separable quadratic `f(x) = ½ Σ wᵢ (xᵢ − cᵢ)²`, gradient `wᵢ(xᵢ − cᵢ)`,
/// unconstrained optimum `cᵢ`, box-constrained optimum `clamp(cᵢ, loᵢ, hiᵢ)`.
struct Quadratic {
    weights: Vec<f64>,
    centers: Vec<f64>,
    lo: DenseVector<f64>,
    hi: DenseVector<f64>,
    alpha: f64,
}

impl ClusterProjectedFirstOrderProblem<f64> for Quadratic {
    fn dimension(&self) -> usize {
        self.centers.len()
    }
    fn bounds(&self) -> (&DenseVector<f64>, &DenseVector<f64>) {
        (&self.lo, &self.hi)
    }
    fn gradient_at(
        &self,
        x: &DenseVector<f64>,
        grad: &mut DenseVector<f64>,
    ) -> Result<(), SolverError> {
        for i in 0..self.dimension() {
            let gi = self.weights[i] * (x.get(i)? - self.centers[i]);
            grad.set(i, gi)?;
        }
        Ok(())
    }
    fn step_scale(&self) -> f64 {
        self.alpha
    }
}

fn dv(v: &[f64]) -> DenseVector<f64> {
    DenseVector::from_vec(v.to_vec()).unwrap()
}

fn ctx(policy: ClusterValidationPolicy) -> ClusterExecutionContext {
    ClusterExecutionContext::new(ClusterCancellationToken::new(), 0, policy)
}

fn cfg(max_iterations: u32, tolerance: f64) -> ProjectedFirstOrderConfig<f64> {
    ProjectedFirstOrderConfig {
        max_iterations,
        tolerance,
    }
}

#[test]
fn converges_to_unconstrained_optimum() {
    let p = Quadratic {
        weights: vec![1.0, 1.0],
        centers: vec![3.0, -2.0],
        lo: dv(&[-10.0, -10.0]),
        hi: dv(&[10.0, 10.0]),
        alpha: 0.5,
    };
    let mut x = dv(&[0.0, 0.0]);
    let mut ws = ClusterProjectedFirstOrderWorkspace::new(2).unwrap();
    let rec = solve_projected_first_order_dyn(
        &p,
        &mut x,
        &mut ws,
        &cfg(1000, 1e-9),
        &ctx(ClusterValidationPolicy::ValidateAllInputs),
    )
    .unwrap();
    assert!(rec.report.status().is_converged());
    assert!((x.get(0).unwrap() - 3.0).abs() < 1e-6);
    assert!((x.get(1).unwrap() + 2.0).abs() < 1e-6);
    assert_eq!(rec.finite, ProjectedFirstOrderFiniteEvidence::Scanned);
    assert!(rec.checked_scope.contains(ValidationScope::FINITE));
}

#[test]
fn projects_to_bound_when_optimum_outside() {
    let p = Quadratic {
        weights: vec![1.0],
        centers: vec![100.0],
        lo: dv(&[-1.0]),
        hi: dv(&[5.0]),
        alpha: 0.5,
    };
    let mut x = dv(&[0.0]);
    let mut ws = ClusterProjectedFirstOrderWorkspace::new(1).unwrap();
    let rec = solve_projected_first_order_dyn(
        &p,
        &mut x,
        &mut ws,
        &cfg(1000, 1e-9),
        &ctx(ClusterValidationPolicy::ValidateAllInputs),
    )
    .unwrap();
    assert!(rec.report.status().is_converged());
    assert!((x.get(0).unwrap() - 5.0).abs() < 1e-6);
}

#[test]
fn not_converged_at_cap_is_solved_status() {
    let p = Quadratic {
        weights: vec![1.0],
        centers: vec![1000.0],
        lo: dv(&[-1.0e6]),
        hi: dv(&[1.0e6]),
        alpha: 0.001,
    };
    let mut x = dv(&[0.0]);
    let mut ws = ClusterProjectedFirstOrderWorkspace::new(1).unwrap();
    let rec = solve_projected_first_order_dyn(
        &p,
        &mut x,
        &mut ws,
        &cfg(2, 1e-12),
        &ctx(ClusterValidationPolicy::ValidateAllInputs),
    )
    .unwrap();
    assert!(!rec.report.status().is_converged());
    assert_eq!(rec.report.iterations_executed(), 2);
}

#[test]
fn first_step_convergence_counts_one() {
    let p = Quadratic {
        weights: vec![1.0],
        centers: vec![2.0],
        lo: dv(&[-10.0]),
        hi: dv(&[10.0]),
        alpha: 0.5,
    };
    let mut x = dv(&[2.0]); // already at optimum
    let mut ws = ClusterProjectedFirstOrderWorkspace::new(1).unwrap();
    let rec = solve_projected_first_order_dyn(
        &p,
        &mut x,
        &mut ws,
        &cfg(1, 1e-9),
        &ctx(ClusterValidationPolicy::ValidateAllInputs),
    )
    .unwrap();
    assert!(rec.report.status().is_converged());
    assert_eq!(rec.report.iterations_executed(), 1); // converged_early(1), not at_cap
}

struct NanGradient {
    lo: DenseVector<f64>,
    hi: DenseVector<f64>,
}
impl ClusterProjectedFirstOrderProblem<f64> for NanGradient {
    fn dimension(&self) -> usize {
        1
    }
    fn bounds(&self) -> (&DenseVector<f64>, &DenseVector<f64>) {
        (&self.lo, &self.hi)
    }
    fn gradient_at(
        &self,
        _x: &DenseVector<f64>,
        grad: &mut DenseVector<f64>,
    ) -> Result<(), SolverError> {
        grad.set(0, f64::NAN)?;
        Ok(())
    }
    fn step_scale(&self) -> f64 {
        0.5
    }
}

#[test]
fn nan_gradient_maps_to_numerical_domain_even_under_trust() {
    let p = NanGradient {
        lo: dv(&[-1.0]),
        hi: dv(&[1.0]),
    };
    let mut x = dv(&[0.0]);
    let mut ws = ClusterProjectedFirstOrderWorkspace::new(1).unwrap();
    let err = solve_projected_first_order_dyn(
        &p,
        &mut x,
        &mut ws,
        &cfg(10, 1e-9),
        &ctx(ClusterValidationPolicy::ValidateAllInputs),
    )
    .unwrap_err();
    assert!(matches!(err, SolverError::NumericalDomain));

    let trust = TrustedByCaller::caller_assertion(ValidationScope::ALL, TrustToken::new(1), None);
    let mut x2 = dv(&[0.0]);
    let mut ws2 = ClusterProjectedFirstOrderWorkspace::new(1).unwrap();
    let err2 = solve_projected_first_order_dyn(
        &p,
        &mut x2,
        &mut ws2,
        &cfg(10, 1e-9),
        &ctx(ClusterValidationPolicy::TrustedByCaller(trust)),
    )
    .unwrap_err();
    assert!(matches!(err2, SolverError::NumericalDomain));
}

#[test]
fn lo_greater_than_hi_is_invalid_input() {
    let p = Quadratic {
        weights: vec![1.0],
        centers: vec![0.0],
        lo: dv(&[5.0]),
        hi: dv(&[-5.0]),
        alpha: 0.5,
    };
    let mut x = dv(&[0.0]);
    let mut ws = ClusterProjectedFirstOrderWorkspace::new(1).unwrap();
    let err = solve_projected_first_order_dyn(
        &p,
        &mut x,
        &mut ws,
        &cfg(10, 1e-9),
        &ctx(ClusterValidationPolicy::ValidateAllInputs),
    )
    .unwrap_err();
    assert!(matches!(err, SolverError::InvalidInput));
}

#[test]
fn dimension_mismatch_is_reported() {
    let p = Quadratic {
        weights: vec![1.0, 1.0],
        centers: vec![0.0, 0.0],
        lo: dv(&[-1.0, -1.0]),
        hi: dv(&[1.0, 1.0]),
        alpha: 0.5,
    };
    let mut x = dv(&[0.0]); // dim 1 vs problem dim 2
    let mut ws = ClusterProjectedFirstOrderWorkspace::new(2).unwrap();
    let err = solve_projected_first_order_dyn(
        &p,
        &mut x,
        &mut ws,
        &cfg(10, 1e-9),
        &ctx(ClusterValidationPolicy::ValidateAllInputs),
    )
    .unwrap_err();
    assert!(matches!(err, SolverError::DimensionMismatch { .. }));
}

#[test]
fn cancellation_before_loop_returns_cancelled() {
    let cancel = ClusterCancellationToken::new();
    cancel.cancel();
    let context =
        ClusterExecutionContext::new(cancel, 0, ClusterValidationPolicy::ValidateAllInputs);
    let p = Quadratic {
        weights: vec![1.0],
        centers: vec![1.0],
        lo: dv(&[-1.0]),
        hi: dv(&[1.0]),
        alpha: 0.5,
    };
    let mut x = dv(&[0.0]);
    let mut ws = ClusterProjectedFirstOrderWorkspace::new(1).unwrap();
    let err =
        solve_projected_first_order_dyn(&p, &mut x, &mut ws, &cfg(10, 1e-9), &context).unwrap_err();
    assert!(matches!(err, SolverError::Cancelled));
}

#[test]
fn trusted_finite_records_trusted_evidence() {
    let p = Quadratic {
        weights: vec![1.0],
        centers: vec![2.0],
        lo: dv(&[-10.0]),
        hi: dv(&[10.0]),
        alpha: 0.5,
    };
    let trust =
        TrustedByCaller::caller_assertion(ValidationScope::ALL, TrustToken::new(7), Some("t"));
    let mut x = dv(&[0.0]);
    let mut ws = ClusterProjectedFirstOrderWorkspace::new(1).unwrap();
    let rec = solve_projected_first_order_dyn(
        &p,
        &mut x,
        &mut ws,
        &cfg(1000, 1e-9),
        &ctx(ClusterValidationPolicy::TrustedByCaller(trust)),
    )
    .unwrap();
    assert!(matches!(
        rec.finite,
        ProjectedFirstOrderFiniteEvidence::Trusted(_)
    ));
    // FINITE was trusted away, not scanned: checked_scope must not claim it.
    assert!(!rec.checked_scope.contains(ValidationScope::FINITE));
}

mod error_mapping;
mod orchestration;
