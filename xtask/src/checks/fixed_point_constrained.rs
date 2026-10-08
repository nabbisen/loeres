//! RFC 043 S1 (+ A1): the constrained kernel over `Q32`, measured — not
//! assumed — against the exact optimum and the `f64` solve of the same
//! problem, on a random corpus of inequality-constrained QPs and a
//! deliberately adversarial batch, **swept across `Q32`'s documented
//! `FRAC_BITS` range** (architect review 097 / RFC 043 Amendment 2).
//!
//! **Not a gate, and not a reported command.** `#[cfg(test)]`-only, the same
//! shape as `fixed_point.rs`.
//!
//! **A is accepted as a measurement; its original conclusion is not.** The
//! first committed version of this harness held `FRAC_BITS = 20` fixed, which
//! review 096 §0.1 had already shown is inside the regime where the
//! cross-multiplied predicate's defect provably cannot fire (zero
//! disagreement below snapshot magnitude `20.48`). A null result there
//! confirms the safe regime is safe; it does not test reachability. A1 is
//! this module re-run with that one constant swept — nothing else changes.
//!
//! **The `converged_but_wrong` classifier below is a threshold, not an
//! absence of one.** `relative > 50.0 && q_deviation > 1e-4` is a pass
//! criterion this module enforces. Calling it "no pinned threshold" (as an
//! earlier revision of this module's own doc did) was imprecise: what is
//! true is that no *gate* pins it and no CI-enforced regression baseline
//! exists, not that the measurement carries no criterion at all.
//!
//! **The reference is `checks::exact::exact_optimum`, used exactly as
//! written — no new numerical code.** RFC 043 Amendment 1: `exact_optimum`
//! is not a box-only or separable-only reference; it solves
//! `minimise ½xᵀQx + cᵀx` subject to `lower ≤ x ≤ upper` **and** `Ax ≤ b`
//! (`exact.rs:37`), for `n ≤ 8`, with `Q` a caller-guaranteed symmetric
//! positive definite matrix. This corpus keeps `Q` tridiagonal and
//! diagonally dominant, which is always SPD.
//!
//! **What is measured here, and what is not.** Neither `ConstrainedSolveReport`
//! (device) nor `ConstrainedSolveRecord` (cluster) exposes the Dykstra
//! multipliers or `max|λ|` directly — the workspace field is private.
//! `report.max_constraint_violation()` **is** public, and is reported below
//! as a **scale proxy**: the predicate's condition-4 operands are the
//! *inner* Dykstra sweeps' midpoint and final violations, not this *outer*
//! terminal value, but the terminal violation bounds the scale the inner
//! snapshots live at, and condition 4's tighter threshold (`100×`, i.e.
//! `max/100`) makes that bound informative. It is not the operand itself,
//! and this module does not claim it is.
//!
//! **`Err(SolverError::Overflow)` is an outcome class, not a panic.** At the
//! coarser precisions this harness deliberately sweeps into, `checked_div`
//! inside the kernel's own Dykstra step can legitimately overflow — that is
//! the type's documented failure channel working, exactly the thing RFC 043
//! §3.2 depends on. A measurement harness records it and moves on; it does
//! not treat it as a bug in itself. Any *other* error is still a hard
//! failure of this harness, since nothing else is expected.
//!
//! **A2 (review 098 / RFC 043 Amendment 3): five measurement-hygiene
//! corrections to A1, alongside S2, not blocking it.** The sweep now covers
//! every `FRAC_BITS` from `1` to `30`, not seven chosen points; `Q32`'s
//! tolerance and `FRAC_BITS` are no longer swept as one coupled parameter
//! (`a2_tolerance_coupling_is_isolated_from_frac_bits` reruns a subset with
//! tolerance fixed in absolute terms); the `converged_but_wrong` classifier's
//! absolute guard is expressed in quantization steps, not a precision-
//! independent constant; per-instance lists are capped at the worst ten,
//! sorted by deviation; and no single absolute tolerance is representable
//! across the documented range — nothing finer than `Q32<12>`'s
//! `2.44e-4`-per-step floor, nothing coarser than `Q32<30>`'s `2`-per-step
//! ceiling. A tolerance that rounds to zero at a given `FRAC_BITS` is
//! correctly rejected by the kernel as `InvalidInput`, not silently treated
//! as exact.
//!
//! **Primary kernel: cluster (dynamic).** The predicate and its helpers are
//! duplicated character-for-character between the device and cluster
//! kernels; this module measures the cluster copy on the full corpus and
//! separately confirms the **device** kernel compiles and runs over `Q32` at
//! all (handoff §0.2), on one fixed instance rather than the full corpus.

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
#[derive(Clone)]
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

