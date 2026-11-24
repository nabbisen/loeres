# RFC 055 - No Public Module That Declares Nothing

**Status.** Implemented (v0.25.0). Amendment 1 was made while Accepted, under RFC 000's in-place-amendment rule.

**Author tier.** `architect`.

**Governing scoping.** The owner's standing rule — *"I prefer to remove something unused to
`#[allow(dead_code)]`. Dead code can harm as technical debt in the future"* — applied to dead code
that is **published**, together with RFC 047 property **D** (*scope stated in the user's
vocabulary*) and property **B** (*no claim without a gate*). Found while writing RFC 054's handoff;
the owner authorized scoping it on 2026-10-31. Every figure was measured on `1acb10e`, workspace
`0.24.3`.

This RFC removes public API, so `docs/specs/loeres-roadmap-milestones-v1.md` §1.5 requires *"a
superseding RFC with an explicit compatibility and safety analysis"*. That analysis is **§3**, and
it is a required section rather than a courtesy.

## 0.1 Amendment 1 (2026-10-31)

Made while Accepted, under RFC 000's in-place-amendment rule, and recorded in the implementation
handoff §0.2. The Status line carries only the design-freeze date while this RFC is in
`accepted/`, because `doc-currency` requires that form there; **this amendment must be named in
the Status line when the file moves to `rfcs/done/`**.

§8 promised that two normative sentences would be decided *"with the sentence quoted in front of
it"* rather than from memory. Both were, and **both survive** — S2's instruction to delete them is
superseded by this section. The pattern is the one RFC 048 and RFC 053 S2 established in the other
direction: **remove the dead reference, keep the live constraint.**

### `loeres-external-design-v1.md:295` — *"Adapter modules must be feature-gated."* survives

It is not a description of the module being removed; it is a constraint on any adapter module ever
added. Its substance — a third-party numerical dependency must not reach the baseline public
surface — is the same rule RFC 008 applied to Rayon and Tokio, and `feature-matrix` and
`supply-chain` are what hold it. Deleting the sentence with the module would drop a constraint the
project still wants, and would leave the reserved `adapter-*` features (§2.4) ungoverned.

**Reworded to be forward-looking rather than deleted.** Replacement text: handoff §2.2.

### `loeres-external-design-v1.md` §4.7 survives; only its first sentence is rewritten

S2 said to delete `:1114`. Reading the section with it quoted shows that would be wrong: §4.7
*Device Diagnostics Without Logging* is a **normative policy** with seven allowed and five
forbidden examples — no strings, no heap-allocated reports, no logging macros as public API, no
tracing spans, no telemetry queues — and `check-public-api`'s forbidden-token sweep and the
zero-bleed gate already hold parts of it. The policy is about the **device path's diagnostics**,
not about which module hosts them; deleting its first sentence would leave a normative section
with no subject and would read as retiring the policy.

**Only the first sentence is rewritten, to name where device diagnostics actually live.**
Replacement text: handoff §2.3. The allowed and forbidden lists and the closing sentence about an
optional no-`std` sink are untouched.

## 1. Summary

**Four `pub mod` declarations in published crates name modules that declare nothing**, each with a
rustdoc comment telling a crates.io reader it is a placeholder for work that has since landed
elsewhere:

| module | its own rustdoc |
| --- | --- |
| `loeres_backend_std::adapter` | *"Feature-gated adapters for ndarray / nalgebra / native BLAS-LAPACK-style storage. … Lands in Milestone 3; this Phase 0 placeholder declares no public items yet."* |
| `loeres_backend_std::batch` | *"Grouped model storage … Lands in Milestone 3; this Phase 0 placeholder …"* |
| `loeres_backend_std::view` | *"Immutable and mutable views into heap-backed dense/sparse buffers. … Lands in Milestone 3 …"* |
| `loeres_device::diagnostic` | *"Compact no_std diagnostics … Lands in Milestone 2; this Phase 0 placeholder …"* |

