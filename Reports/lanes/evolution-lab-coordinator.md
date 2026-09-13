# The evolution lab — coordinator note

*`cargo run --release --bin lab` is the second game: a sealed box of soil under
grow lights where the shipped plants and ants live. Design of record:
[`../evolution-lab-design-guide-2026-08-30.md`](../evolution-lab-design-guide-2026-08-30.md),
with [`../evolution-lab-feasibility-2026-08-30.md`](../evolution-lab-feasibility-2026-08-30.md)
under it.*

**Read this before picking the lab up.** Thirty-one rounds since 2026-08-30.
One to twenty-five are history and moved to
[`../evolution-lab-rounds-archive.md`](../evolution-lab-rounds-archive.md) —
verbatim, priced, and mapped to the three concurrent lines. What stays here is
what still binds.

## What binds on anything you do here

**The owner's standing direction, which reframes the whole programme** (round
three, and it has not been superseded):

> *"Your goals are not tweaking and optimizing evolution now. **Give me the
> tools, data, access to the parameters that need to be tweaked and I do that
> testing myself in the game. That is the game.** If I have access to food,
> water, can cull, can create plants, and creatures, I can figure it out."*

**So: stop balancing, start exposing.** A default that looks wrong is to
**register and report**, never to tune. **Ship new behaviours as default**
(*"ship everything on"*). Reach is not behaviour: nothing born swinging or
listening is `CLAUDE.md`'s second law failing quietly.

**Read the bed at a session, not at a minute** — a few hundred thousand frames,
and a million is several sessions (owner's framing). **The owner's machine is
the ruler, not this container's**: a full box runs at 1–4x there, and every
creature result in archived rounds twelve to eighteen was taken at a few
minutes of play and is true at that length only. `latecensus scenario=…
frames=N` grows the bed, so it is **played by construction**; it does not parse
`ants_at=` (that is `labforage`/`labshot`, and an unknown argument is silently
ignored). **Read it as an order statistic over seeds, never one trajectory** (§Z14) —
adjacent 20,000-frame stops of one run read 3,099 and 16 ants.

**Deliberately not being built yet:** the score and the economy, the guide's
Gate 5. **Gate 2 — does selection have teeth in *this* bed — passes for
creatures** (`creature_arena arm=lethal`: a zeroed brain at 0.0% of animals on
12 of 12 seed-runs — a maximal-effect test licensing only that, and the arena's
own 2.42–3.12x seed noise is why the flight races nulled). **Gate 2 for plants
is untouched.**

**Re-derive file ownership from the open PR list, never from a table in a
note.** `sim::frame::step` is the tick sequence shared by both binaries; its
guard holds a hash taken from the other side of the extraction, so **if it goes
red either a phase moved deliberately (re-take the number) or a phase was added
to one binary's loop and not to `frame::step`**.

**From rounds twenty-five to twenty-eight.** **A one-cell event is unreadable
on a card even ringed and zoomed** — show the stand, the door or the colony
over a long span. **Movement, not stills, is how animals are seen** — but a
*follow camera* ruins a colony card ("shaking gif"), and a scrubbable frame
sequence plays for him where a GIF did not. **Sonnet refuses a brief dense in
genetics vocabulary on a `[bio]` classifier**: use the world's words or run on
Opus. **Conserve tokens** (owner): a lane only for a build he asked for or a
landing needs. **`labforage`, `labstats` and `latecensus` `SUMMARY` lines are
contested by every lane** — keep `main`'s fields first, append yours, never
interleave.

**Not to be re-litigated:** `nectar_only` stays; trophallaxis is a brain output
the genome evolves, never a rule; a queen is three authored values over
existing mechanisms, **never a type the engine knows**; the breeding trade
ships graded only after `GRADED_MAX_SUPPRESSION` is swept; ants are not the
pollinators; the eye is heritable (`TRAIT_SIGHT_RANGE`). **Rest is the absence
of a reason to act — and standing still long enough is now one of those
reasons** (`brain::BrainInput::Stillness`, round 33), the owner completing his
own 2026-09-09 ruling: *"If they never ask to move that is still stuck."*
Direction: *the colony that gardens survives*.

**The perf line's handed-forward list**
([`../evolution-lab-frame-cost-2026-09-01.md`](../evolution-lab-frame-cost-2026-09-01.md)
§18.5): the **~21% in the kernel and rayon**, the moisture pass, then the
pheromone `roundf`, which is **not** behaviour-free. Round 33 closed its
biggest item. **Rebuild the baseline binary after every merge.**

## Rounds twenty-nine to thirty-two, 2026-09-12/13 — archived

*Narratives and records in
[`../evolution-lab-rounds-archive.md`](../evolution-lab-rounds-archive.md).
**What still binds is here.***

**The creature cost is not linear, and the knee is the finding** — an ant costs
**~0.3–0.5 µs below ~400 and ~2.6–2.9 µs above ~600**, so any single µs/ant
figure averages two regimes and describes neither. **Calibrate any
creature-cost harness above 800 ants**; below the knee it reads ~0.3 and looks
broken. The ~1.0 ms floor is measured, not fitted. **It is ant count, not
session age.** The cost is **diffuse — no term over 31%**, so no lever halves
it.

**Nobody here can build the owner's bed** (1000+ long ants against a shipped
median of 111), so **a card is a picture of a different world** — but a bed
**past the knee** from 100 ants is the defect in a box, which beats a replica.
**A scene can fail to contain the defect you are removing**, and then a working
fix and a dead one look identical.

