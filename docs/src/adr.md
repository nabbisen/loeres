# Architecture Decision Records

The accepted architectural decisions are recorded in the requirements
specification, **§15 (Architectural Decisions)** and the requirement
traceability appendix, and are refined by the RFC sequence under `rfcs/`.

Foundational decisions include: Loeres is a library *family*, not one unified
solver (ADR-001); server and edge models are separated by **crates**, not a
runtime switch (ADR-002, ADR-003); `loeres-core` is `no_std` / no-`alloc`
(ADR-004); edge crates never depend on server crates (ADR-005); scalar
capabilities are **stratified** rather than one broad trait (ADR-011, refined to
six tiers by RFC 001); core/device baselines reject runtime trait objects
(ADR-012); typed caller-owned workspaces are the primary device model
(ADR-013); `no_panic`-style tooling is a release gate, not a proof (ADR-014);
and floating-point determinism is target-profile-scoped (ADR-015).

Subsequent RFCs that change the public boundary must record their own decision
and a compatibility/safety analysis (see roadmap §1.5, requirements Appendix C).
