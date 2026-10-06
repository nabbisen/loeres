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

#[test]
fn taking_an_identity_returns_the_value_held_before_the_increment() {
    let counter = AtomicU64::new(41);
    assert_eq!(next_identity_from(&counter).unwrap().value(), 41);
    assert_eq!(counter.load(Ordering::Relaxed), 42);
}

#[test]
fn successive_identities_are_consecutive_and_start_at_the_initial_value() {
    let counter = AtomicU64::new(1);
    let taken: Vec<u64> = (0..4)
        .map(|_| next_identity_from(&counter).unwrap().value())
        .collect();
    assert_eq!(taken, vec![1, 2, 3, 4]);
    assert_eq!(counter.load(Ordering::Relaxed), 5);
}

#[test]
fn the_last_identity_before_saturation_is_issued_and_then_the_counter_refuses() {
    let counter = AtomicU64::new(u64::MAX - 1);
    assert_eq!(next_identity_from(&counter).unwrap().value(), u64::MAX - 1);
    assert_eq!(counter.load(Ordering::Relaxed), u64::MAX);

    for _ in 0..3 {
        assert_eq!(
            next_identity_from(&counter),
            Err(SolverError::InternalInvariantViolation)
        );
        assert_eq!(
            counter.load(Ordering::Relaxed),
            u64::MAX,
            "a saturated counter must not advance or wrap"
        );
    }
}

#[test]
fn a_saturated_counter_never_issues_the_reserved_non_cacheable_identity() {
    let counter = AtomicU64::new(u64::MAX);
    assert_eq!(
        next_identity_from(&counter),
        Err(SolverError::InternalInvariantViolation)
    );
}

#[test]
fn concurrent_takers_share_out_each_identity_exactly_once() {
    use std::sync::Arc;

    let counter = Arc::new(AtomicU64::new(1));
    let handles: Vec<_> = (0..8)
        .map(|_| {
            let counter = Arc::clone(&counter);
            std::thread::spawn(move || {
                (0..1_000)
                    .map(|_| next_identity_from(&counter).unwrap().value())
                    .collect::<Vec<u64>>()
            })
        })
        .collect();

    let mut issued: Vec<u64> = handles
        .into_iter()
        .flat_map(|handle| handle.join().expect("taker thread panicked"))
        .collect();
    issued.sort_unstable();
    let count = issued.len();
    issued.dedup();

    assert_eq!(issued.len(), count, "an identity was issued twice");
    assert_eq!(issued.first().copied(), Some(1));
    assert_eq!(issued.last().copied(), Some(count as u64));
}

#[test]
fn identities_are_shared_out_exactly_once_up_to_the_saturation_boundary() {
    use std::sync::Arc;

    // Saturation is re-checked on every retry, not once before the loop. A taker
    // can load a value below the ceiling, lose the exchange while other takers
    // consume the rest, and come back holding the ceiling; checking only before
    // the loop would then exchange the ceiling for a wrapped value and hand out
    // the reserved non-cacheable identity. Reaching that interleaving needs real
    // contention, so takers ask for twice what remains and the run is repeated.
    const REMAINING: u64 = 16_384;
    const TAKERS: usize = 8;
    const ASKS: usize = (REMAINING as usize / TAKERS) * 2;

    for attempt in 0..8 {
        let counter = Arc::new(AtomicU64::new(u64::MAX - REMAINING));
        let handles: Vec<_> = (0..TAKERS)
            .map(|_| {
                let counter = Arc::clone(&counter);
                std::thread::spawn(move || {
                    (0..ASKS)
                        .filter(|_| next_identity_from(&counter).is_ok())
                        .count()
                })
            })
            .collect();

        let issued: usize = handles
            .into_iter()
            .map(|handle| handle.join().expect("taker thread panicked"))
            .sum();

        assert_eq!(
            counter.load(Ordering::Relaxed),
            u64::MAX,
            "attempt {attempt}: a saturated counter wrapped instead of refusing"
        );
        assert_eq!(
            issued, REMAINING as usize,
            "attempt {attempt}: exactly the remaining identities may be issued"
        );
    }
}
