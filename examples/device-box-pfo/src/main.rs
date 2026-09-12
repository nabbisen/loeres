//! Fixed-size box-constrained projected first-order solve on the edge path.
//!
//! What this example demonstrates (RFC 023 §11.3):
//!
//! - a box/bound-constrained problem over fixed-size static storage,
//! - a **caller-owned typed workspace**, sized by the shared const generic `N`
//!   so a wrong-sized workspace is a compile error rather than a runtime check,
//! - **workspace reuse across calls** — the same workspace solves three
//!   problems with no allocation between them,
//! - non-convergence at the iteration cap arriving as an `Ok` status, not an
//!   error.
//!
//! What it does **not** demonstrate: bare-metal buildability. This is a host
//! program and its own `main` uses `std`; that is permitted and changes nothing
//! about the edge crates, which are `#![no_std]` with no `alloc`. The
//! bare-metal claim belongs to `cargo xtask no-std`, which builds the edge
//! crates for `thumbv7em-none-eabihf`. What this example establishes is
//! **dependency reachability**: no server-side crate enters its graph, which
//! `cargo xtask examples` asserts against the resolved graph rather than
//! against the manifest.

use loeres::{SolveStatus, SolverError, VectorAccess, VectorAccessMut};
use loeres_backend_static::array::FixedVector;
use loeres_device::config::{DeviceSolveConfig, TimingMode};
use loeres_device::problem::ProjectedFirstOrderProblem;
use loeres_device::solve::{ProjectedFirstOrderWorkspace, solve_projected_first_order};

/// Problem dimension. One const generic ties the iterate, the gradient scratch,
/// and the bounds together at compile time.
const N: usize = 3;

/// Separable quadratic `f(x) = ½ Σ qᵢ (xᵢ − cᵢ)²` over a box.
///
/// Gradient `qᵢ (xᵢ − cᵢ)`; unconstrained optimum `cᵢ`; box-constrained optimum
/// `clamp(cᵢ, loᵢ, hiᵢ)`. Small and separable on purpose — the example is about
/// the calling contract, not about the objective.
struct BoxQuadratic {
    lower: FixedVector<f64, N>,
    upper: FixedVector<f64, N>,
    quadratic_diag: [f64; N],
    center: [f64; N],
    step_scale: f64,
}

impl ProjectedFirstOrderProblem<f64, N> for BoxQuadratic {
    type Bounds = FixedVector<f64, N>;

    fn validate_boundary(&self) -> Result<(), SolverError> {
        for (&lo, &hi) in self.lower.as_slice().iter().zip(self.upper.as_slice()) {
            if !lo.is_finite() || !hi.is_finite() {
                return Err(SolverError::NonFiniteInput);
            }
            if lo > hi {
                return Err(SolverError::InvalidInput);
            }
        }
        Ok(())
    }

    fn lower_bound(&self) -> &Self::Bounds {
        &self.lower
    }

    fn upper_bound(&self) -> &Self::Bounds {
        &self.upper
    }

    fn step_scale(&self) -> f64 {
        self.step_scale
    }

    fn gradient_at(
        &self,
        x: &FixedVector<f64, N>,
        grad: &mut FixedVector<f64, N>,
    ) -> Result<(), SolverError> {
        // Fallible access, no indexing: the same panic-averse discipline the
        // crates hold themselves to (requirements PANIC-004).
        for i in 0..N {
            let xi = x.get(i)?;
            let gi = self.quadratic_diag[i] * (xi - self.center[i]);
            grad.set(i, gi)?;
        }
        Ok(())
    }

    fn objective_at(&self, x: &FixedVector<f64, N>) -> Result<f64, SolverError> {
        let mut total = 0.0;
        for i in 0..N {
            let d = x.get(i)? - self.center[i];
            total += 0.5 * self.quadratic_diag[i] * d * d;
        }
        Ok(total)
    }
}

/// One labelled case: a problem, a starting iterate, and the iteration cap.
struct Case {
    label: &'static str,
    problem: BoxQuadratic,
    initial: [f64; N],
    max_iterations: u32,
}

fn main() -> Result<(), SolverError> {
    // The caller owns the workspace. It carries only scratch — the iterate is a
    // separate `&mut x` — and is built once here, then reused for every case
    // below. On a device this buffer would live in a static or on the caller's
    // stack; nothing in the solve path allocates.
    let mut workspace = ProjectedFirstOrderWorkspace::new(FixedVector::from_array([0.0; N]));

    for case in cases() {
        let config = DeviceSolveConfig {
            max_iterations: case.max_iterations,
            tolerance: 1e-9,
            // `EarlyExitAllowed` returns as soon as the iterate stops moving.
            // The `constant-iteration` feature adds a timing-stabilized mode;
            // it stabilizes the iteration *count*, and is not constant-time
            // execution in the cryptographic sense.
            timing_mode: TimingMode::EarlyExitAllowed,
        };

        let mut x = FixedVector::from_array(case.initial);
        let report = solve_projected_first_order(&case.problem, &mut x, &mut workspace, &config)?;

        // Non-convergence at the cap is a *status*, not an error: the solve
        // returned `Ok` and `x` holds the last projected iterate.
        let verdict = match report.status() {
            SolveStatus::Converged => "converged",
            SolveStatus::NotConverged => "not converged (iteration cap reached)",
            // `SolveStatus` is `#[non_exhaustive]` downstream, so a new variant
            // is a future addition to name, never a panic here.
            _ => "unrecognized status (this build of loeres is newer than this example)",
        };
        println!(
            "{:<22} {verdict} in {} iteration(s); x = {:?}, f(x) = {:.6e}",
            case.label,
            report.iterations_executed(),
            x.as_slice(),
            case.problem.objective_at(&x)?,
        );
    }

    Ok(())
}

fn cases() -> [Case; 3] {
    [
        // The optimum is interior, so the box never binds.
        Case {
            label: "interior optimum:",
            problem: BoxQuadratic {
                lower: FixedVector::from_array([-10.0; N]),
                upper: FixedVector::from_array([10.0; N]),
                quadratic_diag: [1.0, 2.0, 0.5],
                center: [1.5, -2.0, 4.0],
                step_scale: 0.4,
            },
            initial: [0.0; N],
            max_iterations: 2_000,
        },
        // The unconstrained optimum lies outside the box, so the solution sits
        // on the boundary: `clamp(center, lower, upper)`.
        Case {
            label: "bounds active:",
            problem: BoxQuadratic {
                lower: FixedVector::from_array([-1.0; N]),
                upper: FixedVector::from_array([1.0; N]),
                quadratic_diag: [1.0, 1.0, 1.0],
                center: [5.0, -5.0, 0.25],
                step_scale: 0.5,
            },
            initial: [0.0; N],
            max_iterations: 2_000,
        },
        // A cap far below what this problem needs. The solve still returns
        // `Ok`; only the status says the criterion was not met.
        Case {
            label: "cap reached:",
            problem: BoxQuadratic {
                lower: FixedVector::from_array([-10.0; N]),
                upper: FixedVector::from_array([10.0; N]),
                quadratic_diag: [1.0, 1.0, 1.0],
                center: [9.0, 9.0, 9.0],
                step_scale: 0.01,
            },
            initial: [0.0; N],
            max_iterations: 5,
        },
    ]
}
