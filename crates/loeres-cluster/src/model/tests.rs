use super::*;

#[test]
fn workspace_rejects_zero_dimension() {
    assert!(matches!(
        ClusterProjectedFirstOrderWorkspace::<f64>::new(0),
        Err(SolverError::InvalidDimension)
    ));
}

#[test]
fn workspace_sizes_to_dimension() {
    let ws = ClusterProjectedFirstOrderWorkspace::<f64>::new(3).unwrap();
    assert_eq!(ws.dimension(), 3);
}

#[test]
fn workspace_reset_preserves_dimension() {
    let mut ws = ClusterProjectedFirstOrderWorkspace::<f64>::new(2).unwrap();
    ws.reset_for_entry();
    assert_eq!(ws.dimension(), 2);
}

#[test]
fn config_rejects_zero_iterations() {
    let c = ProjectedFirstOrderConfig {
        max_iterations: 0,
        tolerance: 1e-6_f64,
    };
    assert!(matches!(c.validate(), Err(SolverError::InvalidInput)));
}

#[test]
fn config_rejects_non_finite_tolerance() {
    let c = ProjectedFirstOrderConfig {
        max_iterations: 10,
        tolerance: f64::NAN,
    };
    assert!(matches!(c.validate(), Err(SolverError::NonFiniteInput)));
}

#[test]
fn config_rejects_non_positive_tolerance() {
    let zero = ProjectedFirstOrderConfig {
        max_iterations: 10,
        tolerance: 0.0_f64,
    };
    assert!(matches!(zero.validate(), Err(SolverError::InvalidInput)));
    let neg = ProjectedFirstOrderConfig {
        max_iterations: 10,
        tolerance: -1.0_f64,
    };
    assert!(matches!(neg.validate(), Err(SolverError::InvalidInput)));
}

#[test]
fn config_accepts_valid() {
    let c = ProjectedFirstOrderConfig {
        max_iterations: 10,
        tolerance: 1e-6_f64,
    };
    assert!(c.validate().is_ok());
}

// ---------------------------------------------------------------------------
// RFC 027 S1: the core quadratic-program contract over the **dynamic** backend.
//
// Imports `loeres` and `loeres-backend-std` only. The static-backend counterpart
// lives in `loeres-device`; neither side imports the other.
// ---------------------------------------------------------------------------

mod quadratic_program_contract {
    use loeres::{
        BoxBounds, ContiguousVectorAccess, LinearInequalities, MatrixView, QuadraticObjective,
        QuadraticProgram, SolverError, VectorView,
    };
    use loeres_backend_std::{DenseMatrix, DenseVector};

    /// `n = 2`, `m = 1`, runtime-dimensioned heap storage.
    struct DynamicProgram {
        q: DenseMatrix<f64>,
        c: DenseVector<f64>,
        lo: DenseVector<f64>,
        hi: DenseVector<f64>,
        a: DenseMatrix<f64>,
        b: DenseVector<f64>,
    }

    impl QuadraticObjective<f64> for DynamicProgram {
        type Hessian = DenseMatrix<f64>;
        type Linear = DenseVector<f64>;
        fn hessian(&self) -> &Self::Hessian {
            &self.q
        }
        fn linear_term(&self) -> &Self::Linear {
            &self.c
        }
    }

    impl BoxBounds<f64> for DynamicProgram {
        type Bound = DenseVector<f64>;
        fn lower_bounds(&self) -> &Self::Bound {
            &self.lo
        }
        fn upper_bounds(&self) -> &Self::Bound {
            &self.hi
        }
    }

    impl LinearInequalities<f64> for DynamicProgram {
        type Constraints = DenseMatrix<f64>;
        type Rhs = DenseVector<f64>;
        fn constraint_matrix(&self) -> &Self::Constraints {
            &self.a
        }
        fn constraint_rhs(&self) -> &Self::Rhs {
            &self.b
        }
    }

