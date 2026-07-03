use super::*;
use loeres::validation::{TrustToken, TrustedByCaller};

fn key(identity: ModelIdentity, epoch: MutationEpoch) -> ValidationEvidenceKey {
    ValidationEvidenceKey {
        model_identity: identity,
        mutation_epoch: epoch,
        solver_family: SolverFamilyId::ProjectedFirstOrder,
        problem_class: ProblemClassId::BoxBoundFirstOrder,
        scalar_family: ScalarFamilyId::Float64,
    }
}

fn scanned(scope: ValidationScope) -> CachedValidationEvidence {
    CachedValidationEvidence {
        model_checked_scope: scope,
        finite: ProjectedFirstOrderFiniteEvidence::Scanned,
    }
}

#[test]
fn cache_hit_requires_exact_key_and_sufficient_scope() {
    let mut cache = ValidationEvidenceCache::new();
    let identity = ModelIdentity(42);
    let epoch = MutationEpoch::INITIAL;
    let k = key(identity, epoch);
    cache.insert(k, scanned(ValidationScope::FINITE)).unwrap();

    let hit = cache.get(&ValidationEvidenceLookup {
        key: k,
        required_model_scope: ValidationScope::FINITE,
    });
    assert!(hit.is_some());

    let wrong_epoch = cache.get(&ValidationEvidenceLookup {
        key: key(identity, MutationEpoch(1)),
        required_model_scope: ValidationScope::FINITE,
    });
    assert!(wrong_epoch.is_none());

    let insufficient = cache.get(&ValidationEvidenceLookup {
        key: k,
        required_model_scope: ValidationScope::FINITE.union(ValidationScope::PROBLEM_CONFIG),
    });
    assert!(insufficient.is_none());
}

#[test]
fn cache_rejects_trusted_and_domain_inapplicable_for_float64() {
    let mut cache = ValidationEvidenceCache::new();
    let k = key(ModelIdentity(7), MutationEpoch::INITIAL);
    let trust =
        TrustedByCaller::caller_assertion(ValidationScope::FINITE, TrustToken::new(99), None);
    let trusted = CachedValidationEvidence {
        model_checked_scope: ValidationScope::FINITE,
        finite: ProjectedFirstOrderFiniteEvidence::Trusted(trust),
    };
    assert!(matches!(
        cache.insert(k, trusted),
        Err(SolverError::InvalidInput)
    ));

    let domain_inapplicable = CachedValidationEvidence {
        model_checked_scope: ValidationScope::FINITE,
        finite: ProjectedFirstOrderFiniteEvidence::DomainInapplicable,
    };
    assert!(matches!(
        cache.insert(k, domain_inapplicable),
        Err(SolverError::InvalidInput)
    ));
}

#[test]
fn cache_rejects_non_cacheable_sentinel_identity() {
    let mut cache = ValidationEvidenceCache::new();
    let k = key(ModelIdentity::NON_CACHEABLE, MutationEpoch::INITIAL);
    assert!(matches!(
        cache.insert(k, scanned(ValidationScope::FINITE)),
        Err(SolverError::InvalidInput)
    ));
}

#[test]
fn invalidate_and_clear_remove_entries() {
    let mut cache = ValidationEvidenceCache::new();
    let a = ModelIdentity(1);
    let b = ModelIdentity(2);
    cache
        .insert(
            key(a, MutationEpoch::INITIAL),
            scanned(ValidationScope::FINITE),
        )
        .unwrap();
    cache
        .insert(
            key(b, MutationEpoch::INITIAL),
            scanned(ValidationScope::FINITE),
        )
        .unwrap();

    cache.invalidate_model(a);
    assert!(
        cache
            .get(&ValidationEvidenceLookup {
                key: key(a, MutationEpoch::INITIAL),
                required_model_scope: ValidationScope::FINITE,
            })
            .is_none()
    );
    assert!(
        cache
            .get(&ValidationEvidenceLookup {
                key: key(b, MutationEpoch::INITIAL),
                required_model_scope: ValidationScope::FINITE,
            })
            .is_some()
    );

    cache.clear();
    assert!(
        cache
            .get(&ValidationEvidenceLookup {
                key: key(b, MutationEpoch::INITIAL),
                required_model_scope: ValidationScope::FINITE,
            })
            .is_none()
    );
}

