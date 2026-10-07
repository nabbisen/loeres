# Examples

Each example names the problem it solves in its first paragraph. The three
API artifacts are marked as such; their form is not the standard for new
examples (`docs/src/documentation-convention.md` §3).

| Example | Problem | Category | Path |
|---|---|---|---|
| [`device-mpc-step`](device-mpc-step/) | model-predictive control of a process with an actuator limit | worked problem | `device-mpc-step/` |
| [`device-box-pfo`](device-box-pfo/) | fixed-size box-constrained solve with a reused workspace | API artifact | `device-box-pfo/` |
| [`cluster-capacity-dispatch`](cluster-capacity-dispatch/) | production units dispatched against a demand floor | worked problem | `cluster-capacity-dispatch/` |
| [`cluster-qp-constrained`](cluster-qp-constrained/) | constrained QP, status beside the constraint violation | API artifact | `cluster-qp-constrained/` |
| [`cluster-batch-solve`](cluster-batch-solve/) | batch of dynamic box problems with per-item outcomes | API artifact | `cluster-batch-solve/` |
| [`cluster-counted-work`](cluster-counted-work/) | counted work of a constrained solve, against size and conditioning | worked problem | `cluster-counted-work/` |

See the root [`README`](../README.md) for the Quick Start and the library overview.
