//! RFC 043 S1: the constrained kernel over `Q32`, measured — not assumed —
//! against the exact optimum and the `f64` solve of the same problem, on a
//! random corpus of inequality-constrained QPs.
//!
//! **Not a gate, and not a reported command.** `#[cfg(test)]`-only, the same
//! shape as `fixed_point.rs`: nothing here is a threshold, and no figure is
//! pinned.
//!
//! **The reference is `checks::exact::exact_optimum`, used exactly as written
//! — no new numerical code.** RFC 043 Amendment 1 corrects the RFC's own §3.1:
//! `exact_optimum` is not a box-only or separable-only reference. It solves
//! `minimise ½xᵀQx + cᵀx` subject to `lower ≤ x ≤ upper` **and** `Ax ≤ b`
//! (`exact.rs:37`), for `n ≤ 8`, with `Q` a caller-guaranteed symmetric
//! positive definite matrix (`Q = 0` returns `None` for every instance,
//! because the routine inverts `Q`). This corpus keeps `Q` tridiagonal and
//! diagonally dominant, which is always SPD, so that precondition is never at
//! risk.
//!
//! **What is measured here, and what is not.** Neither `ConstrainedSolveReport`
//! (device) nor `ConstrainedSolveRecord` (cluster) exposes the Dykstra
//! multipliers or their magnitude — the workspace field is private, and
//! nothing public reads it after a solve. RFC 043's handoff §1 item 3 asks for
//! "the distribution of `max|λ|` reached, and how many saturated"; that figure
//! is **not obtainable through the public API**, and reproducing the
//! multiplier recurrence independently would be new, unvalidated numerical
//! code standing in for the kernel's own internal state — the wrong trade for
//! a measurement slice. The proxy actually reported is `projection_cap_hits`:
//! a necessary precondition for `infeasibility_evidence` to ever be
//! considered, and the only cap-related signal the public API carries.
//!
//! **Primary kernel: cluster (dynamic).** The predicate and its helpers are
//! duplicated character-for-character between
//! `loeres-device/src/solve/constrained.rs` and
//! `loeres-cluster/src/solve/constrained.rs`; this module measures the
//! cluster copy on a randomized corpus (dynamic sizing makes a varied corpus
//! far simpler to build than the device path's const-generic one) and
//! separately confirms the **device** kernel compiles and runs over `Q32` at
//! all (handoff §0.2's "should compile is not does compile"), on a handful of
//! fixed instances rather than the full corpus.

#![cfg(test)]

use loeres::scalar::Q32;
use loeres::{
    BaseScalar, BoxBounds, DivisibleScalar, FiniteScalar, LinearInequalities, MetricScalar,
    QuadraticObjective, SolveStatus, SolverError, VectorAccess,
};
use loeres_backend_std::{DenseMatrix, DenseVector};
use loeres_cluster::{
    ClusterCancellationToken, ClusterConstrainedWorkspace, ClusterExecutionContext,
    ClusterValidationPolicy, ConstrainedProjectedConfig,
    solve_constrained_projected_first_order_dyn,
};

use super::exact::{DenseQp, exact_optimum};

const FRAC_BITS: u32 = 20;
type Q = Q32<FRAC_BITS>;

fn lcg(seed: u64) -> impl FnMut() -> f64 {
    let mut state = seed;
    move || {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((state >> 11) as f64) / ((1u64 << 53) as f64)
    }
}

/// One corpus instance's raw `f64` data: `n = 4` fixed, `m` in `{1, 2, 3}`,
/// `Q` tridiagonal with `off` on the off-diagonal — diagonally dominant
/// (`2 > 2·off` for `off < 1`), hence always symmetric positive definite, so
/// `exact_optimum`'s precondition is never at risk.
struct Instance {
    m: usize,
    q: Vec<f64>,
    c: Vec<f64>,
    lower: Vec<f64>,
    upper: Vec<f64>,
    a: Vec<f64>,
    b: Vec<f64>,
}

const N: usize = 4;

