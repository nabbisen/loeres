# Security Policy

Loeres is a mathematical-optimization library family with an explicit
server/edge threat model (see `docs/src/threat-model.md`, requirements §8, and
external design §5).

## Reporting a vulnerability

**Don't open a public issue.**    
Please report suspected vulnerabilities privately via GitHub's
**"Report a vulnerability"** (Security Advisories) on the repository. Include affected crate(s), version/commit, and a minimal
reproduction where possible.

## Scope notes

- Edge crates (`loeres`, `loeres-backend-static`, `loeres-device`) are
  `no_std` / no-`alloc` and panic-averse by design; reports of `std`/`alloc`
  leakage, hidden allocation, panic paths, or dependency bleed are in scope.
- FFI is restricted to `loeres-cluster` behind an audited, default-off feature.