#[derive(Clone)]
struct MinimalProblem {
    lo: DenseVector<f64>,
    hi: DenseVector<f64>,
}

impl MinimalProblem {
    fn new() -> Self {
        Self {
            lo: DenseVector::from_vec(vec![-1.0]).unwrap(),
            hi: DenseVector::from_vec(vec![1.0]).unwrap(),
        }
    }
}

impl ClusterProjectedFirstOrderProblem<f64> for MinimalProblem {
    fn dimension(&self) -> usize {
        1
    }

    fn bounds(&self) -> (&DenseVector<f64>, &DenseVector<f64>) {
        (&self.lo, &self.hi)
    }

    fn gradient_at(
        &self,
        _x: &DenseVector<f64>,
        grad: &mut DenseVector<f64>,
    ) -> Result<(), SolverError> {
        loeres::VectorAccessMut::set(grad, 0, 0.0)
    }

    fn step_scale(&self) -> f64 {
        1.0
    }
}

#[test]
fn carrier_generates_identity_and_epoch() {
    let carrier = CacheableProjectedFirstOrderProblem::new(MinimalProblem::new()).unwrap();
    assert_ne!(carrier.identity(), ModelIdentity::NON_CACHEABLE);
    assert_eq!(carrier.mutation_epoch(), MutationEpoch::INITIAL);
    assert!(carrier.is_cacheable());
    assert_eq!(
        carrier.projected_first_order_key().model_identity,
        carrier.identity()
    );
}

#[test]
fn carrier_mutate_advances_epoch() {
    let mut carrier = CacheableProjectedFirstOrderProblem::new(MinimalProblem::new()).unwrap();
    carrier
        .mutate(|inner| {
            inner.hi = DenseVector::from_vec(vec![2.0])?;
            Ok(())
        })
        .unwrap();
    assert_eq!(carrier.mutation_epoch(), MutationEpoch(1));
}

#[test]
fn carrier_mutate_advances_epoch_before_returned_error() {
    let mut carrier = CacheableProjectedFirstOrderProblem::new(MinimalProblem::new()).unwrap();
    let old_key = carrier.projected_first_order_key();
    let mut cache = ValidationEvidenceCache::new();
    cache
        .insert(old_key, scanned(ValidationScope::FINITE))
        .unwrap();

    let err = carrier
        .mutate(|inner| {
            inner.hi = DenseVector::from_vec(vec![f64::NAN])?;
            Err(SolverError::InvalidInput)
        })
        .unwrap_err();

    assert!(matches!(err, SolverError::InvalidInput));
    assert_eq!(carrier.mutation_epoch(), MutationEpoch(1));
    assert_ne!(carrier.projected_first_order_key(), old_key);
    assert!(cache.get(&carrier.projected_first_order_lookup()).is_none());
}

#[test]
fn carrier_mutate_advances_epoch_before_caught_panic() {
    let mut carrier = CacheableProjectedFirstOrderProblem::new(MinimalProblem::new()).unwrap();
    let old_key = carrier.projected_first_order_key();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = carrier.mutate(|inner| {
            inner.hi = DenseVector::from_vec(vec![f64::NAN])?;
            panic!("intentional mutation panic");
        });
    }));

    assert!(result.is_err());
    assert_eq!(carrier.mutation_epoch(), MutationEpoch(1));
    assert_ne!(carrier.projected_first_order_key(), old_key);
}

#[test]
fn carrier_clone_gets_distinct_identity() {
    let carrier = CacheableProjectedFirstOrderProblem::new(MinimalProblem::new()).unwrap();
    let cloned = carrier.clone();
    assert_ne!(carrier.identity(), cloned.identity());
    assert_eq!(cloned.mutation_epoch(), MutationEpoch::INITIAL);
}
