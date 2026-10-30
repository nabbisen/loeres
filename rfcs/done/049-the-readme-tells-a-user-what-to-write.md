# RFC 049 - The README Tells A User What To Write

**Status.** Implemented (v0.24.1). Amendment 1 was made while Accepted, under RFC 000's in-place-amendment rule.

**Author tier.** `architect`.

**Governing scoping.** RFC 047 **Phase I — stop misdirecting**, and property **B**: *every claim a
user can read is asserted by a gate*. The three defects below are the remainder of what architect
scoping 104 §5 slice 1 and architect review 105 §4 slice 1 named, after removing the part of that
slice that is **already done** (§2.4). Every figure here was measured on the tree at `36b1e63`,
`0.24.1`, before this RFC was written.

RFC 048 held what current-facing documents say about *this project*. This holds what the README
tells a user to *write*.

## 0.1 Amendment 1 (2026-10-30)

Made while Accepted, under RFC 000's in-place-amendment rule, and recorded in architect review
116. The Status line carries only the design-freeze date while this RFC is in `accepted/`, because
`doc-currency` requires that form there; **this amendment must be named in the Status line when
this RFC moves to `rfcs/done/`**, which `check-rfcs` enforces for `done/` only.

**The defect is in my own measurement.** §2.1 states *four sites* and names `README.md:55,56,59,60`.
There are **eight** tracked wildcard sites in two files. The four this RFC missed are
`docs/specs/loeres-external-design-v1.md:238,239,252,253` — the `toml` fences of its §1.4 **Public
Import Model**, under the sentence *"The intended downstream import model is:"*. That file is one
of the three normative apex specs, so there the wildcard is not stale prose quoting an
instruction: it **is** the project's normative statement of what a user should write, and what it
says to write is the form `deny.toml:49` denies. I measured `README.md` and `crates/*/README.md`,
which was the hypothesis I had formed, and stopped.

Checked rather than assumed: both snippets appeared on 2026-06-30 under RFC 008, the README's one
commit before the apex sync, so neither is derived from the other. They are two copies of one
decision.

**This amendment implements nothing.** It corrects the measurement — §2.1's heading and its
closing paragraph say *eight in two files* — and it adds **no provision to S1, S2, S3 or S4**.
The frozen core as the owner accepted it is unchanged and is authorized: S1 over the README's four
sites, S2, S3, and S4's predicate scoped to `README.md` and `crates/*/README.md`. Its
implementation handoff is `rfcs/handoffs/049-the-readme-tells-a-user-what-to-write/implementation-handoff.md`.

**The four spec sites are deferred to the owner, not to a later slice by default.** Review 116 §3
states three treatments and the cost of each: **A** pin them as the README is pinned, with the
predicate extended to that file's fences and the value taken from the apex block already in it;
**B** drop `version` from §1.4's fences and defer to the README, which is RFC 048's one-home move
and needs no per-release value; **C** leave them, which is the only option that makes the tree
less consistent after this slice than before it, because `README.md` would say `"0.24"` while the
normative spec says `"0.x"`. I recommend **B**, with **A** acceptable. Until the owner chooses,
those four lines are out of scope and the implementer reports rather than touches them.

Monotonicity (roadmap §1.5) is not engaged: nothing here touches a shipped constraint, a public
API, or a gate's existing predicate.

## 1. Summary

`README.md`'s dependency snippet — the one instruction on the most-read page in this project —
tells users to write `version = "0.x"` for four crates. That is a **wildcard requirement**, the
exact form this project's own `deny.toml` denies in its own manifests, and at `0.y` it instructs
the reader to accept every future breaking change. It resolves correctly today and will keep
resolving correctly until the first `0.25.0`, at which point `cargo update` alone moves every such
user across a compatibility boundary.

Two structural defects sit around it: the `## Quick Start` a newcomer reads first opens with three
contributor commands, and the README's own *"Start here"* link points at a chapter whose **Install**
section installs the repository.

This RFC pins the requirement to the last **released** minor, puts the user's instruction first,
makes the book chapter say what it is, and adds one assertion so none of the three can come back.
**No new gate** — the assertion joins `doc-currency`, whose apex block already carries the value it
needs.

## 2. The defect, measured

### 2.1 `version = "0.x"` — eight sites in two files, and the project's own policy forbids it

`README.md:55`, `:56`, `:59`, `:60`:

```toml
loeres-cluster        = { version = "0.x", features = ["parallel-rayon"] }
loeres-backend-std    = { version = "0.x", features = ["dense"] }
loeres-device         = { version = "0.x", default-features = false }
loeres-backend-static = { version = "0.x", default-features = false, features = ["owned-arrays"] }
```

