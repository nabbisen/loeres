//! Model-predictive control of a first-order process with an actuator limit.
//!
//! A heater, a valve or a motor drives a process that must track a setpoint,
//! but the actuator saturates and every move costs energy. Model-predictive
//! control plans a short sequence of future moves, applies only the first, and
//! plans again at the next sample. Each plan is a small box-constrained quadratic
//! programme, solved on the device with a caller-owned workspace.
//!
//! The example is for an embedded controller that must reach no server-side
//! crate and no runtime service. Its problem and its result are the ones the
//! edge path exists for; `cargo xtask examples` asserts the dependency isolation.
//!
//! The reduction from the physical problem to the programme is the part a reader
//! cannot reconstruct, so it is stated here:
//!
//! - **Process.** `x(k+1) = A·x(k) + B·u(k)` with `A = 0.9` and `B = 0.5`. The
//!   actuator accepts `u` only in `[-1, 1]`.
//! - **Plan.** Over `N = 3` moves, stacking the predictions gives `x = F·x0 + G·u`
//!   where `F[k] = A^(k+1)` and `G[k][j] = A^(k-j)·B` for `j ≤ k`, zero otherwise.
//! - **Cost.** `Σₖ (xₖ − r)² + ρ·Σₖ uₖ²` with setpoint `r = 1` and move penalty
//!   `ρ = 0.1`. With `e = F·x0 − r·1`, the cost is `uᵀ(GᵀG + ρI)u + 2eᵀG·u + eᵀe`.
//! - **Programme.** Therefore `H = 2(GᵀG + ρI)`, `g = 2Gᵀe`, and the cost is
//!   `½uᵀHu + gᵀu + eᵀe`: the box-constrained shape the kernel solves, with the
//!   actuator limit as the box.
//! - **Step.** The kernel needs `0 < α < 2/λmax(H)`. The Gershgorin bound
//!   `λmax ≤ maxᵢ Σⱼ |Hᵢⱼ|` gives `α = 1/maxᵢ Σⱼ |Hᵢⱼ|`, which is safely inside.
//!
//! Only the first move is applied. The rest of the plan is discarded and the
//! programme is rebuilt from the measured state at the next sample.
//!
//! What it prints, one row per sample: the state before the move, the move
//! applied, whether the actuator limit is active, and the outcome. A `yes` at the
//! limit is the constraint doing its job. A `not converged` row means the plan was
//! not solved to tolerance, so the example applies no planned move rather than
//! trust it.

use loeres::{SolveStatus, SolverError, VectorAccess, VectorAccessMut};
use loeres_backend_static::array::FixedVector;
use loeres_device::config::{DeviceSolveConfig, TimingMode};
use loeres_device::problem::ProjectedFirstOrderProblem;
use loeres_device::solve::{ProjectedFirstOrderWorkspace, solve_projected_first_order};

/// Moves in one plan.
const N: usize = 3;
/// Process decay and input gain.
const A: f64 = 0.9;
const B: f64 = 0.5;
/// Setpoint and move penalty.
const SETPOINT: f64 = 1.0;
const RHO: f64 = 0.1;
/// Actuator limit: the box is `[-LIMIT, LIMIT]` on every move.
const LIMIT: f64 = 1.0;
/// Samples to run closed loop.
const STEPS: usize = 8;

/// The condensed programme for one sample: `½uᵀHu + gᵀu + constant` over the box.
struct MpcPlan {
    hessian: [[f64; N]; N],
    linear: [f64; N],
    constant: f64,
    lower: FixedVector<f64, N>,
    upper: FixedVector<f64, N>,
    step_scale: f64,
}

