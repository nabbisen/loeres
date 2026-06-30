//! Cluster-owned cooperative cancellation (RFC 008 §3.6 / D5).
//!
//! Runtime-agnostic: this is the single token type observed by the sync,
//! parallel (`parallel-rayon`), and async (`async-tokio`) paths alike — no
//! Tokio or Rayon type appears in the public surface. Cancellation is
//! *cooperative*, not preemptive: it is observed at job boundaries and at the
//! configured polling interval, never interrupting a running job mid-step.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// A cheap-to-clone cooperative cancellation handle.
///
/// All clones share one flag. `cancel()` stores with [`Ordering::Release`] and
/// `is_cancelled()` loads with [`Ordering::Acquire`], so a worker that observes
/// cancellation also observes everything the canceller did beforehand.
#[derive(Clone, Debug, Default)]
pub struct ClusterCancellationToken {
    flag: Arc<AtomicBool>,
}

impl ClusterCancellationToken {
    /// A fresh, un-cancelled token.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Request cancellation. Idempotent; observed cooperatively by workers.
    pub fn cancel(&self) {
        self.flag.store(true, Ordering::Release);
    }

    /// Whether cancellation has been requested.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::Acquire)
    }
}

#[cfg(test)]
mod tests;
