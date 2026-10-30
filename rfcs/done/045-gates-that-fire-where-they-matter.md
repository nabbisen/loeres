# RFC 045 - Gates That Fire Where They Matter

**Status.** Implemented (v0.24.1). Amendment 1 was made while Accepted, under RFC 000's in-place-amendment rule.

**Author tier.** `architect`.

**Governing scoping.** Architect review 107, which measured this, and architect review 106
(the overall strategy) §1.1 and §4, of which this RFC implements the first part. Every figure below was measured by the
architect on the `0.24.1` tree.

## 0.1 Amendment 1 (2026-10-29)

Made while Accepted, under RFC 000's in-place-amendment rule. The Status line carries only the
design-freeze date while this RFC is in `accepted/`, because `doc-currency` requires that form
there; **the amendment must be named in the Status line when this RFC moves to `rfcs/done/`**,
which `check-rfcs` enforces for `done/` only.

**S2 — the scheduled run — is withdrawn.** The owner objected that a daily or weekly GitHub
Actions job is not reasonable or useful in this case, and the objection holds on three counts
that §3.2 got wrong.

**1. The twenty days were a red gate, not an exposure, and §3.2 conflated them.** `tokio` is
`optional` on `loeres-cluster` behind the default-off `async-tokio` feature, declared with only
the `rt` feature; `AsyncFd` lives behind `net`, which this workspace never enables. Nothing was
reachable at any point. §3.2's *"a project that fails closed on advisories and checks only on
pushes has an assurance property it cannot deliver"* reads as though users were at risk. They
were not, and the sentence overstated the harm.

**2. The only consequential window is already gated.** An advisory can reach a user only
through a release, and `release.yml` runs the full twenty gates on every tag push. So the gap
S2 was meant to close sits entirely inside a period during which nothing can be published.

**3. A timer is least reliable exactly when the gap is longest.** Measured on this repository:
18 commits on 2026-10-08, then **nothing for twenty days**, then 2026-10-28. Work happens in
bursts with long quiet stretches. A daily job would have run about twenty times against an
unchanged tree to report the same thing, in a repository quiet enough that GitHub's own
inactivity rules make scheduled workflows unreliable. And §3.2 itself conceded the deciding
point — *"a red run nobody reads is the same as no run"* — without following it: in a quiet
period, nobody is reading.

### What replaces it

Two obligations, both free and both **pull-based** — they fire when someone is actually
looking, which is the only time a finding can be acted on:

