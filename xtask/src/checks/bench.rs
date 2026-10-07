//! `bench` — RFC 037 (theme T4): counted work, reported and not enforced.
//!
//! The owner's fourth visitor question is how effective and how powerful the solver
//! is. This command answers the part that can be answered reproducibly: how much
//! counted work a solve takes, on a corpus that varies **both** problem size and
//! conditioning. RFC 037 §3 explains why size alone would publish a flattering,
//! uninformative curve: iteration count is flat in dimension and grows about fortyfold
//! with conditioning.
//!
//! **This is not a gate, and it must not become one.** `cargo xtask check` and
//! `cargo xtask release-gate` do not run it. Its figures are reported, and figures are
//! not gates. The only pinned counts are in `bench-baseline`, which is a separate, narrow
//! command with its own registration. Do not add `bench` to the gate tuple list; the
//! omission is deliberate.
//!
//! **Measurement, not telemetry.** Every figure comes from the solve record
//! (`ConstrainedSolveRecord` and its `SolveReport`). `loeres_cluster::observe` is not a
//! source: it buckets and redacts by design (RFC 009, the threat model), and telemetry is
//! for operators, not for measurement.
//!
//! **Measured and derived are labelled apart.** The records do not expose per-iteration
//! arithmetic, and this command does not instrument the kernels to count it (RFC 037
//! §4.2; T4 measures rather than modifies). So every figure in the output is marked
//! `[measured]` (read from a solve record) or `[derived]` (computed from the algorithm,
//! and marked as an upper bound where it is one). A derived figure presented as a
//! measurement is the failure this command exists to prevent.
//!
//! **Wall time is absent.** This command reports no elapsed time. Throughput is S4 and
//! carries its host; it is never part of counted work.
//!
//! The corpus, the measurement, the derived figures and the table layout are pure
//! functions with unit tests, because nothing else exercises this command.

use loeres::{BoxBounds, LinearInequalities, QuadraticObjective, SolveStatus, SolverError};
use loeres_backend_static::array::{FixedMatrix, FixedVector};
use loeres_backend_static::workspace::WorkspaceFootprint;
use loeres_backend_std::{DenseMatrix, DenseVector};
use loeres_cluster::{
    ClusterCancellationToken, ClusterConstrainedWorkspace, ClusterExecutionContext,
    ClusterValidationPolicy, ConstrainedProjectedConfig,
    solve_constrained_projected_first_order_dyn,
};

/// The fixed step scale. RFC 032's Gershgorin bound for the corpus is `U = 2 + 2·off`,
/// so `α = 0.3` is admissible across every conditioning in the corpus (`2/U ≥ 0.503`).
/// It is not varied with `off`: that would conflate the two axes being measured.
pub const STEP_SCALE: f64 = 0.3;

/// The fixed solve configuration (RFC 037 §0.1 of the implementation handoff).
const MAX_ITERATIONS: u32 = 20_000;
const TOLERANCE: f64 = 1e-10;
const PROJECTION_MAX_SWEEPS: u32 = 500;
pub const PROJECTION_TOLERANCE: f64 = 1e-10;

/// One member of the corpus family: a tridiagonal SPD `Q` with `off` off the diagonal,
/// `m` sliding-window halfspaces over `n` variables, and a box.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Family {
    pub n: usize,
    pub m: usize,
    pub off: f64,
}

/// Which of the two axes a row belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    /// Fixed conditioning (`off = 0.5`), varying size.
    Size,
    /// Fixed size (`n = 32`, `m = 16`), varying conditioning.
    Conditioning,
}

/// The size axis: `m = n / 2` at `off = 0.5`.
pub fn size_axis() -> Vec<Family> {
    [
        (4, 2),
        (8, 4),
        (16, 8),
        (32, 16),
        (64, 32),
        (128, 64),
        (256, 128),
    ]
    .into_iter()
    .map(|(n, m)| Family { n, m, off: 0.5 })
    .collect()
}