Milestones 2 and 3 are both complete. So these are at once **dead published surface**, **four stale
claims in published documentation**, and — in two cases — **a promise of capability that does not
exist**: `adapter` advertises ndarray/nalgebra/BLAS integrations, which is the same unheld claim
RFC 054 §2.2 deletes from the requirements specification, surviving in rustdoc where no sweep
looked.

This RFC removes the four modules, corrects the documents that map them, and adds one assertion so
the class cannot return. **No behaviour changes. No solve path is touched.**

## 2. The defect, measured

### 2.1 The predicate, and the figure

Every `pub mod` declaration in the five published crates, resolved to its file, checked for at
least one `pub` item:

```text
pub mod declarations resolved: 30
declaring no public item:       4
```

The four are exactly the table in §1. The predicate resolves nested declarations correctly —
`crates/loeres-cluster/src/runtime.rs:9` declares `pub mod cancel;` and `runtime/cancel.rs`
declares items — so 26 of 30 are sound and this is not a sweep looking for something to delete.

The command is in the implementation handoff §0. **Re-derive the figure before acting on it**; if
it is not 4, report the discrepancy rather than adopting either number.

### 2.2 Three of the four are the whole public surface of a crate under its own documented feature set

`adapter`, `batch` and `view` are **unconditional** `pub mod`s;
`crates/loeres-backend-std/src/lib.rs:21-26` gates `dense` behind the `dense` feature and `sparse`
behind `sparse`. `Cargo.toml:16` sets `default = ["dense"]`.

So **`loeres-backend-std` built with `--no-default-features` has a public surface consisting of
exactly three empty modules.** That is measurable, it is what a reader of docs.rs sees when they
toggle features, and it is the clearest statement of the defect available.

### 2.3 `loeres_device::diagnostic` shadows a real module

`loeres::diagnostic` exists and carries the RFC 003 / RFC 014 diagnostic categories
(`docs/specs/loeres-external-design-v1.md:280`). `loeres_device::diagnostic` is empty. A device
user looking for diagnostics finds the empty one first, because it is in the crate they depend on.
That is property **D**'s confusion, caused by us, in the crate whose whole value is predictability.

### 2.4 The project already has a mechanism for reserving capability, and it is not this

`crates/loeres-backend-std/Cargo.toml:14-21` declares `serde`, `parallel-rayon` and
`adapter-ndarray` as reserved **features**, with a comment saying so, and `published-metadata`
checks reserved features — RFC 049's notes record that it already holds them.

**A reserved feature is a declaration; an empty public module is API.** One is checked and invisible
to a user who does not enable it; the other appears in the crate's documentation as something they
can reach. The project decided how to express reservation and then expressed it twice, once in a
way nothing checks and a user cannot tell from a real module.

### 2.5 Why no gate saw this

`xtask/src/checks/public_api.rs:8-12` scans `EDGE_SRC` — `loeres`, `loeres-backend-static`,
`loeres-device` — because it was built for RFC 010's zero-bleed token sweep. Three of the four
modules are in `loeres-backend-std`, which is outside that set, and the fourth is an absence rather
than a forbidden token. **Not a defect in that gate**; a coverage fact that §4's S3 must respect by
being wider than `EDGE_SRC`.

## 3. Compatibility and safety analysis

Required by `docs/specs/loeres-roadmap-milestones-v1.md` §1.5.

### 3.1 What breaks

Removing a `pub mod` removes the paths `loeres_backend_std::adapter`, `loeres_backend_std::batch`,
`loeres_backend_std::view` and `loeres_device::diagnostic`. Code naming any of them fails with
`error[E0432]` or `error[E0433]`.

**The breakable set is narrow and is bounded by the defect itself.** These modules declare no
items, so **nothing can be imported from them** — `use loeres_backend_std::batch::Anything;` does
not compile today. The only code that compiles today and would stop is a bare `use
loeres_backend_std::batch;` (or a path mention) that uses nothing from it, which the compiler
already warns about as an unused import.

