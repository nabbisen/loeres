use super::{DeviceWorkspace, DeviceWorkspaceDiagnostic, WorkspaceFor};
use loeres::DiagnosticSnapshot;

// Test fixtures only: RFC 005 owns the lifecycle traits; concrete solver
// workspaces and problem families are RFC 006-owned.
struct TestWorkspace {
    scratch: [f64; 4],
    entered: bool,
}

impl DeviceWorkspace for TestWorkspace {
    fn reset_for_entry(&mut self) {
        // Overwrite-on-use: logical init, no full-buffer zeroing required.
        self.entered = true;
    }
}

impl DeviceWorkspaceDiagnostic for TestWorkspace {
    fn diagnostic(&self) -> DiagnosticSnapshot {
        DiagnosticSnapshot::EMPTY
    }
}

struct TestProblem;
struct TestFamily;

impl WorkspaceFor<TestProblem> for TestFamily {
    type Workspace = TestWorkspace;
    fn required_workspace_bytes() -> usize {
        core::mem::size_of::<TestWorkspace>()
    }
}

#[test]
fn reset_for_entry_logically_initializes() {
    let mut w = TestWorkspace {
        scratch: [1.0; 4],
        entered: false,
    };
    w.reset_for_entry();
    assert!(w.entered);
}

#[test]
fn diagnostic_accessor_returns_snapshot() {
    let w = TestWorkspace {
        scratch: [0.0; 4],
        entered: false,
    };
    assert_eq!(w.diagnostic(), DiagnosticSnapshot::EMPTY);
}

#[test]
fn workspace_for_reports_size_of_bytes() {
    assert_eq!(
        <TestFamily as WorkspaceFor<TestProblem>>::required_workspace_bytes(),
        core::mem::size_of::<TestWorkspace>()
    );
}

#[test]
fn dirty_workspace_is_reusable_without_clearing() {
    // RFC 005 owns the trait contract; full reuse-after-failure is RFC 006.
    // Here: a dirty workspace re-enters cleanly without any manual clear.
    let mut w = TestWorkspace {
        scratch: [9.9; 4],
        entered: false,
    };
    w.reset_for_entry();
    w.scratch[0] = 1.0; // a "solve" dirties scratch
    w.reset_for_entry(); // re-entry on the same dirty workspace
    assert!(w.entered);
}
