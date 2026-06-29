//! Bounded execution configuration (RFC 005).
//!
//! Runtime policy data — iteration cap, tolerance, timing mode — kept as data,
//! not type-level const generics. RFC 005 validates *structural* config rules;
//! *solver-specific* validation (e.g. whether a kernel forbids zero tolerance)
//! is RFC 006-owned.

use loeres::{FiniteScalar, OrderedScalar, SolverError};

/// Execution timing policy.
///
/// `EarlyExitAllowed` is always available. The `ConstantIteration` variant is
/// gated behind the `constant-iteration` feature, so requesting constant-iteration
/// without that feature fails at compile time (decision M5). `TimingMode` is
/// `#[non_exhaustive]`, so downstream `match`es must include a wildcard arm and
/// stay robust regardless of which features compiled the variant in.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum TimingMode {
    /// Bounded run that may exit early once converged.
    EarlyExitAllowed,
    /// Fixed iteration count for timing stability (requires `constant-iteration`).
    #[cfg(feature = "constant-iteration")]
    ConstantIteration,
}

/// Runtime device-solve configuration.
#[derive(Copy, Clone, Debug)]
pub struct DeviceSolveConfig<S> {
    /// Maximum solver iterations (must be `> 0`).
    pub max_iterations: u32,
    /// Convergence tolerance.
    pub tolerance: S,
    /// Execution timing policy.
    pub timing_mode: TimingMode,
}

impl<S: FiniteScalar + OrderedScalar> DeviceSolveConfig<S> {
    /// Validate the *structural* config rules (RFC 005 §6): `max_iterations > 0`
    /// and a finite, non-negative tolerance.
    ///
    /// Zero tolerance is **not** rejected here — whether a concrete solver forbids
    /// it is RFC 006's decision (decision M6). Returns a structured
    /// [`SolverError`]; never panics.
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
        Ok(())
    }
}

#[cfg(test)]
mod tests;