### 3.2 Who it can affect

- **In this repository: nobody.** `git grep` finds no code reference to any of the four outside
  their own files and `lib.rs`; the only references are documentation (§4 S2).
- **Downstream: no production use exists** (the owner, 2026-10-31). Every published version from
  `0.11.0` to `0.24.2` carries the modules, so a dependent *could* name one; none is known, and
  naming one can have had no effect.

### 3.3 Semver position

**Breaking, therefore a minor** under Cargo's `0.x` rules, as `docs/src/development.md`'s release
version convention requires — *"a breaking change cannot ship as a patch"*. `0.25.0` is already a
minor for the linear-objective step
(`rfcs/handoffs/release-0.25.0/scope-and-decision-points.md` §1.1), so this RFC changes the release
position not at all. **It must not ship in a patch**, and if `0.25.0`'s additive slice is dropped
this RFC still forces the minor on its own.

The `CHANGELOG.md` entry must state the break in the form `0.23.0`'s did — the error codes, who is
affected, and what to do — and here "what to do" is *delete the import*.

### 3.4 Monotonicity, rule by rule

None of §1.5's six rules is touched: no required method is added to a frozen core trait; `alloc` is
not required anywhere; the caller-owned device workspace is unchanged; no cluster concept moves into
core; no dynamic dispatch is introduced; "deterministic" is not reinterpreted. **The change is
subtractive and touches no trait, no signature and no execution path.**

### 3.5 Safety

No behavioural change of any kind: no solve path, no validation path, no error mapping, no feature
resolution, no dependency. `no_std`, no-`alloc`, panic-aversion, bounded iteration and zero-bleed
are unaffected, and the device path's surface gets smaller. **The size-budget and `no-std` gates
should report no change beyond the absence of four empty modules.**

## 4. What to do

### S1 — remove the four modules

Delete `crates/loeres-backend-std/src/{adapter,batch,view}.rs` and
`crates/loeres-device/src/diagnostic.rs`, and their `pub mod` lines at
`crates/loeres-backend-std/src/lib.rs:19,20,27` and `crates/loeres-device/src/lib.rs:31`.

Then the crate-level rustdoc that describes them:

- `crates/loeres-backend-std/src/lib.rs:9-10` lists the module topography as *"`dense`, `sparse`,
  `view`, `batch`, `adapter`"*, and `:16-17` says *"`view`, `batch`, and `adapter` remain
  placeholders"*. Both go to the shipped set.
- `crates/loeres-device/src/lib.rs:25-26` says *"`diagnostic` is reserved for future richer
  diagnostics"*. That sentence goes. If richer device diagnostics are wanted later, they arrive
  with a module that declares something.

**The reserved *features* stay** (§2.4): `serde`, `parallel-rayon`, `adapter-ndarray` and the rest
are a declaration `published-metadata` already checks, and this RFC does not touch them.

### S2 — the documents that map the modules

Two normative apex specifications:

- `docs/specs/loeres-external-design-v1.md:291,292,293` — the `loeres-backend-std` module map rows
  for `view`, `batch`, `adapter` — and `:335`, the `loeres_device::diagnostic` row. Delete the four
  rows.
- `:295` says *"Adapter modules must be feature-gated."* With no adapter module, that sentence has
  no subject. It becomes a forward-looking requirement on any adapter module **added** later, or it
  goes; §8 records that I am not certain which, and the implementation handoff will decide it with
  the sentence in front of it.
- `:1114` says *"`loeres-device::diagnostic` may expose compact diagnostics through result or state
  types."* Delete the sentence; `loeres::diagnostic` is the home (§2.3).

### S3 — one assertion, wider than `EDGE_SRC`

