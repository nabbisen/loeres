# The Destination Sequence

This file is the single definition of the phases toward
[`rfcs/accepted/047-the-state-worth-aiming-at.md`](accepted/047-the-state-worth-aiming-at.md)'s
destination — the order in which that destination is approached.

The order and the cost of each phase are advisory: which phase comes next, what it costs, and
what may be cut are architect recommendations and owner authorizations, not fixed here. This
file is revised as each is measured. It is deliberately not an RFC, so that revising it needs no
amendment (RFC 047 §5 item 1).

The arabic `Phase 0-3` in `ROADMAP.md` and `docs/specs/loeres-roadmap-milestones-v1.md` are the
crate-layering milestones, a different axis from the roman phases below.

This sequence was first written in architect strategy 106 §5, which is cited in
`rfcs/review-evidence-index.md` and is not a tracked file; this file is now the definition
rather than a copy with an authority elsewhere.

## Phase I — stop misdirecting

**State.** shipped
**Properties.** B
**RFCs.** 049, 050, 051

A user's "Quick Start"; per-crate READMEs that open with purpose and usage; a real minor pin.
Not because it is cheap, but because every artefact a newcomer meets currently points at the
wrong thing, and no later work can be judged while that is true.

## Phase II — decide who we are for, and say it

**State.** deferred
**Properties.** D
**RFCs.** —

The audience question, and a published scope statement. Prior to all interface work, because
property D is impossible without it.

## Phase III — the front door's contract

**State.** deferred
**Properties.** A, D, E
**RFCs.** —

Input format, the refusal vocabulary, the exit statuses; its relationship to the conformance
corpus asserted by a gate rather than resembled.

## Phase IV — identity for what users receive

**State.** deferred
**Properties.** C
**RFCs.** —

Extend RFC 028 to the artifacts users install, as one identity story rather than two. This gates
every real channel, so it precedes the channel.

## Phase V — the channel

**State.** deferred
**Properties.** —
**RFCs.** —

Through the user's own package manager and their own trust root.

## Phase VI — point the evidence outward

**State.** deferred
**Properties.** B, C
**RFCs.** —

The conformance corpus and the gates become something a user or their auditor can run against
the artifact they received. Properties B and C completed: a solver whose claims the user can
check without trusting us.

Phases I and II are not prerequisites for each other and Phase I needs nothing. Phases IV and V
are the expensive half and the half with standing obligations.
