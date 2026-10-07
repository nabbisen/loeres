use super::{BoxQuadratic, FRAC_BITS, N, Q};
use loeres::scalar::Q32;
use loeres::{BaseScalar, SolveStatus, VectorAccess};
use loeres_backend_static::array::FixedVector;
use loeres_device::config::{DeviceSolveConfig, TimingMode};
use loeres_device::solve::{ProjectedFirstOrderWorkspace, solve_projected_first_order};

use super::{DenseQp, exact_optimum};

/// One corpus instance's raw `f64` data, before it is built into a `Q32` or
/// `f64` [`BoxQuadratic`] or an exact [`DenseQp`].
struct Instance {
    quadratic_diag: [f64; N],
    center: [f64; N],
    lower: [f64; N],
    upper: [f64; N],
}

fn lcg(seed: u64) -> impl FnMut() -> f64 {
    let mut state = seed;
    move || {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((state >> 11) as f64) / ((1u64 << 53) as f64)
    }
}

/// `count` random separable box QPs: `qᵢ` in `[0.5, 3.0]` (strictly convex),
/// `cᵢ` (the unconstrained optimum) in `[-3, 3]`, and a box that contains the
/// center about half the time and excludes it the other half, so both the
/// interior-optimum and the bounds-active cases are exercised.
fn random_corpus(count: usize, seed: u64) -> Vec<Instance> {
    let mut next = lcg(seed);
    (0..count)
        .map(|_| {
            let mut quadratic_diag = [0.0; N];
            let mut center = [0.0; N];
            let mut lower = [0.0; N];
            let mut upper = [0.0; N];
            for i in 0..N {
                quadratic_diag[i] = 0.5 + next() * 2.5;
                center[i] = (next() - 0.5) * 6.0;
                let half_width = 0.5 + next() * 2.0;
                // Shift the box off-center about half the time, so the
                // unconstrained optimum sometimes lies outside it.
                let offset = if next() < 0.5 {
                    0.0
                } else {
                    (next() - 0.5) * 4.0
                };
                lower[i] = center[i] + offset - half_width;
                upper[i] = center[i] + offset + half_width;
            }
            Instance {
                quadratic_diag,
                center,
                lower,
                upper,
            }
        })
        .collect()
}

fn max_diag(instance: &Instance) -> f64 {
    instance.quadratic_diag.iter().cloned().fold(0.0, f64::max)
}

fn exact_reference(instance: &Instance) -> Vec<f64> {
    let c: Vec<f64> = instance
        .quadratic_diag
        .iter()
        .zip(&instance.center)
        .map(|(q, t)| -(q * t))
        .collect();
    let mut q = vec![0.0; N * N];
    for (i, d) in instance.quadratic_diag.iter().enumerate() {
        q[i * N + i] = *d;
    }
    let problem = DenseQp {
        n: N,
        q,
        c,
        lower: instance.lower.to_vec(),
        upper: instance.upper.to_vec(),
        a: Vec::new(),
        b: Vec::new(),
    };
    exact_optimum(&problem)
        .unwrap_or_else(|| panic!("a feasible box problem must have a KKT point: {instance:?}"))
}

impl core::fmt::Debug for Instance {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Instance {{ q: {:?}, center: {:?}, lower: {:?}, upper: {:?} }}",
            self.quadratic_diag, self.center, self.lower, self.upper
        )
    }
}

fn solve_q(instance: &Instance) -> (SolveStatus, [f64; N]) {
    let build = |v: f64| Q32::<FRAC_BITS>::from_f64(v);
    let problem = BoxQuadratic {
        lower: FixedVector::from_array(instance.lower.map(build)),
        upper: FixedVector::from_array(instance.upper.map(build)),
        quadratic_diag: instance.quadratic_diag.map(build),
        center: instance.center.map(build),
        step_scale: build(0.5 / max_diag(instance)),
    };
    let mut workspace = ProjectedFirstOrderWorkspace::new(FixedVector::from_array([Q::zero(); N]));
    let config = DeviceSolveConfig {
        max_iterations: 5_000,
        tolerance: Q::from_raw(4),
        timing_mode: TimingMode::EarlyExitAllowed,
    };
    let mut x = FixedVector::from_array([Q::zero(); N]);
    let report =
        solve_projected_first_order(&problem, &mut x, &mut workspace, &config).expect("solve");
    let iterate = core::array::from_fn(|i| x.get(i).unwrap().to_f64());
    (report.status(), iterate)
}