In `xtask/src/checks/public_api.rs`, a new function beside `audit_public_file`, over **all five
published crates** rather than the edge three (§2.5): every `pub mod <name>;` in a crate source
file must resolve to a file that declares at least one `pub` item. Register
**`EMPTY PUBLIC MODULE`**, naming the declaring file, its line and the module.

**It is lexical, and its limits are stated rather than discovered:** it covers `pub mod <name>;`
declarations resolved to `<name>.rs` or `<name>/mod.rs` relative to the declaring file, which is
the shape all 30 of this project's declarations take. An inline `pub mod x { }` block and a module
whose only public surface is a `pub use` re-export are **not** handled by the simple form; the
handoff states which of those to cover and which to leave, with the reason. A declaration whose
file cannot be found is a finding, not a skip — **fail closed**.

Tests: an empty module fails, naming it; a module with one `pub fn` passes; a nested declaration
resolves (the `runtime::cancel` shape); and a declaration whose file is missing fails.

## 5. What must not be swept in

1. **`crates/loeres-backend-std/src/internal.rs`.** It is `pub(crate)`, it declares
   `dimension_mismatch`, and it is used. Not public surface, not dead, not in scope.
2. **The reserved features** (§2.4, S1).
3. **`loeres::diagnostic`** and `loeres-backend-static::view` — both real and populated. The
   predicate passes them, which is the evidence that it discriminates.
4. **Any non-empty module**, however thin. The rule is *declares nothing*, not *declares little*.
5. **RFC 007's and RFC 006's bodies.** They are in `rfcs/done/` and are never amended in place.
   This RFC supersedes the module-topography part of their design; the superseding record is this
   RFC, which is what §1.5 asks for.

## 6. Non-goals

- **No new capability.** Nothing is implemented to fill a module; the four are removed because
  nothing fills them.
- **No feature change**, no dependency change, no behaviour change.
- **No API-shape baseline.** Whether `check-public-api` should carry one is an open question
  carried since `rfcs/handoffs/release-0.24.0/scope-and-decision-points.md` §6, and S3 is one
  assertion, not that baseline.
- **No sweep of module rustdoc generally.** Four stale *"Lands in Milestone N"* claims go because
  the modules go, not as a documentation pass.

## 7. Risks

- **A downstream `use` of an empty module breaks.** §3.1 bounds it: the import can have had no
  effect, the compiler already warns about it, and there is no known production use. The CHANGELOG
  entry names the error codes and the one-line fix.
- **S3 is lexical and can be evaded** by an inline `pub mod x { }` block, or made to fire by a
  module whose public surface is only a `pub use`. The second is the dangerous direction — a false
  positive on a legitimate re-export module — and this project has none today, so the handoff
  chooses the narrow form and records the limit.
- **Removing `diagnostic` could read as removing a capability.** It removes an empty module; the
  capability is in `loeres::diagnostic` and always was. The CHANGELOG must say that in the same
  breath, or the entry creates the impression the module's removal is a loss.

## 8. What I could be wrong about

- **`external-design:295`, "Adapter modules must be feature-gated."** I have not decided whether it
  survives as a rule for a future adapter module or goes with the module it describes. It is a
  normative sentence in an apex spec and deciding it from memory is how I have made eight
  measurement errors in this sequence; the handoff decides it with the sentence quoted in front of
  it.
- **That four is the whole set.** The predicate covers top-level and one-level-nested `pub mod`
  declarations in `crates/*/src`. If this project later grows a module tree deeper than
  `runtime/cancel.rs`, the predicate needs the same widening, and nothing will say so.
- **That removal beats declaring them reserved.** The alternative is to keep the modules and write
  an honest *"reserved and empty"* rustdoc. I rejected it because it is a statement of intent no
  gate can assert, and because the owner's standing rule prefers removal to suppression — but a
  future maintainer who wants `batch` to exist in `loeres-backend-std` will have to re-add a path
  that was once published, and that is a cost this RFC accepts rather than avoids.