fn q_tridiagonal(off: f64) -> Vec<f64> {
    let mut q = vec![0.0; N * N];
    for i in 0..N {
        q[i * N + i] = 2.0;
        if i + 1 < N {
            q[i * N + i + 1] = off;
            q[(i + 1) * N + i] = off;
        }
    }
    q
}

fn random_corpus(count: usize, seed: u64) -> Vec<Instance> {
    let mut next = lcg(seed);
    (0..count)
        .map(|_| {
            let m = 1 + (next() * 3.0) as usize; // 1, 2, or 3
            let off = 0.1 + next() * 0.8; // (0.1, 0.9): diagonally dominant
            let c: Vec<f64> = (0..N).map(|_| (next() - 0.5) * 4.0).collect();
            let mut lower = vec![0.0; N];
            let mut upper = vec![0.0; N];
            for i in 0..N {
                let half_width = 1.0 + next() * 2.0;
                lower[i] = -half_width;
                upper[i] = half_width;
            }
            let a: Vec<f64> = (0..m * N).map(|_| next()).collect();
            let b: Vec<f64> = (0..m).map(|_| next() * 2.0 + 0.5).collect();
            Instance {
                m,
                q: q_tridiagonal(off),
                c,
                lower,
                upper,
                a,
                b,
            }
        })
        .collect()
}

fn dense_qp(instance: &Instance) -> DenseQp {
    DenseQp {
        n: N,
        q: instance.q.clone(),
        c: instance.c.clone(),
        lower: instance.lower.clone(),
        upper: instance.upper.clone(),
        a: instance.a.clone(),
        b: instance.b.clone(),
    }
}

/// The dynamic constrained QP adapter, generic over the scalar so the
/// identical problem is solved in `Q` and in `f64`.
struct ClusterProgram<S> {
    q: DenseMatrix<S>,
    c: DenseVector<S>,
    lower: DenseVector<S>,
    upper: DenseVector<S>,
    a: DenseMatrix<S>,
    b: DenseVector<S>,
}

impl<S: BaseScalar> QuadraticObjective<S> for ClusterProgram<S> {
    type Hessian = DenseMatrix<S>;
    type Linear = DenseVector<S>;
    fn hessian(&self) -> &Self::Hessian {
        &self.q
    }
    fn linear_term(&self) -> &Self::Linear {
        &self.c
    }
}

impl<S: BaseScalar> BoxBounds<S> for ClusterProgram<S> {
    type Bound = DenseVector<S>;
    fn lower_bounds(&self) -> &Self::Bound {
        &self.lower
    }
    fn upper_bounds(&self) -> &Self::Bound {
        &self.upper
    }
}

impl<S: BaseScalar> LinearInequalities<S> for ClusterProgram<S> {
    type Constraints = DenseMatrix<S>;
    type Rhs = DenseVector<S>;
    fn constraint_matrix(&self) -> &Self::Constraints {
        &self.a
    }
    fn constraint_rhs(&self) -> &Self::Rhs {
        &self.b
    }
}

fn build_cluster_program<S: BaseScalar>(
    instance: &Instance,
    build: impl Fn(f64) -> S,
) -> ClusterProgram<S> {
    ClusterProgram {
        q: DenseMatrix::from_row_major_vec(N, N, instance.q.iter().map(|&v| build(v)).collect())
            .expect("well-formed"),
        c: DenseVector::from_vec(instance.c.iter().map(|&v| build(v)).collect()).expect("ok"),
        lower: DenseVector::from_vec(instance.lower.iter().map(|&v| build(v)).collect())
            .expect("ok"),
        upper: DenseVector::from_vec(instance.upper.iter().map(|&v| build(v)).collect())
            .expect("ok"),
        a: DenseMatrix::from_row_major_vec(
            instance.m,
            N,
            instance.a.iter().map(|&v| build(v)).collect(),
        )
        .expect("well-formed"),
        b: DenseVector::from_vec(instance.b.iter().map(|&v| build(v)).collect()).expect("ok"),
    }
}