fn solve_f64(instance: &Instance) -> (SolveStatus, [f64; N]) {
    let problem = BoxQuadratic {
        lower: FixedVector::from_array(instance.lower),
        upper: FixedVector::from_array(instance.upper),
        quadratic_diag: instance.quadratic_diag,
        center: instance.center,
        step_scale: 0.5 / max_diag(instance),
    };
    let mut workspace = ProjectedFirstOrderWorkspace::new(FixedVector::from_array([0.0; N]));
    let config = DeviceSolveConfig {
        max_iterations: 5_000,
        tolerance: 1e-10,
        timing_mode: TimingMode::EarlyExitAllowed,
    };
    let mut x = FixedVector::from_array([0.0; N]);
    let report =
        solve_projected_first_order(&problem, &mut x, &mut workspace, &config).expect("solve");
    let iterate = core::array::from_fn(|i| x.get(i).unwrap());
    (report.status(), iterate)
}

fn max_abs_diff(a: &[f64; N], b: &[f64]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y).abs())
        .fold(0.0, f64::max)
}

/// RFC 041 S2: the measurement the handoff asks for. Reports, and does not
/// merely assert, whether any `Q32` solve that reports `Converged` deviates
/// from the exact optimum beyond the configured tolerance — "a wrong answer
/// that converges", the failure mode named in RFC 041 §3.2 and the handoff.
#[test]
fn q32_box_solves_measured_against_the_exact_reference_and_the_f64_solve() {
    let corpus = random_corpus(300, 0x600D_F1ED_u64);
    let mut converged = 0usize;
    let mut not_converged = 0usize;
    let mut worst_deviation_from_exact_when_converged = 0.0_f64;
    let mut worst_deviation_from_f64_solve = 0.0_f64;
    let mut converged_but_wrong: Vec<usize> = Vec::new();
    // Loose enough to absorb Q32's own quantization step (2^-20 ≈ 9.5e-7) over
    // a handful of coordinates, tight enough that a genuinely wrong answer —
    // off by a representable fraction of the box width — would still trip it.
    const WRONG_ANSWER_THRESHOLD: f64 = 1e-3;

    for (i, instance) in corpus.iter().enumerate() {
        let exact = exact_reference(instance);
        let (q_status, q_iterate) = solve_q(instance);
        let (f64_status, f64_iterate) = solve_f64(instance);

        let deviation_from_exact = max_abs_diff(&q_iterate, &exact);
        let deviation_from_f64 = max_abs_diff(&q_iterate, &f64_iterate);
        worst_deviation_from_f64_solve = worst_deviation_from_f64_solve.max(deviation_from_f64);

        match q_status {
            SolveStatus::Converged => {
                converged += 1;
                worst_deviation_from_exact_when_converged =
                    worst_deviation_from_exact_when_converged.max(deviation_from_exact);
                if deviation_from_exact > WRONG_ANSWER_THRESHOLD {
                    converged_but_wrong.push(i);
                }
            }
            _ => not_converged += 1,
        }
        let _ = f64_status;
    }

    eprintln!(
        "Q32 box kernel over {} instances: converged {converged}, not converged {not_converged}",
        corpus.len()
    );
    eprintln!(
        "  worst deviation from the exact optimum, converged instances only: {worst_deviation_from_exact_when_converged:e}  [measured]"
    );
    eprintln!(
        "  worst deviation from the f64 solve of the same problem (precision cost, not correctness): {worst_deviation_from_f64_solve:e}  [measured]"
    );
    if converged_but_wrong.is_empty() {
        eprintln!(
            "  converged-but-wrong instances (deviation > {WRONG_ANSWER_THRESHOLD:e}): none  [measured]"
        );
    } else {
        eprintln!(
            "  converged-but-wrong instances (deviation > {WRONG_ANSWER_THRESHOLD:e}): {} — {converged_but_wrong:?}  [measured]",
            converged_but_wrong.len()
        );
    }

    assert!(
        converged_but_wrong.is_empty(),
        "Q32 reported Converged but was wrong on instances {converged_but_wrong:?} \
         (worst deviation {worst_deviation_from_exact_when_converged:e}); saturation at the \
         bounds is not benign here — see the corpus seed for reproduction"
    );
    assert!(
        converged + not_converged == corpus.len(),
        "every instance must be accounted for"
    );
}
