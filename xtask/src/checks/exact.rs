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
    use super::{DenseQp, MAX_N, exact_optimum};

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
}