/// One solve's outcome: status, `infeasibility_evidence`, `projection_cap_hits`
/// and the returned iterate (as `f64`, via `to_f64`).
struct Outcome {
    status: SolveStatus,
    infeasibility_evidence: bool,
    projection_cap_hits: u32,
    iterate: Vec<f64>,
}

fn solve_cluster<S>(
    instance: &Instance,
    build: impl Fn(f64) -> S,
    to_f64: impl Fn(S) -> f64,
    step_scale: f64,
    tolerance: S,
    projection_tolerance: S,
    projection_max_sweeps: u32,
) -> Result<Outcome, SolverError>
where
    S: FiniteScalar + MetricScalar + DivisibleScalar,
{
    let program = build_cluster_program(instance, &build);
    let mut workspace = ClusterConstrainedWorkspace::new(N, instance.m)?;
    let config = ConstrainedProjectedConfig {
        max_iterations: 5_000,
        tolerance,
        projection_max_sweeps,
        projection_tolerance,
    };
    let context = ClusterExecutionContext::new(
        ClusterCancellationToken::new(),
        0,
        ClusterValidationPolicy::ValidateAllInputs,
    );
    let mut x = DenseVector::from_vec(vec![build(0.0); N]).expect("ok");
    let record = solve_constrained_projected_first_order_dyn(
        &program,
        build(step_scale),
        &mut x,
        &mut workspace,
        &config,
        &context,
    )?;
    let iterate: Vec<f64> = (0..N)
        .map(|i| to_f64(x.get(i).expect("in range")))
        .collect();
    Ok(Outcome {
        status: record.report.status(),
        infeasibility_evidence: record.infeasibility_evidence,
        projection_cap_hits: record.projection_cap_hits,
        iterate,
    })
}

fn max_abs_diff(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y).abs())
        .fold(0.0, f64::max)
}

/// Handoff §0.2: confirm the **device** kernel actually compiles and runs
/// over `Q32`, on a few fixed instances — not the full corpus, which is the
/// cluster kernel's job in this module.
#[test]
fn the_device_constrained_kernel_compiles_and_runs_over_q32() {
    use loeres_backend_static::array::{FixedMatrix, FixedVector};
    use loeres_device::solve::{
        ConstrainedProjectedWorkspace, ConstrainedSolveConfig as DeviceConfig,
        solve_constrained_projected_first_order,
    };

    const DN: usize = 2;
    const DM: usize = 1;
    struct DeviceProgram {
        q: FixedMatrix<Q, DN, DN, 4>,
        c: FixedVector<Q, DN>,
        lower: FixedVector<Q, DN>,
        upper: FixedVector<Q, DN>,
        a: FixedMatrix<Q, DM, DN, 2>,
        b: FixedVector<Q, DM>,
    }
    impl QuadraticObjective<Q> for DeviceProgram {
        type Hessian = FixedMatrix<Q, DN, DN, 4>;
        type Linear = FixedVector<Q, DN>;
        fn hessian(&self) -> &Self::Hessian {
            &self.q
        }
        fn linear_term(&self) -> &Self::Linear {
            &self.c
        }
    }
    impl BoxBounds<Q> for DeviceProgram {
        type Bound = FixedVector<Q, DN>;
        fn lower_bounds(&self) -> &Self::Bound {
            &self.lower
        }
        fn upper_bounds(&self) -> &Self::Bound {
            &self.upper
        }
    }
    impl LinearInequalities<Q> for DeviceProgram {
        type Constraints = FixedMatrix<Q, DM, DN, 2>;
        type Rhs = FixedVector<Q, DM>;
        fn constraint_matrix(&self) -> &Self::Constraints {
            &self.a
        }
        fn constraint_rhs(&self) -> &Self::Rhs {
            &self.b
        }
    }

    let build = Q::from_f64;
    let problem = DeviceProgram {
        q: FixedMatrix::from_row_major_array([build(2.0), build(0.3), build(0.3), build(2.0)]),
        c: FixedVector::from_array([build(-1.0), build(-1.0)]),
        lower: FixedVector::from_array([build(-5.0), build(-5.0)]),
        upper: FixedVector::from_array([build(5.0), build(5.0)]),
        a: FixedMatrix::from_row_major_array([build(1.0), build(1.0)]),
        b: FixedVector::from_array([build(1.0)]),
    };
    let mut workspace = ConstrainedProjectedWorkspace::<Q, DN, DM>::new(
        FixedVector::from_array([Q::zero(); DN]),
        FixedVector::from_array([Q::zero(); DN]),
        FixedVector::from_array([Q::zero(); DN]),
        FixedVector::from_array([Q::zero(); DM]),
        FixedVector::from_array([Q::zero(); DM]),
    );
    let config = DeviceConfig {
        max_iterations: 2_000,
        tolerance: Q::from_raw(4),
        timing_mode: loeres_device::config::TimingMode::EarlyExitAllowed,
        projection_max_sweeps: 500,
        projection_tolerance: Q::from_raw(4),
    };
    let mut x = FixedVector::from_array([Q::zero(); DN]);
    let report = solve_constrained_projected_first_order(
        &problem,
        build(0.4),
        &mut x,
        &mut workspace,
        &config,
    )
    .expect("the device constrained kernel must accept Q32 and solve without error");
    eprintln!(
        "device constrained kernel over Q32: status={:?} iterations={} cap_hits={} violation={:?}",
        report.status(),
        report.iterations_executed(),
        report.projection_cap_hits(),
        report.max_constraint_violation().to_f64(),
    );
}

