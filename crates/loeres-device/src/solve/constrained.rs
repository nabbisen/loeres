//! Box/linear-inequality constrained projected first-order device kernel
//! (RFC 027 §11.3–§11.5).
//!
//! Extends the RFC 006 kernel to `lo ≤ x ≤ hi, Ax ≤ b` via a bounded Dykstra
//! projection over two sets — the polyhedron `{Ax ≤ b}`, solved by Hildreth's
//! dual method, and the box, solved by exact `clamp` with its own increment
//! vector (RFC 027 §11.2; architect review 043 R1: the box is its own Dykstra
//! set, never a bare clamp inside the sweep). `m = 0` is not a case this
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
    pub projection_max_sweeps: u32,
    /// Inner convergence tolerance: a sweep converges when its net effect on
    /// every coordinate of the iterate is within this bound.
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
/// **Footprint is `(4N + 2M)·size_of::<S>() + header`, not the `3N + 2M` RFC
/// 027 §11.4 states.** The fourth `N`-length buffer (`outer_previous`) holds
/// the outer iterate as it stood before the current outer gradient step, kept
/// fixed for the whole projection so the outer convergence check can compare
/// the *final* projected iterate against it — RFC 006's existing outer
/// criterion, which this kernel's outer loop is documented to keep
/// unchanged. `gradient` cannot serve this role: it holds `∇f(x)` needed to
/// form the candidate, and by the time the candidate is projected `x` has
/// already been overwritten, so an undocumented reuse would have to happen
/// *before* the gradient's own value is next needed — impossible, since both
/// pieces of information (the gradient, and the pre-step iterate) are needed
/// simultaneously to form the candidate. `gradient` *is* reused for a
/// different purpose once that candidate is formed — see
/// [`dykstra_project`] — which is the one place this workspace's fields carry
/// two meanings across one solve, and is documented at that reuse.
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
    /// Dykstra increment for the polytope set, same lifecycle as
    /// `box_increment`.
    polytope_increment: FixedVector<S, N>,
    /// Hildreth dual multipliers for the `m` halfspaces, reset to zero at the
    /// start of each sweep's polytope correction.
    multipliers: FixedVector<S, M>,
    /// Precomputed, boundary-validated `‖aᵢ‖²`, computed once per solve.
    row_norms_sq: FixedVector<S, M>,
    diagnostic: loeres::DiagnosticSnapshot,
}

