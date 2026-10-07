# loeres-backend-static

Fixed-size, allocation-free storage and borrowed views over caller-owned memory for the edge.

- **Environment:** `#![no_std]`, no `alloc`
- **Depends on:** `loeres`
- **Status:** Active through v0.20.0. RFC 004 implements fixed-size storage and
  contiguous views; RFC 005 implements workspace footprint support.

## Public surface

- `array` behind `owned-arrays`: `FixedVector` and `FixedMatrix` with static
  dimension invariants and RFC 002 access/contiguous traits.
- `view`: baseline contiguous borrowed vector/matrix views over caller memory.
- `dimension`: static dimension descriptors.
- `workspace`: the allocation-free `WorkspaceFootprint` contract.

Advanced strided/row/column/sub-matrix `static-views` and richer
`diagnostic-snapshot` behavior remain deferred. No feature enables `std`,
`alloc`, server storage, async, logging, or FFI.

## Features

Off unless marked *on*. A feature marked *reserved; no effect yet* is declared but
gates no code today: enabling it changes nothing.

| Feature | Default | What it does |
|---|---|---|
| `owned-arrays` | off | owned fixed-size wrappers around arrays; no heap allocation |
| `static-views` | off | reserved; no effect yet |
| `diagnostic-snapshot` | off | reserved; no effect yet |

See the workspace [README](https://github.com/nabbisen/loeres), the [architecture](https://github.com/nabbisen/loeres/blob/main/docs/src/architecture.md)
chapter, and the [RFC index](https://github.com/nabbisen/loeres/blob/main/rfcs/README.md).

Licensed under Apache-2.0.