/// RFC 043 S1: the cluster constrained kernel over `Q32<20>`, measured on a
/// 300-instance random corpus against the exact optimum and the `f64` solve.
#[test]
fn q32_constrained_solves_measured_against_the_exact_reference_and_the_f64_solve() {
    let corpus = random_corpus(300, 0x600D_C0DE_u64);
    let step_scale = 0.4; // 2/lambda_max <= 2/(2+2*0.9)=0.526; 0.4 is safely inside.

    let mut converged_mismatches = 0usize;
    let mut evidence_q32_only = Vec::new();
    let mut evidence_f64_only = Vec::new();
    let mut q32_cap_hits_count = 0usize;
    let mut f64_cap_hits_count = 0usize;
    let mut converged_but_wrong = Vec::new();
    let mut no_exact_reference = 0usize;
    let mut worst_relative_deviation = 0.0_f64;
    let mut worst_relative_absolute: (usize, f64, f64) = (0, 0.0, 0.0);

    for (i, instance) in corpus.iter().enumerate() {
        let q_outcome = solve_cluster(
            instance,
            Q::from_f64,
            Q::to_f64,
            step_scale,
            Q::from_raw(4),
            Q::from_raw(4),
            2_000,
        )
        .unwrap_or_else(|e| panic!("instance {i}: Q32 solve failed: {e:?}"));
        let f_outcome = solve_cluster(instance, |v| v, |v| v, step_scale, 1e-10, 1e-10, 2_000)
            .unwrap_or_else(|e| panic!("instance {i}: f64 solve failed: {e:?}"));

        if q_outcome.projection_cap_hits > 0 {
            q32_cap_hits_count += 1;
        }
        if f_outcome.projection_cap_hits > 0 {
            f64_cap_hits_count += 1;
        }

        let q_conv = matches!(q_outcome.status, SolveStatus::Converged);
        let f_conv = matches!(f_outcome.status, SolveStatus::Converged);
        if q_conv != f_conv {
            converged_mismatches += 1;
        }

        match (
            q_outcome.infeasibility_evidence,
            f_outcome.infeasibility_evidence,
        ) {
            (true, false) => evidence_q32_only.push(i),
            (false, true) => evidence_f64_only.push(i),
            _ => {}
        }

        if q_conv {
            if let Some(exact) = exact_optimum(&dense_qp(instance)) {
                let q_deviation = max_abs_diff(&q_outcome.iterate, &exact);
                let f_deviation = max_abs_diff(&f_outcome.iterate, &exact);
                // Relative to the f64 solve's own deviation from the same exact
                // optimum (RFC 041 S2's measure), not an absolute constant: a
                // baseline deviation floor of 1e-9 keeps the ratio defined when
                // the f64 solve is itself (near-)exact.
                let relative = q_deviation / f_deviation.max(1e-9);
                if relative > worst_relative_deviation {
                    worst_relative_deviation = relative;
                    worst_relative_absolute = (i, q_deviation, f_deviation);
                }
                // "Wrong" means meaningfully worse than the f64 solve's own
                // approximation error, not merely nonzero.
                if relative > 50.0 && q_deviation > 1e-4 {
                    converged_but_wrong.push((i, q_deviation, f_deviation));
                }
            } else {
                no_exact_reference += 1;
            }
        }
    }

    eprintln!("Q32 constrained solves over {} instances:", corpus.len());
    eprintln!("  converged-status mismatches (Q32 vs f64): {converged_mismatches}  [measured]");
    eprintln!(
        "  infeasibility_evidence: Q32-only {} ({:?}), f64-only {} ({:?})  [measured]",
        evidence_q32_only.len(),
        evidence_q32_only,
        evidence_f64_only.len(),
        evidence_f64_only
    );
    eprintln!(
        "  projection_cap_hits > 0: Q32 {q32_cap_hits_count}/{}, f64 {f64_cap_hits_count}/{}  [measured]",
        corpus.len(),
        corpus.len()
    );
    eprintln!(
        "  (max|lambda| and its saturation count are not exposed by the public API; \
         projection_cap_hits above is the closest available proxy — see this module's doc)"
    );
    eprintln!(
        "  converged instances with no exact reference available (infeasible or n > MAX_N): {no_exact_reference}  [measured]"
    );
    eprintln!(
        "  worst Q32 deviation from exact, relative to f64's own deviation from exact: {worst_relative_deviation:.2}x  [measured/derived]"
    );
    eprintln!(
        "    (at instance {}: Q32 absolute deviation {:e}, f64 absolute deviation {:e} — \
         a large ratio with a tiny f64 deviation is the 1e-9 floor, not a Q32 problem)  [measured]",
        worst_relative_absolute.0, worst_relative_absolute.1, worst_relative_absolute.2
    );
    if converged_but_wrong.is_empty() {
        eprintln!("  converged-but-wrong instances: none  [measured]");
    } else {
        eprintln!(
            "  converged-but-wrong instances: {} — {converged_but_wrong:?}  [measured]",
            converged_but_wrong.len()
        );
    }

    // The deliverable is the measurement (RFC 043 §3.1); report, do not
    // silently pass over, a nonempty result.
    assert!(
        converged_but_wrong.is_empty(),
        "Q32 reported Converged but was wrong (relative to f64's own deviation) on {converged_but_wrong:?}"
    );
}