/// A nearly-parallel pair of rows at `angle`, pushed hard against a tight
/// box, forming a feasible thin sliver (this project's own
/// `qp-adv-parallel-angle-*` mechanism).
fn adversarial_feasible_instance(angle: f64) -> Instance {
    Instance {
        m: 2,
        q: q_tridiagonal(0.5),
        c: vec![-6.0, -6.0, -6.0, -6.0],
        lower: vec![-10.0; N],
        upper: vec![10.0; N],
        a: vec![1.0, 0.0, 0.0, 0.0, angle.cos(), angle.sin(), 0.0, 0.0],
        b: vec![1.0, 1.0],
    }
}

/// A genuinely infeasible, nearly-parallel pair of contradictory rows at
/// `angle`, so the predicate's intended case gets a chance to fire at all.
fn adversarial_infeasible_instance(angle: f64) -> Instance {
    Instance {
        m: 2,
        q: q_tridiagonal(0.5),
        c: vec![0.0; N],
        lower: vec![-10.0; N],
        upper: vec![10.0; N],
        a: vec![1.0, 0.0, 0.0, 0.0, -angle.cos(), -angle.sin(), 0.0, 0.0],
        b: vec![-1.0, -1.0],
    }
}

const ADVERSARIAL_ANGLES: [f64; 4] = [1e-2, 1e-3, 1e-4, 1e-5];
const ADVERSARIAL_CAPS: [u32; 4] = [65, 100, 300, 1_000];

/// One case: a problem and the sweep cap to solve it with. Cap is a solve
/// parameter, not problem data, so it travels with the instance rather than
/// being a single value shared by a whole batch — the adversarial batches
/// vary it per case.
struct Case {
    instance: Instance,
    cap: u32,
}

fn random_cases() -> Vec<Case> {
    random_corpus(300, 0x600D_C0DE_u64)
        .into_iter()
        .map(|instance| Case {
            instance,
            cap: 2_000,
        })
        .collect()
}

fn adversarial_feasible_cases() -> Vec<Case> {
    ADVERSARIAL_ANGLES
        .iter()
        .flat_map(|&angle| {
            ADVERSARIAL_CAPS.iter().map(move |&cap| Case {
                instance: adversarial_feasible_instance(angle),
                cap,
            })
        })
        .collect()
}

fn adversarial_infeasible_cases() -> Vec<Case> {
    ADVERSARIAL_ANGLES
        .iter()
        .flat_map(|&angle| {
            ADVERSARIAL_CAPS.iter().map(move |&cap| Case {
                instance: adversarial_infeasible_instance(angle),
                cap,
            })
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
/// identical problem is solved in `Q32<F>` and in `f64`.
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

/// One solve's outcome: status, `infeasibility_evidence`, `projection_cap_hits`,
/// the terminal violation (a scale proxy — see the module doc), and the
/// returned iterate (as `f64`, via `to_f64`).
struct Outcome {
    status: SolveStatus,
    infeasibility_evidence: bool,
    projection_cap_hits: u32,
    max_constraint_violation: f64,
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
        max_constraint_violation: to_f64(record.max_constraint_violation),
        iterate,
    })
}

fn max_abs_diff(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y).abs())
        .fold(0.0, f64::max)
}

/// The scalar `n`, by repeated addition — byte-for-byte the same formula the
/// kernel's own private `scalar_from` documents and uses
/// (`constrained.rs`'s own doc comment on that helper), reproduced here
/// because it is private to the kernel crates.
fn scalar_from<S: MetricScalar>(n: u32) -> S {
    (0..n).fold(S::zero(), |sum, _| sum.add(S::one()))
}

fn min_max_mean(values: &[f64]) -> (f64, f64, f64) {
    if values.is_empty() {
        return (0.0, 0.0, 0.0);
    }
    let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    (min, max, mean)
}

