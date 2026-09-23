//! Box/linear-inequality constrained projected first-order device kernel
//! (RFC 027 §11.3–§11.5).
//!
//! Extends the RFC 006 kernel to `lo ≤ x ≤ hi, Ax ≤ b` via a bounded Dykstra
//! projection over `m + 1` sets (RFC 027 Amendment 3, §0.3.1): each halfspace
//! `{aᵢᵀx ≤ bᵢ}` is its own Dykstra set, whose increment is always parallel to
//! `aᵢ` and so is stored as the single scalar `λᵢ` (Hildreth's method *is*
//! Dykstra applied to halfspaces); the box is the one remaining set, solved by
//! exact `clamp` with its own `n`-length increment vector (RFC 027 §11.2;
//! architect review 043 R1: never a bare clamp inside the sweep). There is no
//! polyhedron-level increment vector. `m = 0` is not a case this
//! kernel handles: a device problem with no inequalities uses the RFC 006
//! entrypoint instead (RFC 027 §0.2.4), and `M ≥ 1` is const-asserted.
//!
//! Consumes [`loeres::QuadraticProgram`] directly — S1's blanket-implemented
//! core contract — rather than a separate device-only problem trait. The
//! contract carries no `step_scale`, so it is supplied as its own parameter,
//! not sourced from `problem` (RFC 027 §0.2.1: the adapter from a
//! `QuadraticProgram` to a kernel is this function itself).

use loeres::{
    AsCoreReport, DivisibleScalar, FiniteScalar, MatrixAccess, MetricScalar, QuadraticProgram,
    SolveReport, SolveStatus, SolverError, VectorAccess, VectorAccessMut,
};
use loeres_backend_static::array::FixedVector;
use loeres_backend_static::workspace::WorkspaceFootprint;

use crate::config::TimingMode;
use crate::workspace::{DeviceWorkspace, DeviceWorkspaceDiagnostic};

/// Checked `usize -> u32` for dimension error payloads, mirroring
/// `loeres_backend_static::dimension::dim_u32` and `solve.rs`'s own copy —
/// each crate/module owns this tiny helper rather than sharing a `pub(crate)`
/// one across a crate boundary.
fn dim_u32(value: usize) -> Result<u32, SolverError> {
    u32::try_from(value).map_err(|_| SolverError::InvalidDimension)
}

fn require_len(actual: usize, expected: usize) -> Result<(), SolverError> {
    if actual == expected {
        Ok(())
    } else {
        Err(SolverError::DimensionMismatch {
            lhs: dim_u32(actual)?,
            rhs: dim_u32(expected)?,
        })
    }
}

/// Runtime execution policy for the constrained kernel (RFC 027 §11.3): the
/// outer bound and tolerance mirror [`crate::config::DeviceSolveConfig`]
/// unchanged; `projection_max_sweeps` and `projection_tolerance` are new,
/// bounding and terminating the inner Dykstra loop independently of the outer
/// gradient loop.
#[derive(Copy, Clone, Debug)]
pub struct ConstrainedSolveConfig<S> {
    /// Maximum outer gradient iterations (must be `> 0`).
    pub max_iterations: u32,
    /// Outer convergence tolerance, compared against the change the whole
    /// outer step (gradient move plus projection) produced.
    pub tolerance: S,
    /// Execution timing policy for the outer loop.
    pub timing_mode: TimingMode,
    /// Maximum Dykstra sweeps per outer-iteration projection (must be `> 0`).
    ///
    /// **The cap must suit `projection_tolerance`.** Dykstra converges linearly
    /// at a rate set by the angles between constraint normals, so tight
    /// tolerances need far more sweeps than loose ones — on the review-054
    /// regression polytope, roughly 2700 sweeps at `1e-12`. There is no
    /// default: a cap that is too small for the tolerance is not an error, it
    /// is reported as `projection_cap_hits > 0` and a non-zero
    /// `max_constraint_violation` (RFC 027 §11.3, §11.6).
    pub projection_max_sweeps: u32,
    /// Inner convergence tolerance. A projection is converged only when, in
    /// the same sweep, the iterate's change, the multipliers' change, **and**
    /// the terminal constraint violation are all within this bound (RFC 027
    /// §0.3.3) — a dual method may not be stopped on the primal iterate alone.
    pub projection_tolerance: S,
}

