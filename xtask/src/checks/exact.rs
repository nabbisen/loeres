//! An exact reference for a **dense** convex QP, by active-set enumeration. RFC 037 §5.6
//! (Amendment 1), added because §5.3's premise — reusing `conformance/reference.rs` — was
//! false: that reference solves only a separable, diagonal `Q`, and the counted-work
//! corpus is tridiagonal.
//!
//! **Scope: `n ≤ 8` only.** The enumeration's pool is `m + 2n` rows (the constraint rows
//! plus two box faces per variable), with subsets up to size `n`. At `n = 8` the pool is
//! bounded enough to finish; at `n = 16` it is not. Callers must not use this past `n = 8`.
//!
//! **This is new numerical code, so it is cross-validated, not merely written.** A
//! diagonal `Q` is a special case of a dense one, so this enumeration and the existing
//! separable reference must agree on every fixture where both apply — see
//! `conformance::reference`'s cross-validation test. That agreement is the evidence this
//! module is correct; a difference anywhere is a defect in one of the two, and callers of
//! `exact_optimum` (`bench`'s S6 deviation figures) depend on it being this module.
//!
//! No kernel is read here. Strict convexity (`Q` symmetric positive definite) means any
//! candidate with a feasible `x` and `λ ≥ 0` at a KKT point is the unique optimum, exactly
//! as `conformance/reference.rs`'s module doc explains for the diagonal case.
//!
//! **`exact_optimum` does not handle `Q = 0`, despite RFC 039 §2.1's claim that it
//! "already handles `Q = 0`, since a zero Hessian is a dense Hessian".** It inverts `Q` to
//! find the unconstrained centre and the active-set coupling matrix (`apply_q_inv`), and a
//! zero matrix is singular, not merely coupled: `solve` finds no pivot and `exact_optimum`
//! returns `None` for every `Q = 0` problem, feasible or not. `exact_lp_optimum` below is
//! the new numerical code RFC 039 said was not needed (RFC 039 implementation review
//! request A §6 records the finding). It is cross-validated the same way: hand-solved
//! fixtures with an independently-known answer, and the randomized optimality oracle.

/// Absolute slack for feasibility and multiplier signs, matching
/// `conformance/reference.rs`'s own tolerance for the same reasoning.
const SLACK: f64 = 1e-9;

/// The largest `n` this enumeration is scoped to (RFC 037 §5.6).
pub const MAX_N: usize = 8;

/// A dense convex QP: `minimise ½xᵀQx + cᵀx` subject to `lower ≤ x ≤ upper`, `Ax ≤ b`.
/// `Q` (row-major `n×n`) must be symmetric positive definite; that is a caller
/// precondition, not checked here, exactly as the kernels themselves require it.
pub struct DenseQp {
    pub n: usize,
    pub q: Vec<f64>,
    pub c: Vec<f64>,
    pub lower: Vec<f64>,
    pub upper: Vec<f64>,
    /// Row-major `m × n`.
    pub a: Vec<f64>,
    pub b: Vec<f64>,
}