/// `2^-F`, the magnitude of one `Q32<F>` quantization step.
fn quantization_step<const F: u32>() -> f64 {
    2f64.powi(-(F as i32))
}

/// How far past ordinary quantization noise a converged-but-wrong deviation
/// must sit before this harness calls it wrong rather than coarse (RFC 043
/// Amendment 3 / handoff §1.2 item 3). Chosen empirically, not fitted: across
/// every precision and batch this harness measures, the largest deviation
/// that is ordinary quantization is `274` steps (the adversarial feasible
/// sliver's instance `7` at `FRAC_BITS = 20`), and the smallest deviation this
/// harness considers genuinely wrong is `~1.5e9` steps (`FRAC_BITS = 30`'s
/// random corpus) — six orders of magnitude of headroom either side of
/// `1000`, so this is a conservative cut point, not a tight fit to the data.
const CONVERGED_BUT_WRONG_STEP_THRESHOLD: f64 = 1_000.0;

/// Per-batch, per-precision measurement. One of these is produced per
/// `(batch, FRAC_BITS, tolerance regime)` triple.
#[derive(Default)]
struct BatchStats {
    total: usize,
    q32_overflow: usize,
    f64_overflow: usize,
    /// A quantized row or bound collapsed into one the kernel's own input
    /// validation rejects (e.g. an all-zero constraint row after rounding) —
    /// reachable only at the coarse end of the sweep, on this `f64` corpus's
    /// un-quantized data. A legitimate, expected outcome of extreme
    /// quantization, not a harness bug, so it is counted rather than
    /// panicked on, the same policy §1.1 item 4 sets for `Overflow`.
    q32_invalid_input: usize,
    /// A quantized row's norm rounded to zero, or similarly hit the
    /// `SolverError::NumericalDomain` case — the same coarse-quantization
    /// reasoning as `q32_invalid_input`.
    q32_numerical_domain: usize,
    q32_cap_hits: usize,
    f64_cap_hits: usize,
    converged_mismatches: usize,
    evidence_q32_only: Vec<usize>,
    evidence_f64_only: Vec<usize>,
    /// `(case index, Q32's deviation from exact, f64's deviation from exact,
    /// Q32's deviation in quantization steps)`.
    converged_but_wrong: Vec<(usize, f64, f64, f64)>,
    no_exact_reference: usize,
    q32_violations: Vec<f64>,
    f64_violations: Vec<f64>,
}

