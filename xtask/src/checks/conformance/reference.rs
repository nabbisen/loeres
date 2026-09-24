//! RFC 031: independent exact references for the `extended/` and `adversarial/`
//! fixtures. Test-only.
//!
//! Every expected value in those suites was derived in exact rational arithmetic
//! by an active-set enumeration that never runs a kernel (the generator lives in
//! the review-request evidence). The tests here re-derive each value **in this
//! repository**, with a second, independent implementation in `f64`, so the claim
//! "the expected values do not come from the kernels" is checked on every
//! `cargo test`, not only asserted in a comment.
//!
//! The reference solves `minimise ½ Σ qᵢ(xᵢ − tᵢ)²` over `lo ≤ x ≤ hi`, `Ax ≤ b`
//! by KKT enumeration: for each set `S` of constraint rows (the rows of `A` and
//! one row per box face), solve `x = t − Q⁻¹G_Sᵀλ`, `G_S x = h_S`; since `Q > 0`
//! the problem is strictly convex, so **any** candidate with `λ ≥ 0` and a
//! feasible `x` is the optimum. Infeasible fixtures carry a Farkas certificate.

use super::constrained::ConstrainedFixture;
use super::load_constrained_fixtures;

/// Absolute slack for feasibility and multiplier signs. The fixtures' data is
/// order one, so double rounding is around `1e-15`.
const SLACK: f64 = 1e-12;

struct Rows {
    g: Vec<Vec<f64>>,
    h: Vec<f64>,
    /// How many leading rows come from `A` (the rest are box faces).
    from_a: usize,
}

fn rows_of(f: &ConstrainedFixture) -> Rows {
    let (n, m) = (f.dimension, f.constraints);
    let p = &f.problem;
    let mut g = Vec::new();
    let mut h = Vec::new();
    for i in 0..m {
        g.push(p.constraint_matrix[i * n..(i + 1) * n].to_vec());
        h.push(p.constraint_rhs[i]);
    }
    for j in 0..n {
        let mut up = vec![0.0; n];
        up[j] = 1.0;
        g.push(up);
        h.push(p.upper[j]);
        let mut down = vec![0.0; n];
        down[j] = -1.0;
        g.push(down);
        h.push(-p.lower[j]);
    }
    Rows { g, h, from_a: m }
}

/// Gaussian elimination with partial pivoting; `None` for a singular system.
fn solve(mut a: Vec<Vec<f64>>, mut r: Vec<f64>) -> Option<Vec<f64>> {
    let n = r.len();
    for c in 0..n {
        let pivot = (c..n).max_by(|&x, &y| a[x][c].abs().total_cmp(&a[y][c].abs()))?;
        if a[pivot][c].abs() < 1e-13 {
            return None;
        }
        a.swap(c, pivot);
        r.swap(c, pivot);
        let (done, rest) = a.split_at_mut(c + 1);
        let pivot_row = &done[c];
        for (offset, row) in rest.iter_mut().enumerate() {
            let factor = row[c] / pivot_row[c];
            for (value, pivot) in row[c..].iter_mut().zip(&pivot_row[c..]) {
                *value -= factor * pivot;
            }
            r[c + 1 + offset] -= factor * r[c];
        }
    }
    let mut x = vec![0.0; n];
    for i in (0..n).rev() {
        let tail: f64 = (i + 1..n).map(|j| a[i][j] * x[j]).sum();
        x[i] = (r[i] - tail) / a[i][i];
    }
    Some(x)
}

/// Every size-`k` subset of `pool`, in lexicographic order.
fn subsets(pool: usize, k: usize) -> Vec<Vec<usize>> {
    fn go(start: usize, pool: usize, k: usize, cur: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
        if cur.len() == k {
            out.push(cur.clone());
            return;
        }
        for i in start..pool {
            cur.push(i);
            go(i + 1, pool, k, cur, out);
            cur.pop();
        }
    }
    let mut out = Vec::new();
    go(0, pool, k, &mut Vec::new(), &mut out);
    out
}