/// The conditioning axis: `n = 32`, `m = 16`, five values of `off`.
pub fn conditioning_axis() -> Vec<Family> {
    [0.10, 0.50, 0.90, 0.97, 0.99]
        .into_iter()
        .map(|off| Family { n: 32, m: 16, off })
        .collect()
}

/// The whole counted-work corpus, in report order. The `off = 0.50, n = 32, m = 16`
/// point is shared by both axes and must agree between them.
pub fn corpus() -> Vec<(Axis, Family)> {
    size_axis()
        .into_iter()
        .map(|f| (Axis::Size, f))
        .chain(
            conditioning_axis()
                .into_iter()
                .map(|f| (Axis::Conditioning, f)),
        )
        .collect()
}

/// Row-major `Q`: `Q[i][i] = 2`, `Q[i][i±1] = off`, all else zero.
pub fn q_row_major(n: usize, off: f64) -> Vec<f64> {
    let mut q = vec![0.0; n * n];
    for i in 0..n {
        q[i * n + i] = 2.0;
        if i + 1 < n {
            q[i * n + i + 1] = off;
            q[(i + 1) * n + i] = off;
        }
    }
    q
}

/// Row-major `A` (`m × n`): row `r` has `1.0` at columns `(r + k) mod n` for
/// `k ∈ {0, 1, 2}` — each row caps a sliding window of three variables.
pub fn a_row_major(n: usize, m: usize) -> Vec<f64> {
    let mut a = vec![0.0; m * n];
    for r in 0..m {
        for k in 0..3 {
            a[r * n + (r + k) % n] = 1.0;
        }
    }
    a
}

/// The dynamic QP for one family member. `Q`, `c`, the box and `A` are fixed by the
/// family; `b` is `1` throughout.
#[derive(Clone)]
pub struct Program {
    q: DenseMatrix<f64>,
    c: DenseVector<f64>,
    lower: DenseVector<f64>,
    upper: DenseVector<f64>,
    a: DenseMatrix<f64>,
    b: DenseVector<f64>,
}

impl Program {
    pub fn new(family: Family) -> Result<Self, SolverError> {
        let Family { n, m, off } = family;
        Ok(Self {
            q: matrix(n, n, q_row_major(n, off))?,
            c: vector(vec![-1.0; n])?,
            lower: vector(vec![0.0; n])?,
            upper: vector(vec![10.0; n])?,
            a: matrix(m, n, a_row_major(n, m))?,
            b: vector(vec![1.0; m])?,
        })
    }
}

fn matrix(rows: usize, cols: usize, data: Vec<f64>) -> Result<DenseMatrix<f64>, SolverError> {
    DenseMatrix::from_row_major_vec(rows, cols, data).map_err(|_| SolverError::InvalidDimension)
}

fn vector(values: Vec<f64>) -> Result<DenseVector<f64>, SolverError> {
    DenseVector::from_vec(values).map_err(|_| SolverError::InvalidDimension)
}

impl QuadraticObjective<f64> for Program {
    type Hessian = DenseMatrix<f64>;
    type Linear = DenseVector<f64>;
    fn hessian(&self) -> &Self::Hessian {
        &self.q
    }
    fn linear_term(&self) -> &Self::Linear {
        &self.c
    }
}

impl BoxBounds<f64> for Program {
    type Bound = DenseVector<f64>;
    fn lower_bounds(&self) -> &Self::Bound {
        &self.lower
    }
    fn upper_bounds(&self) -> &Self::Bound {
        &self.upper
    }
}

impl LinearInequalities<f64> for Program {
    type Constraints = DenseMatrix<f64>;
    type Rhs = DenseVector<f64>;
    fn constraint_matrix(&self) -> &Self::Constraints {
        &self.a
    }
    fn constraint_rhs(&self) -> &Self::Rhs {
        &self.b
    }
}

use loeres_device::config::TimingMode;
use loeres_device::solve::{
    ConstrainedProjectedWorkspace, ConstrainedSolveConfig, solve_constrained_projected_first_order,
};

