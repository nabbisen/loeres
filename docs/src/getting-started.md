# Getting Started

This chapter is a tutorial. It takes you from a clean checkout to one solved
problem and shows how to read the result. It does not describe the API; the
[Device User Guide](device-user-guide.md) and the
[Cluster User Guide](cluster-user-guide.md) do that.

## 1. Install

You need `git` and [rustup](https://rustup.rs/). The repository pins its
toolchain in `rust-toolchain.toml` (stable, with `rustfmt`, `clippy` and the
`thumbv7em-none-eabihf` target), so `rustup` installs the right one the first
time you run `cargo` inside the checkout.

```sh
git clone https://github.com/nabbisen/loeres.git
cd loeres
cargo check --workspace --all-features
```

The first build downloads the toolchain and compiles the workspace. A clean
`cargo check` means the install worked.

## 2. Solve one problem

The example `device-mpc-step` plans the moves of a process that must track a
setpoint of `1.0`. The actuator saturates at `±1`, and every move costs energy.
At each sample it plans three moves ahead, applies only the first, and plans
again from the new state. It does this for eight samples.

```sh
cargo run --manifest-path examples/device-mpc-step/Cargo.toml
```

The run prints:

```text
step    state     move at limit  outcome
   0   0.0000   1.0000      yes  applied
   1   0.5000   0.8724       no  applied
   2   0.8862   0.3463       no  applied
   3   0.9707   0.2312       no  applied
   4   0.9892   0.2060       no  applied
   5   0.9933   0.2004       no  applied
   6   0.9942   0.1992       no  applied
   7   0.9944   0.1990       no  applied

after 8 samples: state 0.9944, setpoint 1
```

## 3. Read the result

Each row is one sample.

- **step** is the sample index, from `0`.
- **state** is the process state *before* the move.
- **move** is the first planned input, the one applied. The actuator accepts
  `-1` to `1`.
- **at limit** is `yes` when the move sits on the actuator's bound. The
  constraint is then doing the work, not the tracking.
- **outcome** is `applied` when the plan converged. `not converged` would mean
  the plan was not solved to tolerance, so the example applies zero rather than
  trust it. No row does that here.

Read down the columns. At step `0` the state is `0`, far below the setpoint, so
the plan wants the largest move it can make: the move is `1.0000`, on the limit.
From step `1` the state is close enough that the plan asks for less, and the
move falls away from the limit.

The final state, `0.9944`, is near the setpoint but not on it. That is the
model, not a solver error: every move costs something, so the plan accepts a
small offset to keep moves small.

## 4. Change one thing and run it again

Open `examples/device-mpc-step/src/main.rs` and find

```rust
const SETPOINT: f64 = 1.0;
```

Change it to `0.5`, then run the example again:

```sh
cargo run --manifest-path examples/device-mpc-step/Cargo.toml
```

The first move is now `0.7767`, below the limit, so the `at limit` column reads
`no` from the start. The state settles at `0.4972`, again a little under the
setpoint for the same reason as before. Put the setpoint back to `1.0` when you
are done.

## 5. Find the solve

The part that does the work is `plan_move`, in the same file. Read it with the
example's output in mind:

- it builds the plan from the measured state;
- it solves into a workspace that the caller owns and reuses for every sample.
  The device crates have no `alloc`, so the solve itself cannot allocate;
- it returns a move only when the status is `Converged`.

The status is the reason to read the code rather than just the numbers. A
solve can return `Ok` and still not have converged; the example refuses to act
on such a result, and so should yours.

## 6. Next steps

- [Device User Guide](device-user-guide.md): the device path in full.
- [Cluster User Guide](cluster-user-guide.md): the server path, batching and
  cancellation.
- [Verification & Evidence](verification.md): what the gates check.
- [Documentation Convention](documentation-convention.md): if you are adding an
  example or changing the documentation.