/// The exact optimum of a feasible fixture, or `None` if no KKT point exists.
///
/// Rows of `A` alone are tried first (box multipliers zero; the box is still
/// checked for feasibility), then, for small `n`, the box faces as well. Either
/// way a KKT point found is the unique optimum.
fn optimum(f: &ConstrainedFixture) -> Option<Vec<f64>> {
    let n = f.dimension;
    let p = &f.problem;
    let rows = rows_of(f);
    let feasible = |x: &[f64]| {
        rows.g
            .iter()
            .zip(&rows.h)
            .all(|(g, h)| g.iter().zip(x).map(|(a, b)| a * b).sum::<f64>() <= h + SLACK)
    };
    if feasible(&p.center) {
        return Some(p.center.clone());
    }
    let mut pools = vec![rows.from_a];
    if n <= 6 {
        pools.push(rows.g.len());
    }
    for pool in pools {
        for k in 1..=n.min(pool) {
            for set in subsets(pool, k) {
                let gram: Vec<Vec<f64>> = set
                    .iter()
                    .map(|&i| {
                        set.iter()
                            .map(|&l| {
                                (0..n)
                                    .map(|j| rows.g[i][j] * rows.g[l][j] / p.quadratic_diag[j])
                                    .sum()
                            })
                            .collect()
                    })
                    .collect();
                let rhs: Vec<f64> = set
                    .iter()
                    .map(|&i| (0..n).map(|j| rows.g[i][j] * p.center[j]).sum::<f64>() - rows.h[i])
                    .collect();
                let Some(lambda) = solve(gram, rhs) else {
                    continue;
                };
                if lambda.iter().any(|&l| l < -SLACK) {
                    continue;
                }
                let x: Vec<f64> = (0..n)
                    .map(|j| {
                        let pull: f64 = set
                            .iter()
                            .zip(&lambda)
                            .map(|(&i, &l)| l * rows.g[i][j])
                            .sum();
                        p.center[j] - pull / p.quadratic_diag[j]
                    })
                    .collect();
                if feasible(&x) {
                    return Some(x);
                }
            }
        }
    }
    None
}

fn suite(name: &str) -> Vec<ConstrainedFixture> {
    load_constrained_fixtures(name).unwrap()
}

fn non_smoke() -> Vec<ConstrainedFixture> {
    let mut all = suite("extended");
    all.extend(suite("adversarial"));
    all
}

#[test]
fn every_expected_solution_matches_an_independent_active_set_reference() {
    let mut checked = 0;
    for f in non_smoke() {
        if f.variant != "solve" {
            continue;
        }
        assert!(
            !f.expected.solution.is_empty(),
            "{}: a solve fixture must carry an expected solution",
            f.fixture_id
        );
        let reference = optimum(&f)
            .unwrap_or_else(|| panic!("{}: the reference found no KKT point", f.fixture_id));
        for (i, (want, got)) in f.expected.solution.iter().zip(&reference).enumerate() {
            let allowed = 1e-12 + 1e-9 * want.abs();
            assert!(
                (want - got).abs() <= allowed,
                "{}: solution[{i}] is {want} in the fixture but {got} from the reference",
                f.fixture_id
            );
        }
        checked += 1;
    }
    assert!(checked >= 20, "only {checked} solve fixtures were checked");
}

#[test]
fn every_infeasible_fixture_carries_a_valid_farkas_certificate() {
    let mut checked = 0;
    for f in non_smoke() {
        if f.variant != "infeasible" {
            continue;
        }
        let lambda = f
            .expected
            .infeasibility_certificate
            .as_ref()
            .unwrap_or_else(|| panic!("{}: no infeasibility certificate", f.fixture_id));
        let (n, m) = (f.dimension, f.constraints);
        assert!(lambda.iter().all(|&l| l >= 0.0), "{}", f.fixture_id);
        for j in 0..n {
            let column: f64 = (0..m)
                .map(|i| lambda[i] * f.problem.constraint_matrix[i * n + j])
                .sum();
            assert!(
                column.abs() <= SLACK,
                "{}: Σ λᵢ aᵢ[{j}] = {column}, not zero",
                f.fixture_id
            );
        }
        let sum_b: f64 = (0..m)
            .map(|i| lambda[i] * f.problem.constraint_rhs[i])
            .sum();
        assert!(
            sum_b < 0.0,
            "{}: Σ λᵢ bᵢ = {sum_b} is not negative",
            f.fixture_id
        );
        // Every point violates some row by at least this: the λ-weighted mean of
        // the row violations is the constant −Σλb / Σλ.
        let bound = -sum_b / lambda.iter().sum::<f64>();
        let stated = f.expected.violation_min.expect("violation_min");
        assert!(
            stated <= bound + SLACK,
            "{}: violation_min {stated} exceeds what the certificate proves ({bound})",
            f.fixture_id
        );
        checked += 1;
    }
    assert!(
        checked >= 3,
        "only {checked} infeasible fixtures were checked"
    );
}

