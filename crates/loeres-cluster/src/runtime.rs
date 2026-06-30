//! Runtime-facing orchestration: configuration, policies, cancellation, and the
//! batch executors (RFC 008 §3.1 / §3.5 / §3.6).
//!
//! The public surface here is runtime-agnostic (RFC 008 F4): no Tokio or Rayon
//! type appears. Parallel and async backends live behind `parallel-rayon` and
//! `async-tokio` as internal implementation details of [`executor`].

pub mod cancel;
pub(crate) mod executor;

pub use cancel::ClusterCancellationToken;

use loeres::validation::{TrustedByCaller, ValidationScope, ValidationState};

/// How a batch is executed across workers.
///
/// `Parallel` is honored only when `parallel-rayon` is enabled; without that
/// feature it is normalized deterministically to `Sequential` (see
/// [`ClusterSolveConfig::effective_execution`]).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum BatchExecutionPolicy {
    /// Run items one at a time on the calling thread.
    #[default]
    Sequential,
    /// Run items across a worker pool (requires `parallel-rayon`).
    Parallel,
}

/// Reserved knob for future budget-aware dispatch (RFC 008 §3.4 / D8 / T3).
///
/// In v0.13.0 this does not yet affect dispatch: with a single solution variant
/// and no production kernel there is no monomorphization spread to arbitrate,
/// and the enforcing size-budget gate is owned by RFC 010. The variant set is
/// deliberately small and active; `AutoByBudget` is intentionally **absent**
/// until RFC 010 supplies a real metric.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum DispatchPolicy {
    /// Prefer fully generic kernels (default).
    #[default]
    PreferGenericKernels,
    /// Prefer routing through the dynamic dispatch barrier.
    PreferHybridDispatch,
}

/// Cluster validation policy, mapped onto the RFC 012 vocabulary (RFC 008 §3.1 /
/// F1 / D9). This is **not** a parallel trust model. The policy is *resolved*
/// against supplied evidence by [`resolve`](Self::resolve), which performs **no
/// validation scans** and does not perform or replace backend/model validation —
/// it only decides, from the policy plus any provided [`ValidationState`], the
/// effective state or a [`MissingCoverage`] gap.
///
/// In v0.13.0 there is no production std-side kernel and no cluster model
/// scanner, so the resolver never fabricates evidence: the validating job (the
/// future-kernel seam) runs the scans and records the coverage that `resolve`
/// then checks.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ClusterValidationPolicy {
    /// Require that every input has actually been validated: the validating job
    /// runs the required scans and records a [`ValidationState::Validated`]
    /// coverage, which [`resolve`](Self::resolve) verifies covers the required
    /// scope. The resolver runs no scans and does not accept a trust assertion in
    /// lieu of validation.
    ValidateAllInputs,
    /// Respect a backend/caller-provided [`ValidationState`] (validated *or*
    /// trusted); reject missing required coverage rather than silently trusting
    /// it. RFC 012 is representation-only and no backend emits a populated state
    /// yet, so in v0.13.0 this consumes whatever the job supplies.
    RespectBackendValidationState,
    /// The caller assumes responsibility for a scope, carried as RFC 012
    /// [`TrustedByCaller`] evidence; the asserted scope is skipped.
    TrustedByCaller(TrustedByCaller),
}

/// Why a required validation scope was not established.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct MissingCoverage {
    /// The coverage the dispatch boundary required.
    pub required: ValidationScope,
    /// The coverage actually established/asserted.
    pub covered: ValidationScope,
}

