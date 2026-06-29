//! Deterministic solve entrypoints.
//!
//! The baseline box/bound-constrained projected first-order device kernel
//! (RFC 006): bounded-iteration, allocation-free, validates inputs before the
//! loop, never panics on expected failure, and reports outcomes through the
//! RFC 014 core [`SolveReport`](loeres::SolveReport) wrapped in
//! [`DeviceSolveReport`]. Available under the `owned-arrays` feature, since the
//! primal/gradient work vectors are fixed-size owned static arrays.

#[cfg(feature = "owned-arrays")]
pub use owned::{DeviceSolveReport, ProjectedFirstOrderWorkspace, solve_projected_first_order};

#[cfg(all(test, feature = "owned-arrays"))]
mod tests;

#[cfg(feature = "owned-arrays")]
mod owned {
    use loeres::{
        AsCoreReport, ContiguousVectorAccess, DiagnosticSnapshot, FiniteScalar, MetricScalar,
        SolveReport, SolveStatus, SolverError,
    };
    use loeres_backend_static::array::FixedVector;

    use crate::config::{DeviceSolveConfig, TimingMode};
    use crate::problem::ProjectedFirstOrderProblem;
    use crate::workspace::{DeviceWorkspace, DeviceWorkspaceDiagnostic};

    /// Checked `usize -> u32` for dimension error payloads (B1).
    ///
    /// Mirrors `loeres_backend_static::dimension::dim_u32`: never truncates;
    /// returns [`SolverError::InvalidDimension`] if the extent exceeds `u32`.
    fn dim_u32(value: usize) -> Result<u32, SolverError> {
        u32::try_from(value).map_err(|_| SolverError::InvalidDimension)
    }

    /// Device-side solve outcome.
    ///
    /// A thin wrapper over the RFC 014 core [`SolveReport`] (RFC 006 §3.5):
    /// non-convergence at the iteration cap is an `Ok(DeviceSolveReport)` whose
    /// status is [`SolveStatus::NotConverged`], never a [`SolverError`]. Derives
    /// the core report through [`AsCoreReport`].
    #[derive(Copy, Clone, Debug, Eq, PartialEq)]
    pub struct DeviceSolveReport {
        core: SolveReport,
    }

    impl DeviceSolveReport {
        /// Wrap a core [`SolveReport`].
        #[inline]
        #[must_use]
        pub const fn from_core(core: SolveReport) -> Self {
            Self { core }
        }

        /// The wrapped core report.
        #[inline]
        #[must_use]
        pub const fn core(&self) -> SolveReport {
            self.core
        }

        /// The solve status.
        #[inline]
        #[must_use]
        pub const fn status(&self) -> SolveStatus {
            self.core.status()
        }

        /// The number of iterations actually executed.
        #[inline]
        #[must_use]
        pub const fn iterations_executed(&self) -> u32 {
            self.core.iterations_executed()
        }
    }

    impl AsCoreReport for DeviceSolveReport {
        #[inline]
        fn as_core_report(&self) -> SolveReport {
            self.core
        }
    }

    /// Scratch workspace for the projected first-order kernel.
    ///
    /// Pure scratch (I2): the solution vector is a separate `&mut x` parameter,
    /// not stored here. Carries the gradient buffer the oracle writes into. Bound
    /// to the kernel concretely by the shared `N`, so a wrong-sized workspace is
    /// a compile error. Its RFC 005 [`WorkspaceFor`] sizing impl is supplied by
    /// the concrete problem family.
    ///
    /// [`WorkspaceFor`]: crate::workspace::WorkspaceFor
    pub struct ProjectedFirstOrderWorkspace<S, const N: usize> {
        gradient: FixedVector<S, N>,
        diagnostic: DiagnosticSnapshot,
    }

    impl<S, const N: usize> ProjectedFirstOrderWorkspace<S, N> {
        /// Build a workspace from a caller-owned gradient scratch buffer.
        ///
        /// The buffer contents are irrelevant: the kernel overwrites the gradient
        /// each iteration before reading it (overwrite-on-use).
        #[inline]
        pub const fn new(gradient: FixedVector<S, N>) -> Self {
            Self {
                gradient,
                diagnostic: DiagnosticSnapshot::EMPTY,
            }
        }
    }