/// The exact optimum, or `None` if the problem is infeasible or `n > MAX_N`.
pub fn exact_optimum(problem: &DenseQp) -> Option<Vec<f64>> {
    let n = problem.n;
    if n == 0 || n > MAX_N {
        return None;
    }
    let m = problem.b.len();

    // Rows: the `m` constraint rows, then the `2n` box faces.
    let mut g: Vec<Vec<f64>> = Vec::with_capacity(m + 2 * n);
    let mut h: Vec<f64> = Vec::with_capacity(m + 2 * n);
    for i in 0..m {
        g.push(problem.a[i * n..(i + 1) * n].to_vec());
        h.push(problem.b[i]);
    }
    for j in 0..n {
        let mut up = vec![0.0; n];
        up[j] = 1.0;
        g.push(up);
        h.push(problem.upper[j]);
        let mut down = vec![0.0; n];
        down[j] = -1.0;
        g.push(down);
        h.push(-problem.lower[j]);
    }
    let feasible = |x: &[f64]| {
        g.iter()
            .zip(&h)
            .all(|(row, limit)| dot(row, x) <= limit + SLACK)
    };

    // Q⁻¹, one column at a time: n small, so n linear solves are cheap.
    let q_rows: Vec<Vec<f64>> = (0..n)
        .map(|i| problem.q[i * n..(i + 1) * n].to_vec())
        .collect();
    let mut q_inv_cols: Vec<Vec<f64>> = Vec::with_capacity(n);
    for j in 0..n {
        let mut e = vec![0.0; n];
        e[j] = 1.0;
        q_inv_cols.push(solve(q_rows.clone(), e)?);
    }
    let apply_q_inv = |v: &[f64]| -> Vec<f64> {
        (0..n)
            .map(|r| (0..n).map(|col| q_inv_cols[col][r] * v[col]).sum())
            .collect()
    };

    // The unconstrained optimum: Q·center = -c.
    let neg_c: Vec<f64> = problem.c.iter().map(|v| -v).collect();
    let center = apply_q_inv(&neg_c);
    if feasible(&center) {
        return Some(center);
    }

    // Q⁻¹gᵢᵀ for every row, reused across every active-set candidate.
    let q_inv_g: Vec<Vec<f64>> = g.iter().map(|row| apply_q_inv(row)).collect();

    for pool in [m, g.len()] {
        for k in 1..=n.min(pool) {
            for set in subsets(pool, k) {
                let gram: Vec<Vec<f64>> = set
                    .iter()
                    .map(|&i| set.iter().map(|&l| dot(&g[i], &q_inv_g[l])).collect())
                    .collect();
                let rhs: Vec<f64> = set.iter().map(|&i| dot(&g[i], &center) - h[i]).collect();
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
                            .map(|(&i, &l)| l * q_inv_g[i][j])
                            .sum();
                        center[j] - pull
                    })
                    .collect();
                if feasible(&x) {
                    return Some(x);
                }
            }
        }
        if pool == g.len() {
            break;
        }
    }
    None
}

/// The exact optimum of a dense **LP** (`Q = 0`): minimise `cᵀx` subject to
/// `lower ≤ x ≤ upper`, `Ax ≤ b`. See the module doc for why `exact_optimum` cannot serve
/// this case.
///
/// At a vertex of a bounded polytope, a linear objective's minimiser sits where exactly
/// `n` linearly independent constraints are active (a degenerate vertex has more active
/// constraints than `n`, but any `n` of them that are linearly independent still pin it,
/// and this enumeration tries every `n`-subset of the row pool). For such a set `S`
/// (`|S| = n`), stationarity of the Lagrangian `cᵀx + Σ λᵢ(Gᵢx − hᵢ)` requires
/// `Gₛᵀλ = −c` — no `Qx` term, unlike `exact_optimum` — and the active constraints pin `x`
/// directly: `Gₛx = hₛ`. A candidate is the global optimum when `λ ≥ 0` (dual feasibility)
/// and `x` satisfies every row (primal feasibility): KKT is sufficient here because the
/// feasible region is a polytope (convex) and the objective is affine (trivially convex).
///
/// Scope, row pool, feasibility slack and sign convention for `λ` match `exact_optimum`
/// exactly; `n ≤ MAX_N`. Unlike `exact_optimum`, only `k = n` subsets are tried: with no
/// `Q` to supply the missing degrees of freedom, a `k < n` active set does not pin a
/// point, so trying one would be meaningless rather than merely redundant.
pub fn exact_lp_optimum(problem: &DenseQp) -> Option<Vec<f64>> {
    let n = problem.n;
    if n == 0 || n > MAX_N {
        return None;
    }
    let m = problem.b.len();

    let mut g: Vec<Vec<f64>> = Vec::with_capacity(m + 2 * n);
    let mut h: Vec<f64> = Vec::with_capacity(m + 2 * n);
    for i in 0..m {
        g.push(problem.a[i * n..(i + 1) * n].to_vec());
        h.push(problem.b[i]);
    }
    for j in 0..n {
        let mut up = vec![0.0; n];
        up[j] = 1.0;
        g.push(up);
        h.push(problem.upper[j]);
        let mut down = vec![0.0; n];
        down[j] = -1.0;
        g.push(down);
        h.push(-problem.lower[j]);
    }
    let feasible = |x: &[f64]| {
        g.iter()
            .zip(&h)
            .all(|(row, limit)| dot(row, x) <= limit + SLACK)
    };
    let neg_c: Vec<f64> = problem.c.iter().map(|v| -v).collect();

    for set in subsets(g.len(), n) {
        // Gₛᵀ: row `r`, column `col` is active row `set[col]`'s `r`-th coefficient.
        let gt: Vec<Vec<f64>> = (0..n)
            .map(|r| set.iter().map(|&i| g[i][r]).collect())
            .collect();
        let Some(lambda) = solve(gt, neg_c.clone()) else {
            continue;
        };
        if lambda.iter().any(|&l| l < -SLACK) {
            continue;
        }
        let gs: Vec<Vec<f64>> = set.iter().map(|&i| g[i].clone()).collect();
        let hs: Vec<f64> = set.iter().map(|&i| h[i]).collect();
        let Some(x) = solve(gs, hs) else {
            continue;
        };
        if feasible(&x) {
            return Some(x);
        }
    }
    None
}

fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
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
        let pivot_row = a[c][c..].to_vec();
        let pivot_r = r[c];
        for row in (c + 1)..n {
            let factor = a[row][c] / a[c][c];
            for (offset, &value) in pivot_row.iter().enumerate() {
                a[row][c + offset] -= factor * value;
            }
            r[row] -= factor * pivot_r;
        }
    }
    let mut x = vec![0.0; n];
    for c in (0..n).rev() {
        let sum: f64 = (c + 1..n).map(|j| a[c][j] * x[j]).sum();
        x[c] = (r[c] - sum) / a[c][c];
    }
    Some(x)
}

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

#[cfg(test)]
mod tests {
    use super::{DenseQp, MAX_N, dot, exact_lp_optimum, exact_optimum};

    /// `minimise x² + y² - 2x - 4y` over `[0,10]²`: unconstrained optimum `(1, 2)`, inside
    /// the box.
    #[test]
    fn an_unconstrained_interior_optimum_is_found_directly() {
        let problem = DenseQp {
            n: 2,
            q: vec![2.0, 0.0, 0.0, 2.0],
            c: vec![-2.0, -4.0],
            lower: vec![0.0, 0.0],
            upper: vec![10.0, 10.0],
            a: vec![],
            b: vec![],
        };
        let x = exact_optimum(&problem).expect("feasible");
        assert!(
            (x[0] - 1.0).abs() < 1e-9 && (x[1] - 2.0).abs() < 1e-9,
            "{x:?}"
        );
    }

    /// A non-diagonal (coupled) `Q`: `minimise (x-y)² + x² + y² - 2x - 2y`, i.e.
    /// `Q = [[4,-2],[-2,4]]`, `c = (-2,-2)`, unconstrained optimum `(1, 1)` by symmetry.
    #[test]
    fn a_coupled_q_is_solved_correctly() {
        let problem = DenseQp {
            n: 2,
            q: vec![4.0, -2.0, -2.0, 4.0],
            c: vec![-2.0, -2.0],
            lower: vec![-10.0, -10.0],
            upper: vec![10.0, 10.0],
            a: vec![],
            b: vec![],
        };
        let x = exact_optimum(&problem).expect("feasible");
        assert!(
            (x[0] - 1.0).abs() < 1e-9 && (x[1] - 1.0).abs() < 1e-9,
            "{x:?}"
        );
    }

    /// Box-active: the unconstrained optimum `(5, 5)` sits outside `[0,1]²`, so the
    /// optimum is the clamped corner `(1, 1)`.
    #[test]
    fn a_box_active_optimum_is_found() {
        let problem = DenseQp {
            n: 2,
            q: vec![2.0, 0.0, 0.0, 2.0],
            c: vec![-10.0, -10.0],
            lower: vec![0.0, 0.0],
            upper: vec![1.0, 1.0],
            a: vec![],
            b: vec![],
        };
        let x = exact_optimum(&problem).expect("feasible");
        assert!(
            (x[0] - 1.0).abs() < 1e-9 && (x[1] - 1.0).abs() < 1e-9,
            "{x:?}"
        );
    }