/// RFC 043 §2.3: a random corpus may simply not be adversarial enough to
/// stress multiplier growth, since real snapshots are correlated along a
/// trajectory and the false positive needs `max|λ|` to actually climb.
/// Nearly-parallel constraint rows are this project's own established
/// mechanism for that (`conformance/adversarial`'s `qp-adv-parallel-angle-*`
/// family): a thin sliver forces many Dykstra sweeps per outer iteration,
/// which is exactly where multipliers have room to grow. This deliberately
/// hunts for the false positive rather than waiting to see it by chance.
#[test]
fn q32_near_parallel_rows_with_small_sweep_caps_hunted_for_the_false_positive() {
    let angles: [f64; 4] = [1e-2, 1e-3, 1e-4, 1e-5];
    let caps = [65u32, 100, 300, 1_000];
    let mut evidence_q32_only = Vec::new();
    let mut evidence_f64_only = Vec::new();
    let mut tried = 0usize;
    let mut q32_capped = 0usize;
    let mut f64_capped = 0usize;

    for &angle in &angles {
        // Two nearly-parallel rows in the first two coordinates, both near
        // their bound at the thin sliver's vertex; `c` pushes the
        // unconstrained optimum well past both rows, so they both bind hard.
        let instance = Instance {
            m: 2,
            q: q_tridiagonal(0.5),
            c: vec![-6.0, -6.0, -6.0, -6.0],
            lower: vec![-10.0; N],
            upper: vec![10.0; N],
            a: vec![1.0, 0.0, 0.0, 0.0, angle.cos(), angle.sin(), 0.0, 0.0],
            b: vec![1.0, 1.0],
        };
        for &cap in &caps {
            tried += 1;
            let q_outcome = solve_cluster(
                &instance,
                Q::from_f64,
                Q::to_f64,
                0.4,
                Q::from_raw(4),
                Q::from_raw(4),
                cap,
            )
            .unwrap_or_else(|e| panic!("angle {angle}, cap {cap}: Q32 solve failed: {e:?}"));
            let f_outcome = solve_cluster(&instance, |v| v, |v| v, 0.4, 1e-10, 1e-10, cap)
                .unwrap_or_else(|e| panic!("angle {angle}, cap {cap}: f64 solve failed: {e:?}"));
            if q_outcome.projection_cap_hits > 0 {
                q32_capped += 1;
            }
            if f_outcome.projection_cap_hits > 0 {
                f64_capped += 1;
            }
            match (
                q_outcome.infeasibility_evidence,
                f_outcome.infeasibility_evidence,
            ) {
                (true, false) => evidence_q32_only.push((angle, cap)),
                (false, true) => evidence_f64_only.push((angle, cap)),
                _ => {}
            }
        }
    }

    // A genuinely infeasible instance too: two contradictory near-parallel
    // rows (`x0 <= -1` and, at a slight angle, `x0 >= 1`-ish), so the
    // predicate's intended case — real divergence — gets a chance to fire at
    // all before concluding Q32 agrees or disagrees with f64 on it.
    for &angle in &angles {
        for &cap in &caps {
            tried += 1;
            let instance = Instance {
                m: 2,
                q: q_tridiagonal(0.5),
                c: vec![0.0; N],
                lower: vec![-10.0; N],
                upper: vec![10.0; N],
                a: vec![1.0, 0.0, 0.0, 0.0, -angle.cos(), -angle.sin(), 0.0, 0.0],
                b: vec![-1.0, -1.0],
            };
            let q_outcome = solve_cluster(
                &instance,
                Q::from_f64,
                Q::to_f64,
                0.4,
                Q::from_raw(4),
                Q::from_raw(4),
                cap,
            )
            .unwrap_or_else(|e| {
                panic!("infeasible angle {angle}, cap {cap}: Q32 solve failed: {e:?}")
            });
            let f_outcome = solve_cluster(&instance, |v| v, |v| v, 0.4, 1e-10, 1e-10, cap)
                .unwrap_or_else(|e| {
                    panic!("infeasible angle {angle}, cap {cap}: f64 solve failed: {e:?}")
                });
            if q_outcome.projection_cap_hits > 0 {
                q32_capped += 1;
            }
            if f_outcome.projection_cap_hits > 0 {
                f64_capped += 1;
            }
            match (
                q_outcome.infeasibility_evidence,
                f_outcome.infeasibility_evidence,
            ) {
                (true, false) => evidence_q32_only.push((angle, cap)),
                (false, true) => evidence_f64_only.push((angle, cap)),
                _ => {}
            }
        }
    }

    eprintln!(
        "near-parallel adversarial hunt (feasible thin-sliver batch + a genuinely infeasible \
         batch): {tried} (angle, cap) combinations, {} angles x {} caps x 2 batches",
        angles.len(),
        caps.len()
    );
    eprintln!(
        "  projection_cap_hits > 0: Q32 {q32_capped}/{tried}, f64 {f64_capped}/{tried}  [measured]"
    );
    eprintln!(
        "  infeasibility_evidence: Q32-only {} {evidence_q32_only:?}, f64-only {} {evidence_f64_only:?}  [measured]",
        evidence_q32_only.len(),
        evidence_f64_only.len()
    );
    if evidence_q32_only.is_empty() {
        eprintln!(
            "  the false positive did not fire on this adversarial batch either — not because \
             it cannot, but because it was not found here  [measured]"
        );
    }
}