/// Runs `cases` through both `Q32<F>` and `f64`, at the declared step scale,
/// with `Q32`'s outer and projection tolerance both set to `tolerance` —
/// an explicit parameter rather than a constant derived from `F`, per RFC 043
/// Amendment 3 / handoff §1.2 item 2: the caller decides whether `tolerance`
/// moves with `FRAC_BITS` or stays fixed, and this function sweeps only `F`.
/// `Err(SolverError::Overflow)` is counted as an outcome class (module doc);
/// any other error is a hard failure of this harness, since nothing else is
/// expected of these well-formed instances.
fn measure_cases_at<const F: u32>(
    cases: &[Case],
    step_scale: f64,
    tolerance: Q32<F>,
) -> BatchStats {
    let mut s = BatchStats {
        total: cases.len(),
        ..Default::default()
    };
    for (i, case) in cases.iter().enumerate() {
        let q_result = solve_cluster(
            &case.instance,
            Q32::<F>::from_f64,
            Q32::<F>::to_f64,
            step_scale,
            tolerance,
            tolerance,
            case.cap,
        );
        let f_result = solve_cluster(
            &case.instance,
            |v| v,
            |v| v,
            step_scale,
            1e-10,
            1e-10,
            case.cap,
        );

        let q_outcome = match q_result {
            Ok(o) => o,
            Err(SolverError::Overflow) => {
                s.q32_overflow += 1;
                continue;
            }
            Err(SolverError::InvalidInput) => {
                s.q32_invalid_input += 1;
                continue;
            }
            Err(SolverError::NumericalDomain) => {
                s.q32_numerical_domain += 1;
                continue;
            }
            Err(e) => panic!("case {i} at FRAC_BITS={F}: Q32 solve failed unexpectedly: {e:?}"),
        };
        let f_outcome = match f_result {
            Ok(o) => o,
            Err(SolverError::Overflow) => {
                s.f64_overflow += 1;
                continue;
            }
            Err(e) => panic!("case {i} at FRAC_BITS={F}: f64 solve failed unexpectedly: {e:?}"),
        };

        if q_outcome.projection_cap_hits > 0 {
            s.q32_cap_hits += 1;
        }
        if f_outcome.projection_cap_hits > 0 {
            s.f64_cap_hits += 1;
        }
        s.q32_violations.push(q_outcome.max_constraint_violation);
        s.f64_violations.push(f_outcome.max_constraint_violation);

        let q_conv = matches!(q_outcome.status, SolveStatus::Converged);
        let f_conv = matches!(f_outcome.status, SolveStatus::Converged);
        if q_conv != f_conv {
            s.converged_mismatches += 1;
        }

        match (
            q_outcome.infeasibility_evidence,
            f_outcome.infeasibility_evidence,
        ) {
            (true, false) => s.evidence_q32_only.push(i),
            (false, true) => s.evidence_f64_only.push(i),
            _ => {}
        }

        if q_conv {
            if let Some(exact) = exact_optimum(&dense_qp(&case.instance)) {
                let q_deviation = max_abs_diff(&q_outcome.iterate, &exact);
                let f_deviation = max_abs_diff(&f_outcome.iterate, &exact);
                let deviation_steps = q_deviation / quantization_step::<F>();
                // Relative to the f64 solve's own deviation from the same
                // exact optimum (RFC 041 S2's measure), not an absolute
                // constant; a 1e-9 floor keeps the ratio defined when f64
                // is itself (near-)exact. The absolute guard is expressed in
                // quantization steps, not an absolute constant (RFC 043
                // Amendment 3 / handoff §1.2 item 3) — a precision-independent
                // constant like the former `1e-4` misreads a coarser `Q32` as
                // wrong when it is merely coarser: at `FRAC_BITS = 12` a step
                // is `2.44e-4`, already above that constant. This *is* a
                // threshold (module doc), not an absence of one.
                let relative = q_deviation / f_deviation.max(1e-9);
                if relative > 50.0 && deviation_steps > CONVERGED_BUT_WRONG_STEP_THRESHOLD {
                    s.converged_but_wrong
                        .push((i, q_deviation, f_deviation, deviation_steps));
                }
            } else {
                s.no_exact_reference += 1;
            }
        }
    }
    s
}

/// `(scalar_from(19)/scalar_from(10), scalar_from(99)/scalar_from(100))` at
/// `Q32<F>`, reproducing RFC 043 Amendment 2's structural finding
/// independently rather than copying its table.
fn effective_factors<const F: u32>() -> (f64, f64) {
    let factor_3 = scalar_from::<Q32<F>>(19).to_f64() / scalar_from::<Q32<F>>(10).to_f64();
    let factor_4 = scalar_from::<Q32<F>>(99).to_f64() / scalar_from::<Q32<F>>(100).to_f64();
    (factor_3, factor_4)
}

