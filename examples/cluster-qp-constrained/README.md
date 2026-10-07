# A constrained quadratic program on the server path.

This is an **API artifact**, not a worked problem: it demonstrates how to
implement the `loeres::problem` contract over dynamic dense storage and solve
it through the typed cluster entrypoint, reading the terminal constraint
violation beside the status, rather than modelling a problem a domain reader
would recognise. The documentation convention keeps this form for the three
conformance examples; it is not the standard for new examples.

Run:

```text
cargo run --manifest-path examples/cluster-qp-constrained/Cargo.toml
```

See also: the root [`README`](../../README.md) and the
[cluster user guide](../../docs/src/cluster-user-guide.md).