    /// A row of `A` active: `x0 + x1 <= 1`, with `Q = I`, `c = (-4,-4)` — the unconstrained
    /// optimum `(4,4)` is pulled onto the line `x0+x1=1` with `x0=x1=0.5` by symmetry.
    #[test]
    fn an_active_row_of_a_is_found() {
        let problem = DenseQp {
            n: 2,
            q: vec![2.0, 0.0, 0.0, 2.0],
            c: vec![-4.0, -4.0],
            lower: vec![0.0, 0.0],
            upper: vec![10.0, 10.0],
            a: vec![1.0, 1.0],
            b: vec![1.0],
        };
        let x = exact_optimum(&problem).expect("feasible");
        assert!(
            (x[0] - 0.5).abs() < 1e-9 && (x[1] - 0.5).abs() < 1e-9,
            "{x:?}"
        );
    }

    /// An infeasible box (`lower > upper`) has no KKT point, and the center itself is
    /// infeasible, so every candidate is rejected.
    #[test]
    fn an_infeasible_problem_returns_none() {
        let problem = DenseQp {
            n: 1,
            q: vec![2.0],
            c: vec![0.0],
            lower: vec![5.0],
            upper: vec![1.0],
            a: vec![],
            b: vec![],
        };
        assert!(exact_optimum(&problem).is_none());
    }

    #[test]
    fn n_past_the_scoped_bound_is_refused() {
        let problem = DenseQp {
            n: MAX_N + 1,
            q: vec![0.0; (MAX_N + 1) * (MAX_N + 1)],
            c: vec![0.0; MAX_N + 1],
            lower: vec![0.0; MAX_N + 1],
            upper: vec![0.0; MAX_N + 1],
            a: vec![],
            b: vec![],
        };
        assert!(exact_optimum(&problem).is_none());
    }

    /// RFC 037 C4.1.1: the `n = 4, m = 2, off = 0.50` corpus point, with its exact optimum
    /// derived independently of `exact_optimum` — by KKT enumeration in **exact rational
    /// arithmetic** (architect review 088 §2), not by this routine or by `f64`:
    ///
    /// `x* = (7/17, 5/17, 5/17, 7/17)`, both rows of `A` exactly active:
    /// `7/17 + 5/17 + 5/17 = 1`.
    ///
    /// The architect's independent check against the rationals agreed to `5.6e-17` per
    /// component; `1e-9` here leaves ample margin for `f64` Gaussian elimination.
    #[test]
    fn a_coupled_corpus_point_matches_an_independently_derived_exact_vector() {
        // The corpus family (RFC 037 §0.1): tridiagonal Q with off = 0.50, c = -1, box
        // [0, 10], and the two sliding-window rows of A for n = 4.
        let problem = DenseQp {
            n: 4,
            q: vec![
                2.0, 0.5, 0.0, 0.0, //
                0.5, 2.0, 0.5, 0.0, //
                0.0, 0.5, 2.0, 0.5, //
                0.0, 0.0, 0.5, 2.0,
            ],
            c: vec![-1.0; 4],
            lower: vec![0.0; 4],
            upper: vec![10.0; 4],
            a: vec![
                1.0, 1.0, 1.0, 0.0, //
                0.0, 1.0, 1.0, 1.0,
            ],
            b: vec![1.0, 1.0],
        };
        let x = exact_optimum(&problem).expect("feasible");
        // 7/17, 5/17, 5/17, 7/17, written as fractions so a later reader can re-derive them.
        let expected = [7.0 / 17.0, 5.0 / 17.0, 5.0 / 17.0, 7.0 / 17.0];
        for (i, (got, want)) in x.iter().zip(&expected).enumerate() {
            assert!(
                (got - want).abs() < 1e-9,
                "coordinate {i}: got {got}, expected {want} (exact: {})",
                ["7/17", "5/17", "5/17", "7/17"][i]
            );
        }
    }