fn report_batch(label: &str, frac_bits: u32, stats: &BatchStats) {
    eprintln!("  [{label}] FRAC_BITS={frac_bits}: {} cases", stats.total);
    eprintln!(
        "    overflow (outcome class, not a panic): Q32 {}, f64 {}  [measured]",
        stats.q32_overflow, stats.f64_overflow
    );
    if stats.q32_invalid_input > 0 || stats.q32_numerical_domain > 0 {
        eprintln!(
            "    Q32 input validation outcome classes (coarse-quantization artifacts, not panics): \
             InvalidInput {}, NumericalDomain {}  [measured]",
            stats.q32_invalid_input, stats.q32_numerical_domain
        );
    }
    eprintln!(
        "    projection_cap_hits > 0: Q32 {}, f64 {}  [measured]",
        stats.q32_cap_hits, stats.f64_cap_hits
    );
    eprintln!(
        "    converged-status mismatches: {}  [measured]",
        stats.converged_mismatches
    );
    eprintln!(
        "    infeasibility_evidence: Q32-only {} {:?}, f64-only {} {:?}  [measured]",
        stats.evidence_q32_only.len(),
        stats.evidence_q32_only,
        stats.evidence_f64_only.len(),
        stats.evidence_f64_only
    );
    let (q_min, q_max, q_mean) = min_max_mean(&stats.q32_violations);
    let (f_min, f_max, f_mean) = min_max_mean(&stats.f64_violations);
    eprintln!(
        "    terminal violation (scale proxy, not the predicate's own operand): \
         Q32 min={q_min:.3e} max={q_max:.3e} mean={q_mean:.3e}; \
         f64 min={f_min:.3e} max={f_max:.3e} mean={f_mean:.3e}  [measured]"
    );
    if stats.converged_but_wrong.is_empty() {
        eprintln!(
            "    converged-but-wrong (threshold: relative>50x and >{CONVERGED_BUT_WRONG_STEP_THRESHOLD:.0} \
             quantization steps): none  [measured]"
        );
    } else {
        let mut worst = stats.converged_but_wrong.clone();
        worst.sort_by(|a, b| b.1.partial_cmp(&a.1).expect("no NaN deviation"));
        worst.truncate(10);
        eprintln!(
            "    converged-but-wrong (threshold: relative>50x and >{CONVERGED_BUT_WRONG_STEP_THRESHOLD:.0} \
             quantization steps): {} total, worst {} shown as \
             (index, Q32 deviation, f64 deviation, Q32 deviation in steps): {:?}  [measured]",
            stats.converged_but_wrong.len(),
            worst.len(),
            worst
        );
    }
    if stats.no_exact_reference > 0 {
        eprintln!(
            "    converged with no exact reference available: {}  [measured]",
            stats.no_exact_reference
        );
    }
}

/// RFC 043 A1 (architect review 097 §5 / Amendment 2) + A2 item 1 (review 098
/// / Amendment 3): the existing corpus and the existing adversarial batch,
/// swept across **every** `FRAC_BITS` in `Q32`'s documented `1..=30` range,
/// not seven chosen points — the seven-point sweep omitted `25` and `27`,
/// and `27` is the row where the condition-3 factor is `1.6`, inside the
/// band RFC 034 proved unsound. No new instances, no new seeds, no changed
/// solve configuration beyond the swept constant itself. `Q32`'s tolerance
/// is held coupled to `FRAC_BITS` here (`from_raw(4)`, four quantization
/// steps) — the regime A1 used; `a2_tolerance_coupling_is_isolated_from_frac_bits`
/// below reruns a subset with tolerance fixed in absolute terms instead, per
/// handoff §1.2 item 2.
#[test]
fn a1_frac_bits_sweep_over_the_existing_corpus_and_adversarial_batch() {
    let random = random_cases();
    let adversarial_feasible = adversarial_feasible_cases();
    let adversarial_infeasible = adversarial_infeasible_cases();
    let step_scale = 0.4; // 2/lambda_max <= 2/(2+2*0.9)=0.526; 0.4 is safely inside.

    eprintln!(
        "RFC 043 A1+A2 — FRAC_BITS swept 1..=30 over the random corpus and the adversarial batch:"
    );
    eprintln!(
        "  FRAC_BITS | effective factor (19/10, want 1.9) | effective factor (99/100, want 0.99)"
    );

    // Each arm below is one FRAC_BITS value. Rust const generics need the
    // value at compile time, so this is thirty explicit calls rather than a
    // runtime loop — the same shape bench.rs's device_instances! macro
    // exists to avoid for a *table* of sizes; thirty fixed values once is
    // plainer written out than macro-generated, and costs a few seconds.
    macro_rules! sweep_one {
        ($frac_bits:literal) => {{
            let (factor_3, factor_4) = effective_factors::<$frac_bits>();
            eprintln!("  {:>9} | {factor_3:.4} | {factor_4:.4}", $frac_bits);
            let tolerance = Q32::<$frac_bits>::from_raw(4);
            report_batch(
                "random corpus",
                $frac_bits,
                &measure_cases_at::<$frac_bits>(&random, step_scale, tolerance),
            );
            report_batch(
                "adversarial (feasible thin sliver)",
                $frac_bits,
                &measure_cases_at::<$frac_bits>(&adversarial_feasible, step_scale, tolerance),
            );
            report_batch(
                "adversarial (genuinely infeasible)",
                $frac_bits,
                &measure_cases_at::<$frac_bits>(&adversarial_infeasible, step_scale, tolerance),
            );
        }};
    }

    sweep_one!(1);
    sweep_one!(2);
    sweep_one!(3);
    sweep_one!(4);
    sweep_one!(5);
    sweep_one!(6);
    sweep_one!(7);
    sweep_one!(8);
    sweep_one!(9);
    sweep_one!(10);
    sweep_one!(11);
    sweep_one!(12);
    sweep_one!(13);
    sweep_one!(14);
    sweep_one!(15);
    sweep_one!(16);
    sweep_one!(17);
    sweep_one!(18);
    sweep_one!(19);
    sweep_one!(20);
    sweep_one!(21);
    sweep_one!(22);
    sweep_one!(23);
    sweep_one!(24);
    sweep_one!(25);
    sweep_one!(26);
    sweep_one!(27);
    sweep_one!(28);
    sweep_one!(29);
    sweep_one!(30);
}