`deny.toml:49` reads `wildcards = "deny"`, and the `supply-chain` gate fails on a wildcard
dependency (RFC 026). **So the README instructs a user to write the one requirement form this
project will not accept in its own tree.**

**Measured, not reasoned.** A scratch crate depending on `loeres-device = { version = "0.x" }`
resolves, offline from the local cargo cache, to `loeres-device 0.24.0` — exactly as
`version = "0.24"` does. The requirement is **not broken today**, and that is the problem: at
`0.y` the minor *is* the compatibility unit, so `0.x` (`>=0.0.0, <1.0.0`) admits every future
minor. The first `0.25.0` moves every reader of this snippet across a breaking boundary through
`cargo update` alone, with nothing to warn them. This project has already published the lesson:
`docs/src/specifications.md` tells `0.23` users they must move their requirement by hand to pick
up a correctness fix, *because* `0.24.0` is a minor.

An invisible defect that becomes a user's broken build at a time of our choosing is worse than a
visible one, and it is the second principle failing in the one place every user looks.

**Amendment 1: the same four lines exist in a normative apex spec.**
`docs/specs/loeres-external-design-v1.md:238,239,252,253`, the `toml` fences of its §1.4 **Public
Import Model**, carry the identical wildcard — so the tracked total is **eight sites in two
files**, not four. Those four are **deferred**, not fixed by this RFC: §0.1 states why, and
architect review 116 §3 states the three treatments and what each costs. S1 and S4 below are
unchanged and cover `README.md` only.

### 2.2 The `## Quick Start` is still a contributor's, inside out

`README.md:10-18` — the first heading after the opening bullets:

```sh
cargo check --workspace --all-features
cargo xtask check        # developer aggregate; not release approval
cargo xtask examples     # builds and runs each example, checks its dependency isolation
```

These are correct, useful, and for someone who has cloned the repository. The **user's**
instruction — the "choose by where the solve runs" table and the dependency snippet — is in the
same section, *below* them, at `:44-60`. A reader who stops at the first code block has been told
to check out a workspace.

### 2.3 "Start here" points at a chapter that installs the repository

`README.md:92`:

> - Start here: *Getting Started* — **install**, solve one problem, read the result.

(The bullet links `docs/src/getting-started.md`; the target is written here without its link
syntax because `check-rfcs` and `link-audit` resolve every bracket-parenthesis link they find
relative to the containing file, inline code spans included, so a link reproduced from a
root-relative document fails both gates from inside `rfcs/proposed/`. A sentence naming that
syntax literally trips it too, which is how this parenthesis reached its third draft.)

`docs/src/getting-started.md:8-21`, its `## 1. Install`:

> You need `git` and rustup… `git clone https://github.com/nabbisen/loeres.git`

The chapter is a good from-source tour and §2–§6 are worth keeping as they are. But the README's
own recommended path for a new reader never mentions adding a dependency, while the README itself
does, two sections earlier, in a form `deny.toml` forbids. **Two doors, two answers**, which is
the confusion RFC 047 property D exists to forbid — and it is our own doing, not an accident of the
ecosystem.

### 2.4 What is already done, and must not be "fixed" again

Checked before writing, because 104 §0 inherited this from review 103 and it is no longer true:

- **The per-crate READMEs already open with purpose and environment.** `crates/loeres/README.md`,
  `loeres-backend-static`, `loeres-backend-std`, `loeres-cluster`, `loeres-device` each begin with
  one or two sentences saying what the crate is for, most with **Environment** and **Depends on**
  bullets. They are not RFC archaeology. **Out of scope.**
- **`published-metadata` already holds the packaged READMEs**: no relative link target (they 404
  on crates.io), a `## Features` section naming every declared feature, `LICENSE` and `README.md`
  present in the tarball. The **root** `README.md` is *deliberately exempt* from the link rule
  (`xtask/src/checks/published_metadata.rs:66-70`) because it ships with no crate and its relative
  links are correct on GitHub. **Do not change that exemption**, and do not put this RFC's
  assertion in that file.
- **No per-crate README advertises a version requirement at all.** Adding one would create four
  more claims to hold for no gain this RFC needs. **Out of scope** (§5).

## 3. What to do

### S1 — pin to the last released minor, and say why

Replace `"0.x"` with `"0.24"` at all four sites, and add one sentence saying that at `0.y` the
minor is the compatibility unit, so the requirement names a minor deliberately rather than
accepting the next one. Measured: `"0.24"` resolves to the same `0.24.0` the wildcard resolves to
today, so this changes what a user gets **now** not at all, and changes what they get at `0.25.0`
entirely.

