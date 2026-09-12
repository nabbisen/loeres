# Verification & Evidence

What the repository checks, how strongly, and what each result is allowed to
mean. This page introduces no claim of its own: every statement here summarizes a
gate that already exists, and each gate's own RFC is the normative source.

For how to run any of this locally, see [Local Development](development.md).

## Evidence classes

The distinction that makes the rest of the page readable:

| Class | What a green result means |
|---|---|
| **Enforced** | the assertion held, and a violation fails the gate |
| **Advisory / reporting** | a measurement was recorded; there is no threshold to fail |
| **Unavailable** | the gate could not run its subject, and says so — **never promoted to a pass** |

`unavailable` is not a soft pass, and the reverse is also true: it is not always
tolerated. Whether an absence is acceptable depends on whether it is expected by
design.

- A missing **maintainer-held corpus** — the architecture-review documents behind
  `review-evidence` — is legitimately absent from a clean extraction, so the gate
  reports `unavailable` and continues.
- A missing **tool** is an environment defect, so `supply-chain` reports
  `unavailable` **and fails** when `cargo-deny` is not runnable.

That asymmetry is deliberate. A gate that cannot tell the two apart either blocks
on a file it was never meant to ship or passes with nothing checked.

## The gate set

`cargo xtask check` is the developer aggregate. Each line names the gate and what
it actually asserts.

| Gate | Class | Asserts |
|---|---|---|
| `host-check` | Enforced | the workspace checks and tests on the host toolchain |
| `zero-bleed` | Enforced | no forbidden server↔edge dependency edge exists |
| `no-std` | Enforced | the edge crates build for `thumbv7em-none-eabihf` with no `std` and no `alloc` |
| `feature-matrix` | Enforced | the declared feature combinations build |
| `target-profiles` | Enforced (per profile class) | manifest-driven target builds; see the profile classes below |
| `check-rfcs` | Enforced | RFC folder/status agreement, index integrity, links, and no amendment inside a shipped RFC |
| `doc-currency` | Enforced | apex metadata, implemented scope, navigation, and a stale-phrase ledger |
| `review-evidence` | Enforced + corpus-dependent | every review citation resolves to a hash-pinned row; hash, coverage, and row-count checks when the corpus is present |
| `check-public-api` | Enforced | the public surface matches its recorded shape |
| `panic-audit` | Enforced | the `no_std` production crates carry no `unwrap`, `expect`, `panic!`, `todo!`, or `unimplemented!` |
| `size-budget` | **Advisory** | records size measurements against a baseline; no threshold |
| `unsafe-audit` | Enforced | the core forbids `unsafe` |
| `supply-chain` | Enforced (fails when unavailable) | RustSec advisories, an exhaustive license allow-list, duplicate/wildcard bans, crates.io-only sources, and zero external dependencies per edge crate |
| `examples` | Enforced | each example builds under its declared feature set and its **resolved** graph carries no forbidden crate |
| `conformance` | Enforced | the smoke corpus, comparing the real device and cluster solvers |
| `link-audit` | Enforced | every relative Markdown link resolves |

`target-profiles` is manifest-driven and its profiles carry their own classes:
**mandatory** profiles fail the aggregate on a missing target or failed command,
**advisory-installed** profiles report `unavailable` when an optional target is
not installed, and **documented-only** profiles are listed without compilation. A
profile appearing in the manifest does not mean it was executed.

## Claims that are deliberately kept apart

Two pairs of adjacent-sounding results that mean different things.

**Dependency reachability is not bare-metal buildability.** The `examples` gate
proves a device integrator reaches no server-side crate — read from the
example's resolved dependency graph, against its own lockfile, because the
examples are excluded from the workspace so that graph cannot be an artifact of
feature unification with a sibling. It says nothing about whether the example
would link on a microcontroller. That claim is `no-std`'s, against
`thumbv7em-none-eabihf`. An example is a host program and its own `main` may use
`std`; the edge crates remain `#![no_std]` with no `alloc` regardless.

**Panic-averse is not panic-free.** `panic-audit` scans for panicking
constructs, the core forbids `unsafe`, and the public solve paths use fallible
access rather than indexing. That is panic-averse evidence. No entrypoint is
documented as formally proven panic-free, and no such claim may be inferred from
a passing gate.

## Review evidence is evidence, not authority

Normative documents cite architecture reviews by number, and
`rfcs/review-evidence-index.md` pins each one's identity by SHA-256 so a citation
resolves to a named artifact rather than an unreachable file.

A review is evidence that a decision was considered — never the source of its
authority. Design authority rests with the architect role and approval with the
project owner. Each row records the **author tier** it was requested under, from
a closed set, captured at request time and never reconstructed from a filename or
from a review's own self-description.

## The release candidate gate

`cargo xtask release-gate` is not an everyday alias. On a clean tracked revision
it runs the ordered source suite, builds a tracked-input archive, validates its
paths, types, exact file set, and SHA-256, then repeats the applicable suite
inside a gate-owned clean extraction — so the gates are proven against what a
consumer would actually receive, not only against the working tree.

A passing local dry run is not tagged-release evidence, and neither mode
publishes crates or creates a release. Tagged CI additionally proves that the
peeled canonical tag equals `HEAD`.

## What none of this establishes

Read [Terms of Engineering Use](https://github.com/nabbisen/loeres/blob/main/TERMS_OF_USE.md)
before relying on any of the above. No artifact in this repository — no test
result, gate output, release evidence bundle, or conformance fixture —
constitutes evidence of compliance with any functional-safety or medical-device
standard. Producing certification evidence is the integrator's responsibility.
