//! Optional native/legacy solver gateway boundary (RFC 009).
//!
//! This module defines safe Rust boundary categories and a pure Rust mock job
//! used to exercise gateway status/error mapping. It does not link a native
//! library and contains no unsafe code.

use loeres::{BaseScalar, SolveReport, SolverError};
use loeres_backend_std::DenseVector;

use crate::batch::{BatchItemOutcome, ClusterSolution};
use crate::solve::{ClusterExecutionContext, ClusterJob};

/// Coarse gateway backend category.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum GatewayBackendKind {
    /// Pure Rust mock backend.
    Mock,
    /// Native in-process backend.
    Native,
    /// Out-of-process service backend.
    ExternalService,
    /// Backend did not self-identify.
    Unknown,
}

/// Adapter-side thread-safety classification.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum GatewayThreadSafety {
    /// Reentrant backend.
    Reentrant,
    /// Concurrent only with one workspace per call.
    IndependentWorkspaceOnly,
    /// Adapter provides internal synchronization.
    GloballySynchronized,
    /// Raw backend is single-thread-only and must be wrapped or rejected.
    SingleThreadOnly,
}

/// Coarse gateway failure category before mapping to Loeres outcomes.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum GatewayFailureKind {
    /// Backend/resource unavailable.
    BackendUnavailable,
    /// Gateway model rejected structurally.
    InvalidModel,
    /// Numerical backend failure.
    NumericalFailure,
    /// Gateway timeout.
    Timeout,
    /// Cooperative cancellation.
    Cancelled,
    /// Foreign panic/abort/signal category. Recovery depends on concrete adapter.
    ForeignPanicOrAbort,
    /// Adapter contract was violated.
    ContractViolation,
}

/// Map a gateway failure into the existing allocation-free solver error
/// vocabulary used by `BatchItemOutcome::Failed`.
#[must_use]
pub const fn solver_error_from_gateway_failure(failure: GatewayFailureKind) -> SolverError {
    match failure {
        GatewayFailureKind::BackendUnavailable | GatewayFailureKind::ForeignPanicOrAbort => {
            SolverError::BackendUnavailable
        }
        GatewayFailureKind::InvalidModel => SolverError::InvalidInput,
        GatewayFailureKind::NumericalFailure => SolverError::NumericalDomain,
        GatewayFailureKind::Timeout | GatewayFailureKind::Cancelled => SolverError::Cancelled,
        GatewayFailureKind::ContractViolation => SolverError::InternalInvariantViolation,
    }
}

/// Pure Rust mock gateway response.
#[derive(Clone, Debug)]
pub enum MockGatewayResponse<S> {
    /// Structured terminal report and produced dense solution.
    Solved {
        /// Produced dense solution.
        solution: DenseVector<S>,
        /// Structured terminal report; may be not converged.
        report: SolveReport,
    },
    /// Gateway failed with a structured category.
    Failed(GatewayFailureKind),
}

/// Safe mock gateway job used by RFC 009 tests and examples.
#[derive(Clone, Debug)]
pub struct MockGatewayJob<S> {
    response: MockGatewayResponse<S>,
    thread_safety: GatewayThreadSafety,
}

impl<S> MockGatewayJob<S> {
    /// Build a mock gateway job, rejecting raw single-thread-only adapters.
    ///
    /// A concrete single-thread backend must be wrapped to present as
    /// [`GatewayThreadSafety::GloballySynchronized`] before it can satisfy the
    /// `ClusterJob` seam.
    pub fn try_new(
        response: MockGatewayResponse<S>,
        thread_safety: GatewayThreadSafety,
    ) -> Result<Self, GatewayFailureKind> {
        if matches!(thread_safety, GatewayThreadSafety::SingleThreadOnly) {
            return Err(GatewayFailureKind::ContractViolation);
        }
        Ok(Self {
            response,
            thread_safety,
        })
    }

    /// Thread-safety class exposed by the adapter.
    #[must_use]
    pub const fn thread_safety(&self) -> GatewayThreadSafety {
        self.thread_safety
    }

    /// Backend kind for the mock adapter.
    #[must_use]
    pub const fn backend_kind(&self) -> GatewayBackendKind {
        GatewayBackendKind::Mock
    }
}

impl<S> ClusterJob<S> for MockGatewayJob<S>
where
    S: BaseScalar + Clone + Send + Sync + 'static,
{
    fn run_boxed(&self, _ctx: &ClusterExecutionContext) -> BatchItemOutcome<S> {
        match &self.response {
            MockGatewayResponse::Solved { solution, report } => BatchItemOutcome::Solved {
                solution: ClusterSolution::DenseVector(solution.clone()),
                report: *report,
            },
            MockGatewayResponse::Failed(
                GatewayFailureKind::Cancelled | GatewayFailureKind::Timeout,
            ) => BatchItemOutcome::Cancelled,
            MockGatewayResponse::Failed(failure) => BatchItemOutcome::Failed {
                error: solver_error_from_gateway_failure(*failure),
            },
        }
    }
}

#[cfg(test)]
mod tests;
