use super::{DeviceSolveConfig, TimingMode};
use loeres::SolverError;

fn cfg(max: u32, tol: f64, mode: TimingMode) -> DeviceSolveConfig<f64> {
    DeviceSolveConfig {
        max_iterations: max,
        tolerance: tol,
        timing_mode: mode,
    }
}

#[test]
fn valid_config_passes() {
    assert!(
        cfg(100, 1e-9, TimingMode::EarlyExitAllowed)
            .validate()
            .is_ok()
    );
}

#[test]
fn zero_tolerance_allowed_at_rfc005_level() {
    // RFC 005 does not reject zero tolerance; that is RFC 006's decision (M6).
    assert!(
        cfg(50, 0.0, TimingMode::EarlyExitAllowed)
            .validate()
            .is_ok()
    );
}

#[test]
fn zero_max_iterations_rejected() {
    assert!(matches!(
        cfg(0, 1e-6, TimingMode::EarlyExitAllowed).validate(),
        Err(SolverError::InvalidInput)
    ));
}

#[test]
fn negative_tolerance_rejected() {
    assert!(matches!(
        cfg(10, -1e-6, TimingMode::EarlyExitAllowed).validate(),
        Err(SolverError::InvalidInput)
    ));
}

#[test]
fn non_finite_tolerance_rejected() {
    assert!(matches!(
        cfg(10, f64::NAN, TimingMode::EarlyExitAllowed).validate(),
        Err(SolverError::NonFiniteInput)
    ));
    assert!(matches!(
        cfg(10, f64::INFINITY, TimingMode::EarlyExitAllowed).validate(),
        Err(SolverError::NonFiniteInput)
    ));
}

#[test]
fn timing_mode_early_exit_is_copy_eq() {
    let m = TimingMode::EarlyExitAllowed;
    assert_eq!(m, TimingMode::EarlyExitAllowed);
}

#[cfg(feature = "constant-iteration")]
#[test]
fn constant_iteration_available_under_feature() {
    assert!(
        cfg(10, 1e-6, TimingMode::ConstantIteration)
            .validate()
            .is_ok()
    );
}
