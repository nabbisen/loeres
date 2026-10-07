# Model-predictive control of a first-order process with an actuator limit.

A heater, a valve or a motor drives a process that must track a setpoint, but
the actuator saturates and every move costs energy. Model-predictive control
plans a short sequence of future moves, applies only the first, and plans
again at the next sample. Each plan is a small box-constrained quadratic
programme, solved on the device with a caller-owned workspace.

Run:

```text
cargo run --manifest-path examples/device-mpc-step/Cargo.toml
```

See also: the root [`README`](../../README.md) and the
[device user guide](../../docs/src/device-user-guide.md).
