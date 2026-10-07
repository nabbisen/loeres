//! RFC 040 measurement fixture.
//!
//! Calls `solve_constrained_projected_first_order` at one fixed `(N, M) = (8, 4)` so
//! the monomorphised solver code is emitted and reachable, then measured on this
//! crate's own compiled object (`cargo rustc --release --target thumbv7em-none-eabihf
//! -- --emit=obj`, summing `.text*`/`.rodata*` with `size -A`). This is a measurement
//! fixture, not an example: see the crate's own `README.md` and its `Cargo.toml`.
//!
//! **An instantiation that does not call the kernel measures nothing.** The
//! architect's first probe (RFC 040 §0.2) computed only
//! `size_of::<ConstrainedProjectedWorkspace<f64, 8, 4>>()`, a compile-time constant
//! that never reaches the solver, and measured 14 bytes — the wrapper and a panic
//! shim. `reference_solve` below takes `seed` as a genuine runtime argument (not a
//! literal), so the compiler cannot constant-fold the solve away.
//!
//! `(N, M)`, the release profile and `panic = "abort"` are this measurement's declared
//! constants (RFC 040 §2.1, §5): a different instantiation, profile or panic strategy
//! is a different number, and the report must say so — `cargo xtask size-budget`
//! prints all three beside the figure.

#![no_std]

use loeres::{BoxBounds, LinearInequalities, QuadraticObjective};
use loeres_backend_static::array::{FixedMatrix, FixedVector};
use loeres_device::config::TimingMode;
use loeres_device::solve::{
    ConstrainedProjectedWorkspace, ConstrainedSolveConfig, solve_constrained_projected_first_order,
};

/// The reference instantiation's declared dimensions. A different `(N, M)` is a
/// different number (RFC 040 §2.1).
pub const N: usize = 8;
pub const M: usize = 4;
const NN: usize = N * N;
const MN: usize = M * N;

/// A tridiagonal box-and-row problem at the fixed `(N, M)`, mirroring `bench`'s own
/// corpus family shape (not imported — this fixture depends on nothing outside the
/// edge crates).
struct Problem {
    q: FixedMatrix<f64, N, N, NN>,
    c: FixedVector<f64, N>,
    lower: FixedVector<f64, N>,
    upper: FixedVector<f64, N>,
    a: FixedMatrix<f64, M, N, MN>,
    b: FixedVector<f64, M>,
}

impl QuadraticObjective<f64> for Problem {
    type Hessian = FixedMatrix<f64, N, N, NN>;
    type Linear = FixedVector<f64, N>;
    fn hessian(&self) -> &Self::Hessian {
        &self.q
    }
    fn linear_term(&self) -> &Self::Linear {
        &self.c
    }
}

impl BoxBounds<f64> for Problem {
    type Bound = FixedVector<f64, N>;
    fn lower_bounds(&self) -> &Self::Bound {
        &self.lower
    }
    fn upper_bounds(&self) -> &Self::Bound {
        &self.upper
    }
}

impl LinearInequalities<f64> for Problem {
    type Constraints = FixedMatrix<f64, M, N, MN>;
    type Rhs = FixedVector<f64, M>;
    fn constraint_matrix(&self) -> &Self::Constraints {
        &self.a
    }
    fn constraint_rhs(&self) -> &Self::Rhs {
        &self.b
    }
}

/// Builds the fixed `(N, M)` problem with `seed` perturbing the linear term, so the
/// solver's own input is a runtime value, never a literal the compiler could fold.
fn problem(seed: f64) -> Problem {
    let mut q_data = [0.0_f64; NN];
    for i in 0..N {
        q_data[i * N + i] = 2.0;
        if i + 1 < N {
            q_data[i * N + i + 1] = 0.5;
            q_data[(i + 1) * N + i] = 0.5;
        }
    }
    let mut a_data = [0.0_f64; MN];
    for r in 0..M {
        for k in 0..3 {
            a_data[r * N + (r + k) % N] = 1.0;
        }
    }
    Problem {
        q: FixedMatrix::from_row_major_array(q_data),
        c: FixedVector::from_array([-1.0 - seed; N]),
        lower: FixedVector::from_array([0.0; N]),
        upper: FixedVector::from_array([10.0; N]),
        a: FixedMatrix::from_row_major_array(a_data),
        b: FixedVector::from_array([1.0; M]),
    }
}

/// Calls the device-constrained kernel once at the declared `(N, M)`. The return value
/// folds the report's own fields together, so nothing about the solve is dead code: a
/// caller elsewhere could observe it, and the compiler must assume one might.
#[unsafe(no_mangle)]
pub extern "C" fn reference_solve(seed: f64) -> u32 {
    let problem = problem(seed);
    let mut workspace = ConstrainedProjectedWorkspace::<f64, N, M>::new(
        FixedVector::from_array([0.0; N]),
        FixedVector::from_array([0.0; N]),
        FixedVector::from_array([0.0; N]),
        FixedVector::from_array([0.0; M]),
        FixedVector::from_array([0.0; M]),
    );
    let config = ConstrainedSolveConfig {
        max_iterations: 500,
        tolerance: 1e-9,
        timing_mode: TimingMode::EarlyExitAllowed,
        projection_max_sweeps: 100,
        projection_tolerance: 1e-9,
    };
    let mut x = FixedVector::from_array([0.0; N]);
    match solve_constrained_projected_first_order(&problem, 0.3, &mut x, &mut workspace, &config) {
        Ok(report) => report.iterations_executed() ^ report.projection_cap_hits(),
        Err(_) => u32::MAX,
    }
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