1. **`rfcs/handoffs/release-*/finalization-checklist.md` states that advisory drift at cut time
   is expected and is normal maintenance, not an emergency**, with the fix pattern: a scoped
   `cargo update -p <crate>`, never a bare one (architect review 102's finding), then verify every changed
   lockfile line is a version string.
2. **The implementation-handoff template states that a `supply-chain` failure on a dependency
   the implementer did not touch is not theirs** — report it and fix it scoped, rather than
   wondering what they broke. This is the only real cost the twenty-day window carried: an
   unrelated failure appearing mid-slice and reading as self-inflicted.

§3.4's cost reporting (S4) now folds into S1 alone.

**What this does not change:** S1 and S3 stand. The seventeen unwired gates are the finding
that matters, and the measurement behind them — three seconds warm, against a three-minute
workflow — is unaffected by any of the above.

## 1. Summary

This project has twenty gates and enforces **three** of them against the tree on an ordinary
commit. The other seventeen run only on a tag push, through `release-gate`, or when a human
remembers to type `cargo xtask check`.

The whole suite takes **three seconds** warm, while the `ci` workflow already spends **three
minutes**. So the reason seventeen gates hold nothing is not cost — they are simply wired to
the wrong trigger.

A related gap looked like a missing timer and is not. `main` carried a lockfile failing its
own `supply-chain` gate for about twenty days before anyone ran a gate, and this RFC first
proposed a scheduled run for it. **Amendment 1 withdraws that**: the twenty days were a red
gate rather than an exposure, the only window in which an advisory can reach a user is a
release, and `release.yml` already runs all twenty gates on every tag push.

This RFC wires what already exists and writes down what is deliberately exempt. **It adds no
new gate** — architect review 106 §4's four new assertions are deliberately left to their own RFC
(§4).

## 2. What is enforced today, measured

| trigger | what it enforces against the tree |
| --- | --- |
| push / PR to `main` (`ci.yml`, `msrv.yml`, `no-std.yml`) | `cargo fmt --check`, `cargo clippy -D warnings`, `cargo check`, `cargo test`, an MSRV `cargo check`, and **three** gates: `zero-bleed`, `supply-chain`, `no-std` |
| tag push (`release.yml`) | `cargo xtask release-gate` — all twenty |
| a human | all twenty, on remembering |

The seventeen that gate nothing per commit: `host-check`, `feature-matrix`, `target-profiles`,
`check-rfcs`, `doc-currency`, `review-evidence`, `check-public-api`, `panic-audit`,
`size-budget`, `unsafe-audit`, `examples`, `conformance`, `differential`, `link-audit`,
`published-metadata`, `doc-build`, `bench-baseline`.

**`cargo test` does not substitute.** The gate modules' `#[cfg(test)]` tests exercise each
gate's *logic* over synthetic input; they do not run the gate against the repository. A stale
apex block passes all 657 tests and fails only `cargo xtask check`. CI looks thorough for a
reason that does not hold.

## 3. Obligations

### 3.1 S1 — `cargo xtask check` on every push and pull request

Replace `ci.yml`'s two individually-named xtask invocations — `zero-bleed` and
`supply-chain` — with the aggregate. The third gate a push reaches, `no-std`, is invoked from
`no-std.yml`, which this RFC leaves alone; that is why §2's table counts three gates on a push
but only two of them live in the file S1 edits. The provisioning this needs was measured, not
guessed:

| requirement | status |
| --- | --- |
| `thumbv7em-none-eabihf` target | **needed** — `no-std`, `target-profiles`, `size-budget`. `no-std.yml` already provisions it with `dtolnay/rust-toolchain`'s `targets:`; copy that |
| `cargo-deny` at the pinned `0.20.2` | **needed** — `supply-chain`. `ci.yml` already installs it in one job |
| GNU `size` (binutils) | **needed** — `size_budget`. Present on `ubuntu-latest` (`GNU size 2.47` locally); `size_budget` returns `false` if it is missing, so absence fails closed rather than silently skipping |
| `mdbook` | **not needed.** `doc-build` runs `cargo doc --workspace --all-features --no-deps` under `RUSTDOCFLAGS=-D warnings`. A full `cargo xtask check` invokes `mdbook` **zero** times; only `release-gate`'s clean-extraction step does |

**The three-second figure is warm and local; CI is cold.** Do not promise it. What is known is
that `release.yml` runs the same twenty gates in **6m 57s** cold, including `mdbook` and
`cargo-deny` installs that S1 does not all need. Measure the first run and report it (§3.4).

Keep `no-std.yml` and `msrv.yml` as they are: `no-std` is covered twice, which is cheap, and
`msrv.yml` pins a different toolchain, which the aggregate does not.

### 3.2 S2 — a scheduled run — **WITHDRAWN, see Amendment 1**

Add a `schedule:` trigger running the full gate on a timer — daily is defensible, weekly is the
minimum that would have caught the `tokio` case.

**This is the only obligation here that addresses a failure that already happened.** `main` sat
at `2b3ae10` from 2026-10-08; RUSTSEC-2026-0386 was fixed in `8da67e2` on 2026-10-28. For about
twenty days `main` failed its own `supply-chain` gate and nothing could say so, because no push
happened. A project that fails closed on advisories and checks only on pushes has an assurance
property it cannot deliver.

A scheduled failure must be visible without someone opening the Actions tab. How — a failing
status, an issue, a notification — is an implementation question; **say which you chose and
why**, because an unread red run is the same as no run.

### 3.3 S3 — write down what is deliberately exempt

An exemption that is written down is a decision; one that is merely true is an accident waiting
to be found by someone else. Record in `docs/src/development.md`:

1. **The RFC 028 anchor is hand-recomputed and verified three ways at every cut, on purpose.**
   Its value is that a machine did not do it. This is an exemption, not an omission.
2. **`no-std`, `panic-audit` and `check-public-api` cover only the edge crates** — `loeres`,
   `loeres-backend-static`, `loeres-device`. Defensible (a `std` crate may legitimately panic),
   but `loeres-cluster` and `loeres-backend-std` therefore have no equivalent assertion, and
   that should be stated rather than inferred.
3. **Two targets are asserted**: host and `thumbv7em-none-eabihf`. No other.
4. **`size-budget`'s name misleads and the behaviour should be stated.** `GateKind::Advisory`
   changes only the *label when it passes* ("advisory baseline reported"); a `false` prints
   `FAIL` and the suite's result is the conjunction of all gates, so it **is** enforced. What
   is advisory is the *reported rlib figure*; the pinned reference-instantiation delta is
   enforced, and a missing `size` fails the suite.
5. **`bench`, `throughput` and `lp` are not gates** — standalone commands. The performance
   figures are therefore measured when someone chooses to, and `bench-baseline` is the only
   part of that which binds.

### 3.4 S4 — report what it cost

CI minutes are a real cost and this RFC increases them. Report, from the first runs, the `ci`
workflow's duration before and after S1. (Amendment 1 withdrew S2, so there is no scheduled run
to measure.) **If the increase is material, say so** — the fallback is to split the aggregate
across the existing parallel jobs rather than to drop gates, and that decision needs a number,
not an impression.