/// The device corpus: a `const` table of `(N, M)` instantiations driven by a macro (RFC
/// 037 §2.2). The device entry point is const-generic, so the corpus cannot be a runtime
/// loop. Each row also fixes the two capacities `N·N` and `M·N`, because generic const
/// expressions are not stable on the MSRV. A size is added by adding a row here, and the
/// corpus's `(n, m)` pairs must all appear in this table.
macro_rules! device_instances {
    ($(($n:literal, $m:literal)),* $(,)?) => {
        /// The `(n, m)` pairs the device path can run. Every size of the device corpus
        /// must appear here; `device_run` reports any that does not.
        pub const DEVICE_SIZES: &[(usize, usize)] = &[$(($n, $m)),*];

        fn device_run(n: usize, m: usize, off: f64) -> Option<Result<DevicePoint, String>> {
            match (n, m) {
                $( ($n, $m) => Some(run_device::<$n, $m, { $n * $n }, { $m * $n }>(off)), )*
                _ => None,
            }
        }
    };
}

device_instances! {
    (4, 2),
    (8, 4),
    (16, 8),
    (32, 16),
    (64, 32),
    (128, 64),
    (256, 128),
}

/// The device problem for one `(N, M)` instantiation, with the same family as the
/// cluster path.
pub struct DeviceProgram<const N: usize, const M: usize, const NN: usize, const MN: usize> {
    q: FixedMatrix<f64, N, N, NN>,
    c: FixedVector<f64, N>,
    lower: FixedVector<f64, N>,
    upper: FixedVector<f64, N>,
    a: FixedMatrix<f64, M, N, MN>,
    b: FixedVector<f64, M>,
}

impl<const N: usize, const M: usize, const NN: usize, const MN: usize> DeviceProgram<N, M, NN, MN> {
    /// Build the family member with the given `off`. The large fields are boxed by the
    /// caller, so the 256-variable instantiation does not sit on a thread stack.
    pub fn new(off: f64) -> Result<Box<Self>, String> {
        Ok(Box::new(Self {
            q: FixedMatrix::from_row_major_array(array_of::<NN>(q_row_major(N, off))?),
            c: FixedVector::from_array([-1.0; N]),
            lower: FixedVector::from_array([0.0; N]),
            upper: FixedVector::from_array([10.0; N]),
            a: FixedMatrix::from_row_major_array(array_of::<MN>(a_row_major(N, M))?),
            b: FixedVector::from_array([1.0; M]),
        }))
    }
}

fn array_of<const K: usize>(values: Vec<f64>) -> Result<[f64; K], String> {
    values
        .try_into()
        .map_err(|v: Vec<f64>| format!("expected {K} entries, got {}", v.len()))
}

impl<const N: usize, const M: usize, const NN: usize, const MN: usize> QuadraticObjective<f64>
    for DeviceProgram<N, M, NN, MN>
{
    type Hessian = FixedMatrix<f64, N, N, NN>;
    type Linear = FixedVector<f64, N>;
    fn hessian(&self) -> &Self::Hessian {
        &self.q
    }
    fn linear_term(&self) -> &Self::Linear {
        &self.c
    }
}

impl<const N: usize, const M: usize, const NN: usize, const MN: usize> BoxBounds<f64>
    for DeviceProgram<N, M, NN, MN>
{
    type Bound = FixedVector<f64, N>;
    fn lower_bounds(&self) -> &Self::Bound {
        &self.lower
    }
    fn upper_bounds(&self) -> &Self::Bound {
        &self.upper
    }
}

impl<const N: usize, const M: usize, const NN: usize, const MN: usize> LinearInequalities<f64>
    for DeviceProgram<N, M, NN, MN>
{
    type Constraints = FixedMatrix<f64, M, N, MN>;
    type Rhs = FixedVector<f64, M>;
    fn constraint_matrix(&self) -> &Self::Constraints {
        &self.a
    }
    fn constraint_rhs(&self) -> &Self::Rhs {
        &self.b
    }
}

/// One device solve. The counted work is `[measured]`; the footprint is measured with
/// `size_of` and the documented formula is `[derived]` from RFC 027.
#[derive(Clone, Debug, PartialEq)]
pub struct DevicePoint {
    pub measured: Measured,
    /// `WorkspaceFootprint::footprint_bytes()`: `size_of` of the workspace struct, `[measured]`.
    pub footprint_bytes: usize,
    /// `(3N + 2M)·size_of::<f64>() + 16`, RFC 027's documented formula, `[derived]`.
    pub documented_bytes: usize,
}