/// RFC 043 §2's own measurement, moved into tracked code (handoff §1: "also
/// move the §2 differential into tracked code"). Reproduces **only** the
/// cross-multiplied comparisons `has_infeasibility_evidence` currently uses —
/// conditions 3 and 4 of RFC 034 Amendment 1, `10·final ≥ 19·midpoint` and
/// `100·final ≥ 99·midpoint` — through the public `Q32`/`f64` arithmetic, on
/// independently drawn snapshot quadruples. This is the predicate's defect in
/// isolation, not a real solve; §2.3 of the RFC is explicit that correlated,
/// trajectory-drawn snapshots (the kernel measurement above) may disagree far
/// less often than this.
/// The four comparison constants, built once per scalar type.
struct Factors<S> {
    ten: S,
    nineteen: S,
    hundred: S,
    ninety_nine: S,
}

/// One snapshot quadruple: `max|λ|` and the terminal violation at the
/// midpoint and final sweep.
struct Quad<S> {
    midpoint_multiplier: S,
    midpoint_violation: S,
    final_multiplier: S,
    final_violation: S,
}

fn cross_multiplied_conditions<S: MetricScalar>(factors: &Factors<S>, snap: &Quad<S>) -> bool {
    let multipliers_diverging =
        factors.ten.mul(snap.final_multiplier) >= factors.nineteen.mul(snap.midpoint_multiplier);
    let violation_not_shrinking = factors.hundred.mul(snap.final_violation)
        >= factors.ninety_nine.mul(snap.midpoint_violation);
    multipliers_diverging && violation_not_shrinking
}

