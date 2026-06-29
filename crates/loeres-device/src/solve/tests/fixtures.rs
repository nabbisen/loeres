//! Shared test fixtures for the projected first-order kernel tests.

use crate::config::{DeviceSolveConfig, TimingMode};
use crate::problem::ProjectedFirstOrderProblem;
use crate::solve::ProjectedFirstOrderWorkspace;
use crate::workspace::WorkspaceFor;
use loeres::SolverError;
use loeres_backend_static::array::FixedVector;

pub(super) struct Quadratic<const N: usize> {
    pub(super) target: FixedVector<f64, N>,
    pub(super) lo: FixedVector<f64, N>,
    pub(super) hi: FixedVector<f64, N>,
    pub(super) alpha: f64,
}

impl<const N: usize> ProjectedFirstOrderProblem<f64, N> for Quadratic<N> {
    type Bounds = FixedVector<f64, N>;

    fn validate_boundary(&self) -> Result<(), SolverError> {
        for (l, h) in self.lo.as_slice().iter().zip(self.hi.as_slice()) {
            if !l.is_finite() || !h.is_finite() {
                return Err(SolverError::NonFiniteInput);
            }
            if *l > *h {
                return Err(SolverError::InvalidInput);
            }
        }
        Ok(())
    }

    fn lower_bound(&self) -> &FixedVector<f64, N> {
        &self.lo
    }

    fn upper_bound(&self) -> &FixedVector<f64, N> {
        &self.hi
    }

    fn step_scale(&self) -> f64 {
        self.alpha
    }

    fn gradient_at(
        &self,
        x: &FixedVector<f64, N>,
        grad: &mut FixedVector<f64, N>,
    ) -> Result<(), SolverError> {
        for ((g, &xi), &ti) in grad
            .as_mut_slice()
            .iter_mut()
            .zip(x.as_slice())
            .zip(self.target.as_slice())
        {
            *g = xi - ti;
        }
        Ok(())
    }

    fn objective_at(&self, x: &FixedVector<f64, N>) -> Result<f64, SolverError> {
        let mut acc = 0.0;
        for (&xi, &ti) in x.as_slice().iter().zip(self.target.as_slice()) {
            let d = xi - ti;
            acc += 0.5 * d * d;
        }
        Ok(acc)
    }
}

impl<const N: usize> WorkspaceFor<Quadratic<N>> for Quadratic<N> {
    type Workspace = ProjectedFirstOrderWorkspace<f64, N>;

    fn required_workspace_bytes() -> usize {
        core::mem::size_of::<ProjectedFirstOrderWorkspace<f64, N>>()
    }
}

pub(super) fn quad2() -> Quadratic<2> {
    Quadratic {
        target: FixedVector::from_array([0.5, -0.5]),
        lo: FixedVector::from_array([-1.0, -1.0]),
        hi: FixedVector::from_array([1.0, 1.0]),
        alpha: 0.5,
    }
}

pub(super) fn workspace<const N: usize>() -> ProjectedFirstOrderWorkspace<f64, N> {
    ProjectedFirstOrderWorkspace::new(FixedVector::from_array([0.0; N]))
}

pub(super) fn config(
    max_iterations: u32,
    tolerance: f64,
    timing_mode: TimingMode,
) -> DeviceSolveConfig<f64> {
    DeviceSolveConfig {
        max_iterations,
        tolerance,
        timing_mode,
    }
}
pub(super) struct Mismatch {
    pub(super) lo: FixedVector<f64, 2>,
    pub(super) hi: FixedVector<f64, 2>,
}

impl ProjectedFirstOrderProblem<f64, 3> for Mismatch {
    type Bounds = FixedVector<f64, 2>;

    fn validate_boundary(&self) -> Result<(), SolverError> {
        Ok(())
    }
    fn lower_bound(&self) -> &FixedVector<f64, 2> {
        &self.lo
    }
    fn upper_bound(&self) -> &FixedVector<f64, 2> {
        &self.hi
    }
    fn step_scale(&self) -> f64 {
        0.5
    }
    fn gradient_at(
        &self,
        _x: &FixedVector<f64, 3>,
        grad: &mut FixedVector<f64, 3>,
    ) -> Result<(), SolverError> {
        for g in grad.as_mut_slice().iter_mut() {
            *g = 0.0;
        }
        Ok(())
    }
    fn objective_at(&self, _x: &FixedVector<f64, 3>) -> Result<f64, SolverError> {
        Ok(0.0)
    }
}

pub(super) fn quad2_with_alpha(alpha: f64) -> Quadratic<2> {
    Quadratic {
        target: FixedVector::from_array([0.5, -0.5]),
        lo: FixedVector::from_array([-1.0, -1.0]),
        hi: FixedVector::from_array([1.0, 1.0]),
        alpha,
    }
}

pub(super) struct NanGradient {
    pub(super) lo: FixedVector<f64, 2>,
    pub(super) hi: FixedVector<f64, 2>,
}

impl ProjectedFirstOrderProblem<f64, 2> for NanGradient {
    type Bounds = FixedVector<f64, 2>;
    fn validate_boundary(&self) -> Result<(), SolverError> {
        Ok(())
    }
    fn lower_bound(&self) -> &FixedVector<f64, 2> {
        &self.lo
    }
    fn upper_bound(&self) -> &FixedVector<f64, 2> {
        &self.hi
    }
    fn step_scale(&self) -> f64 {
        0.5
    }
    fn gradient_at(
        &self,
        _x: &FixedVector<f64, 2>,
        grad: &mut FixedVector<f64, 2>,
    ) -> Result<(), SolverError> {
        for g in grad.as_mut_slice().iter_mut() {
            *g = f64::NAN;
        }
        Ok(())
    }
    fn objective_at(&self, _x: &FixedVector<f64, 2>) -> Result<f64, SolverError> {
        Ok(0.0)
    }
}

pub(super) struct LaxNanBounds {
    pub(super) lo: FixedVector<f64, 2>,
    pub(super) hi: FixedVector<f64, 2>,
}

impl ProjectedFirstOrderProblem<f64, 2> for LaxNanBounds {
    type Bounds = FixedVector<f64, 2>;
    fn validate_boundary(&self) -> Result<(), SolverError> {
        Ok(())
    }
    fn lower_bound(&self) -> &FixedVector<f64, 2> {
        &self.lo
    }
    fn upper_bound(&self) -> &FixedVector<f64, 2> {
        &self.hi
    }
    fn step_scale(&self) -> f64 {
        0.5
    }
    fn gradient_at(
        &self,
        x: &FixedVector<f64, 2>,
        grad: &mut FixedVector<f64, 2>,
    ) -> Result<(), SolverError> {
        for (g, &xi) in grad.as_mut_slice().iter_mut().zip(x.as_slice()) {
            *g = xi;
        }
        Ok(())
    }
    fn objective_at(&self, _x: &FixedVector<f64, 2>) -> Result<f64, SolverError> {
        Ok(0.0)
    }
}