impl<S: FiniteScalar + MetricScalar> ConstrainedSolveConfig<S> {
    /// Validate the *structural* config rules (RFC 005 §6 pattern, extended):
    /// both caps `> 0`, both tolerances finite and non-negative. Whether zero
    /// tolerance is meaningful is a caller concern, not rejected here.
    pub fn validate(&self) -> Result<(), SolverError> {
        if self.max_iterations == 0 {
            return Err(SolverError::InvalidInput);
        }
        if !self.tolerance.is_finite() {
            return Err(SolverError::NonFiniteInput);
        }
        if self.tolerance < S::zero() {
            return Err(SolverError::InvalidInput);
        }
        if self.projection_max_sweeps == 0 {
            return Err(SolverError::InvalidInput);
        }
        if !self.projection_tolerance.is_finite() {
            return Err(SolverError::NonFiniteInput);
        }
        if self.projection_tolerance < S::zero() {
            return Err(SolverError::InvalidInput);
        }
        Ok(())
    }
}

/// Constrained-kernel solve outcome (RFC 027 §11.3): a thin wrapper over the
/// RFC 014 core [`SolveReport`], carrying the two fields the RFC requires so
/// the kernel never claims exact feasibility it did not verify —
/// `projection_cap_hits` counts outer iterations whose Dykstra projection hit
/// `projection_max_sweeps` without converging; `max_constraint_violation` is
/// `max(0, maxᵢ(aᵢᵀx − bᵢ))` at the returned iterate (box violation is zero by
/// construction, since the box step is exact `clamp` and always the final
/// operation of a completed sweep).
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ConstrainedSolveReport<S> {
    core: SolveReport,
    projection_cap_hits: u32,
    max_constraint_violation: S,
}

