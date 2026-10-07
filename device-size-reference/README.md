# `device-size-reference`

This is a **measurement fixture**, not an example. It exists only so
`cargo xtask size-budget` has one concrete instantiation of the device-constrained
kernel to measure: generic code has no size until it is instantiated, and this crate's
`reference_solve` calls `solve_constrained_projected_first_order` at a fixed
`(N, M) = (8, 4)` with a genuine runtime input, so the monomorphised solver code is
actually emitted (RFC 040 §2.1, §0.2 — an instantiation that only computes
`size_of::<Workspace>()` measures nothing, because that is a compile-time constant).

It is deliberately **outside** `examples/`, so RFC 038's per-example README and
registry rules do not apply to it: it says nothing about a problem a domain reader
would recognise, because it is not one.

Build and measure it as `cargo xtask size-budget` does:

```sh
cargo rustc --locked --manifest-path device-size-reference/Cargo.toml --release \
  --target thumbv7em-none-eabihf -- --emit=obj
size -A target/thumbv7em-none-eabihf/release/deps/device_size_reference-*.o
```

`(N, M)`, the release profile and `panic = "abort"` (declared in this crate's own
`Cargo.toml`) are constants of the measurement: a different instantiation is a
different number (RFC 040 §2.1).
