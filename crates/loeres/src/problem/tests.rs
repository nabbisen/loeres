//! Tests for the RFC 027 §11.1 quadratic-program contract, over the core
//! borrowed views. The static and dynamic backend cases live beside those
//! backends' consumers (`loeres-device`, `loeres-cluster`), so neither backend
//! is imported here or by the other.

use crate::access::{MatrixView, VectorView, VectorViewMut};
use crate::error::SolverError;
use crate::problem::{
    BoxBounds, LinearInequalities, ProgramShape, QuadraticObjective, QuadraticProgram,
};
use crate::scalar::BaseScalar;

/// A quadratic program held entirely as borrowed core views.
struct ViewProgram<'a> {
    q: MatrixView<'a, f64>,
    c: VectorView<'a, f64>,
    lo: VectorView<'a, f64>,
    hi: VectorView<'a, f64>,
    a: MatrixView<'a, f64>,
    b: VectorView<'a, f64>,
}

impl<'a> QuadraticObjective<f64> for ViewProgram<'a> {
    type Hessian = MatrixView<'a, f64>;
    type Linear = VectorView<'a, f64>;
    fn hessian(&self) -> &Self::Hessian {
        &self.q
    }
    fn linear_term(&self) -> &Self::Linear {
        &self.c
    }
}

impl<'a> BoxBounds<f64> for ViewProgram<'a> {
    type Bound = VectorView<'a, f64>;
    fn lower_bounds(&self) -> &Self::Bound {
        &self.lo
    }
    fn upper_bounds(&self) -> &Self::Bound {
        &self.hi
    }
}

impl<'a> LinearInequalities<f64> for ViewProgram<'a> {
    type Constraints = MatrixView<'a, f64>;
    type Rhs = VectorView<'a, f64>;
    fn constraint_matrix(&self) -> &Self::Constraints {
        &self.a
    }
    fn constraint_rhs(&self) -> &Self::Rhs {
        &self.b
    }
}

/// Backing storage for a 2-variable, 1-constraint program:
/// `Q = [[2, 1], [1, 3]]`, `c = [-1, 4]`, `-5 ≤ x ≤ 5`, `x₀ + x₁ ≤ 1`.
struct Data {
    q: [f64; 4],
    c: [f64; 2],
    lo: [f64; 2],
    hi: [f64; 2],
    a: [f64; 2],
    b: [f64; 1],
}

const DATA: Data = Data {
    q: [2.0, 1.0, 1.0, 3.0],
    c: [-1.0, 4.0],
    lo: [-5.0, -5.0],
    hi: [5.0, 5.0],
    a: [1.0, 1.0],
    b: [1.0],
};

fn program(d: &Data) -> ViewProgram<'_> {
    ViewProgram {
        q: MatrixView::from_row_major(&d.q, 2, 2).expect("2x2 backing"),
        c: VectorView::from_slice(&d.c),
        lo: VectorView::from_slice(&d.lo),
        hi: VectorView::from_slice(&d.hi),
        a: MatrixView::from_row_major(&d.a, 1, 2).expect("1x2 backing"),
        b: VectorView::from_slice(&d.b),
    }
}

/// Compiles only if `T` is a `QuadraticProgram` — which it is by the blanket
/// impl, never by hand.
fn assert_quadratic_program<S: BaseScalar, T: QuadraticProgram<S>>(_: &T) {}

#[test]
fn a_type_implementing_the_three_parts_is_a_quadratic_program() {
    let d = DATA;
    assert_quadratic_program::<f64, _>(&program(&d));
}

#[test]
fn gradient_is_qx_plus_c() {
    let d = DATA;
    let p = program(&d);
    let x = [0.5_f64, -2.0];
    let mut grad = [f64::NAN; 2];
    p.gradient_into(
        &VectorView::from_slice(&x),
        &mut VectorViewMut::from_slice_mut(&mut grad),
    )
    .expect("shapes agree");
    // Q x + c = [2·0.5 + 1·(−2) − 1, 1·0.5 + 3·(−2) + 4] = [−2, −1.5]
    assert_eq!(grad, [-2.0, -1.5]);
}