impl<S: Copy> ConstrainedSolveReport<S> {
    /// Wrap a core [`SolveReport`] with the constrained-kernel fields.
    #[inline]
    #[must_use]
    pub const fn from_core(
        core: SolveReport,
        projection_cap_hits: u32,
        max_constraint_violation: S,
    ) -> Self {
        Self {
            core,
            projection_cap_hits,
            max_constraint_violation,
        }
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

    /// The number of outer iterations actually executed.
    #[inline]
    #[must_use]
    pub const fn iterations_executed(&self) -> u32 {
        self.core.iterations_executed()
    }

    /// How many outer iterations' projection hit `projection_max_sweeps`
    /// without converging. Zero does not by itself prove every projection was
    /// exact — the cap could sit exactly at the convergence point — but a
    /// non-zero count is honest evidence the projection was inexact at least
    /// once.
    #[inline]
    #[must_use]
    pub const fn projection_cap_hits(&self) -> u32 {
        self.projection_cap_hits
    }

    /// `max(0, maxᵢ(aᵢᵀx − bᵢ))` at the returned iterate. Zero means every
    /// linear inequality holds (the box always holds exactly, by
    /// construction); positive means the returned point is only
    /// feasible-approximate.
    #[inline]
    #[must_use]
    pub const fn max_constraint_violation(&self) -> S {
        self.max_constraint_violation
    }
}

impl<S: Copy> AsCoreReport for ConstrainedSolveReport<S> {
    #[inline]
    fn as_core_report(&self) -> SolveReport {
        self.core
    }
}

/// Scratch workspace for the constrained projected first-order kernel.
///
/// `M ≥ 1` is const-asserted (RFC 027 §0.2.4): a device problem with no
/// inequalities is `ProjectedFirstOrderWorkspace` and the RFC 006 entrypoint,
/// never an `M = 0` instantiation of this type.
///
/// Footprint is `(3N + 2M)·size_of::<S>() + header` (RFC 027 §11.4): three
/// `N`-length buffers — `outer_previous`, the box increment, and `gradient` —
/// and two `M`-length ones. `outer_previous` must survive unchanged across
/// every sweep of a projection so the outer criterion can compare the final
/// projected iterate against it (RFC 006's criterion, kept unchanged), which
/// rules out reusing it or the box increment as scratch; `gradient`'s role
/// ends once the candidate is formed, so [`dykstra_project`] reuses it as the
/// per-sweep "iterate before this sweep" snapshot.
pub struct ConstrainedProjectedWorkspace<S, const N: usize, const M: usize> {
    /// `∇f(x)` scratch at the top of each outer iteration; reused as the
    /// per-Dykstra-sweep "iterate before this sweep" snapshot once its
    /// gradient role is done for that iteration (see [`dykstra_project`]).
    gradient: FixedVector<S, N>,
    /// The outer iterate before the current outer gradient step, held fixed
    /// for the whole projection.
    outer_previous: FixedVector<S, N>,
    /// Dykstra increment for the box set, persisted across projection sweeps
    /// within one outer iteration and reset to zero at the start of each.
    box_increment: FixedVector<S, N>,
    /// Hildreth multipliers `λᵢ` — the per-halfspace Dykstra increments (each
    /// increment is `λᵢ·aᵢ`, parallel to `aᵢ`, so a scalar suffices; RFC 027
    /// §0.3.1). Reset to zero **once per projection call**, never per sweep:
    /// resetting them per sweep discards the state that makes the scheme
    /// Dykstra (§0.3.2). They do not persist between outer iterations, each of
    /// which projects a fresh candidate.
    multipliers: FixedVector<S, M>,
    /// Precomputed, boundary-validated `‖aᵢ‖²`, computed once per solve.
    row_norms_sq: FixedVector<S, M>,
    diagnostic: loeres::DiagnosticSnapshot,
}

impl<S, const N: usize, const M: usize> ConstrainedProjectedWorkspace<S, N, M> {
    /// Build a workspace from five caller-owned scratch buffers. Contents are
    /// irrelevant: every buffer is overwritten before it is read
    /// (overwrite-on-use, RFC 005 §7).
    #[inline]
    pub const fn new(
        gradient: FixedVector<S, N>,
        outer_previous: FixedVector<S, N>,
        box_increment: FixedVector<S, N>,
        multipliers: FixedVector<S, M>,
        row_norms_sq: FixedVector<S, M>,
    ) -> Self {
        const {
            assert!(
                M > 0,
                "ConstrainedProjectedWorkspace requires M > 0; a device problem with \
                 no inequalities uses the RFC 006 entrypoint (RFC 027 §0.2.4)"
            );
        }
        Self {
            gradient,
            outer_previous,
            box_increment,
            multipliers,
            row_norms_sq,
            diagnostic: loeres::DiagnosticSnapshot::EMPTY,
        }
    }
}

impl<S, const N: usize, const M: usize> DeviceWorkspace for ConstrainedProjectedWorkspace<S, N, M> {
    #[inline]
    fn reset_for_entry(&mut self) {
        // Overwrite-on-use: every N/M buffer is reset to zero (or otherwise
        // overwritten) at the point it is first used within a solve, not
        // here — `dykstra_project` zeroes the box increment and multipliers at
        // the start of every call, and boundary validation overwrites `row_norms_sq`
        // before the loop reads it.
        self.diagnostic = loeres::DiagnosticSnapshot::EMPTY;
    }
}

impl<S, const N: usize, const M: usize> DeviceWorkspaceDiagnostic
    for ConstrainedProjectedWorkspace<S, N, M>
{
    #[inline]
    fn diagnostic(&self) -> loeres::DiagnosticSnapshot {
        self.diagnostic
    }
}

impl<S, const N: usize, const M: usize> WorkspaceFootprint
    for ConstrainedProjectedWorkspace<S, N, M>
{
    #[inline]
    fn footprint_bytes() -> usize {
        core::mem::size_of::<Self>()
    }
}

/// Zero every coordinate of an owned fixed vector, without indexing.
#[inline]
fn zero_in_place<S: FiniteScalar, const K: usize>(v: &mut FixedVector<S, K>) {
    for value in v.as_mut_slice() {
        *value = S::zero();
    }
}

/// Copy `src` into `dst`, without indexing. Same-length `FixedVector`s by
/// construction (shared `K`), so no bounds check is needed.
#[inline]
fn copy_in_place<S: Copy, const K: usize>(dst: &mut FixedVector<S, K>, src: &FixedVector<S, K>) {
    for (d, &s) in dst.as_mut_slice().iter_mut().zip(src.as_slice()) {
        *d = s;
    }
}

/// Boundary validation (RFC 027 §11.5), structural and always run, never
/// skippable: shapes agree with `N`/`M`; `lo ≤ hi`, both finite; `step_scale`
/// finite and `> 0`; the config's own structural rules. Writes the
/// precomputed, validated `‖aᵢ‖²` into `workspace.row_norms_sq`.
///
/// `Q` symmetry/positive-semidefiniteness is a caller precondition, never
/// checked (RFC 027 §11.5) — nothing here reads `Q` or `c` at all, since the
/// gradient is consumed only through [`QuadraticProgram`]'s own oracle.
fn validate_boundary<P, S, const N: usize, const M: usize>(
    problem: &P,
    step_scale: S,
    config: &ConstrainedSolveConfig<S>,
    workspace: &mut ConstrainedProjectedWorkspace<S, N, M>,
) -> Result<(), SolverError>
where
    P: QuadraticProgram<S>,
    S: FiniteScalar + MetricScalar + DivisibleScalar,
{
    config.validate()?;

    let shape = problem.shape()?;
    require_len(shape.variables, N)?;
    require_len(shape.constraints, M)?;

    if !step_scale.is_finite() {
        return Err(SolverError::NonFiniteInput);
    }
    if step_scale <= S::zero() {
        return Err(SolverError::InvalidInput);
    }

    let lo = problem.lower_bounds();
    let hi = problem.upper_bounds();
    for j in 0..N {
        let loj = lo.get(j)?;
        let hij = hi.get(j)?;
        if !loj.is_finite() || !hij.is_finite() {
            return Err(SolverError::NonFiniteInput);
        }
        if loj > hij {
            return Err(SolverError::InvalidInput);
        }
    }

    let a = problem.constraint_matrix();
    for i in 0..M {
        let mut sum_sq = S::zero();
        for j in 0..N {
            let aij = a.get(i, j)?;
            if !aij.is_finite() {
                return Err(SolverError::NonFiniteInput);
            }
            sum_sq = sum_sq.add(aij.mul(aij));
            if !sum_sq.is_finite() {
                return Err(SolverError::Overflow);
            }
        }
        // A sum of squares is never negative; `<= 0` therefore means exactly
        // zero, the all-zero row §13/§0.2.6 requires to be `InvalidInput` — a
        // zero-*row matrix* (m = 0) is a different, valid case this kernel
        // never sees (M >= 1 is const-asserted).
        if sum_sq <= S::zero() {
            return Err(SolverError::InvalidInput);
        }
        workspace.row_norms_sq.set(i, sum_sq)?;
    }

    Ok(())
}

/// Run the bounded Dykstra projection `x ← Π_C(x)` where `C = {lo ≤ x ≤ hi}
/// ∩ {Ax ≤ b}` (RFC 027 §11.2, §11.3, Amendment 3), mutating `x` in place.
/// Resets the box increment and the multipliers to zero at the start of every
/// call — each call projects a fresh candidate, and Dykstra's increments are
/// internal to that one projection, not carried between outer iterations.
///
/// Dykstra over `m + 1` sets (§0.3.1): each sweep visits every halfspace once,
/// then the box. For halfspace `i` with `λᵢ` its current increment coefficient
/// (increment `λᵢ·aᵢ`), the Dykstra step is `y = x + λᵢaᵢ`, `x ← Pᵢ(y)`,
/// `λᵢ ← max(0, (aᵢᵀy − bᵢ)/‖aᵢ‖²)`, which simplifies to `λᵢ' = max(0, λᵢ +
/// (aᵢᵀx − bᵢ)/‖aᵢ‖²)` and `x ← x − (λᵢ' − λᵢ)aᵢ`. The box step is the same
/// with its own vector increment. One cyclic pass per sweep is correct in this
/// formulation because each halfspace projection within the pass is exact.
///
/// Converged only when, in the same sweep, `maxⱼ|Δxⱼ|`, `maxᵢ|Δλᵢ|` **and** the
/// terminal constraint violation are all within `projection_tolerance`
/// (§0.3.3); the violation pass is evaluated only in a sweep where the first
/// two already hold. Returns whether the sweep cap bound (`true`) without that
/// happening.
///
/// `workspace.gradient` is reused here as the "iterate before this sweep"
/// snapshot: its gradient role for this outer iteration is over by the time
/// this function is called (the candidate has already been formed from it),
/// and it is not read again until the next outer iteration's
/// `gradient_into` overwrites it fresh. `outer_previous` is not reused this
/// way because it must survive unchanged across every sweep of this call.
fn dykstra_project<P, S, const N: usize, const M: usize>(
    problem: &P,
    x: &mut FixedVector<S, N>,
    workspace: &mut ConstrainedProjectedWorkspace<S, N, M>,
    config: &ConstrainedSolveConfig<S>,
) -> Result<bool, SolverError>
where
    P: QuadraticProgram<S>,
    S: FiniteScalar + MetricScalar + DivisibleScalar,
{
    zero_in_place(&mut workspace.box_increment);
    zero_in_place(&mut workspace.multipliers);

    let lo = problem.lower_bounds();
    let hi = problem.upper_bounds();
    let a = problem.constraint_matrix();
    let b = problem.constraint_rhs();

    let mut cap_hit = true;

    for _sweep in 0..config.projection_max_sweeps {
        // `workspace.gradient` now holds x-before-this-sweep (see doc above).
        copy_in_place(&mut workspace.gradient, x);

        // --- one cyclic Hildreth pass over the halfspaces ---
        let mut lambda_change = S::zero();
        for i in 0..M {
            let mut dot = S::zero();
            for j in 0..N {
                dot = dot.add(a.get(i, j)?.mul(x.get(j)?));
            }
            let bi = b.get(i)?;
            if !bi.is_finite() {
                return Err(SolverError::NonFiniteInput);
            }
            let residual = dot.sub(bi);
            let norm_sq = workspace.row_norms_sq.get(i)?;
            // RFC 027 §11.2: `x − aᵢ·max(0,(aᵢᵀx − bᵢ)/‖aᵢ‖²)` — the shift is
            // `+residual / norm_sq` (∂g/∂λᵢ = residual; the stationary shift
            // solves `residual − Δλᵢ·‖aᵢ‖² = 0`), not `−residual`.
            let step = residual.checked_div(norm_sq)?;
            let old_lambda = workspace.multipliers.get(i)?;
            let new_lambda = old_lambda.add(step).max(S::zero());
            let diff = new_lambda.sub(old_lambda);
            lambda_change = lambda_change.max(diff.abs());
            for j in 0..N {
                let aij = a.get(i, j)?;
                let xj = x.get(j)?;
                let adjusted = xj.sub(diff.mul(aij));
                if !adjusted.is_finite() {
                    return Err(SolverError::Overflow);
                }
                x.set(j, adjusted)?;
            }
            workspace.multipliers.set(i, new_lambda)?;
        }

        // --- box correction: target = x + box_increment(old), exact clamp ---
        let mut change = S::zero();
        for j in 0..N {
            let loj = lo.get(j)?;
            let hij = hi.get(j)?;
            let old_box_increment = workspace.box_increment.get(j)?;
            let target_j = x.get(j)?.add(old_box_increment);
            let projected_j = target_j.clamp(loj, hij);
            let new_box_increment = target_j.sub(projected_j);
            workspace.box_increment.set(j, new_box_increment)?;
            x.set(j, projected_j)?;

            let previous_iterate_j = workspace.gradient.get(j)?;
            let delta = projected_j.sub(previous_iterate_j).abs();
            change = change.max(delta);
        }

        let tolerance = config.projection_tolerance;
        if change.lte_tolerance(tolerance)
            && lambda_change.lte_tolerance(tolerance)
            && max_constraint_violation(problem, x)?.lte_tolerance(tolerance)
        {
            cap_hit = false;
            break;
        }
    }

    Ok(cap_hit)
}

/// `max(0, maxᵢ(aᵢᵀx − bᵢ))` at `x` — the terminal constraint violation
/// (RFC 027 §11.3). Box violation is not included: the box step is the last
/// operation of every completed sweep and is exact, so it is zero by
/// construction whenever at least one sweep ran (`M ≥ 1` guarantees the loop
/// body executes at least once, so `dykstra_project` never returns without
/// having run the box step).
fn max_constraint_violation<P, S, const N: usize>(
    problem: &P,
    x: &FixedVector<S, N>,
) -> Result<S, SolverError>
where
    P: QuadraticProgram<S>,
    S: FiniteScalar + MetricScalar,
{
    // `M` does not appear in any parameter type, so it cannot be inferred at
    // the call site; the constraint count is read at runtime from `shape()`
    // instead (already validated equal to `M` at the solve boundary).
    let m = problem.shape()?.constraints;
    let a = problem.constraint_matrix();
    let b = problem.constraint_rhs();
    let mut worst = S::zero();
    for i in 0..m {
        let mut dot = S::zero();
        for j in 0..N {
            dot = dot.add(a.get(i, j)?.mul(x.get(j)?));
        }
        let violation = dot.sub(b.get(i)?);
        worst = worst.max(violation);
    }
    Ok(worst.max(S::zero()))
}

/// Run the box/linear-inequality constrained projected first-order device
/// kernel (RFC 027 §11.3).
///
/// `x` is both the initial guess and, on return, the final projected iterate.
/// `step_scale` is supplied directly rather than read from `problem`, since
/// [`QuadraticProgram`] carries no execution parameter (RFC 027 §0.2.1).
/// Validation (`config`, then boundary) runs before the loop; thereafter each
/// outer iteration computes `∇f(x)`, forms the candidate `x − step_scale·∇f(x)`,
/// and projects it onto `C = {lo ≤ x ≤ hi} ∩ {Ax ≤ b}` via bounded Dykstra
/// sweeps, stopping when the outer step's net change is within
/// `config.tolerance` (RFC 006's existing outer criterion, unchanged).
/// `Converged` additionally requires the terminal constraint violation to be
/// within `config.projection_tolerance` (RFC 027 §0.5.1): a stationary but
/// infeasible iterate is `NotConverged` with `NoProgress`.
///
/// Timing modes mirror RFC 006 exactly: under `EarlyExitAllowed` the kernel
/// returns as soon as the outer criterion is met; under `ConstantIteration`
/// it always runs the full `max_iterations`.
///
/// Non-convergence is an `Ok` outcome, never a [`SolverError`]; errors are
/// reserved for invalid configuration, invalid bounds or constraints,
/// dimension mismatch, and oracle/numerical failures.
pub fn solve_constrained_projected_first_order<P, S, const N: usize, const M: usize>(
    problem: &P,
    step_scale: S,
    x: &mut FixedVector<S, N>,
    workspace: &mut ConstrainedProjectedWorkspace<S, N, M>,
    config: &ConstrainedSolveConfig<S>,
) -> Result<ConstrainedSolveReport<S>, SolverError>
where
    P: QuadraticProgram<S>,
    S: FiniteScalar + MetricScalar + DivisibleScalar,
{
    workspace.reset_for_entry();
    validate_boundary(problem, step_scale, config, workspace)?;

    for xi in x.as_slice() {
        if !xi.is_finite() {
            return Err(SolverError::NonFiniteInput);
        }
    }

    let tolerance = config.tolerance;
    let max_iterations = config.max_iterations;
    let constant_iteration = match config.timing_mode {
        TimingMode::EarlyExitAllowed => false,
        #[cfg(feature = "constant-iteration")]
        TimingMode::ConstantIteration => true,
    };

    let mut converged = false;
    let mut projection_cap_hits: u32 = 0;
    let mut executed: u32 = 0;
    while executed < max_iterations {
        copy_in_place(&mut workspace.outer_previous, x);
        problem.gradient_into(x, &mut workspace.gradient)?;
        for j in 0..N {
            let gj = workspace.gradient.get(j)?;
            if !gj.is_finite() {
                return Err(SolverError::NonFiniteInput);
            }
            let candidate_j = x.get(j)?.sub(step_scale.mul(gj));
            if !candidate_j.is_finite() {
                return Err(SolverError::Overflow);
            }
            x.set(j, candidate_j)?;
        }

        let cap_hit = dykstra_project(problem, x, workspace, config)?;
        if cap_hit {
            projection_cap_hits += 1;
        }

        executed += 1;
        let mut change = S::zero();
        for j in 0..N {
            let delta = x.get(j)?.sub(workspace.outer_previous.get(j)?).abs();
            change = change.max(delta);
        }
        if change.lte_tolerance(tolerance) {
            converged = true;
            if !constant_iteration {
                // RFC 027 §0.5.1: a stationary outer step is `Converged` only
                // at a feasible iterate; a capped projection has a fixed point
                // even when the polyhedron is empty.
                let violation = max_constraint_violation(problem, x)?;
                let core = if violation.lte_tolerance(config.projection_tolerance) {
                    SolveReport::converged_early(executed)
                } else {
                    SolveReport::not_converged_stalled(executed)
                };
                return Ok(ConstrainedSolveReport::from_core(
                    core,
                    projection_cap_hits,
                    violation,
                ));
            }
        }
    }

    let violation = max_constraint_violation(problem, x)?;
    let core = if !converged {
        SolveReport::not_converged_cap(max_iterations)
    } else if violation.lte_tolerance(config.projection_tolerance) {
        SolveReport::converged_at_cap(max_iterations)
    } else {
        // RFC 027 §0.5.1, as for the early exit above.
        SolveReport::not_converged_stalled(max_iterations)
    };
    Ok(ConstrainedSolveReport::from_core(
        core,
        projection_cap_hits,
        violation,
    ))
}

#[cfg(test)]
mod tests;