**The answer, recorded here because this is where the question was asked** (architect review
108 §4). It is not material and the aggregate is not split. Cold in CI the twenty gates take
**29 seconds**. The job that runs them took **2m 32s** (run `40775245512`) and **2m 47s** (run
`40792044395`), of which **1m 51s–1m 56s** is `cargo install cargo-deny` — a cost the
`supply-chain` job this replaced already paid, and the reason that job was itself the old
workflow's bound, at 2m 17s against a 2m 20s total. So the marginal cost of wiring the other
seventeen gates is **about fifteen seconds** of job wall time. The `ci` workflow's total went
from ~2m 20s to **2m 53s**; the first run's 4m 32s was 1m 28s of queueing before any job
started, not compute. The lever worth pulling is a prebuilt-binary `cargo-deny` install, which
would leave the job faster than it was before this RFC — splitting twenty gates across jobs
would recover nineteen seconds.

## 4. Non-goals

- **Architect review 106 §4's four new assertions** — an API-shape baseline, RFC-index section
  membership, user-facing commands and version pins, and procedures verified rather than
  documented. Each needs its own design; an API-shape baseline against a generic,
  feature-gated surface in particular is a real design problem (review 100 §8 raised it and
  left it open). **This RFC adds no new gate**, so that none of them is smuggled in without
  the design it deserves.
- Changing what any existing gate checks.
- Making the RFC 028 anchor automatic (§3.3 item 1).
- Any library change.

## 5. Release position

CI configuration and documentation. No callable API, no behaviour change, so this is a
**patch** and the workspace stays at `0.24.1`.

## 6. Risks

- **Functional — Low.** No library code changes.
- **Business — Low.** CI minutes.
- **Operational — Low, and the unusual part: this *reduces* operational load.** It moves
  seventeen manual invocations into CI and removes the dependence on someone remembering.
  The counter-risk is flakiness: `conformance` and `bench-baseline` will now run per-push on
  shared runners. `release.yml` runs them already, so there is precedent, but per-push is a
  different exposure than per-tag, and S4's measurement is what would reveal it.
- **Security — Medium, improved by S2.** A project that fails closed on advisories but checks
  only on pushes will learn about the next one late, exactly as it did about this one.