fn run_device<const N: usize, const M: usize, const NN: usize, const MN: usize>(
    off: f64,
) -> Result<DevicePoint, String> {
    let problem = DeviceProgram::<N, M, NN, MN>::new(off)?;
    let mut workspace = ConstrainedProjectedWorkspace::<f64, N, M>::new(
        FixedVector::from_array([0.0; N]),
        FixedVector::from_array([0.0; N]),
        FixedVector::from_array([0.0; N]),
        FixedVector::from_array([0.0; M]),
        FixedVector::from_array([0.0; M]),
    );
    let config = ConstrainedSolveConfig {
        max_iterations: MAX_ITERATIONS,
        tolerance: TOLERANCE,
        timing_mode: TimingMode::EarlyExitAllowed,
        projection_max_sweeps: PROJECTION_MAX_SWEEPS,
        projection_tolerance: PROJECTION_TOLERANCE,
    };
    let mut x = FixedVector::from_array([0.0; N]);
    let report = solve_constrained_projected_first_order(
        &*problem,
        STEP_SCALE,
        &mut x,
        &mut workspace,
        &config,
    )
    .map_err(|e| format!("device solve failed: {e:?}"))?;
    Ok(DevicePoint {
        measured: Measured {
            outer_iterations: report.iterations_executed(),
            status: render_status(report.status()),
            termination: format!("{:?}", report.core().termination()),
            cap_hits: report.projection_cap_hits(),
            violation: report.max_constraint_violation(),
            infeasibility_evidence: report.infeasibility_evidence(),
        },
        footprint_bytes:
            <ConstrainedProjectedWorkspace<f64, N, M> as WorkspaceFootprint>::footprint_bytes(),
        documented_bytes: (3 * N + 2 * M) * core::mem::size_of::<f64>() + 16,
    })
}

/// One device solve of a family member, or an error naming it if the device path has
/// no instantiation for its size.
pub fn measure_device(family: Family) -> Result<DevicePoint, String> {
    device_run(family.n, family.m, family.off).unwrap_or_else(|| {
        Err(format!(
            "no device instantiation for n={}, m={}; the device corpus supports {:?}; add a row to device_instances!",
            family.n, family.m, DEVICE_SIZES
        ))
    })
}

/// Counted work read from one solve record. Every field is `[measured]`.
#[derive(Clone, Debug, PartialEq)]
pub struct Measured {
    /// `report.iterations_executed()`: outer (gradient) iterations.
    pub outer_iterations: u32,
    /// `report.status()`, rendered.
    pub status: String,
    /// `report.termination()`, rendered.
    pub termination: String,
    /// `projection_cap_hits`: outer iterations whose projection hit its sweep cap.
    pub cap_hits: u32,
    /// `max_constraint_violation` at the returned iterate.
    pub violation: f64,
    /// The heuristic `infeasibility_evidence` observation (RFC 034). Printed as the
    /// record gives it; it is not a status and is not interpreted here.
    pub infeasibility_evidence: bool,
}

/// Figures computed from the algorithm, not read from a record. Every field is
/// `[derived]`, and where a field is an upper bound it says so in its name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Derived {
    /// A dense `Q·x` costs `n²` multiply-adds per outer iteration. Exact for the dense
    /// kernel; it is a count of the product, not a measured instruction count.
    pub q_multiply_adds_per_outer: u64,
    /// Upper bound on the projection's row operations per outer iteration:
    /// `projection_max_sweeps · m · n`. The projection cannot exceed its sweep cap, so
    /// this bounds the cost; the actual sweeps are not in the record.
    pub projection_ops_upper_bound_per_outer: u64,
}

/// The three legs RFC 037 §5.3 requires a `Converged` to satisfy, as the record shows them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Leg {
    /// Holds: the record shows it.
    Holds,
    /// Fails: the record shows it does not hold. A finding.
    Fails,
    /// Cannot be settled from the record. Reported as such, never as holding.
    Unverifiable,
}