impl MpcPlan {
    /// Builds the programme from the measured state `x0`.
    fn new(x0: f64) -> Self {
        // `g[k][j]` is the effect of move `j` on the state after move `k`;
        // `f[k]` is the free response of that state to `x0`.
        let g: [[f64; N]; N] = std::array::from_fn(|k| {
            std::array::from_fn(|j| {
                if j <= k {
                    A.powi((k - j) as i32) * B
                } else {
                    0.0
                }
            })
        });
        let f: [f64; N] = std::array::from_fn(|k| A.powi(k as i32 + 1));
        let e: [f64; N] = std::array::from_fn(|k| f[k] * x0 - SETPOINT);

        let hessian: [[f64; N]; N] = std::array::from_fn(|i| {
            std::array::from_fn(|j| {
                let gtg: f64 = g.iter().map(|row| row[i] * row[j]).sum();
                let ridge = if i == j { RHO } else { 0.0 };
                2.0 * (gtg + ridge)
            })
        });
        let linear: [f64; N] = std::array::from_fn(|i| {
            2.0 * g.iter().zip(&e).map(|(row, ek)| row[i] * ek).sum::<f64>()
        });
        let constant = e.iter().map(|ek| ek * ek).sum();

        let radius = hessian
            .iter()
            .map(|row| row.iter().map(|h| h.abs()).sum::<f64>())
            .fold(0.0, f64::max);

        Self {
            hessian,
            linear,
            constant,
            lower: FixedVector::from_array([-LIMIT; N]),
            upper: FixedVector::from_array([LIMIT; N]),
            step_scale: 1.0 / radius,
        }
    }
}

/// Reads the iterate into a plain array, failing through the fallible accessor.
fn read(x: &FixedVector<f64, N>) -> Result<[f64; N], SolverError> {
    let mut values = [0.0; N];
    for (i, slot) in values.iter_mut().enumerate() {
        *slot = x.get(i)?;
    }
    Ok(values)
}

impl ProjectedFirstOrderProblem<f64, N> for MpcPlan {
    type Bounds = FixedVector<f64, N>;

    fn validate_boundary(&self) -> Result<(), SolverError> {
        for (lo, hi) in self.lower.as_slice().iter().zip(self.upper.as_slice()) {
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
        let u = read(x)?;
        for (i, row) in self.hessian.iter().enumerate() {
            let hu: f64 = row.iter().zip(&u).map(|(h, uj)| h * uj).sum();
            grad.set(i, hu + self.linear[i])?;
        }
        Ok(())
    }

    fn objective_at(&self, x: &FixedVector<f64, N>) -> Result<f64, SolverError> {
        let u = read(x)?;
        let mut total = self.constant;
        for (i, row) in self.hessian.iter().enumerate() {
            let hu: f64 = row.iter().zip(&u).map(|(h, uj)| h * uj).sum();
            total += u[i] * (0.5 * hu + self.linear[i]);
        }
        Ok(total)
    }
}

// README-EXAMPLE-BEGIN
/// One sample: plan from the measured state and return the first move, or
/// `None` if the plan did not converge, in which case no move is trusted.
fn plan_move(
    state: f64,
    workspace: &mut ProjectedFirstOrderWorkspace<f64, N>,
    config: &DeviceSolveConfig<f64>,
) -> Result<Option<f64>, SolverError> {
    let plan = MpcPlan::new(state);
    let mut moves = FixedVector::from_array([0.0; N]);
    let report = solve_projected_first_order(&plan, &mut moves, workspace, config)?;
    match report.status() {
        SolveStatus::Converged => Ok(Some(moves.get(0)?)),
        // `SolveStatus` is `#[non_exhaustive]` downstream: every status other
        // than `Converged` is treated as untrusted, so a future variant is held.
        _ => Ok(None),
    }
}
// README-EXAMPLE-END

fn main() -> Result<(), SolverError> {
    // One workspace for the whole run: the plans differ, the scratch does not.
    let mut workspace = ProjectedFirstOrderWorkspace::new(FixedVector::from_array([0.0; N]));
    let config = DeviceSolveConfig {
        max_iterations: 20_000,
        tolerance: 1e-9,
        timing_mode: TimingMode::EarlyExitAllowed,
    };

    println!(
        "{:>4} {:>8} {:>8} {:>8}  outcome",
        "step", "state", "move", "at limit"
    );

    let mut state = 0.0;
    for step in 0..STEPS {
        let (first_move, outcome) = match plan_move(state, &mut workspace, &config)? {
            Some(u) => (u, "applied"),
            // No trusted move this sample: apply zero, the assumed rest position.
            None => (0.0, "not converged: zero move applied"),
        };
        let at_limit = (first_move.abs() - LIMIT).abs() < 1e-6;
        println!(
            "{step:>4} {state:>8.4} {first_move:>8.4} {:>8}  {outcome}",
            if at_limit { "yes" } else { "no" },
        );

        // Act on the first move only; the process is the same model the plan used.
        state = A * state + B * first_move;
    }

    println!("\nafter {STEPS} samples: state {state:.4}, setpoint {SETPOINT}");
    Ok(())
}