#[test]
fn gradient_accumulates_in_the_documented_order() {
    // cᵢ first, then Qᵢⱼ·xⱼ for ascending j. With these magnitudes a different
    // order changes the floating-point result, so this pins the order itself.
    let q = [1.0_f64, 1e-16, 0.0, 1.0];
    let c = [1e16_f64, 0.0];
    let lo = [-1.0_f64; 2];
    let hi = [1.0_f64; 2];
    let a: [f64; 0] = [];
    let b: [f64; 0] = [];
    let p = ViewProgram {
        q: MatrixView::from_row_major(&q, 2, 2).expect("2x2"),
        c: VectorView::from_slice(&c),
        lo: VectorView::from_slice(&lo),
        hi: VectorView::from_slice(&hi),
        a: MatrixView::from_row_major(&a, 0, 2).expect("0x2"),
        b: VectorView::from_slice(&b),
    };
    let x = [-1e16_f64, 1.0];
    let mut grad = [0.0_f64; 2];
    p.gradient_into(
        &VectorView::from_slice(&x),
        &mut VectorViewMut::from_slice_mut(&mut grad),
    )
    .expect("shapes agree");
    let expected = (1e16_f64 + 1.0 * -1e16) + 1e-16 * 1.0;
    assert_eq!(grad[0].to_bits(), expected.to_bits());
}

#[test]
fn gradient_rejects_mismatched_iterate_and_output_lengths() {
    let d = DATA;
    let p = program(&d);
    let short = [0.0_f64; 1];
    let mut grad = [0.0_f64; 2];
    assert_eq!(
        p.gradient_into(
            &VectorView::from_slice(&short),
            &mut VectorViewMut::from_slice_mut(&mut grad)
        ),
        Err(SolverError::DimensionMismatch { lhs: 1, rhs: 2 })
    );

    let x = [0.0_f64; 2];
    let mut long = [0.0_f64; 3];
    assert_eq!(
        p.gradient_into(
            &VectorView::from_slice(&x),
            &mut VectorViewMut::from_slice_mut(&mut long)
        ),
        Err(SolverError::DimensionMismatch { lhs: 3, rhs: 2 })
    );
}

#[test]
fn gradient_rejects_a_non_square_hessian() {
    let d = DATA;
    let mut p = program(&d);
    let wide = [1.0_f64; 6];
    p.q = MatrixView::from_row_major(&wide, 2, 3).expect("2x3");
    let x = [0.0_f64; 2];
    let mut grad = [0.0_f64; 2];
    assert_eq!(
        p.gradient_into(
            &VectorView::from_slice(&x),
            &mut VectorViewMut::from_slice_mut(&mut grad)
        ),
        Err(SolverError::DimensionMismatch { lhs: 3, rhs: 2 })
    );
}

#[test]
fn gradient_writes_nothing_when_a_shape_check_fails() {
    let d = DATA;
    let p = program(&d);
    let short = [0.0_f64; 1];
    let mut grad = [7.0_f64; 2];
    let _ = p.gradient_into(
        &VectorView::from_slice(&short),
        &mut VectorViewMut::from_slice_mut(&mut grad),
    );
    assert_eq!(grad, [7.0, 7.0]);
}

#[test]
fn shape_reports_variables_and_constraints() {
    let d = DATA;
    assert_eq!(
        program(&d).shape(),
        Ok(ProgramShape {
            variables: 2,
            constraints: 1
        })
    );
}

#[test]
fn a_program_without_inequalities_has_zero_constraints() {
    let d = DATA;
    let empty_a: [f64; 0] = [];
    let empty_b: [f64; 0] = [];
    for cols in [2, 0] {
        let mut p = program(&d);
        p.a = MatrixView::from_row_major(&empty_a, 0, cols).expect("zero-row backing");
        p.b = VectorView::from_slice(&empty_b);
        assert_eq!(
            p.shape(),
            Ok(ProgramShape {
                variables: 2,
                constraints: 0
            }),
            "0x{cols} constraint matrix"
        );
    }
}