/// Whether one `Converged` record is truthful on each leg.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Legs {
    /// Feasible within `projection_tolerance` (RFC 027 Amendment 5).
    pub feasible: Leg,
    /// Stationary at the final iteration (RFC 029): the solve stopped on the convergence
    /// criterion, not on the iteration cap.
    pub stationary: Leg,
    /// Produced by an uncapped projection (RFC 033). The record counts cap hits over all
    /// outer iterations, not only the final one, so zero cap hits settles it and any
    /// nonzero count cannot be settled from the record.
    pub uncapped: Leg,
}

/// The legs for a measured `Converged`. Returns `None` for any other status: the legs
/// are conditions of `Converged`, and say nothing about a `NotConverged`.
pub fn legs(measured: &Measured) -> Option<Legs> {
    if measured.status != "converged" {
        return None;
    }
    Some(Legs {
        feasible: if measured.violation <= PROJECTION_TOLERANCE {
            Leg::Holds
        } else {
            Leg::Fails
        },
        stationary: if measured.termination == "ConvergenceCriterion" {
            Leg::Holds
        } else {
            Leg::Fails
        },
        uncapped: if measured.cap_hits == 0 {
            Leg::Holds
        } else {
            Leg::Unverifiable
        },
    })
}

/// The truthfulness summary over a set of measured points. Every count is over the
/// `Converged` points only.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Truthfulness {
    pub converged: usize,
    /// Converged, and every leg holds.
    pub truthful: usize,
    /// Converged, and at least one leg fails. A finding, not a statistic.
    pub failing: usize,
    /// Converged, no leg fails, and at least one leg is unverifiable from the record.
    pub unverifiable: usize,
}

pub fn truthfulness<'a>(points: impl IntoIterator<Item = &'a Measured>) -> Truthfulness {
    let mut t = Truthfulness::default();
    for measured in points {
        let Some(l) = legs(measured) else { continue };
        t.converged += 1;
        let legs = [l.feasible, l.stationary, l.uncapped];
        if legs.contains(&Leg::Fails) {
            t.failing += 1;
        } else if legs.contains(&Leg::Unverifiable) {
            t.unverifiable += 1;
        } else {
            t.truthful += 1;
        }
    }
    t
}

/// The derived figures for one family member.
pub fn derived(family: Family) -> Derived {
    let n = family.n as u64;
    let m = family.m as u64;
    Derived {
        q_multiply_adds_per_outer: n * n,
        projection_ops_upper_bound_per_outer: u64::from(PROJECTION_MAX_SWEEPS) * m * n,
    }
}

/// One solve of the family, measured from its record.
pub fn measure(family: Family) -> Result<Measured, String> {
    let program = Program::new(family).map_err(|e| format!("corpus rejected: {e:?}"))?;
    let mut workspace = ClusterConstrainedWorkspace::new(family.n, family.m)
        .map_err(|e| format!("workspace rejected: {e:?}"))?;
    let config = ConstrainedProjectedConfig {
        max_iterations: MAX_ITERATIONS,
        tolerance: TOLERANCE,
        projection_max_sweeps: PROJECTION_MAX_SWEEPS,
        projection_tolerance: PROJECTION_TOLERANCE,
    };
    let context = ClusterExecutionContext::new(
        ClusterCancellationToken::new(),
        0,
        ClusterValidationPolicy::ValidateAllInputs,
    );
    let mut x = vector(vec![0.0; family.n]).map_err(|e| format!("start rejected: {e:?}"))?;
    let record = solve_constrained_projected_first_order_dyn(
        &program,
        STEP_SCALE,
        &mut x,
        &mut workspace,
        &config,
        &context,
    )
    .map_err(|e| format!("solve failed: {e:?}"))?;
    Ok(Measured {
        outer_iterations: record.report.iterations_executed(),
        status: render_status(record.report.status()),
        termination: format!("{:?}", record.report.termination()),
        cap_hits: record.projection_cap_hits,
        violation: record.max_constraint_violation,
        infeasibility_evidence: record.infeasibility_evidence,
    })
}

