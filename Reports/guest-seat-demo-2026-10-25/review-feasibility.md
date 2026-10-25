# Guest seat: adversarial review and feasibility (step 1)

*2026-10-25, guest seat thread. Reviewed `designs/seat-guest.md` and its critique against `main` 9e32979e and a
step-0 probe (`examples/guestprobe.rs`, on branch `claude/project-thread-rdj55n`). The reviewer did not write the
design.*

## Verdict: feasible with changes, as "the beggar" only

The beggar's ten-minute loop (rub your way into the smell, beg in the brood pile, get nipped when the disguise
slips, eat a larva and pay for it) can be a playable demo on today's engine **if the colony's whole reaction to the
guest is a new body rule that reads scent** (the design's route (c)). Everything else in the design is either out of
reach for a demo or needs work no demo should carry:

| Part of the design | Demo? | Why |
|---|---|---|
| Inspection ladder (ignore, antennate, display, nip) | **Yes, the core** | New body rule; the probe shows nothing in the engine grades a visitor today |
| Rub, soak, beg | **Yes** | Small guest-only calls beside the ladder; scent and blend arithmetic already exist |
| Eat a larva, hide, jump | **Yes** | Bite on brood only; hide is body physics; jump is the ant's shipped Impulse |
| Steal, go limp (carried), lay your egg, lineage play | No | Each needs new engine work (multi-cell carry, guest brood stage) |
| The herd (mutualist) | No | Needs honeydew, a herd body and a tending rule, and leans on trail B |
| Arms race / bloodline drift | No | Tolerance spread is tiny and trained lab ants do not mutate at birth (enemy seat, mind.rs:1728) |
| Memory layer | No | Not needed: the ladder is a body rule |

## What the probe measured (step 0 of the design, done)

A beetle put on the mound beside the door of a grown `played_bed` colony (81-170 ants, 21-36 brood at frame 30,000),
marked steered with no input so it stands and never bites. 3 seeds per arm, 4,000 frames watched.

| Ants' brain | Beetle's colony number | Beetle's scent | Lived (frames) | Bites taken | Fed |
|---|---|---|---|---|---|
| trained (lab default) | its own | far / near the nest's | 49, 99, 879 | 2-8, all by the mind's Attack | 0 |
| trained | the host's | far / near | 4,000 (all three) | 0 | 176 J once (a mind Feed) |
| hand-built (`oldant`) | its own | far / near | 74, 839, 904 | 5-11 (the old fight) | 0 |
| hand-built | the host's | far / near | 74, 344, 529 | 6-11 | 0 |

What it says:

1. **Smell changes nothing today.** Near and far scent gave byte-identical results in every arm, the exactly-zero
   delta the code predicts: the trained mind reads colony number (`options.rs` describe, NESTMATE/STRANGER), and the
   hand-built ant's kin test refuses another species first (`creature.rs:11817`). The design's premise is confirmed.
2. **A guest with its own number dies in seconds**: a median of ~100 frames among 160 trained ants. Without a rule
   that takes the bite away from the mind, there is no game.
3. **Giving the guest the host's number (route a) makes it untouchable and occasionally fed**, so "family by
   number" is binary: nothing to earn. Route (a) is rejected for the demo.
4. **The stranger alarm never fires** (`stranger_touches` 0 in every run): minds skip `act()`, and the old ant's
   alarm ignores other kinds.

## Must-fix problems the design and critique both missed

1. **The shipped beetle cannot get into the nest.** It is a 2x2 rigid body (`beetle.ron: body: Rigid`); the ant is a
   2-cell chain (`ant.ron: body: Chain(2)`), and a grown nest is 1-2-cell tunnels crowded with ants and roots
   (ASCII look in the probe, `nestlook=1`: the played bed's nest at frame 30,000 is ~10 rows deep, its galleries 1-2
   cells wide). A 2x2 body has nowhere to stand down there; steered on the surface it stuck in a shrub after 2 cells. **Fix:** a new species, `rove`, with the ant's body and walk and
   the beetle's look, a hand-built brain (no mind: `give_new_ant_minds` only touches species `ant`).
2. **The hook must run before the mind.** The lab runs the trained mind by default (`bin/lab.rs:306`) and a mind ant
   returns into `mind::tick` before `act()` (`creature.rs:8022`). Anything hung on `act()` is dead in the lab. The
   ladder's contact check goes immediately before that return so it serves both brains.
3. **Two bites must be gated, not one.** The trained mind's Attack (offered on any other-colony animal,
   `options.rs`) and the hand-built ant's fight both kill the guest. The rule must remove both on a registered
   visitor, so the ladder's nip is the only harm and it can be graded.
4. **Do not touch the species gate.** The design's decision 2 (loosen `kin_crosses_kinds` or a guest exception in
   `is_living_kin_id`) reaches the mouth, the eye, nursing and the nest blend for every animal. The ladder can
   compute the scent distance itself for registered visitors; beg is a guest-only call, not nursing. Decision 2
   becomes unnecessary for the demo.
5. **The disguise would be won in four rubs and kept forever.** `blend_with_kin` steps 0.1 of the gap per call and
   nothing pulls the guest back. With ants clustered on one nest odour (probe: nest scent stays within ~0.1 of the
   founders' point), a guest 2.5 away would be family after ~10 rubs and stay family. **Fix:** a smaller per-rub
   step, and the guest's scent relaxing back toward its own every tick, so the disguise is upkeep: the core loop.
6. **Ants are nearly identical, so "some guards are pickier" needs a source.** Tolerance spread is ~0.05 per birth
   and trained ants do not mutate in the box. **Fix:** the ladder's dice are per ant (a stable per-id offset), plus
   alarm at the spot and the ant's hunger. Say plainly in the demo notes that pickiness is dice, not a gene.

## Smaller corrections

- A rubbed worker drifting toward the guest is real (symmetric blend) but self-healing: an ant at its own nest
  re-blends at beta 0.1 a tick (`blend_with_nest`), so it washes out within a few dozen ticks. No cap needed; show it.
- The display rung's alarm puff is a ~3 s flash that repels ants a little (keeper and enemy measured it: -7 to -23%
  ant-ticks near a puff). It is a visible mark, not a lasting one. The ladder reads alarm at the spot, so a puff
  makes the next ant pickier: that is the "alarm raises the odds" scene, carried by the rule itself.
- Beg needs a donor with something to give. Mind adults share 50-160 times per 1,000 frames (keeper's count), and
  carry crops; the share should come from crop first, then bank above `start_energy`, as `nurse()` does.
- The steer hooks move any hand-built-brain body (verified: a steered beetle moved, its B bite took brood and ants).
  So the guest needs no new steering, only a Jump field and the seat keys.
- Frame cost: one contact check per ant tick only while a visitor is registered, against a guest of 2 cells. With
  `World::visitors` at `None` it is one Option read per ant tick.

## Owner decisions this leaves (none blocks the demo)

- Body rules deciding behaviour (the ladder, the share) are hand-made by this project's standard. The demo keeps them
  off everywhere but `demo=guest`; whether they ever ship in the lab is the owner's call.
- A parasite in the main colony: the demo runs in its own scenario box.
- Cruelty (eating brood): in the demo, with its cost shown.
