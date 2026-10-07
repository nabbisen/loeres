# Batch projected first-order solve on the server path.

This is an **API artifact**, not a worked problem: it demonstrates running
several dynamic box-constrained problems through `solve_batch` under
`parallel-rayon` and reading per-item outcomes, rather than modelling a
problem a domain reader would recognise. The documentation convention keeps
this form for the three conformance examples; it is not the standard for new
examples.

Run:

```text
cargo run --manifest-path examples/cluster-batch-solve/Cargo.toml
```

See also: the root [`README`](../../README.md) and the
[cluster user guide](../../docs/src/cluster-user-guide.md).
