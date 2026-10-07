# Loeres

Loeres is a Rust workspace of optimization kernels for constrained quadratic programs (`min ½xᵀQx + cᵀx`), with a hard compile-time boundary between two execution environments.

- **Two paths, one contract.** A cluster path with dynamic sizes, heap, threads and batching; a device path that is `no_std`, has no `alloc`, bounds its iteration and takes caller-owned workspaces.
- **Box and linear-inequality QPs.** Projected first-order solves over a box, and over a box with `Ax ≤ b` (RFC 027).
- **Honest status.** Non-convergence is a status returned in `Ok`; rejected input is an error. Infeasibility is not a status: a heuristic `infeasibility_evidence` field is reported beside it.
- **Checked, not asserted.** A conformance corpus and a randomized differential test against exact answers gate `cargo xtask check`.

## Quick Start

Build and check from source. The toolchain, components and bare-metal target come from `rust-toolchain.toml`.

```sh
cargo check --workspace --all-features
cargo xtask check        # developer aggregate; not release approval
cargo xtask examples     # builds and runs each example, checks its dependency isolation
```

A device-side control step, copied from [`examples/device-mpc-step`](examples/device-mpc-step/src/main.rs). Run the whole example with `cargo run --manifest-path examples/device-mpc-step/Cargo.toml`:

```rust
/// One sample: plan from the measured state and return the first move, or
/// `None` if the plan did not converge, in which case no move is trusted.
fn plan_move(
    state: f64,
    workspace: &mut ProjectedFirstOrderWorkspace<f64, N>,
    config: &DeviceSolveConfig<f64>,
) -> Result<Option<f64>, SolverError> {
    let plan = MpcPlan::new(state);
    let mut moves = FixedVector::from_array([0.0; N]);
    let report = solve_projected_first_order(&plan, &mut moves, workspace, config)?;
    match report.status() {
        SolveStatus::Converged => Ok(Some(moves.get(0)?)),
        // `SolveStatus` is `#[non_exhaustive]` downstream: every status other
        // than `Converged` is treated as untrusted, so a future variant is held.
        _ => Ok(None),
    }
}
```

The problem type `MpcPlan` and the `main` that calls this function are in that file.

Choose the crates by where the solve runs:

| You are building | Use | Storage | Threads |
|---|---|---|---|
| an embedded controller with bounded time | `loeres-device` + `loeres-backend-static` | fixed-size, caller-owned | none |
| a server, batch or scheduler | `loeres-cluster` + `loeres-backend-std` | heap, dynamic sizes | optional (`parallel-rayon`) |

The dependency snippets for both:

```toml
# Cluster / server user
loeres-cluster        = { version = "0.x", features = ["parallel-rayon"] }
loeres-backend-std    = { version = "0.x", features = ["dense"] }

# Device / edge user
loeres-device         = { version = "0.x", default-features = false }
loeres-backend-static = { version = "0.x", default-features = false, features = ["owned-arrays"] }
```

## Reading a result

A solve returns `Ok` with a status even when it did not converge. Read the status first, then the figures:

- `Converged` means feasible and stationary at the final iterate, with a projection that did not hit its cap (RFC 027 Amendment 5, RFC 029, RFC 033).
- `NotConverged` is a result to handle, not an exception. Check the termination reason and the constraint violation before acting on the iterate.

## Examples

Each example names the problem it solves in its first paragraph.

| Example | Problem | Path |
|---|---|---|
| [`device-mpc-step`](examples/device-mpc-step/) | model-predictive control of a process with an actuator limit | device |
| [`device-box-pfo`](examples/device-box-pfo/) | fixed-size box-constrained solve with a reused workspace | device |
| [`cluster-capacity-dispatch`](examples/cluster-capacity-dispatch/) | production units dispatched against a demand floor | cluster |
| [`cluster-qp-constrained`](examples/cluster-qp-constrained/) | constrained QP, status beside the constraint violation | cluster |
| [`cluster-batch-solve`](examples/cluster-batch-solve/) | batch of dynamic box problems with per-item outcomes | cluster |
| [`cluster-counted-work`](examples/cluster-counted-work/) | counted work of a constrained solve, against size and conditioning | cluster |

## Design Notes

- **Five crates, one contract.** `loeres` (`no_std`, no `alloc`) defines the scalar, vector, solver-outcome, validation, error and problem contracts. `loeres-backend-std` and `loeres-backend-static` own storage; `loeres-cluster` and `loeres-device` own the solve paths. The dependency graph is acyclic and environment-separated.
- **Narrow solver scope.** Box and `Ax ≤ b` QPs are solved. LP is expressible as `Q = 0` and solves soundly — a converged result is always optimal — but without a convergence guarantee: the projection's sweep cap or, for a wide box and a small step scale, the outer iteration cap (zero projection cap hits in that case) can bind first; no SOCP contract exists. `Q` must be symmetric positive semidefinite, a caller precondition that is not verified. Device and cluster agree within tolerance, not bitwise. The full statement is in [Terms of Engineering Use](TERMS_OF_USE.md).
- **Caller-owned workspaces on device.** No hidden allocation; the memory footprint is reviewable before execution.
- **Bounded server integrations.** Observability is metadata-only, the gateway is mock-only, and validation caching is process-local. No concrete native adapter or persistent or distributed cache ships.

## More Detail

- Start here: [Getting Started](docs/src/getting-started.md) — install, solve one problem, read the result.
- Book: [`docs/src/`](docs/src/) — introduction, architecture, threat model, and the maintainer bridge to the specifications (mdBook).
- Specifications: [`docs/specs/`](docs/specs/) — requirements, external design, roadmap and milestones.
- RFCs: the [RFC index](rfcs/README.md) lists every RFC by state.
- Release currency: [release currency](docs/src/specifications.md#release-currency) — which release carries which RFCs, and the publication status.
- Contributing: [`CONTRIBUTING.md`](CONTRIBUTING.md), and the [documentation convention](docs/src/documentation-convention.md) that governs this file and the examples.
- Roadmap and status: [`ROADMAP.md`](ROADMAP.md).

## License

Licensed under the Apache License, Version 2.0. See [`LICENSE`](LICENSE) and [`NOTICE`](NOTICE).