    /// RFC 037 C4.1.2: a randomized optimality oracle. For a convex QP, `x*` is optimal
    /// iff `∇f(x*)ᵀ(y − x*) ≥ 0` for every feasible `y` — no second solver needed, so this
    /// validates `exact_optimum` on a genuinely coupled `Q` without another implementation
    /// to trust. Seeded deterministically (the same 64-bit LCG the kernel tests use), so a
    /// failure is reproducible.
    fn lcg(seed: u64) -> impl FnMut() -> f64 {
        let mut state = seed;
        move || {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((state >> 11) as f64) / ((1u64 << 53) as f64)
        }
    }

    fn gradient(q: &[f64], c: &[f64], n: usize, x: &[f64]) -> Vec<f64> {
        (0..n)
            .map(|i| c[i] + (0..n).map(|j| q[i * n + j] * x[j]).sum::<f64>())
            .collect()
    }

    fn feasible(problem: &DenseQp, y: &[f64]) -> bool {
        let n = problem.n;
        if (0..n).any(|i| y[i] < problem.lower[i] - 1e-12 || y[i] > problem.upper[i] + 1e-12) {
            return false;
        }
        let m = problem.b.len();
        (0..m).all(|r| dot(&problem.a[r * n..(r + 1) * n], y) <= problem.b[r] + 1e-9)
    }

    /// A box each coordinate can be sampled from without missing any feasible point: the
    /// box bound tightened by every row whose coefficients are non-negative and whose
    /// other variables' lower bounds are zero — true of every row in these test problems.
    /// Not a general LP bound; a cheap one that holds for this family.
    fn sampling_upper_bound(problem: &DenseQp) -> Vec<f64> {
        let n = problem.n;
        let m = problem.b.len();
        let mut bound = problem.upper.clone();
        for r in 0..m {
            let row = &problem.a[r * n..(r + 1) * n];
            for (i, &coeff) in row.iter().enumerate() {
                if coeff > 0.0 {
                    bound[i] = bound[i].min(problem.b[r] / coeff);
                }
            }
        }
        bound
    }

    #[test]
    fn a_deliberately_wrong_point_is_rejected_by_the_optimality_oracle_while_the_true_optimum_passes()
     {
        let problem = DenseQp {
            n: 4,
            q: vec![
                2.0, 0.5, 0.0, 0.0, //
                0.5, 2.0, 0.5, 0.0, //
                0.0, 0.5, 2.0, 0.5, //
                0.0, 0.0, 0.5, 2.0,
            ],
            c: vec![-1.0; 4],
            lower: vec![0.0; 4],
            upper: vec![10.0; 4],
            a: vec![
                1.0, 1.0, 1.0, 0.0, //
                0.0, 1.0, 1.0, 1.0,
            ],
            b: vec![1.0, 1.0],
        };
        let true_x = exact_optimum(&problem).expect("feasible");
        let wrong_x = [0.40, 0.30, 0.30, 0.40];

        let grad_true = gradient(&problem.q, &problem.c, problem.n, &true_x);
        let grad_wrong = gradient(&problem.q, &problem.c, problem.n, &wrong_x);
        let bound = sampling_upper_bound(&problem);

        let mut next = lcg(0x0ddc0de_10add1e5);
        let (mut worst_true, mut worst_wrong) = (f64::INFINITY, f64::INFINITY);
        let mut feasible_count = 0usize;
        let target = 10_000;
        for _ in 0..2_000_000 {
            if feasible_count >= target {
                break;
            }
            let y: Vec<f64> = (0..problem.n).map(|i| next() * bound[i]).collect();
            if !feasible(&problem, &y) {
                continue;
            }
            feasible_count += 1;
            let value_true: f64 = grad_true
                .iter()
                .zip(&y)
                .zip(&true_x)
                .map(|((g, yi), xi)| g * (yi - xi))
                .sum();
            let value_wrong: f64 = grad_wrong
                .iter()
                .zip(&y)
                .zip(&wrong_x)
                .map(|((g, yi), xi)| g * (yi - xi))
                .sum();
            worst_true = worst_true.min(value_true);
            worst_wrong = worst_wrong.min(value_wrong);
        }
        assert!(
            feasible_count >= target,
            "only {feasible_count} feasible samples"
        );
        eprintln!(
            "optimality oracle: {feasible_count} feasible samples; true optimum worst {worst_true:e}; wrong point worst {worst_wrong:e}"
        );
        assert!(
            worst_true > -1e-9,
            "the true optimum failed the oracle: {worst_true:e}"
        );
        assert!(
            worst_wrong < -1e-3,
            "the wrong point was not caught by the oracle: {worst_wrong:e}"
        );
    }