fn render_status(status: SolveStatus) -> String {
    match status {
        SolveStatus::Converged => "converged".to_owned(),
        SolveStatus::NotConverged => "not converged".to_owned(),
        // `SolveStatus` is `#[non_exhaustive]` downstream: name a future variant.
        _ => "unrecognized".to_owned(),
    }
}

/// The report's header line. The labels are part of the output, not decoration: a
/// reader must be able to tell a read figure from a computed one without the source.
pub fn header() -> String {
    format!(
        "{:<8} {:>4} {:>4} {:>5}  {:>10} {:>8} {:>12} {:>12} {:>14} {:>16}",
        "axis",
        "n",
        "m",
        "off",
        "iters[m]",
        "caps[m]",
        "violation[m]",
        "status[m]",
        "Qx mul-add[d]",
        "proj ops ≤[d]",
    )
}

/// One report row. `measured` and `derived` come from the same family member.
pub fn row(axis: Axis, family: Family, measured: &Measured, derived: &Derived) -> String {
    let axis_name = match axis {
        Axis::Size => "size",
        Axis::Conditioning => "cond",
    };
    format!(
        "{:<8} {:>4} {:>4} {:>5.2}  {:>10} {:>8} {:>12.3e} {:>12} {:>14} {:>16}",
        axis_name,
        family.n,
        family.m,
        family.off,
        measured.outer_iterations,
        measured.cap_hits,
        measured.violation,
        measured.status,
        derived.q_multiply_adds_per_outer,
        derived.projection_ops_upper_bound_per_outer,
    )
}

/// The legend printed beneath the table. It states what each label means in words.
pub fn legend() -> String {
    [
        "[m] measured: read from the solve record (ConstrainedSolveRecord, SolveReport).",
        "[d] derived: computed from the algorithm. Not measured; the kernels are not instrumented.",
        "    Qx mul-add  = n² per outer iteration (dense Q·x).",
        "    proj ops ≤  = projection_max_sweeps · m · n per outer iteration: an upper bound,",
        "                  because the actual sweep count is not in the record.",
        "    footprint   = size_of of the device workspace struct (WorkspaceFootprint), measured.",
        "    documented  = (3N + 2M)·8 + 16, RFC 027's formula, derived. Equal or not is a finding.",
        "Counted work is reproducible for the host target only (RFC 037 §4.3).",
        "No wall time is reported (RFC 037 §4.4: throughput is S4, advisory, with its host).",
    ]
    .join("\n")
}

/// The device report's header. The device path is measured on the host build and is
/// reported, never asserted, for the device target (RFC 037 §4.3).
pub fn device_header() -> String {
    format!(
        "{:<8} {:>4} {:>4} {:>5}  {:>10} {:>8} {:>12} {:>12} {:>14} {:>14}",
        "axis",
        "n",
        "m",
        "off",
        "iters[m]",
        "caps[m]",
        "violation[m]",
        "status[m]",
        "footprint[m]",
        "documented[d]",
    )
}

/// One device report row.
pub fn device_row(axis: Axis, family: Family, point: &DevicePoint) -> String {
    let measured = &point.measured;
    format!(
        "{:<8} {:>4} {:>4} {:>5.2}  {:>10} {:>8} {:>12.3e} {:>12} {:>14} {:>14}",
        axis_name(axis),
        family.n,
        family.m,
        family.off,
        measured.outer_iterations,
        measured.cap_hits,
        measured.violation,
        measured.status,
        point.footprint_bytes,
        point.documented_bytes,
    )
}

fn axis_name(axis: Axis) -> &'static str {
    match axis {
        Axis::Size => "size",
        Axis::Conditioning => "cond",
    }
}