**`review.py get <id>` is the only authoritative read; `inbox` is a filtered
view** — cards absent from `inbox` entirely, and a card with three marker notes
reading as having none. **A card can be archived carrying no stored response
even after he answers**, so write the verdict into the record rather than
pointing at the card. **The queue is for visual evaluations ONLY** (owner
ruling); anything else routes through the coordinator.

**Adding a material silently breaks every census naming materials**
(`spoil` broke five; both misses were in `examples/`, where every measurement
here comes from). **Read a conservation failure as a question about the ruler
before the engine.** **Before filing a bug run `bugindex.py --branches`, not
`--check`.**

**Ask "did it fire at all" of a negative verdict, not only of a harness** —
three idle animations read as failures; the mechanism had reached 8 of 22
animals. **A repair can remove the picture and leave the mechanism, and an
inherited census hides it: census the mechanism, not its consequence.**
**`life_half_life: 40000` survives on a floor, not on population**; removing
death makes the colony hungrier, not larger. **The nest is not where the colony
lives** on two beds in three (#350); **the flitter is caged by the canopy, not
broken in flight** (§Z15).

**A *look* ships default-off pending his eye; *ship everything on* governs
behaviours.** **Verify live** — the zoom buffer panicked in the real app under
xvfb while all 1,687 tests passed, because every test applied the budget before
drawing and `main.rs` did not. **Green CI is not mergeability**
([`../session-programs.md`](../session-programs.md)).

## Round thirty-three, 2026-09-13 — parallelism says no, and the latch

*Record: [`../evolution-lab-round-33-2026-09-13.md`](../evolution-lab-round-33-2026-09-13.md);
landed #391, #392, #395, #396, #398, #400.*

**The creature pass can run across cores and is not worth switching on here**
(#398, ships `ParMode::Off`; exact — `par=verify` names the disagreeing brain
input, 0 mismatches). It costs **3–4% of the frame**: a speculation pays only
if `hit rate x parallel speedup > 1`, here `0.35 x 2.1` on four cores. **Both
terms belong to the box and the bed, not the code** — `antcost par=on,off`
re-runs it elsewhere. **The owner's #1 is still open; the knee is the lead.**

**Animals were not resting, they could not move** (#396). `brain::squash`
returns negative and is clamped to `[0,1]`, so **every degree of "would rather
not" landed on exactly 0.0** — the shipped ant 77.5% of its decision ticks, and
**18.5–21.9% of a bed went quiet and was never seen moving again**. **A pooled
idle rate cannot tell "everyone rests briefly" from "a few froze" — both give
75%; census per animal.** **`live_slots` 846 → 870 re-derived every species'
`mutation_rate`, so `main` before `f9dd3295` is not a valid control arm.**

**`fire_trigger` returning success is not evidence a lane woke** — read
`last_run` on the trigger and `updated_at` on the session; two pokes to an idle
lane recorded no run and were twice reported as delivered. **Before archiving a
session sweep `list_triggers` and delete every trigger bound to it, including
ones the lane armed itself** — one fired into an archived session and errored
where the owner saw it.

**A stalled lane's finished work is the coordinator's to land**; two pokes is
enough. Every conflict across six landings was a **generated** file —
**regenerate it, never `--theirs` the whole file**, which drops its prose.

## The earlier rounds

All of one to thirty-two is in
[`../evolution-lab-rounds-archive.md`](../evolution-lab-rounds-archive.md),
priced per round and mapped to its owning report. **Read the one round, not the
file** — they are concurrent lines braided into one sequence, and knowing which
is yours is most of the saving:

| line | rounds | design of record |
|---|---|---|
| the lab as an instrument — interface, shelf, rosters, persistence, soil, scenarios, forage | 3, 4, 5, 7, 9, 10, 11, 21, 25 | `evolution-lab-gui-physics-2026-08-30.md` |
| frame cost and the speed dial | 2, 6, 8, 17, 18, 19 | `evolution-lab-frame-cost-2026-09-01.md` |
| creatures — groups, kin, armour, castes, verbs, gates, lifespan, fission | 12, 13, 14, 15, 16, 20, 22, 23, 24, 25, 29 | `creature-signature-and-castes-2026-09-06.md`, `evolution-lab-late-game-design-2026-09-12.md` |
| the ecology — fruit, seed, nectar, flowers, the pollinator | 26, 27, 28, 29 | `evolution-lab-ecology-design-2026-09-10.md`, `evolution-lab-flight-design-2026-09-11.md` |

*Cross-cutting rather than on one line: 30 colony survival, 31 the playtest
gap, 32 the creature cost, 33 parallelism and rest.*

## Environment notes that cost time here

- **The container suspends between tool calls**, so a backgrounded job makes
  no progress while the session is idle. **Let CI gate the full suite** — it
  runs on branch pushes as well as pull requests.
- **Every push cancels the in-flight suite and restarts a ~19-minute clock**,
  so batch commits. Jobs all reading "cancelled" two seconds in is the
  concurrency group superseding a push run with a pull_request run.
- `rust-toolchain.toml` pins 1.98 and CI has a build cache, so plain
  `cargo clippy --all-targets --release --locked -- -D warnings` matches CI.
- **The lab window captures with no display** —
  `PIXEL_PHYSICS_SCREENSHOT_AFTER_FRAMES=N` under `xvfb-run` with lavapipe.
  `labshot` renders the *world* and shows no interface; `examples/labui.rs`
  renders the bar headlessly and scripts clicks.
- **`/tmp` is shared between agents in this container.** Set a private
  `TMPDIR`, or one lane captures another's frame.
- **`review.py inbox --mark-seen` marked all 199 cards seen**, not just the
  caller's. Use `get <id>` rather than trusting an empty inbox.
- **Several agents in one container makes every timing untrustworthy** —
  pin `RAYON_NUM_THREADS` or compare arms inside one run.
- **A cloud lane is reached by poke, never by `SendMessage`**, and a poke
  lands at the lane's *next turn boundary* — a lane that ends its turn
  "waiting" is idle until poked. Lanes arm auto-merge on their own PRs;
  disable it at subscription. Delete every poke at the close. Mechanism and
  the four failures it cost: [`../session-programs.md`](../session-programs.md).