#[test]
fn shape_rejects_a_program_with_no_variables() {
    let empty: [f64; 0] = [];
    let p = ViewProgram {
        q: MatrixView::from_row_major(&empty, 0, 0).expect("0x0"),
        c: VectorView::from_slice(&empty),
        lo: VectorView::from_slice(&empty),
        hi: VectorView::from_slice(&empty),
        a: MatrixView::from_row_major(&empty, 0, 0).expect("0x0"),
        b: VectorView::from_slice(&empty),
    };
    assert_eq!(p.shape(), Err(SolverError::InvalidDimension));
}

#[test]
fn shape_rejects_every_disagreeing_extent() {
    let d = DATA;
    let one = [0.0_f64; 1];
    let three = [0.0_f64; 3];
    let q3 = [0.0_f64; 6];
    let a3 = [0.0_f64; 3];

    let mut p = program(&d);
    p.q = MatrixView::from_row_major(&q3, 3, 2).expect("3x2");
    assert_eq!(
        p.shape(),
        Err(SolverError::DimensionMismatch { lhs: 3, rhs: 2 }),
        "Q rows"
    );

    let mut p = program(&d);
    p.q = MatrixView::from_row_major(&q3, 2, 3).expect("2x3");
    assert_eq!(
        p.shape(),
        Err(SolverError::DimensionMismatch { lhs: 3, rhs: 2 }),
        "Q cols"
    );

    let mut p = program(&d);
    p.lo = VectorView::from_slice(&one);
    assert_eq!(
        p.shape(),
        Err(SolverError::DimensionMismatch { lhs: 1, rhs: 2 }),
        "lo"
    );

    let mut p = program(&d);
    p.hi = VectorView::from_slice(&three);
    assert_eq!(
        p.shape(),
        Err(SolverError::DimensionMismatch { lhs: 3, rhs: 2 }),
        "hi"
    );

    let mut p = program(&d);
    p.a = MatrixView::from_row_major(&a3, 1, 3).expect("1x3");
    assert_eq!(
        p.shape(),
        Err(SolverError::DimensionMismatch { lhs: 3, rhs: 2 }),
        "A cols"
    );

    let mut p = program(&d);
    p.b = VectorView::from_slice(&three);
    assert_eq!(
        p.shape(),
        Err(SolverError::DimensionMismatch { lhs: 3, rhs: 1 }),
        "b"
    );
}

#[test]
fn shape_reads_no_element() {
    // A NaN-filled program is structurally sound: shape is structure only, and
    // numeric validation is the kernel's under RFC 012's trust policy.
    let q = [f64::NAN; 4];
    let v = [f64::NAN; 2];
    let a = [f64::NAN; 2];
    let b = [f64::NAN; 1];
    let p = ViewProgram {
        q: MatrixView::from_row_major(&q, 2, 2).expect("2x2"),
        c: VectorView::from_slice(&v),
        lo: VectorView::from_slice(&v),
        hi: VectorView::from_slice(&v),
        a: MatrixView::from_row_major(&a, 1, 2).expect("1x2"),
        b: VectorView::from_slice(&b),
    };
    assert_eq!(
        p.shape(),
        Ok(ProgramShape {
            variables: 2,
            constraints: 1
        })
    );
}

// ---------------------------------------------------------------------------
// RFC 032: curvature bounds and the suggested step.
// ---------------------------------------------------------------------------