/// Runs the whole corpus on both paths and prints the report. Returns `false` if any
/// solve fails; the failing row names its family. This is a report, not a gate: the return
/// value only says whether the report is complete.
pub fn run() -> bool {
    println!("[bench] RFC 037 counted work, both axes, both paths (reported, not enforced)");
    let mut complete = true;
    let mut current: Option<Axis> = None;
    println!();
    println!("cluster path (dynamic, host build):");
    for (axis, family) in corpus() {
        if current != Some(axis) {
            current = Some(axis);
            println!();
            println!("{}", header());
        }
        match measure(family) {
            Ok(measured) => println!("{}", row(axis, family, &measured, &derived(family))),
            Err(error) => {
                complete = false;
                println!(
                    "{:<8} {:>4} {:>4} {:>5.2}  FAILED: {error}",
                    axis_name(axis),
                    family.n,
                    family.m,
                    family.off
                );
            }
        }
    }
    println!();
    println!("device path (const-generic instantiations, host build; footprint in bytes):");
    current = None;
    for (axis, family) in corpus() {
        if current != Some(axis) {
            current = Some(axis);
            println!();
            println!("{}", device_header());
        }
        match measure_device(family) {
            Ok(point) => println!("{}", device_row(axis, family, &point)),
            Err(error) => {
                complete = false;
                println!(
                    "{:<8} {:>4} {:>4} {:>5.2}  FAILED: {error}",
                    axis_name(axis),
                    family.n,
                    family.m,
                    family.off
                );
            }
        }
    }
    println!();
    println!("{}", legend());
    println!();
    print_truthfulness(&mut complete);
    complete
}

/// RFC 037 §5.3: how often `Converged` is truthful on the three legs, over the cluster
/// path (the device records are identical; see the device table). And the exact-deviation
/// measure the RFC asks for, which this command cannot yet produce: see the note it prints.
fn print_truthfulness(complete: &mut bool) {
    let mut points = Vec::new();
    for (_, family) in corpus() {
        match measure(family) {
            Ok(measured) => points.push(measured),
            Err(_) => *complete = false,
        }
    }
    let t = truthfulness(&points);
    println!("effectiveness: Converged truthfulness over the corpus (cluster path)");
    println!(
        "  Converged points: {}; truthful on all three legs: {}; failing a leg: {}; unverifiable from the record: {}",
        t.converged, t.truthful, t.failing, t.unverifiable
    );
    println!(
        "  legs: feasible within projection_tolerance (RFC 027 Am. 5); stationary, i.e. ConvergenceCriterion (RFC 029);"
    );
    println!("        uncapped final projection, settled by zero cap hits (RFC 033).");
    println!(
        "  deviation from the exact optimum: NOT REPORTED. The exact reference in conformance/reference.rs"
    );
    println!(
        "  handles separable (diagonal) Q only; the corpus family is tridiagonal. See the review request."
    );
}

#[cfg(test)]
mod tests {
    use super::{
        Axis, Family, a_row_major, conditioning_axis, corpus, derived, header, q_row_major, row,
        size_axis,
    };

    #[test]
    fn the_corpus_has_eleven_points_and_the_shared_point_appears_on_both_axes() {
        let all = corpus();
        assert_eq!(all.len(), 12, "7 size + 5 conditioning");
        let shared = Family {
            n: 32,
            m: 16,
            off: 0.5,
        };
        let on_size = size_axis().contains(&shared);
        let on_cond = conditioning_axis().iter().any(|f| f.off == 0.5);
        assert!(
            on_size && on_cond,
            "the off = 0.50, n = 32, m = 16 point must be on both axes"
        );
    }

    #[test]
    fn the_size_axis_holds_conditioning_fixed_and_the_conditioning_axis_holds_size_fixed() {
        assert!(size_axis().iter().all(|f| f.off == 0.5 && f.m == f.n / 2));
        assert!(conditioning_axis().iter().all(|f| f.n == 32 && f.m == 16));
    }

    #[test]
    fn every_corpus_size_has_a_device_instantiation() {
        for (_, family) in super::corpus() {
            assert!(
                super::DEVICE_SIZES.contains(&(family.n, family.m)),
                "n={}, m={} has no device instantiation",
                family.n,
                family.m
            );
        }
    }