/// RFC 043 Amendment 3 / handoff §1.2 item 2: A1 swept `FRAC_BITS` and
/// `Q32`'s tolerance together (`from_raw(4)` moves with `FRAC_BITS`), so a
/// figure that changed across the sweep might be precision-driven or
/// tolerance-driven, and the sweep alone cannot say which.
///
/// **This item's original target no longer exists to reproduce.** Amendment
/// 3's claim — "the `FRAC_BITS = 24` evidence disagreement persists
/// index-for-index" under a fixed tolerance — was about the pre-S2,
/// cross-multiplied predicate. S2 (this same revision) replaces that
/// predicate, and on the genuinely-infeasible batch the disagreement A1
/// found is gone under *either* tolerance regime (both `evidence_q32_only`
/// columns are empty at every precision below). Asserting "persists
/// index-for-index" on two empty sets would be vacuous, not a reproduction,
/// so this does not assert that. What the fixed-vs-coupled comparison still
/// isolates, unaffected by S2 (which touches only `has_infeasibility_evidence`,
/// not the convergence/cap-hit logic), is the **feasible thin-sliver**
/// batch's converged-status mismatches, which A1 reported as varying with
/// `FRAC_BITS` under the coupled regime (`11/16` below 12, `8/16` at
/// 12–14, `4/16` at 15–17, `0/16` at 18–28, `16/16`-adjacent again at 29–30
/// — see the full sweep above). Reproduced here under a **fixed** `1e-3`
/// tolerance instead.
#[test]
fn a2_tolerance_coupling_is_isolated_from_frac_bits() {
    let adversarial_feasible = adversarial_feasible_cases();
    let adversarial_infeasible = adversarial_infeasible_cases();
    let step_scale = 0.4;

    eprintln!("RFC 043 A2 item 2 — isolating FRAC_BITS from Q32's tolerance:");
    eprintln!(
        "  FRAC_BITS | feasible-sliver converged-status mismatches: coupled (from_raw(4)) vs fixed (1e-3) | infeasible-batch evidence Q32-only: coupled vs fixed"
    );

    macro_rules! compare_one {
        ($frac_bits:literal) => {{
            let coupled_feasible = measure_cases_at::<$frac_bits>(
                &adversarial_feasible,
                step_scale,
                Q32::<$frac_bits>::from_raw(4),
            );
            let fixed_feasible = measure_cases_at::<$frac_bits>(
                &adversarial_feasible,
                step_scale,
                Q32::<$frac_bits>::from_f64(1e-3),
            );
            let coupled_infeasible = measure_cases_at::<$frac_bits>(
                &adversarial_infeasible,
                step_scale,
                Q32::<$frac_bits>::from_raw(4),
            );
            let fixed_infeasible = measure_cases_at::<$frac_bits>(
                &adversarial_infeasible,
                step_scale,
                Q32::<$frac_bits>::from_f64(1e-3),
            );
            eprintln!(
                "  {:>9} | {} vs {} | {:?} vs {:?}  [measured]",
                $frac_bits,
                coupled_feasible.converged_mismatches,
                fixed_feasible.converged_mismatches,
                coupled_infeasible.evidence_q32_only,
                fixed_infeasible.evidence_q32_only,
            );
        }};
    }

    compare_one!(12);
    compare_one!(16);
    compare_one!(20);
    compare_one!(24);
    compare_one!(26);
    compare_one!(28);
    compare_one!(30);
}