    #[test]
    fn the_optimality_oracle_holds_for_every_coupled_instance_within_scope() {
        // Several tridiagonal, window-constrained instances across the scoped n, mirroring
        // the corpus family's shape but not importing it, since `exact` does not depend on
        // `bench`.
        for (n, m, off) in [
            (2usize, 1, 0.3),
            (4, 2, 0.1),
            (4, 2, 0.9),
            (6, 3, 0.5),
            (8, 4, 0.7),
        ] {
            let mut q = vec![0.0; n * n];
            for i in 0..n {
                q[i * n + i] = 2.0;
                if i + 1 < n {
                    q[i * n + i + 1] = off;
                    q[(i + 1) * n + i] = off;
                }
            }
            let mut a = vec![0.0; m * n];
            for r in 0..m {
                for k in 0..3.min(n) {
                    a[r * n + (r + k) % n] = 1.0;
                }
            }
            let problem = DenseQp {
                n,
                q,
                c: vec![-1.0; n],
                lower: vec![0.0; n],
                upper: vec![10.0; n],
                a,
                b: vec![1.0; m],
            };
            let x = exact_optimum(&problem)
                .unwrap_or_else(|| panic!("n={n}, m={m}, off={off}: no KKT point found"));
            let grad = gradient(&problem.q, &problem.c, n, &x);
            let bound = sampling_upper_bound(&problem);
            let mut next = lcg(0x5EED_u64
                .wrapping_add(n as u64)
                .wrapping_add((m as u64) << 8));
            let mut worst = f64::INFINITY;
            let mut feasible_count = 0usize;
            for _ in 0..500_000 {
                if feasible_count >= 2_000 {
                    break;
                }
                let y: Vec<f64> = (0..n).map(|i| next() * bound[i]).collect();
                if !feasible(&problem, &y) {
                    continue;
                }
                feasible_count += 1;
                let value: f64 = grad
                    .iter()
                    .zip(&y)
                    .zip(&x)
                    .map(|((g, yi), xi)| g * (yi - xi))
                    .sum();
                worst = worst.min(value);
            }
            assert!(
                feasible_count >= 2_000,
                "n={n}, m={m}, off={off}: only {feasible_count} feasible samples"
            );
            assert!(
                worst > -1e-9,
                "n={n}, m={m}, off={off}: the optimality oracle failed: {worst:e}"
            );
        }
    }

    // RFC 039: `exact_lp_optimum` (`Q = 0`).

    fn lp(
        n: usize,
        c: Vec<f64>,
        lower: Vec<f64>,
        upper: Vec<f64>,
        a: Vec<f64>,
        b: Vec<f64>,
    ) -> DenseQp {
        DenseQp {
            n,
            q: vec![0.0; n * n],
            c,
            lower,
            upper,
            a,
            b,
        }
    }

    #[test]
    fn a_single_vertex_lp_matches_the_hand_solved_answer() {
        // minimise -(x0 + 0.5 x1) over x0 + x1 <= 1, box [0,1]^2: the unique vertex (1, 0).
        let problem = lp(
            2,
            vec![-1.0, -0.5],
            vec![0.0, 0.0],
            vec![1.0, 1.0],
            vec![1.0, 1.0],
            vec![1.0],
        );
        let x = exact_lp_optimum(&problem).expect("feasible");
        assert!(
            (x[0] - 1.0).abs() < 1e-9 && (x[1] - 0.0).abs() < 1e-9,
            "{x:?}"
        );
    }

    #[test]
    fn a_degenerate_vertex_with_more_active_rows_than_n_is_found() {
        // minimise -(x0 + x1) over x0 + x1 <= 2, box [0,1]^2: at (1, 1) three rows are
        // active (both box faces and the general row), one more than n = 2.
        let problem = lp(
            2,
            vec![-1.0, -1.0],
            vec![0.0, 0.0],
            vec![1.0, 1.0],
            vec![1.0, 1.0],
            vec![2.0],
        );
        let x = exact_lp_optimum(&problem).expect("feasible");
        assert!(
            (x[0] - 1.0).abs() < 1e-9 && (x[1] - 1.0).abs() < 1e-9,
            "{x:?}"
        );
    }

