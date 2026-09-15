//! RFC 027 S1: the core quadratic-program contract over the **static** backend.
//!
//! Imports `loeres` and `loeres-backend-static` only. The dynamic-backend
//! counterpart lives in `loeres-cluster`; neither side imports the other, which
//! is the storage-agnostic claim made concrete.

use loeres::{
    BoxBounds, LinearInequalities, MatrixView, QuadraticObjective, QuadraticProgram, SolverError,
};
use loeres_backend_static::array::{FixedMatrix, FixedVector};

/// `n = 2`, `m = 1`, entirely fixed-size owned storage.
struct StaticProgram {
    q: FixedMatrix<f64, 2, 2, 4>,
    c: FixedVector<f64, 2>,
    lo: FixedVector<f64, 2>,
    hi: FixedVector<f64, 2>,
    a: FixedMatrix<f64, 1, 2, 2>,
    b: FixedVector<f64, 1>,
}

impl QuadraticObjective<f64> for StaticProgram {
    type Hessian = FixedMatrix<f64, 2, 2, 4>;
    type Linear = FixedVector<f64, 2>;
    fn hessian(&self) -> &Self::Hessian {
        &self.q
    }
    fn linear_term(&self) -> &Self::Linear {
        &self.c
    }
}

impl BoxBounds<f64> for StaticProgram {
    type Bound = FixedVector<f64, 2>;
    fn lower_bounds(&self) -> &Self::Bound {
        &self.lo
    }
    fn upper_bounds(&self) -> &Self::Bound {
        &self.hi
    }
}

impl LinearInequalities<f64> for StaticProgram {
    type Constraints = FixedMatrix<f64, 1, 2, 2>;
    type Rhs = FixedVector<f64, 1>;
    fn constraint_matrix(&self) -> &Self::Constraints {
        &self.a
    }
    fn constraint_rhs(&self) -> &Self::Rhs {
        &self.b
    }
}

fn static_program() -> StaticProgram {
    StaticProgram {
        q: FixedMatrix::from_row_major_array([2.0, 1.0, 1.0, 3.0]),
        c: FixedVector::from_array([-1.0, 4.0]),
        lo: FixedVector::from_array([-5.0, -5.0]),
        hi: FixedVector::from_array([5.0, 5.0]),
        a: FixedMatrix::from_row_major_array([1.0, 1.0]),
        b: FixedVector::from_array([1.0]),
    }
}

fn assert_quadratic_program<T: QuadraticProgram<f64>>(_: &T) {}

#[test]
fn a_static_backend_satisfies_the_contract() -> Result<(), SolverError> {
    let p = static_program();
    assert_quadratic_program(&p);
    let shape = p.shape()?;
    assert_eq!((shape.variables, shape.constraints), (2, 1));
    Ok(())
}

#[test]
fn the_oracle_writes_into_fixed_storage_without_allocating() {
    let p = static_program();
    let x = FixedVector::from_array([0.5_f64, -2.0]);
    let mut grad = FixedVector::from_array([f64::NAN; 2]);
    p.gradient_into(&x, &mut grad).expect("shapes agree");
    assert_eq!(grad.as_slice(), &[-2.0, -1.5]);
}

/// `FixedMatrix` cannot have zero rows (its `R > 0` invariant is a compile-time
/// assertion), so a static program with no inequality constraints uses the core
/// borrowed view over an empty slice — still allocation-free, still `'static`.
struct UnconstrainedStaticProgram {
    inner: StaticProgram,
    a: MatrixView<'static, f64>,
    b: loeres::VectorView<'static, f64>,
}

impl QuadraticObjective<f64> for UnconstrainedStaticProgram {
    type Hessian = FixedMatrix<f64, 2, 2, 4>;
    type Linear = FixedVector<f64, 2>;
    fn hessian(&self) -> &Self::Hessian {
        &self.inner.q
    }
    fn linear_term(&self) -> &Self::Linear {
        &self.inner.c
    }
}

impl BoxBounds<f64> for UnconstrainedStaticProgram {
    type Bound = FixedVector<f64, 2>;
    fn lower_bounds(&self) -> &Self::Bound {
        &self.inner.lo
    }
    fn upper_bounds(&self) -> &Self::Bound {
        &self.inner.hi
    }
}

impl LinearInequalities<f64> for UnconstrainedStaticProgram {
    type Constraints = MatrixView<'static, f64>;
    type Rhs = loeres::VectorView<'static, f64>;
    fn constraint_matrix(&self) -> &Self::Constraints {
        &self.a
    }
    fn constraint_rhs(&self) -> &Self::Rhs {
        &self.b
    }
}

#[test]
fn a_static_program_without_inequalities_expresses_m_equals_zero() -> Result<(), SolverError> {
    const EMPTY: &[f64] = &[];
    let p = UnconstrainedStaticProgram {
        inner: static_program(),
        a: MatrixView::from_row_major(EMPTY, 0, 2)?,
        b: loeres::VectorView::from_slice(EMPTY),
    };
    assert_quadratic_program(&p);
    let shape = p.shape()?;
    assert_eq!((shape.variables, shape.constraints), (2, 0));
    Ok(())
}