/// Run `check` on a program whose `Q` is the given row-major `rows × cols`
/// matrix; every other part is inert (`c`, bounds zero, no constraints).
fn with_hessian<R>(
    q: &[f64],
    rows: usize,
    cols: usize,
    check: impl FnOnce(&ViewProgram<'_>) -> R,
) -> R {
    let zeros = vec![0.0; rows.max(cols)];
    let none: [f64; 0] = [];
    let p = ViewProgram {
        q: MatrixView::from_row_major(q, rows, cols).expect("Q backing"),
        c: VectorView::from_slice(&zeros[..rows]),
        lo: VectorView::from_slice(&zeros[..rows]),
        hi: VectorView::from_slice(&zeros[..rows]),
        a: MatrixView::from_row_major(&none, 0, cols).expect("zero-row backing"),
        b: VectorView::from_slice(&none),
    };
    check(&p)
}

fn bounds_of(q: &[f64], n: usize) -> Result<(f64, f64), SolverError> {
    with_hessian(q, n, n, |p| {
        p.curvature_bounds()
            .map(|b| (b.lambda_max_upper, b.lambda_max_lower))
    })
}

#[test]
fn the_bounds_of_a_known_matrix_are_the_gershgorin_row_sum_and_the_largest_diagonal() {
    // Q = [[2, 1], [1, 3]]: row sums 3 and 4, diagonal 2 and 3. Its eigenvalues
    // are (5 ± √5)/2 = 1.38 and 3.62, inside [L, U] = [3, 4].
    assert_eq!(bounds_of(&[2.0, 1.0, 1.0, 3.0], 2), Ok((4.0, 3.0)));
    // Off-diagonal signs do not matter to U: it sums magnitudes.
    assert_eq!(bounds_of(&[2.0, -1.0, -1.0, 3.0], 2), Ok((4.0, 3.0)));
    with_hessian(&[2.0, 1.0, 1.0, 3.0], 2, 2, |p| {
        assert_eq!(p.suggested_step_scale(), Ok(0.25));
    });
}

#[test]
fn a_diagonal_q_has_coinciding_bounds() {
    assert_eq!(bounds_of(&[1.0, 0.0, 0.0, 5.0], 2), Ok((5.0, 5.0)));
}

#[test]
fn the_bounds_reject_malformed_or_non_finite_curvature() {
    assert_eq!(
        with_hessian(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3, |p| p
            .curvature_bounds()),
        Err(SolverError::DimensionMismatch { lhs: 3, rhs: 2 })
    );
    assert_eq!(
        with_hessian(&[], 0, 0, |p| p.curvature_bounds()),
        Err(SolverError::InvalidDimension)
    );
    assert_eq!(
        bounds_of(&[1.0, f64::NAN, 0.0, 1.0], 2),
        Err(SolverError::NonFiniteInput)
    );
    assert_eq!(
        bounds_of(&[1.0, 0.0, f64::INFINITY, 1.0], 2),
        Err(SolverError::NonFiniteInput)
    );
    // Finite elements whose magnitudes sum past the range are an overflow, not a
    // silently infinite bound.
    assert_eq!(
        bounds_of(&[f64::MAX, f64::MAX, 0.0, 1.0], 2),
        Err(SolverError::Overflow)
    );
}

#[test]
fn an_all_zero_q_has_no_suggested_step() {
    // No curvature (a linear objective): 1/U would be 1/0.
    with_hessian(&[0.0; 4], 2, 2, |p| {
        assert_eq!(p.curvature_bounds().map(|b| b.lambda_max_upper), Ok(0.0));
        assert_eq!(p.suggested_step_scale(), Err(SolverError::NumericalDomain));
    });
}

/// A small deterministic generator; no dependency.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 11) as f64) / ((1u64 << 53) as f64)
    }
}

/// The largest eigenvalue of a symmetric positive semidefinite `n × n` matrix by
/// power iteration, **in the test only** — the shipped code computes no
/// eigenvalue and iterates nothing. The Rayleigh quotient of the final iterate
/// is returned; for PSD `Q` it approaches `λ_max` from below.
fn power_iteration_lambda_max(q: &[f64], n: usize) -> f64 {
    let mut v: Vec<f64> = (0..n).map(|i| 1.0 + 0.37 * i as f64).collect();
    let mut lambda = 0.0;
    for _ in 0..100_000 {
        let w: Vec<f64> = (0..n)
            .map(|i| (0..n).map(|j| q[i * n + j] * v[j]).sum())
            .collect();
        let norm = w.iter().map(|x| x * x).sum::<f64>().sqrt();
        if norm == 0.0 {
            return 0.0;
        }
        let next: Vec<f64> = w.iter().map(|x| x / norm).collect();
        let quotient: f64 = (0..n)
            .map(|i| next[i] * (0..n).map(|j| q[i * n + j] * next[j]).sum::<f64>())
            .sum();
        let done = (quotient - lambda).abs() <= 1e-15 * quotient.abs().max(1.0);
        lambda = quotient;
        v = next;
        if done {
            break;
        }
    }
    lambda
}

