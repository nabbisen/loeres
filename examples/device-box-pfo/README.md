# Fixed-size box-constrained projected first-order solve on the edge path.

This is an **API artifact**, not a worked problem: it demonstrates how to call
the device-side solver (a caller-owned typed workspace, workspace reuse across
calls, non-convergence surfaced as an `Ok` status) and how to establish
dependency reachability for the edge path, rather than modelling a problem a
domain reader would recognise. The documentation convention keeps this form
for the three conformance examples; it is not the standard for new examples.

Run:

```text
cargo run --manifest-path examples/device-box-pfo/Cargo.toml
```

See also: the root [`README`](../../README.md) and the
[device user guide](../../docs/src/device-user-guide.md).