impl ClusterValidationPolicy {
    /// Resolve the policy against the `required` scope and any `provided`
    /// evidence, yielding the effective [`ValidationState`] or a
    /// [`MissingCoverage`] rejection.
    ///
    /// This is **pure policy/evidence resolution**: it runs no validation scans
    /// and does not perform or replace backend/model validation. It never
    /// fabricates a [`ValidationState::Validated`] — that arises only by passing
    /// through a `provided` coverage that the validating job already recorded.
    ///
    /// # Errors
    /// Returns [`MissingCoverage`] when the provided/asserted scope does not
    /// cover `required` under the policy — including when `ValidateAllInputs`
    /// receives no recorded validation, or a trust assertion where validation is
    /// required.
    pub fn resolve(
        &self,
        required: ValidationScope,
        provided: Option<ValidationState>,
    ) -> Result<ValidationState, MissingCoverage> {
        match self {
            Self::ValidateAllInputs => match provided {
                Some(ValidationState::Validated(cov)) if cov.scope().contains(required) => {
                    Ok(ValidationState::Validated(cov))
                }
                Some(state) => Err(MissingCoverage {
                    required,
                    covered: state_scope(&state),
                }),
                None => Err(MissingCoverage {
                    required,
                    covered: ValidationScope::EMPTY,
                }),
            },
            Self::RespectBackendValidationState => match provided {
                Some(ValidationState::Validated(cov)) if cov.scope().contains(required) => {
                    Ok(ValidationState::Validated(cov))
                }
                Some(ValidationState::Trusted(t)) if t.scope.contains(required) => {
                    Ok(ValidationState::Trusted(t))
                }
                Some(state) => Err(MissingCoverage {
                    required,
                    covered: state_scope(&state),
                }),
                None => Err(MissingCoverage {
                    required,
                    covered: ValidationScope::EMPTY,
                }),
            },
            Self::TrustedByCaller(t) if t.scope.contains(required) => {
                Ok(ValidationState::Trusted(*t))
            }
            Self::TrustedByCaller(t) => Err(MissingCoverage {
                required,
                covered: t.scope,
            }),
        }
    }
}

/// The coverage scope a [`ValidationState`] establishes or asserts.
fn state_scope(state: &ValidationState) -> ValidationScope {
    match state {
        ValidationState::Unvalidated => ValidationScope::EMPTY,
        ValidationState::Validated(cov) => cov.scope(),
        ValidationState::Trusted(t) => t.scope,
    }
}

/// Server-side batch configuration. Runtime-agnostic and `std`.
#[derive(Clone, Debug)]
pub struct ClusterSolveConfig {
    /// Maximum worker parallelism (must be non-zero).
    pub max_parallelism: usize,
    /// Optional cooperative wall-clock budget, observed at item boundaries.
    pub timeout: Option<std::time::Duration>,
    /// Hint for *job-internal* cancellation polling, surfaced to jobs via
    /// [`ClusterExecutionContext::poll_interval`](crate::solve::ClusterExecutionContext::poll_interval);
    /// `0` means "poll as often as practical". The executor itself checks
    /// cancellation (and the timeout deadline) at every item boundary regardless
    /// of this value.
    pub cancellation_poll_interval: u32,
    /// Validation policy (RFC 012 vocabulary).
    pub validation_policy: ClusterValidationPolicy,
    /// Reserved dispatch knob.
    pub dispatch_policy: DispatchPolicy,
    /// Sequential vs parallel execution.
    pub execution_policy: BatchExecutionPolicy,
}

impl Default for ClusterSolveConfig {
    fn default() -> Self {
        Self {
            max_parallelism: 1,
            timeout: None,
            cancellation_poll_interval: 0,
            validation_policy: ClusterValidationPolicy::ValidateAllInputs,
            dispatch_policy: DispatchPolicy::PreferGenericKernels,
            execution_policy: BatchExecutionPolicy::Sequential,
        }
    }
}

impl ClusterSolveConfig {
    /// Validate global configuration before dispatch.
    ///
    /// # Errors
    /// [`ClusterError::InvalidConfig`] when `max_parallelism == 0`.
    pub fn validate(&self) -> Result<(), ClusterError> {
        if self.max_parallelism == 0 {
            return Err(ClusterError::InvalidConfig);
        }
        Ok(())
    }

    /// The execution policy actually used, normalizing `Parallel` to
    /// `Sequential` when `parallel-rayon` is not compiled in (deterministic
    /// fallback — RFC 008 T3 spirit).
    #[must_use]
    pub fn effective_execution(&self) -> BatchExecutionPolicy {
        match self.execution_policy {
            BatchExecutionPolicy::Parallel if cfg!(feature = "parallel-rayon") => {
                BatchExecutionPolicy::Parallel
            }
            _ => BatchExecutionPolicy::Sequential,
        }
    }
}

/// Orchestration-level failure that prevents producing a batch report at all
/// (RFC 008 §3.2 / D4). Per-item solver failures live in
/// [`BatchItemOutcome`](crate::batch::BatchItemOutcome), never here.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ClusterError {
    /// Invalid global configuration (e.g. `max_parallelism == 0`).
    InvalidConfig,
    /// The executor or worker pool failed to initialize.
    ExecutorInit,
    /// The runtime shut down before the batch completed.
    Shutdown,
}

#[cfg(test)]
mod tests;
