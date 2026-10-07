# RFC 040 implementation handoff — A Device Budget Worth Enforcing

**To:** implementer (mid-capability model)
**From:** architect
**Authorized by.** The owner accepted RFC 040 on 2026-10-07. The RFC is
`rfcs/accepted/040-a-device-budget-worth-enforcing.md`; read it first. Architect review 091
§0 holds the scoping, including the architect's withdrawn 32 000-byte proposal and why.
**Base revision:** `main` at or after the RFC 040 acceptance commit.

**Two review requests.** **A** covers S1 (the reference instantiation and the advisory
measurement). **B** covers S2 (pin the baseline, enforce the delta, demote the rlib figure).
`cargo xtask release-gate` is required in both.

## 0. What the architect measured, including two dead ends

Do not re-walk these. Each was measured on `36ed7b1`, release,
`thumbv7em-none-eabihf`.

### 0.1 Three artifacts, one usable

| Artifact | Contents | Verdict |
|---|---|---|
| `libloeres_device.rlib` | 25 512 bytes: **`.rmeta` 23 845**, all other sections **193** | measures compiler metadata |
| a `staticlib` of a crate depending on it | **8 406 254 bytes**, dominated by `compiler_builtins` — `__aeabi_dadd`, `__adddf3`, `acos`, hundreds more | conflates the runtime with our code |
| the crate's **own object**, via `cargo rustc -- --emit=obj` | only our instantiated code | **use this** |

The rlib figure is 93% metadata because the device entry points are generic over `const N`
and `const M`, so a standalone build emits almost no instantiated code. That is why the
architect's 32 000-byte ceiling was withdrawn before anyone implemented it.

### 0.2 An instantiation that does not call the kernel measures nothing

The architect's first probe wrote a `#[no_mangle] extern "C"` wrapper whose body was only
`size_of::<ConstrainedProjectedWorkspace<f64, 8, 4>>()`. The object's sections:

```text
.text                                           0
.text.reference_solve                           8
.text._…rust_begin_unwind                       6
```

**Fourteen bytes** — the wrapper and a panic shim, and none of the solver, because `size_of`
is a compile-time constant. The reference instantiation must **call**
`solve_constrained_projected_first_order` with a real problem, or the measurement is of
nothing.

### 0.3 Sections are per-function, so sum by prefix

The target emits `.text.<symbol>` and `.rodata.<symbol>` sections, as the listing above
shows. An exact match on `.text` reads **0**. Sum every section whose name begins `.text` or
`.rodata`.

### 0.4 The import path

`ConstrainedProjectedWorkspace`, `ConstrainedSolveConfig`, `ConstrainedSolveReport` and
`solve_constrained_projected_first_order` are re-exported from
`loeres_device::solve` (`crates/loeres-device/src/solve.rs:23-26`), **not** from
`loeres_device::solve::constrained`, which is private. The architect's first attempt used the
latter and got `E0603`.

## 1. S1 — the reference instantiation and the measurement

### 1.1 The crate

A `no_std` crate, **excluded from the workspace** with its own lockfile, as the examples are.
It is a measurement fixture, not an example: say so in its own `README.md`, which RFC 038's
gate now requires for anything under `examples/` — so put it **outside** `examples/` unless
you also register it in RFC 038's registry and both tables. The architect's recommendation is
a sibling directory, so RFC 038's rules do not reach it and its purpose is not confused with
a worked problem. If you place it elsewhere, say where and why.

It must:

- declare `#![no_std]` and a `#[panic_handler]`;
- expose one `#[no_mangle] pub extern "C" fn` that **calls** the device kernel at a fixed
  `(N, M)` with a real problem, consuming the result so nothing is optimised away;
- declare `(N, M)`, the profile and the panic strategy as constants of the measurement.

### 1.2 The measurement in `size-budget`

Build the fixture for `thumbv7em-none-eabihf`, release, with `--emit=obj`; find the
fixture's object; sum `.text*` and `.rodata*` with `size -A`.

Report it in RFC 010 §3.7's vocabulary. In S1 it is **advisory**. If the tool or the object is
missing, the measurement is **unavailable**, and RFC 010 §3.7 says an unavailable required
measurement **fails** the command — it must not pass silently. `size`, `llvm-size`, `nm`,
`llvm-nm` and `objdump` are all present on the reviewed environment; pick one and name it in
the report, as RFC 019's candidate gate names its expected tools.

Print `(N, M)`, the profile, the panic strategy and the tool beside the figure. A different
instantiation is a different number and the report must make that impossible to miss.

## 2. S2 — pin it, and bound the delta

- Pin the measured `.text` and `.rodata` as a `const` baseline.
- Fail when either moves by more than a declared fraction. **Set the fraction from S1's
  figures and your judgement, and justify it in the request** — RFC 040 §2.3 deliberately
  states no number, because the architect froze one ahead of the measurement once and was
  wrong by two orders of magnitude. Propose; the architect rules.
- The remedy on failure, in the module doc: re-measure, understand the change, and update the
  pin in the same commit as the change that moved it — the `bench-baseline` workflow.
- Promote these two figures to **enforced**.
- **Demote the rlib figure** to advisory with its composition stated — 23 845 of 25 512 bytes
  are `.rmeta` — so nobody mistakes it for a device budget again. Keep it; do not delete it.
- Leave the type-size assertions untouched.

After S2, `size-budget` must no longer print "threshold pending owner RFC" for the enforced
figures. It may still print something equivalent for RFC 010 §3.7's two unimplemented items
(stack sensitivity beyond the existing type assertions, and cluster monomorphization growth),
which stay out of scope.

## 3. Non-scope

- **No kernel change**, no public API change, no new dependency in any published crate.
- No absolute byte ceiling.
- No threshold on the rlib file size, ever.
- No linker script and no flashable image. The object-level measurement is chosen so that no
  linking is involved and **no hardware claim is made**: nothing in this repository runs on
  hardware, and RFC 040 does not change that. Do not describe the figure as what runs on a
  device — it is the code the compiler emits for one instantiation.
- Not RFC 010 §3.7's stack-sensitivity or monomorphization-growth items.
- No `#[allow(…)]`. Do not tag; do not publish.

## 4. Required evidence

```text
cargo fmt --all -- --check          # run last, immediately before committing
cargo clippy --workspace --all-features --all-targets -- -D warnings
cargo test --workspace --all-features
cargo +1.85 check --workspace --all-features
cargo xtask check                   # 20 gates
cargo xtask size-budget             # standalone
cargo xtask release-gate --intended-tag 0.22.3      # must PASS
```

State in each request:

1. **Examples affected** — expected `none`; the fixture is not an example (§1.1).
2. For S1: the measured `.text` and `.rodata`, with `(N, M)`, the profile, the panic strategy
   and the tool. Confirm the figure is **not** 14 bytes — if it is, the kernel is not being
   called (§0.2).
3. For S1: that the measurement fails as `unavailable` when the tool is absent — break it
   once, by pointing at a tool name that does not exist, and record the failure.
4. For S2: the proposed fraction and its justification, and the break-and-restore record
   showing the delta gate failing when the pin is moved.
5. Whether the fixture went outside `examples/` as recommended, and if not, how RFC 038's
   registry and both tables were satisfied.
6. Anything in this handoff that does not match the tree.