/// A random symmetric positive semidefinite `Q = AᵀA` — PSD by construction —
/// from a random `k × n` `A`.
fn random_psd(rng: &mut Lcg, n: usize) -> Vec<f64> {
    let k = 1 + (rng.next() * 6.0) as usize;
    let a: Vec<f64> = (0..k * n).map(|_| rng.next() * 2.0 - 1.0).collect();
    let mut q = vec![0.0; n * n];
    for i in 0..n {
        for j in 0..n {
            q[i * n + j] = (0..k).map(|r| a[r * n + i] * a[r * n + j]).sum();
        }
    }
    q
}

/// RFC 032 exit criterion 3: `L ≤ λ_max ≤ U` against an **independently
/// computed** `λ_max`, over random PSD `Q` that is deliberately not diagonal
/// (for a diagonal `Q`, `U` and `L` coincide and the test would say nothing
/// about the band), and the suggested step converges.
#[test]
fn random_psd_matrices_satisfy_l_le_lambda_max_le_u() {
    let mut rng = Lcg(0x9E37_79B9_7F4A_7C15);
    let (mut worst_upper, mut worst_lower, mut wide_band) = (0.0_f64, 0.0_f64, 0);
    for instance in 0..600 {
        let n = 2 + (rng.next() * 5.0) as usize;
        let q = random_psd(&mut rng, n);
        let (upper, lower) = bounds_of(&q, n).unwrap();
        let lambda_max = power_iteration_lambda_max(&q, n);

        let slack = 1e-9 * upper.max(1.0);
        assert!(
            lower <= lambda_max + slack,
            "instance {instance}: L {lower} exceeds λ_max {lambda_max} (n {n}, Q {q:?})"
        );
        assert!(
            lambda_max <= upper + slack,
            "instance {instance}: λ_max {lambda_max} exceeds U {upper} (n {n}, Q {q:?})"
        );
        worst_upper = worst_upper.max(upper / lambda_max);
        worst_lower = worst_lower.max(lambda_max / lower);
        if upper > 1.2 * lower {
            wide_band += 1;
        }

        // The suggested step 1/U is inside (0, 2/λ_max): the iteration
        // x ← x − αQx contracts every component.
        with_hessian(&q, n, n, |p| {
            let alpha = p.suggested_step_scale().unwrap();
            assert!(
                alpha > 0.0 && alpha * lambda_max < 2.0,
                "instance {instance}"
            );
            let mut x: Vec<f64> = (0..n).map(|i| 1.0 + i as f64).collect();
            let start: f64 = x.iter().map(|v| v * v).sum::<f64>().sqrt();
            for _ in 0..200 {
                let qx: Vec<f64> = (0..n)
                    .map(|i| (0..n).map(|j| q[i * n + j] * x[j]).sum())
                    .collect();
                for (xi, g) in x.iter_mut().zip(qx) {
                    *xi -= alpha * g;
                }
            }
            let end: f64 = x.iter().map(|v| v * v).sum::<f64>().sqrt();
            assert!(
                end.is_finite() && end <= start,
                "instance {instance}: {start} -> {end}"
            );
        });
    }
    // The generator really exercises the band: a good share of instances have
    // `U` noticeably above `L`, and the bounds are not vacuously tight.
    assert!(wide_band >= 300, "only {wide_band} of 600 had U > 1.2 L");
    assert!(
        worst_upper > 1.05 && worst_lower > 1.05,
        "{worst_upper} {worst_lower}"
    );
}