    #[test]
    fn a_tie_along_an_optimal_face_returns_a_point_on_the_face_not_a_fixed_vertex() {
        // minimise -(x0 + x1) over x0 + x1 <= 1, box [0,1]^2: every point with
        // x0 + x1 = 1, 0 <= x0, x1 <= 1 is optimal. The answer is non-unique, so the
        // check is face membership, not equality to one vertex.
        let problem = lp(
            2,
            vec![-1.0, -1.0],
            vec![0.0, 0.0],
            vec![1.0, 1.0],
            vec![1.0, 1.0],
            vec![1.0],
        );
        let x = exact_lp_optimum(&problem).expect("feasible");
        assert!((x[0] + x[1] - 1.0).abs() < 1e-9, "{x:?}");
        assert!(
            (0.0..=1.0).contains(&x[0]) && (0.0..=1.0).contains(&x[1]),
            "{x:?}"
        );
    }

    #[test]
    fn an_unopposed_direction_is_bounded_only_by_a_distant_box_face() {
        // minimise -(x0 + x1) over x1 <= 0.5 (x0 unconstrained by any general row), box
        // x0 in [0, 1e6], x1 in [0, 1]: the general row never opposes x0, so x0 only stops
        // at its distant box face.
        let problem = lp(
            2,
            vec![-1.0, -1.0],
            vec![0.0, 0.0],
            vec![1e6, 1.0],
            vec![0.0, 1.0],
            vec![0.5],
        );
        let x = exact_lp_optimum(&problem).expect("feasible");
        assert!(
            (x[0] - 1e6).abs() < 1e-6 && (x[1] - 0.5).abs() < 1e-9,
            "{x:?}"
        );
    }

    #[test]
    fn an_infeasible_lp_returns_none_not_a_wrong_answer() {
        // x0 <= 0 and x0 >= 1 simultaneously: no feasible point exists.
        let problem = lp(1, vec![-1.0], vec![1.0], vec![0.0], vec![], vec![]);
        assert!(exact_lp_optimum(&problem).is_none());
    }

    #[test]
    fn exact_lp_optimum_passes_the_randomized_optimality_oracle_on_random_instances() {
        // Independent-ish cross-check: `gradient` and `feasible` are shared with
        // `exact_optimum`'s own oracle tests above, but the candidate under test comes
        // from a different code path (no `Q` inversion at all).
        let mut seed = lcg(0xA17E5_u64);
        for instance in 0..20 {
            let n = 4;
            let m = 3;
            let c: Vec<f64> = (0..n).map(|_| seed() * 2.0 - 1.0).collect();
            let a: Vec<f64> = (0..m * n).map(|_| seed()).collect();
            let b: Vec<f64> = (0..m).map(|_| seed() + 0.5).collect();
            let problem = lp(n, c, vec![0.0; n], vec![1.0; n], a, b);
            let Some(x) = exact_lp_optimum(&problem) else {
                continue;
            };
            let grad = gradient(&problem.q, &problem.c, n, &x);
            let bound = sampling_upper_bound(&problem);
            let mut next = lcg(0xFEED_u64.wrapping_add(instance));
            let mut worst = f64::INFINITY;
            let mut feasible_count = 0usize;
            for _ in 0..200_000 {
                if feasible_count >= 2_000 {
                    break;
                }
                let y: Vec<f64> = (0..n).map(|i| next() * bound[i]).collect();
                if !feasible(&problem, &y) {
                    continue;
                }
                feasible_count += 1;
                let value: f64 = grad
                    .iter()
                    .zip(&y)
                    .zip(&x)
                    .map(|((g, yi), xi)| g * (yi - xi))
                    .sum();
                worst = worst.min(value);
            }
            assert!(
                feasible_count >= 2_000,
                "instance {instance}: only {feasible_count} feasible samples"
            );
            assert!(
                worst > -1e-9,
                "instance {instance}: the optimality oracle failed: {worst:e}"
            );
        }
    }
}