    fn dynamic_program() -> Result<DynamicProgram, SolverError> {
        Ok(DynamicProgram {
            q: DenseMatrix::from_row_major_vec(2, 2, vec![2.0, 1.0, 1.0, 3.0])?,
            c: DenseVector::from_vec(vec![-1.0, 4.0])?,
            lo: DenseVector::from_vec(vec![-5.0, -5.0])?,
            hi: DenseVector::from_vec(vec![5.0, 5.0])?,
            a: DenseMatrix::from_row_major_vec(1, 2, vec![1.0, 1.0])?,
            b: DenseVector::from_vec(vec![1.0])?,
        })
    }

    fn assert_quadratic_program<T: QuadraticProgram<f64>>(_: &T) {}

    #[test]
    fn a_dynamic_backend_satisfies_the_contract() -> Result<(), SolverError> {
        let p = dynamic_program()?;
        assert_quadratic_program(&p);
        let shape = p.shape()?;
        assert_eq!((shape.variables, shape.constraints), (2, 1));
        Ok(())
    }

    #[test]
    fn the_oracle_writes_into_dense_storage() -> Result<(), SolverError> {
        let p = dynamic_program()?;
        let x = DenseVector::from_vec(vec![0.5, -2.0])?;
        let mut grad = DenseVector::from_vec(vec![f64::NAN; 2])?;
        p.gradient_into(&x, &mut grad)?;
        assert_eq!(grad.as_contiguous(), Some(&[-2.0, -1.5][..]));
        Ok(())
    }

    /// `DenseMatrix` rejects a zero-row shape at construction, so it cannot
    /// represent `m = 0`. This pins that fact rather than letting it be
    /// rediscovered in RFC 027 S3.
    #[test]
    fn a_dense_matrix_cannot_hold_zero_constraints() {
        assert_eq!(
            DenseMatrix::<f64>::from_row_major_vec(0, 2, Vec::new()).err(),
            Some(SolverError::InvalidDimension)
        );
    }

    /// The contract still expresses `m = 0` over a dynamic backend: the
    /// constraint storage is a separate associated type, so the core borrowed
    /// view over an empty slice serves.
    struct UnconstrainedDynamicProgram {
        inner: DynamicProgram,
        a: MatrixView<'static, f64>,
        b: VectorView<'static, f64>,
    }

    impl QuadraticObjective<f64> for UnconstrainedDynamicProgram {
        type Hessian = DenseMatrix<f64>;
        type Linear = DenseVector<f64>;
        fn hessian(&self) -> &Self::Hessian {
            &self.inner.q
        }
        fn linear_term(&self) -> &Self::Linear {
            &self.inner.c
        }
    }

    impl BoxBounds<f64> for UnconstrainedDynamicProgram {
        type Bound = DenseVector<f64>;
        fn lower_bounds(&self) -> &Self::Bound {
            &self.inner.lo
        }
        fn upper_bounds(&self) -> &Self::Bound {
            &self.inner.hi
        }
    }

    impl LinearInequalities<f64> for UnconstrainedDynamicProgram {
        type Constraints = MatrixView<'static, f64>;
        type Rhs = VectorView<'static, f64>;
        fn constraint_matrix(&self) -> &Self::Constraints {
            &self.a
        }
        fn constraint_rhs(&self) -> &Self::Rhs {
            &self.b
        }
    }

    #[test]
    fn a_dynamic_program_without_inequalities_expresses_m_equals_zero() -> Result<(), SolverError> {
        const EMPTY: &[f64] = &[];
        let p = UnconstrainedDynamicProgram {
            inner: dynamic_program()?,
            a: MatrixView::from_row_major(EMPTY, 0, 2)?,
            b: VectorView::from_slice(EMPTY),
        };
        assert_quadratic_program(&p);
        let shape = p.shape()?;
        assert_eq!((shape.variables, shape.constraints), (2, 0));
        Ok(())
    }
}