impl<S, const N: usize, const M: usize> ConstrainedProjectedWorkspace<S, N, M> {
    /// Build a workspace from six caller-owned scratch buffers. Contents are
    /// irrelevant: every buffer is overwritten before it is read
    /// (overwrite-on-use, RFC 005 §7).
    #[inline]
    pub const fn new(
        gradient: FixedVector<S, N>,
        outer_previous: FixedVector<S, N>,
        box_increment: FixedVector<S, N>,
        polytope_increment: FixedVector<S, N>,
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
            polytope_increment,
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
        // here — `dykstra_project` zeroes the Dykstra increments at the start
        // of every call, and boundary validation overwrites `row_norms_sq`
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
/// ∩ {Ax ≤ b}` (RFC 027 §11.2, §11.3), mutating `x` in place. Resets the
/// Dykstra increments to zero at the start of every call — each call projects
/// a fresh candidate, and Dykstra's increments are internal to that one
/// projection, not carried between outer iterations.
///
/// Each sweep: (1) forms the polytope target `x + polytope_increment` and
/// runs **one** cyclic pass of Hildreth's dual coordinate update over the `M`
/// constraints, with the multipliers persisting across sweeps (reset once,
/// at the start of the whole call). Each pass still maintains `x = target −
/// Aᵀ·Δλ` as an exact invariant, where `Δλ` is *this sweep's* change in the
/// multipliers — which is what lets the new polytope increment be read off
/// as `target − x` afterward with no additional storage; (2) forms the box
/// target `x +
/// box_increment` and applies the exact `clamp`, recording the new box
/// increment the same way (review 043 R1: never a bare clamp — the box's own
/// increment is applied and updated every sweep, exactly like the
/// polytope's).
///
/// `workspace.gradient` is reused here as the "iterate before this sweep"
/// snapshot: its gradient role for this outer iteration is over by the time
/// this function is called (the candidate has already been formed from it),
/// and it is not read again until the next outer iteration's
/// `gradient_into` overwrites it fresh. This is the one field with two
/// meanings across a solve; `outer_previous` is not reused this way because
/// it must survive unchanged across every sweep of this call.
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
    zero_in_place(&mut workspace.polytope_increment);
    // Reset once per call, not once per sweep: the multipliers are Hildreth's
    // *dual* state and must persist across sweeps to correctly detect an
    // infeasible polyhedron. Resetting them every sweep lets the coupled
    // target/x/increment system settle into a stable-but-infeasible fixed
    // point (verified against a genuinely infeasible pair of opposing
    // halfspaces, which converged falsely under a per-sweep reset) — with
    // persistent multipliers, two directly conflicting constraints instead
    // drive `λ` without bound and `x` never stabilizes, correctly surfacing
    // as non-convergence (RFC 027 §11.6). The `target − x` reconstruction of
    // the new polytope increment below only ever depends on *this sweep's*
    // change in `λ` (`new_lambda − old_lambda` per constraint), so it needs
    // no adjustment for `λ` persisting.
    zero_in_place(&mut workspace.multipliers);

    let lo = problem.lower_bounds();
    let hi = problem.upper_bounds();
    let a = problem.constraint_matrix();
    let b = problem.constraint_rhs();

    let mut cap_hit = true;

    for _sweep in 0..config.projection_max_sweeps {
        // `workspace.gradient` now holds x-before-this-sweep (see doc above).
        copy_in_place(&mut workspace.gradient, x);

        // --- polytope correction: target = x + polytope_increment(old) ---
        for j in 0..N {
            let target_j = x.get(j)?.add(workspace.polytope_increment.get(j)?);
            x.set(j, target_j)?;
        }
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
            // RFC 027 §11.2: `x − aᵢ·max(0,(aᵢᵀx − bᵢ)/‖aᵢ‖²)` — the coordinate
            // shift is `+residual / norm_sq`, matching dual coordinate ascent
            // (∂g/∂λᵢ = residual; the stationary shift solves
            // `residual − Δλᵢ·‖aᵢ‖² = 0`). Not `−residual`.
            let step = residual.checked_div(norm_sq)?;
            let old_lambda = workspace.multipliers.get(i)?;
            let new_lambda = old_lambda.add(step).max(S::zero());
            let diff = new_lambda.sub(old_lambda);
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
        // new_polytope_increment[j] = target_j - x_j(after Hildreth)
        //                           = (previous_iterate_j + old_increment_j) - x_j
        for j in 0..N {
            let previous = workspace.gradient.get(j)?;
            let old_increment = workspace.polytope_increment.get(j)?;
            let target_j = previous.add(old_increment);
            let new_increment = target_j.sub(x.get(j)?);
            workspace.polytope_increment.set(j, new_increment)?;
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

        if change.lte_tolerance(config.projection_tolerance) {
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
                let violation = max_constraint_violation(problem, x)?;
                return Ok(ConstrainedSolveReport::from_core(
                    SolveReport::converged_early(executed),
                    projection_cap_hits,
                    violation,
                ));
            }
        }
    }

    let core = if converged {
        SolveReport::converged_at_cap(max_iterations)
    } else {
        SolveReport::not_converged_cap(max_iterations)
    };
    let violation = max_constraint_violation(problem, x)?;
    Ok(ConstrainedSolveReport::from_core(
        core,
        projection_cap_hits,
        violation,
    ))
}

#[cfg(test)]
mod tests;