/// RFC 031 §3.1: sizes above the smoke corpus, including `n = 5` and `m = 5`, and
/// on cluster a runtime dimension used nowhere in smoke.
#[test]
fn the_extended_suite_covers_the_sizes_the_rfc_names() {
    let extended = suite("extended");
    assert!(
        extended
            .iter()
            .all(|f| f.dimension > 3 || f.constraints > 3)
    );
    assert!(
        extended
            .iter()
            .any(|f| f.dimension >= 5 && f.constraints >= 5 && f.has_device()),
        "no n >= 5, m >= 5 fixture on both kernels"
    );
    let smoke_dims: std::collections::BTreeSet<usize> =
        suite("smoke").iter().map(|f| f.dimension).collect();
    assert!(
        extended
            .iter()
            .any(|f| !f.has_device() && !smoke_dims.contains(&f.dimension)),
        "no cluster-only fixture at a dimension the smoke corpus never uses"
    );
}

/// RFC 031 §5: one fixture family per property.
#[test]
fn the_adversarial_suite_has_one_family_per_property() {
    let adversarial = suite("adversarial");
    for prefix in [
        "qp-adv-parallel-",
        "qp-adv-degenerate-box-",
        "qp-adv-ill-conditioned-",
        "qp-adv-barely-feasible-",
        "qp-adv-cancelling-",
    ] {
        let family = adversarial
            .iter()
            .filter(|f| f.fixture_id.starts_with(prefix))
            .count();
        assert!(family >= 2, "family `{prefix}` has {family} fixture(s)");
    }
    // A degenerate box has a zero-width coordinate; the family must contain one.
    assert!(adversarial.iter().any(|f| {
        f.fixture_id.starts_with("qp-adv-degenerate-box-")
            && f.problem
                .lower
                .iter()
                .zip(&f.problem.upper)
                .any(|(lo, hi)| lo == hi)
    }));
    // Cancelling geometry is kept, not avoided (RFC 027 §0.3.4).
    assert!(adversarial.iter().any(|f| f.variant == "infeasible"));
}

/// The nearly-parallel family decreases in angle, and the fixtures say so: each
/// records the angle between its normals, and the smallest used is stated.
#[test]
fn the_nearly_parallel_family_records_decreasing_angles_and_the_smallest() {
    let mut angles: Vec<f64> = suite("adversarial")
        .iter()
        .filter(|f| f.fixture_id.starts_with("qp-adv-parallel-angle-"))
        .map(|f| {
            let (a, b) = (
                &f.problem.constraint_matrix[0..2],
                &f.problem.constraint_matrix[2..4],
            );
            let cos = (a[0] * b[0] + a[1] * b[1])
                / ((a[0] * a[0] + a[1] * a[1]).sqrt() * (b[0] * b[0] + b[1] * b[1]).sqrt());
            cos.acos()
        })
        .collect();
    angles.sort_by(f64::total_cmp);
    assert!(angles.len() >= 6, "{angles:?}");
    assert!(angles.windows(2).all(|w| w[0] < w[1]), "{angles:?}");
    // The smallest angle is atan(0.001), as the fixtures' comments state.
    assert!((angles[0] - 0.001_f64.atan()).abs() < 1e-9, "{angles:?}");
}