### S2 — the user's instruction goes first

Move the "choose by where the solve runs" table and the dependency snippet to the top of
`## Quick Start`. Move the three contributor commands under a heading that says what they are —
`## Building from source` or similar — and keep them in the README; they are not misdirection once
they are labelled. Change no word of the opening bullets at `:1-8`: they are accurate, they state
the problem class and the honest-status discipline, and they are the best paragraph in the file.

### S3 — the book chapter says what it is

One paragraph at the top of `docs/src/getting-started.md`, before `## 1. Install`: this chapter is
a from-source tour; a reader who wants to use the library in their own project should take the
README's dependency snippet instead, linked. Adjust the README's *"Start here"* line so it does not
promise "install" in the sense a library user reads it.

**§2–§6 of that chapter are not touched.** Rewriting it into a user's tutorial needs the front
door, which is Phase III and not decided; this RFC only stops the two doors contradicting each
other.

### S4 — the assertion

One check, in `xtask/src/checks/doc_currency.rs`, so **no new gate**. It belongs there rather than
in `published_metadata.rs` for two reasons: that file's stated boundary is the *packaged* crates
and the root README is explicitly exempt from its rules, and the value this predicate needs is
already parsed by `doc-currency`'s apex block.

**The predicate, exactly:**

1. Every `version = "…"` inside a `toml` code fence in `README.md` must equal
   `MAJOR.MINOR` of the apex block's **`Last reconciled repository release`** field — `0.24` while
   that field reads `0.24.0`.
2. No `version = "…"` anywhere in `README.md` or any `crates/*/README.md` may be a wildcard
   requirement — containing `*`, `x` or `X` in place of a number. This is `deny.toml:49`'s rule,
   applied to what we tell users rather than only to what we compile.
3. The failure message names the crate and the site, not the file.

### 3.1 Why the *released* version and not the workspace version

This is the design decision in the RFC and it has a cost either way.

- Against `workspace.package.version` (`0.24.1`): the assertion would demand `"0.24"` today, and
  would be satisfied — but the moment the workspace bumps to `0.25.0`, it would demand the README
  advertise `"0.25"` **before anything is published at `0.25`**. A user following it would get a
  resolution failure. The gate would be enforcing a wrong instruction.
- Against the apex block's `Last reconciled repository release` (`0.24.0`): the README always names
  a minor a user can actually install. It moves when a release is reconciled, which is the cadence
  the block already has and which RFC 048 just made machine-held.

The second is correct, and it composes with RFC 048 rather than adding a second source of truth.
Its cost is that the README lags the tree by one release during development — which is right: the
README instructs users, and users install releases.

## 4. Non-goals

- **No front door, no new crate, no channel decision.** Phase III and Phase V, both undecided.
- **No per-crate README install snippet** (§2.4). Four more claims, no gain here.
- **No change to `published_metadata.rs`**, and none to the root README's link exemption.
- **No rewrite of `docs/src/getting-started.md` §2–§6.**
- **No change to the README's opening bullets, the "Reading a result" section, or the examples.**
- **No new gate, no version change, no tag.**

## 5. Risks

- **Functional — Low.** Documentation and one check. No library change.
- **Business — Low, and this is the slice's whole point.** The only user-visible effect is that the
  instruction a user copies becomes one they can keep.
- **Operational — Low.** The predicate's value comes from a block a gate already parses; the
  README's pin changes once per reconciled minor, and the check says so when it must.
- **Security — Low.** No channel, no artifact, no trust-root change. Worth one sentence only
  because a wildcard requirement is a supply-chain shape this project denies itself: a user on
  `0.x` takes whatever the next minor contains, which is the opposite of the identity discipline
  RFC 046 established for our own tools.

## 6. What I could be wrong about

- **§3.1 assumes a user installs releases, not the tree.** For a contributor reading the README
  inside a checkout, the lag is a small wart: the README says `"0.24"` while the tree is `0.24.1`.
  I judge that correct and worth stating in the README's own sentence; the owner may prefer the
  workspace version and accept §3.1's cost instead.
- **S2's placement is a judgement about what a newcomer reads first**, not a measurement. I have
  not watched anyone read this file.
- **The wildcard's resolution was measured offline from this machine's cargo cache**, which held
  `0.24.0` for all five crates. The resolution is not in doubt — `0.x` is `>=0.0.0, <1.0.0` — but
  the measurement is of this cache, and the RFC should not be read as having queried the registry.