    impl<S, const N: usize> DeviceWorkspace for ProjectedFirstOrderWorkspace<S, N> {
        #[inline]
        fn reset_for_entry(&mut self) {
            // Overwrite-on-use: the gradient is rewritten by the oracle each
            // iteration, so only the diagnostic needs resetting here.
            self.diagnostic = DiagnosticSnapshot::EMPTY;
        }
    }

    impl<S, const N: usize> DeviceWorkspaceDiagnostic for ProjectedFirstOrderWorkspace<S, N> {
        #[inline]
        fn diagnostic(&self) -> DiagnosticSnapshot {
            self.diagnostic
        }
    }

    /// One projected gradient-descent coordinate update.
    ///
    /// Returns the projected value `clamp(xi - alpha * gi, loi, hii)` and the
    /// magnitude of the change `|projected - xi|`. Rejects non-finite gradient or
    /// bound coordinates (B3); with finite `xi`, `gi`, `alpha`, and bounds the
    /// `clamp`-projected result and change are finite, so no result-overflow path
    /// is reachable.
    fn project_one<S>(xi: S, gi: S, loi: S, hii: S, alpha: S) -> Result<(S, S), SolverError>
    where
        S: FiniteScalar + MetricScalar,
    {
        if !gi.is_finite() || !loi.is_finite() || !hii.is_finite() {
            return Err(SolverError::NonFiniteInput);
        }
        let projected = xi.sub(alpha.mul(gi)).clamp(loi, hii);
        let change = projected.sub(xi).abs();
        Ok((projected, change))
    }

    /// Apply one projected gradient step over the whole iterate, returning the
    /// largest per-coordinate change `max_i |x_next[i] - x[i]|` (I7).
    ///
    /// Uses the RFC 002 contiguous fast path for the bounds when available,
    /// falling back to per-element [`VectorAccess::get`](loeres::VectorAccess::get)
    /// otherwise (RFC 006 §3.7). Panic-averse: no indexing, no unwrap; the primal
    /// and gradient are read through fixed-size slices and the bounds through
    /// bounds-checked accessors.
    fn projected_gradient_step<S, B, const N: usize>(
        x: &mut FixedVector<S, N>,
        grad: &FixedVector<S, N>,
        lo: &B,
        hi: &B,
        alpha: S,
    ) -> Result<S, SolverError>
    where
        S: FiniteScalar + MetricScalar,
        B: ContiguousVectorAccess<Scalar = S>,
    {
        let n = x.len();
        // `grad` shares `N` with `x` by type, so only the run-time bound lengths
        // can disagree.
        if lo.len() != n {
            return Err(SolverError::DimensionMismatch {
                lhs: dim_u32(n)?,
                rhs: dim_u32(lo.len())?,
            });
        }
        if hi.len() != n {
            return Err(SolverError::DimensionMismatch {
                lhs: dim_u32(n)?,
                rhs: dim_u32(hi.len())?,
            });
        }

        let grad_slice = grad.as_slice();
        let x_slice = x.as_mut_slice();
        let mut max_change = S::zero();

        match (lo.as_contiguous(), hi.as_contiguous()) {
            (Some(lo_slice), Some(hi_slice)) => {
                // Defensive (M1): a correct `ContiguousVectorAccess` returns a
                // slice of length `len()`, already checked equal to `n`. Guard
                // against a non-conforming third-party `Bounds` impl rather than
                // letting `zip` silently skip tail coordinates.
                if lo_slice.len() != n || hi_slice.len() != n {
                    return Err(SolverError::InternalInvariantViolation);
                }
                for (((xi, &gi), &loi), &hii) in x_slice
                    .iter_mut()
                    .zip(grad_slice)
                    .zip(lo_slice)
                    .zip(hi_slice)
                {
                    let (projected, change) = project_one(*xi, gi, loi, hii, alpha)?;
                    *xi = projected;
                    max_change = max_change.max(change);
                }
            }
            _ => {
                for (i, (xi, &gi)) in x_slice.iter_mut().zip(grad_slice).enumerate() {
                    let loi = lo.get(i)?;
                    let hii = hi.get(i)?;
                    let (projected, change) = project_one(*xi, gi, loi, hii, alpha)?;
                    *xi = projected;
                    max_change = max_change.max(change);
                }
            }
        }

        Ok(max_change)
    }

