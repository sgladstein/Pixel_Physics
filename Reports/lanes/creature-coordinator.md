# Creature line — coordinator note, 2026-08-30

**Session `session_01NH9p7WAbCeXKCKnCQcsNYR`.** Successor to
`creature-line-handoff-2026-08-30.md`, which was written *for* this session by
`session_01JncNrRQacprPV1tRw3BXoh`. Three lanes running.

## What I changed about the suggested plan, and why

The handoff's §7 suggested **Lane A = vision (E15)**, Lane B = size the sense,
Lane C = reproduction economics + larder. I kept its counted constraint and two
of its three packages, and swapped which one gets the source lane.

**Vision does not go first.** E15 is authorised as a *direction* and its design
is not settled — the handoff itself asks for Lane B to size the sense before it
is built, on the grounds that the measured 46-cell mean beetle→trail distance
already rules out a short radius. Putting vision in the source lane *while* the
measurement that parameterises it is still being taken is the
*check-that-a-planned-step-can-demonstrate-itself* failure, which has cost this
project a whole phase once (three plant levers that all fired and moved no
pixel). So vision waits one cycle for its own sizing.

**`birth_grant` + E14 takes the source lane instead.** It is the only fully
specified, fully authorised package in the line: E12 (S6 built and firing),
E14 (*"Yes — let them starve"*), and `birth_grant` itself. Its arithmetic is
known — the shipped ant banks 567 against a birth bar of 1,860 — and what it
buys is the thing the whole milestone is named for: the ant **in the game**
breeds. It also makes `deaths` a live counter and puts the first carrion in the
world.

**Lane C's first deliverable was deleted by doing it.** The handoff assigned it
"land the economics report". I landed it myself before dispatching (below), so
Lane C got the whole larder question instead of half a lane.

## The split as dispatched

| Lane | Session | Owns | Package |
|---|---|---|---|
| A | `session_01At2jenzCTJqUHB82KQqRC6` | `src/sim/{creature,organism,brain}.rs`, `assets/species/*.ron` | `birth_grant` + E14 as one shared-budget reallocation |
| B | `session_01Ly6QZZzyPsUDCz4tdy1cCC` | `examples/vision_probe.rs` (new) | How far must a sight line reach, and what does it cost |
| C | `session_01HMZCjV8qq3DoN2YfDJrXpv` | `examples/larder_probe.rs` (new) | Does `store_in_body` have two reachable ends |

**One lane in the hot trio, two in `examples/` on distinct new files.** The
handoff's constraint is honoured rather than overruled. Checked before
dispatch and worth recording, because it is what makes B and C non-colliding:
**examples are auto-discovered — neither lane needs a `Cargo.toml` edit.**
Both were told to leave `examples/common/mod.rs` alone.

## Landed before dispatch

Both documents the line depends on were invisible — on branches, not on `main`,
one of them without a PR. A lane cannot be dispatched against a branch name it
has to be told.

- **PR #137** — E15 in the plan's decision log, plus the handoff note itself.
  Branch existed with a PR already open; merged.
- **PR #139** — `Reports/creature-reproduction-economics.md`, 783 lines, which
  had **no PR at all** since 2026-08-29. This is `session-programs.md`'s *the
  PR list is not the work list* exactly: `branchcheck.sh --prs` is what
  surfaces it and nothing else does.

## A finding for whoever picks up S8 — E10's premise is false, and I verified it rather than relaying it

The handoff reports this and flags it as blocked on the owner. **Confirmed by
reading the code, not by grepping a name** (the failure mode that produced the
"no standing larder exists" error on this same line):

- `creature.rs:1287` — `let mut spent = def.idle_cost + synapse_tax;`
- `creature.rs:1329` — `spent += def.move_cost;`, charged once per successful
  `step_chain`, which moves the **whole** chain.

Both flat per organism. Nothing in either cost path reads `chain.len()`.

**So E10's premise — *"per-cell metabolic cost already prices a longer body, so
no cost system is owed"* — is false, and the consequence is sharper than "a
gene would ratchet".** A longer body today is *strictly free and strictly
better*: 2.9x the ink on screen, and chain bodies block **2–6%** of their moves
against **25–43%** for rigid bodies of the same size, at **identical** metabolic
cost. Body length as a gene would go to its maximum immediately — not because
long is good, but because long is **free**. That is the degenerate-codomain
trap in a third costume, and it is the same shape as the `store_in_body`
question Lane C is measuring.

**Deliberately not put to the owner yet.** Lane A is at this moment changing
the cost model the decision would be made against — `idle_cost`, `move_cost`,
`eat_energy`, the synapse tax and `reproduce_threshold` all change meaning
under E14. Asking now is asking for a ruling on ground that is moving. Raise it
when Lane A reports, with the re-derived constants in hand.