    #[test]
    fn q_is_tridiagonal_symmetric_with_the_diagonal_two() {
        let n = 4;
        let q = q_row_major(n, 0.25);
        for i in 0..n {
            assert_eq!(q[i * n + i], 2.0);
            for j in 0..n {
                assert_eq!(q[i * n + j], q[j * n + i], "symmetry at {i},{j}");
                if i.abs_diff(j) > 1 {
                    assert_eq!(q[i * n + j], 0.0, "band at {i},{j}");
                }
            }
        }
        assert_eq!(q[1], 0.25, "Q[0][1] is the first off-diagonal entry");
    }

    #[test]
    fn each_row_of_a_caps_a_sliding_window_of_three() {
        let (n, m) = (5, 3);
        let a = a_row_major(n, m);
        for r in 0..m {
            let row = &a[r * n..(r + 1) * n];
            assert_eq!(row.iter().sum::<f64>(), 3.0, "row {r} has three ones");
            for k in 0..3 {
                assert_eq!(row[(r + k) % n], 1.0);
            }
        }
    }

    #[test]
    fn derived_figures_follow_the_stated_formulas() {
        let d = derived(Family {
            n: 32,
            m: 16,
            off: 0.5,
        });
        assert_eq!(d.q_multiply_adds_per_outer, 32 * 32);
        assert_eq!(d.projection_ops_upper_bound_per_outer, 500 * 16 * 32);
    }

    #[test]
    fn every_row_carries_both_labels_and_the_header_names_them() {
        let h = header();
        assert!(h.contains("[m]") && h.contains("[d]"), "{h}");
        let family = Family {
            n: 4,
            m: 2,
            off: 0.5,
        };
        let line = row(Axis::Size, family, &fixed_measurement(), &derived(family));
        assert!(line.starts_with("size"), "{line}");
        assert!(line.contains("converged"), "{line}");
    }

    fn fixed_measurement() -> super::Measured {
        super::Measured {
            outer_iterations: 30,
            status: "converged".to_owned(),
            termination: "ConvergenceCriterion".to_owned(),
            cap_hits: 0,
            violation: 0.0,
            infeasibility_evidence: false,
        }
    }

    #[test]
    fn a_stationary_feasible_capped_free_converged_point_is_truthful_on_all_three_legs() {
        let m = fixed_measurement();
        let l = super::legs(&m).expect("converged");
        assert_eq!(
            (l.feasible, l.stationary, l.uncapped),
            (super::Leg::Holds, super::Leg::Holds, super::Leg::Holds)
        );
    }

    #[test]
    fn a_converged_point_that_violates_the_projection_tolerance_fails_the_feasible_leg() {
        let mut m = fixed_measurement();
        m.violation = 2e-10;
        assert_eq!(
            super::legs(&m).expect("converged").feasible,
            super::Leg::Fails
        );
    }

    #[test]
    fn a_converged_point_that_stopped_on_the_iteration_cap_fails_the_stationary_leg() {
        let mut m = fixed_measurement();
        m.termination = "IterationCap".to_owned();
        assert_eq!(
            super::legs(&m).expect("converged").stationary,
            super::Leg::Fails
        );
    }

    #[test]
    fn nonzero_cap_hits_are_unverifiable_from_the_record_not_assumed_to_hold() {
        let mut m = fixed_measurement();
        m.cap_hits = 2;
        assert_eq!(
            super::legs(&m).expect("converged").uncapped,
            super::Leg::Unverifiable
        );
    }

    #[test]
    fn the_legs_are_conditions_of_converged_and_say_nothing_about_other_statuses() {
        let mut m = fixed_measurement();
        m.status = "not converged".to_owned();
        assert!(super::legs(&m).is_none());
    }

    #[test]
    fn the_summary_counts_failing_before_unverifiable_and_ignores_non_converged() {
        let good = fixed_measurement();
        let mut capped = fixed_measurement();
        capped.cap_hits = 1;
        let mut bad = fixed_measurement();
        bad.violation = 1.0;
        let mut open = fixed_measurement();
        open.status = "not converged".to_owned();
        let t = super::truthfulness([&good, &capped, &bad, &open]);
        assert_eq!(
            (t.converged, t.truthful, t.failing, t.unverifiable),
            (3, 1, 1, 1)
        );
    }
}