/// RFC 043 S6 item 2: group the missed-evidence figures A1/A2 already
/// collect by the four sweep caps, at the `FRAC_BITS` band where the onset
/// happens (`19..=22`, Amendment 4). **No new measurement** — the same
/// 16-case genuinely-infeasible batch, split by its own `cap` field (one of
/// 4 angles × 4 caps) instead of pooled across all 16. If the onset tracks
/// the cap rather than firing uniformly across every cap at once, that is
/// evidence `max|λ|` scales with sweep count and the step is the cap
/// distribution, not a single operand crossing a fixed bound (Amendment 4's
/// own open question).
#[test]
fn s6_missed_evidence_onset_grouped_by_sweep_cap() {
    let cases = adversarial_infeasible_cases();
    let step_scale = 0.4;

    eprintln!(
        "RFC 043 S6 item 2 — missed evidence (f64-only count), grouped by sweep cap, FRAC_BITS 19..=22:"
    );
    eprintln!(
        "  FRAC_BITS | cap 65 | cap 100 | cap 300 | cap 1000  (each out of 4, one per angle)"
    );

    macro_rules! group_one {
        ($frac_bits:literal) => {{
            let tolerance = Q32::<$frac_bits>::from_raw(4);
            let counts: Vec<usize> = ADVERSARIAL_CAPS
                .iter()
                .map(|&cap| {
                    let group: Vec<Case> = cases
                        .iter()
                        .filter(|case| case.cap == cap)
                        .map(|case| Case {
                            instance: case.instance.clone(),
                            cap: case.cap,
                        })
                        .collect();
                    measure_cases_at::<$frac_bits>(&group, step_scale, tolerance)
                        .evidence_f64_only
                        .len()
                })
                .collect();
            eprintln!(
                "  {:>9} | {:>6} | {:>7} | {:>7} | {:>8}  [measured]",
                $frac_bits, counts[0], counts[1], counts[2], counts[3]
            );
        }};
    }

    group_one!(19);
    group_one!(20);
    group_one!(21);
    group_one!(22);
}

/// Handoff §0.2: confirm the **device** kernel actually compiles and runs
/// over `Q32`, on one fixed instance — not the full corpus, which is the
/// cluster kernel's job in this module.
#[test]
fn the_device_constrained_kernel_compiles_and_runs_over_q32() {
    use loeres_backend_static::array::{FixedMatrix, FixedVector};
    use loeres_device::solve::{
        ConstrainedProjectedWorkspace, ConstrainedSolveConfig as DeviceConfig,
        solve_constrained_projected_first_order,
    };

    type Q = Q32<20>;
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

/// RFC 043 §2's own measurement, moved into tracked code. Reproduces
/// **only** the cross-multiplied comparisons `has_infeasibility_evidence`
/// currently uses — conditions 3 and 4 of RFC 034 Amendment 1,
/// `10·final ≥ 19·midpoint` and `100·final ≥ 99·midpoint` — through the
/// public `Q32`/`f64` arithmetic, at `FRAC_BITS = 20`, on independently
/// drawn snapshot quadruples. This is the predicate's defect in isolation at
/// one precision, not a real solve and not the structural (cross-precision)
/// finding — that is `a1_frac_bits_sweep...` above.
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
fn the_cross_multiplied_predicate_manufactures_evidence_under_q32_but_never_hides_it_at_frac_bits_20()
 {
    type Q = Q32<20>;
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
        // At FRAC_BITS = 20 the factors themselves are intact (Amendment 2's
        // table), so this specific precision's defect is one-directional.
        // That is NOT a general claim across every FRAC_BITS — see the
        // sweep above, which finds the opposite at 24.
        assert_eq!(
            f64_only, 0,
            "scale {scale}: at FRAC_BITS=20 the cross-multiplied form hid real evidence Q32 \
             should have seen, which review 096 did not find and which would need its own \
             investigation"
        );
    }
}
