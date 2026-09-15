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
