# loeres

Shared mathematical contracts for the Loeres family: stratified scalars,
storage-agnostic vector/matrix access, solver outcome/status and validation
vocabulary, dimensions, and allocation-free errors/diagnostics. Defines **no**
storage, runtime, or OS assumptions. The `problem` namespace is reserved and
ships no generic public LP/QP/SOCP/problem-family contract.

- **Environment:** `#![no_std]`, no `alloc`
- **Depends on:** nothing (defines contracts only)
- **Status:** Active. Implemented core surfaces: `scalar` (RFC 001, six capability
  tiers), `access` / `dimension` (RFC 002, storage-agnostic vector/matrix
  contracts + views), `error` / `diagnostic` (RFC 003, allocation-free error
  topology), `solver` (RFC 014, outcome/status taxonomy), and `validation`
  (RFC 012, the validation-state vocabulary). `problem` remains a reserved
  empty namespace pending a separately accepted generic-family RFC.

See the workspace [README](../../README.md), the [architecture](../../docs/src/architecture.md)
chapter, and the [RFC index](../../rfcs/README.md).

Licensed under Apache-2.0.
