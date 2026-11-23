# RFC 040 — `size-budget` must find its object under any cargo layout

**Author tier.** `architect`. **Date:** 2026-11-23.
**Governing text.** `rfcs/done/040-a-device-budget-worth-enforcing.md` — **unchanged.** RFC 040's
design is the measurement (`.text` + `.rodata` of one concrete device-kernel instantiation,
bounded delta from a pinned baseline); only where the code looks for the compiled object is
corrected. A `done/` RFC is never amended in place and this needs no amendment. Finding is
architect review 127 E1.
**Base.** `5076dd4d0d8e87d5463937956e323e87169edf25`.
**Release.** `0.25.0`. **No version bump** — this changes no public surface. The bump was already
performed by RFC 055's slice, so the workspace is at `0.25.0`; run
`cargo xtask release-gate --intended-tag 0.25.0`.
**Priority: this is release-blocking.** `cargo xtask check` fails on today's toolchain, so a
`0.25.0` tag pushed before this lands would fail its own tagged `release-gate` job and the release
would not be distributable (RFC 021 §7).

One item, in one function. Read §1 before touching anything: the problem is not what the failure
message suggests.

## 1. The defect, measured

The stable toolchain moved while nothing in the tree did:

```text
recorded in the 0.25.0 candidate evidence (2026-11-01): rustc 1.99.1 (58cf8f9c3 2026-10-13)
today:                                                  rustc 1.100.0 (0ef1bc386 2026-11-09)
                                                         cargo  1.100.0 (3acc79c0c 2026-11-05)
```

`cargo xtask size-budget` now reports:

```text
  unavailable: no compiled object found for device-size-reference
[size-budget] FAIL
```

**The object is built; it is in a different place.** `find_reference_object`
(`xtask/src/checks/size_budget.rs:169-186`) reads exactly one directory,
`device-size-reference/target/<triple>/release/deps`, which under cargo 1.100.0 **does not
exist**. The object is written to the package's artifact directory instead:

```text
device-size-reference/target/thumbv7em-none-eabihf/release/build/device-size-reference/<hash>/out/device_size_reference-<hash>.o
```

**Confirm this yourself from a removed target directory**, not from a warm one — `rm -rf
device-size-reference/target` first, then run the gate's own command. I checked it that way; a
stale cache would have been the other plausible cause and it is not this.

**The measurement is healthy, and that is the half that matters.** `size -A` on the object at its
new path, summing `.text*` + `.rodata*`:

```text
10728 bytes   against the pinned REFERENCE_BASELINE_BYTES = 10736
delta 0.07%   against REFERENCE_DELTA_FRACTION = 10%
```

Eight bytes across a toolchain minor bump. **Do not touch `REFERENCE_BASELINE_BYTES`.** The pin is
not stale and the bound is not too tight; the module doc's instruction to re-measure and re-pin
applies when *the measured code path* changed, and it did not.

## 2. The correction

**Widen `find_reference_object` from one directory to the subtree.** Recurse under

```text
device-size-reference/target/<DEVICE_TARGET>/release/
```

and consider every `*.o` whose file name starts with `REFERENCE_CRATE` (`device_size_reference`),
taking the newest by modification time — which is exactly what the function already does inside
`deps/`, applied one level up. That matches the old layout and the new one with no toolchain check
and no version test.

Measured, so you know the search is unambiguous: after a clean build there is **exactly one** `*.o`
anywhere under `release/`, and its name starts with the crate name.

**Three things not to do, each for a stated reason:**

1. **Do not hardcode the new path.** It is as brittle as the old one, one layout later, and this
   is the second layout this gate has seen.
2. **Do not use `--message-format=json` to locate it.** Measured: the `compiler-artifact` message
   for `device_size_reference` lists the `.rlib` and the `.rmeta` and **not** the `.o`, even though
   the object is emitted beside the `.rmeta`. It looks like the principled answer and it does not
   work.
3. **Do not add a fallback that passes when no object is found.** The `unavailable → false` path is
   correct and must stay: an enforced measurement that cannot be taken is a failure, not a skip.
   This slice makes the object findable; it does not make its absence tolerable.

Reuse an existing recursive walker if one fits — `util::collect_ext` collects by extension and is
already shared by `public_api` and `link_audit`. Do not write a second directory walker.

Update the function's doc comment to say that the object's location is **searched for rather than
known**, and why: cargo moved it between 1.99 and 1.100, and the gate should not need editing the
next time it moves.

## 3. The test

The existing `size-budget` work has no unit test for this, because the function reads a real build
tree. **Do not build a fixture that fakes a cargo layout** — it would pin the layout this slice
exists to stop depending on.

Instead, make the search function **pure over a directory listing**: a small function that takes an
iterator of candidate paths and returns the newest whose file name starts with the crate name, with
the filesystem walk outside it. Then two tests on literals:

1. candidates spanning **both** layouts — one `…/release/deps/device_size_reference-aaa.o` and one
   `…/release/build/device-size-reference/bbb/out/device_size_reference-bbb.o` — return the newer,
   proving neither layout is privileged;
2. candidates containing only an unrelated `*.o` (a different crate name) → `None`, proving the
   name filter still discriminates.

## 4. Verification

1. `rm -rf device-size-reference/target`, then `cargo xtask size-budget` → **PASS**, and quote the
   `enforced: device-size-reference (…)` line including the measured bytes and the delta. That line
   is the evidence this slice restores.
2. `cargo xtask check` → **PASS, 20 gates**, on the gate's own exit status, never through a pipe.
   **19 of 20 pass today; this must bring it to 20.**
3. `cargo xtask release-gate --intended-tag 0.25.0` → PASS. Whole stream to a file.
4. **Record the toolchain you measured on**, with `rustc -vV` and `cargo -V` quoted in your
   request. This slice exists because the toolchain moved, so the version is part of the result.
5. **Prove it still fails when it should:** temporarily move the found object aside (`mv` it, do
   not delete the directory), confirm `size-budget` reports `unavailable` and FAILs, then move it
   back. The absence of an enforced measurement must remain a failure.
6. The review request at
   `.git-exclude/review-request/040-size-budget-object-path/REVIEW_REQUEST.md`.

**Not yours**, and not part of this slice: `REFERENCE_BASELINE_BYTES`, the delta fraction, the
advisory rlib figure, the `cargo::unused_dependencies` warning on
`crates/loeres-device/Cargo.toml:22` (architect review 127 E2 — a manifest and feature question
with a semver component, recorded for `0.25.1`), moving any RFC to `rfcs/done/`, the tag, the push,
and publication.
