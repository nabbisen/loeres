//! Solver outcome and status taxonomy for `loeres` (RFC 014).
//!
//! This module owns the single shared vocabulary for solver outcomes, so device
//! and cluster crates do not each grow a parallel taxonomy. Its central rule is
//! a clean status/error split:
//!
//! > **Status** is expected, bounded solver progress and is returned in `Ok`.
//! > **Error** ([`crate::error::SolverError`]) is a boundary-validation
//! > rejection or fail-safe condition and is returned in `Err`. The same
//! > condition is never representable as both.
//!
//! The headline consequence: **non-convergence at the iteration cap is a
//! status, not an error** — `Ok(`[`SolveReport::not_converged_cap`]`)`, never
//! an `Err`. Every type here is `Copy` plain data: no `std`, no `alloc`, no
//! scalar generic, no allocation.

/// Outcome of a single solver step. A step never reports reaching the iteration
/// cap — that is a property of the bounded driver loop, not of any one step.
#[repr(u8)]
#[non_exhaustive]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum StepOutcome {
    /// The step made usable progress; the driver may continue iterating.
    Continue,
    /// The convergence criterion was satisfied at this step.
    Converged,
    /// The step produced no usable progress (below the configured floor); the
    /// driver decides whether this is terminal under the active timing policy.
    NoProgress,
}

/// Did the solver meet its convergence criterion? Returned (inside a
/// [`SolveReport`]) in `Ok` — both variants are expected, bounded outcomes.
#[repr(u8)]
#[non_exhaustive]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum SolveStatus {
    /// The convergence criterion was satisfied within the iteration cap.
    Converged,
    /// The solver terminated without meeting the convergence criterion. Bounded,
    /// expected progress information — never an error.
    NotConverged,
}

impl SolveStatus {
    /// `true` iff this status is [`SolveStatus::Converged`].
    #[inline]
    #[must_use]
    pub const fn is_converged(self) -> bool {
        matches!(self, SolveStatus::Converged)
    }
}

/// Why did the bounded loop stop? Orthogonal to [`SolveStatus`] in concept but
/// constrained in combination (see [`SolveReport`]).
#[repr(u8)]
#[non_exhaustive]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum TerminationReason {
    /// Stopped because the convergence criterion was met (early-exit mode).
    ConvergenceCriterion,
    /// Stopped because the configured iteration cap was reached.
    IterationCap,
    /// Stopped early because the solver detected no usable progress.
    NoProgress,
}

/// How many steps ran, and why the loop stopped. Any `(u32, TerminationReason)`
/// pair is individually well-formed, so this carries a public constructor.
#[repr(C)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct IterationReport {
    iterations_executed: u32,
    termination: TerminationReason,
}

impl IterationReport {
    /// Construct a report from a completed-step count and a termination reason.
    #[inline]
    #[must_use]
    pub const fn new(iterations_executed: u32, termination: TerminationReason) -> Self {
        Self {
            iterations_executed,
            termination,
        }
    }

    /// Number of completed solver-step calls.
    #[inline]
    #[must_use]
    pub const fn iterations_executed(&self) -> u32 {
        self.iterations_executed
    }

    /// Why the bounded loop stopped.
    #[inline]
    #[must_use]
    pub const fn termination(&self) -> TerminationReason {
        self.termination
    }
}

/// Scalar-agnostic terminal report for a bounded solve. Carries no objective,
/// residual, or solution value — those travel in caller-owned workspace or a
/// separate typed output, keeping this a single small `Copy` type of uniform
/// size across every solver.
///
/// Fields are private; construct only through the named constructors, which
/// admit exactly the valid `(SolveStatus, TerminationReason)` combinations
/// (RFC 014 §3.3). `Converged + NoProgress` and `NotConverged +
/// ConvergenceCriterion` are unconstructable by design.
#[repr(C)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct SolveReport {
    status: SolveStatus,
    iteration: IterationReport,
}

impl SolveReport {
    /// Converged and stopped as soon as the criterion was met (early-exit).
    #[inline]
    #[must_use]
    pub const fn converged_early(iterations_executed: u32) -> Self {
        Self {
            status: SolveStatus::Converged,
            iteration: IterationReport::new(
                iterations_executed,
                TerminationReason::ConvergenceCriterion,
            ),
        }
    }

    /// Converged, but ran to the configured cap (constant-iteration mode).
    #[inline]
    #[must_use]
    pub const fn converged_at_cap(iterations_executed: u32) -> Self {
        Self {
            status: SolveStatus::Converged,
            iteration: IterationReport::new(iterations_executed, TerminationReason::IterationCap),
        }
    }

    /// Did not converge; the iteration cap was reached.
    #[inline]
    #[must_use]
    pub const fn not_converged_cap(iterations_executed: u32) -> Self {
        Self {
            status: SolveStatus::NotConverged,
            iteration: IterationReport::new(iterations_executed, TerminationReason::IterationCap),
        }
    }

    /// Did not converge; stopped early on no usable progress.
    #[inline]
    #[must_use]
    pub const fn not_converged_stalled(iterations_executed: u32) -> Self {
        Self {
            status: SolveStatus::NotConverged,
            iteration: IterationReport::new(iterations_executed, TerminationReason::NoProgress),
        }
    }

    /// Did the solver meet its convergence criterion?
    #[inline]
    #[must_use]
    pub const fn status(&self) -> SolveStatus {
        self.status
    }

    /// The iteration count and termination reason.
    #[inline]
    #[must_use]
    pub const fn iteration(&self) -> IterationReport {
        self.iteration
    }

    /// Number of completed solver-step calls.
    #[inline]
    #[must_use]
    pub const fn iterations_executed(&self) -> u32 {
        self.iteration.iterations_executed
    }

    /// Why the bounded loop stopped.
    #[inline]
    #[must_use]
    pub const fn termination(&self) -> TerminationReason {
        self.iteration.termination
    }
}

/// Projection from a crate-specific report onto the core [`SolveReport`].
///
/// Device and cluster crates may define richer report types (timing, telemetry,
/// optional diagnostics), but every such type must project losslessly onto the
/// core status/termination, so no execution crate can expose a terminal status
/// outside this taxonomy. This is a static-dispatch trait: `dyn AsCoreReport`
/// must not appear in edge-facing public signatures (enforced by RFC 010's
/// `check-public-api`).
pub trait AsCoreReport {
    /// Project this report onto the canonical core report.
    fn as_core_report(&self) -> SolveReport;

    /// The core convergence status of this report.
    #[inline]
    fn core_status(&self) -> SolveStatus {
        self.as_core_report().status()
    }
}

// RFC 014 §4.1: data-free enums are one byte under `#[repr(u8)]`; the reports
// stay within the same 16-byte ceiling used for `SolverError`.
const _: () = {
    assert!(core::mem::size_of::<StepOutcome>() <= 2);
    assert!(core::mem::size_of::<SolveStatus>() <= 2);
    assert!(core::mem::size_of::<TerminationReason>() <= 2);
    assert!(core::mem::size_of::<IterationReport>() <= 12);
    assert!(core::mem::size_of::<SolveReport>() <= 16);
};
