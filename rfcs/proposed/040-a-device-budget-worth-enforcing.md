# RFC 040 - A Device Budget Worth Enforcing

**Status.** Proposed (2026-10-07).

**Author tier.** `architect`.

**Governing scoping.** Architect review 091 §0, authorised by the owner on 2026-10-07. Every
figure below was measured by the architect on `36ed7b1`.

## 1. Summary

`cargo xtask size-budget` has reported the device artifact as an advisory baseline since
RFC 010, with the message "threshold pending owner RFC". RFC 010 §3.7 says what is owed:

> The exact byte budgets are owned by **RFC 003, RFC 006, RFC 008, and RFC 011**. This RFC
> defines the existence and common reporting format of the checker.

and lists what the command "must measure and compare" over time: `.text` size, `.rodata`
size, stack-sensitive type sizes, device artifact binary size, and cluster monomorphization
growth. **`.text` and `.rodata` are measured nowhere.** The command reports the rlib file
size and the type sizes.

The architect proposed freezing a threshold on that rlib file and was wrong to. Measured on
`libloeres_device.rlib`, release, `thumbv7em-none-eabihf`:

| Component | Bytes |
|---|---:|
| `.rmeta` — compiler metadata | **23 845** |
| every other section, summed | **193** |
| rlib file | 25 512 |

**Ninety-three per cent is metadata and 193 bytes are sections.** The cause is structural:
the device entry points are generic over `const N` and `const M`, so a standalone build
emits almost no instantiated code. A threshold there would track `.rmeta` growth, and would
have been frozen into an RFC as a device budget.

This RFC measures the thing that reaches a device instead.

## 2. Design

### 2.1 A reference instantiation

Generic code has no size until it is instantiated, so the measurement needs one concrete
instantiation to measure. A small `no_std` crate — excluded from the workspace, like the
examples — **calls** `solve_constrained_projected_first_order` at one fixed `(N, M)` through
a `#[no_mangle] extern "C"` wrapper, so the monomorphisation is emitted and reachable.

It must make a **real call**. The architect's probe instantiated only
`size_of::<ConstrainedProjectedWorkspace<f64, 8, 4>>()` and the resulting object held
**14 bytes** of `.text` — the wrapper and a panic shim, and none of the solver, because
`size_of` is a compile-time constant. A reference instantiation that does not call the
kernel measures nothing.

`(N, M)` is a declared constant of the measurement, recorded beside the figure. A different
instantiation is a different number, and the report must make that impossible to miss.

### 2.2 Measure the crate's own object, not an archive

Three artifacts were measured; only one isolates our code.

| Artifact | What it contains | Verdict |
|---|---|---|
| the device **rlib** | 93% `.rmeta`, 193 bytes of sections | measures metadata |
| a **staticlib** | **8.4 MB**, dominated by `compiler_builtins` — every `__aeabi_*`, `acos`, `__adddf3` | conflates the runtime with our code |
| the crate's own **object** (`cargo rustc -- --emit=obj`) | only our instantiated code | **this one** |

`size -A` on the object, summing `.text*` and `.rodata*` sections, is the measurement.
`size`, `llvm-size`, `nm`, `llvm-nm` and `objdump` are all present on the reviewed
environment, and RFC 019's candidate gate already declares its expected tool set — the
measurement must fail as `unavailable` in RFC 010 §3.7's sense if the tool is missing, never
pass silently.

Sections are summed by prefix because the target emits per-function sections
(`.text.reference_solve` and so on), so an exact-name match would read zero.

### 2.3 A pinned baseline and a bounded delta, not an absolute ceiling

The threshold is **not** an absolute byte ceiling. An absolute number invites a guess at how
much headroom is reasonable, and the architect's 32 000 was exactly that guess.

Instead, in the pattern `bench-baseline` already establishes for counted work: pin the
measured `.text` and `.rodata` of the reference instantiation, and fail when either moves by
more than a declared fraction. A change of kind becomes visible; routine drift does not
cost a review. Re-measuring and updating the pin in the same commit as the change that moved
it is the intended workflow, and the module doc must say so.

The fraction is a declared constant of this RFC and is set with the first measurement, not
guessed in advance — S1 reports the figure, S2 pins it. **This RFC deliberately states no
number**, because the architect has already once frozen a number ahead of the measurement
and it was wrong by two orders of magnitude.

### 2.4 What `size-budget` reports afterwards

`.text` and `.rodata` of the reference instantiation become **enforced** in RFC 010 §3.7's
three-class vocabulary. The rlib file size stays, demoted to what it is: an advisory figure
whose composition is stated, so nobody mistakes it for a device budget again. The type-size
assertions are untouched.

RFC 010 §3.7's two remaining items — stack-sensitive sizes beyond the existing type
assertions, and cluster monomorphization growth — stay unimplemented and are explicitly out
of scope here.

## 3. Slices

| Slice | Content |
|---|---|
| **S1** | The reference instantiation crate and the object-level measurement, reported advisorily, with `(N, M)` and the tool recorded. No threshold yet. |
| **S2** | Pin the baseline and enforce the bounded delta; set the fraction from S1's figures. Demote the rlib figure and state its composition. |

## 4. Explicit non-scope

- **No kernel change**, no public API change, no new dependency in any published crate.
- No absolute byte ceiling (§2.3).
- No threshold on the rlib file size, ever — §1 is why.
- Not RFC 010 §3.7's stack-sensitivity or monomorphization-growth items.
- No linker script and no flashable image: the object-level measurement is chosen precisely
  so that no linking, and no hardware claim, is involved. Nothing in this repository runs on
  hardware and this RFC does not change that.

## 5. Risks

| Risk | Mitigation |
|---|---|
| The figure depends on the chosen `(N, M)` | It is a declared constant, recorded beside every report, and a different instantiation is documented as a different number. |
| A toolchain change moves `.text` with no source change | That is the same floating-`stable` exposure `docs/src/development.md` already records for clippy. The bounded delta absorbs ordinary drift; a jump is meant to be looked at. |
| The reference instantiation drifts from how anyone uses the library | It is a measurement fixture, not an example, and must say so in its own README under RFC 038's rules. |
| `-O` level or `panic` strategy changes the figure | Both are declared with the measurement, as the host and toolchain are for RFC 037's wall-clock figures. |

## 6. Exit criteria

1. A reference instantiation exists that **calls** the device kernel at a declared `(N, M)`.
2. `size-budget` reports `.text` and `.rodata` for it, measured on the crate's own object,
   failing as `unavailable` rather than passing if the tool is absent.
3. The baseline is pinned and a declared delta is enforced; the fraction is set from a
   measurement, not guessed.
4. The rlib figure remains, labelled advisory, with its `.rmeta` composition stated.
5. `cargo xtask size-budget` no longer reports "threshold pending owner RFC" for the
   enforced figures.
6. `cargo xtask check` passes; `cargo xtask release-gate` passes in every slice.