    /// Run the baseline box/bound-constrained projected first-order kernel.
    ///
    /// `x` is both the initial guess and, on return, the final projected iterate
    /// (I2). The workspace supplies gradient scratch. Validation (`config`, then
    /// problem boundary) runs before the loop; thereafter the kernel iterates
    /// `x <- clamp(x - alpha * grad f(x), lo, hi)` and stops when the largest
    /// coordinate change is within `config.tolerance` (I7).
    ///
    /// Timing modes (I8): under `EarlyExitAllowed` the kernel returns as soon as
    /// the criterion is met (`converged_early`) or at the cap
    /// (`not_converged_cap`). Under `ConstantIteration` it always runs the full
    /// `max_iterations` and reports `converged_at_cap` / `not_converged_cap`, so
    /// `iterations_executed == max_iterations`.
    ///
    /// Non-convergence is an `Ok` outcome, never a [`SolverError`]; errors are
    /// reserved for invalid configuration, invalid bounds, dimension mismatch,
    /// and oracle failures.
    pub fn solve_projected_first_order<P, S, const N: usize>(
        problem: &P,
        x: &mut FixedVector<S, N>,
        workspace: &mut ProjectedFirstOrderWorkspace<S, N>,
        config: &DeviceSolveConfig<S>,
    ) -> Result<DeviceSolveReport, SolverError>
    where
        P: ProjectedFirstOrderProblem<S, N>,
        S: FiniteScalar + MetricScalar,
    {
        workspace.reset_for_entry();
        config.validate()?;
        problem.validate_boundary()?;

        let alpha = problem.step_scale();
        // B2: the problem-provided step scale must be finite and strictly
        // positive. A zero scale produces zero iterate change (false
        // convergence); a negative scale inverts the descent direction.
        if !alpha.is_finite() {
            return Err(SolverError::NonFiniteInput);
        }
        if alpha <= S::zero() {
            return Err(SolverError::InvalidInput);
        }
        // B3: reject a non-finite initial iterate up front. Gradient and bound
        // coordinates are checked per step in `project_one`; with those finite
        // and `alpha` finite, the projected iterate stays finite by induction.
        for xi in x.as_slice() {
            if !xi.is_finite() {
                return Err(SolverError::NonFiniteInput);
            }
        }

        let tolerance = config.tolerance;
        let max_iterations = config.max_iterations;
        // In-crate exhaustive match: `TimingMode` is `#[non_exhaustive]` only for
        // downstream crates; here the `constant-iteration` cfg gates the variant.
        let constant_iteration = match config.timing_mode {
            TimingMode::EarlyExitAllowed => false,
            #[cfg(feature = "constant-iteration")]
            TimingMode::ConstantIteration => true,
        };

        let mut converged = false;
        let mut executed: u32 = 0;
        while executed < max_iterations {
            problem.gradient_at(x, &mut workspace.gradient)?;
            let change = projected_gradient_step(
                x,
                &workspace.gradient,
                problem.lower_bound(),
                problem.upper_bound(),
                alpha,
            )?;
            executed += 1;
            if change.lte_tolerance(tolerance) {
                converged = true;
                if !constant_iteration {
                    return Ok(DeviceSolveReport::from_core(SolveReport::converged_early(
                        executed,
                    )));
                }
            }
        }

        let report = if converged {
            SolveReport::converged_at_cap(max_iterations)
        } else {
            SolveReport::not_converged_cap(max_iterations)
        };
        Ok(DeviceSolveReport::from_core(report))
    }
}