#[test]
fn the_cross_multiplied_predicate_manufactures_evidence_under_q32_but_never_hides_it() {
    let scales = [10.0_f64, 25.0, 120.0, 1_000.0, 2_000.0];
    const DRAWS: usize = 200_000;

    let q_factors = Factors {
        ten: Q::from_f64(10.0),
        nineteen: Q::from_f64(19.0),
        hundred: Q::from_f64(100.0),
        ninety_nine: Q::from_f64(99.0),
    };
    let f_factors = Factors {
        ten: 10.0,
        nineteen: 19.0,
        hundred: 100.0,
        ninety_nine: 99.0,
    };

    eprintln!(
        "cross-multiplied predicate, Q32<20> vs f64, {DRAWS} independently drawn snapshot quadruples per scale:"
    );
    eprintln!(
        "  max magnitude | Q32 evidence where f64 has none | f64 evidence where Q32 has none"
    );
    for &scale in &scales {
        let mut next = lcg(0x5CA1E_u64.wrapping_add(scale.to_bits()));
        let mut q32_only = 0usize;
        let mut f64_only = 0usize;
        for _ in 0..DRAWS {
            // Snapshot magnitudes (multipliers and violations are both
            // non-negative by construction in the real kernel) drawn
            // uniformly in [0, scale].
            let f_quad = Quad {
                midpoint_multiplier: next() * scale,
                midpoint_violation: next() * scale,
                final_multiplier: next() * scale,
                final_violation: next() * scale,
            };
            let q_quad = Quad {
                midpoint_multiplier: Q::from_f64(f_quad.midpoint_multiplier),
                midpoint_violation: Q::from_f64(f_quad.midpoint_violation),
                final_multiplier: Q::from_f64(f_quad.final_multiplier),
                final_violation: Q::from_f64(f_quad.final_violation),
            };

            let f_result = cross_multiplied_conditions(&f_factors, &f_quad);
            let q_result = cross_multiplied_conditions(&q_factors, &q_quad);
            match (q_result, f_result) {
                (true, false) => q32_only += 1,
                (false, true) => f64_only += 1,
                _ => {}
            }
        }
        eprintln!("  {scale:>13} | {q32_only:>32} | {f64_only:>32}");
        // RFC 043 §2's finding, re-verified independently: saturation can
        // only manufacture evidence, never suppress it.
        assert_eq!(
            f64_only, 0,
            "scale {scale}: the cross-multiplied form hid real evidence Q32 should have seen \
             — this would mean the defect also produces false negatives, which RFC 043 §2 says \
             it does not"
        );
    }
}
