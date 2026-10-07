# How much work does a constrained solve take, as the problem grows and as it gets harder?

A planner who builds a constrained model wants to know two things before
committing to it: how the solve's work grows with the number of variables,
and how it grows when the variables are more tightly coupled. This program
answers both on a family of problems whose difficulty is set by one number,
and prints the work each solve did.

Run:

```text
cargo run --manifest-path examples/cluster-counted-work/Cargo.toml
```

See also: the root [`README`](../../README.md) and the
[cluster user guide](../../docs/src/cluster-user-guide.md).
