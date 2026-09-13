# The evolution lab — coordinator note

*`cargo run --release --bin lab` is the second game: a sealed box of soil under
grow lights where the shipped plants and ants live. Design of record:
[`../evolution-lab-design-guide-2026-08-30.md`](../evolution-lab-design-guide-2026-08-30.md),
with [`../evolution-lab-feasibility-2026-08-30.md`](../evolution-lab-feasibility-2026-08-30.md)
under it.*

**Read this before picking the lab up.** Twenty-nine rounds have been run here
since 2026-08-30. One to twenty-five are history and moved (2026-09-08, 2026-09-09, 2026-09-10) to
[`../evolution-lab-rounds-archive.md`](../evolution-lab-rounds-archive.md) —
verbatim, with a priced contents table and a per-round map of which round belongs
to which of the three concurrent lines. What stays here is what still binds.

## What binds on anything you do here

**The owner's standing direction, which reframes the whole programme** (round
three, and it has not been superseded):

> *"Your goals are not tweaking and optimizing evolution now. **Give me the
> tools, data, access to the parameters that need to be tweaked and I do that
> testing myself in the game. That is the game.** If I have access to food,
> water, can cull, can create plants, and creatures, I can figure it out."*

**So: stop balancing, start exposing.** A default that looks wrong is something
to **register and report**, never to tune. **Ship new behaviours as default**
(round twenty; the owner in round twenty-nine: *"ship everything on"*). Reach is not behaviour: nothing born
swinging or listening is `CLAUDE.md`'s second law failing quietly.

**Read the bed at a session, not at a minute.** The owner's own framing — a
session is a few hundred thousand frames and a million is several sessions.
Every creature result in archived rounds twelve to eighteen was taken at a few
minutes of play and is true at that length only — **the owner's machine is the
ruler, not this container's**: a full box runs at 1–4x there, so 300,000
frames is a long session. A session census costs six to eight minutes a bed on
one core, so it is the cheap default. **Run it on the played bed** (`ants_at=`,
round twenty-five): the frame-0 bed is the harness talking. **And read it as an
order statistic over seeds, never as one trajectory** (§Z14, round
twenty-nine): the bed is chaotic enough that adjacent 20,000-frame stops of one
run read 3,099 and 16 ants, and a single-seed series cannot be compared across
any change that perturbs behaviour at all. [`../open-bugs-handoff.md` §Z6](../open-bugs-handoff.md)
carries the starvation table every fix has to clear.

**Deliberately not being built yet:** the score and the economy, the guide's
Gate 5. **Gate 2 — does selection have teeth in *this* bed — passes for
creatures**: `creature_arena arm=lethal`, a zeroed brain against the shipped
one, six seeds, two beds, puts the zeroed brain at **0.0% of animals on 12 of
12 seed-runs**. It is a maximal-effect test and licenses only that; the
arena's own 2.42–3.12x seed noise is why the flight races nulled. **Gate 2 for
plants is untouched.**

**Re-derive file ownership from the open PR list, never from a table in a
note.** `sim::frame::step` is the tick sequence shared by both binaries, and
its guard `frame_step_matches_the_sequence_app_update_ran_before_extraction`
holds a hash taken from the other side of the extraction — **if it goes red
either a phase moved deliberately (re-take the number) or a phase was added to
one binary's loop and not to `frame::step`**.

**The perf line's handed-forward list** (archived round nineteen,
[`../evolution-lab-frame-cost-2026-09-01.md`](../evolution-lab-frame-cost-2026-09-01.md)
§18.5): the **~21% in the kernel and rayon**, then the moisture pass, then the
pheromone `roundf`, which is **not** behaviour-free. **Rebuild the baseline binary after every merge.**

## Round twenty-nine, 2026-09-12 — the bed comes back, and the colony learns to die

*Record: [`../evolution-lab-round-29-2026-09-12.md`](../evolution-lab-round-29-2026-09-12.md);
designs of record
[`../evolution-lab-flight-design-2026-09-11.md`](../evolution-lab-flight-design-2026-09-11.md),
[`../evolution-lab-late-game-design-2026-09-12.md`](../evolution-lab-late-game-design-2026-09-12.md),
[`../evolution-lab-fission-design-2026-09-12.md`](../evolution-lab-fission-design-2026-09-12.md).
Pointer and what binds.*

**Landed:** eighteen PRs — the flitter flies; the seed is cargo; the nest door
drains; one odour per nest with drift on; ants die of age.

**What binds** (the round's detail is in its record; these are the rulings
that outlive it). **The played bed's baseline moved on 2026-09-12**: nest
scent drift at 0.15 changes the second half of every large-colony session, so
**every played-bed population figure past ~100,000 frames taken before #347 is
stale**, the late-game design's §0 among them. **The dial is inert on today's
trunk** (peaks 12 / 12 / 212 on seeds 1–3), and **a control shorter than the
mechanism's onset proves nothing.** **The nest is not where the colony lives**
on two beds in three (#350): budding, cohesion and the owner's "time away
turns enemy" all wait on that, and the shipped drift value is his call. **The
flitter is caged by the canopy, not broken in flight** (§Z15) — a design lane
from `climbable`. **Owner verdicts:** seed cargo and the lifespan arm chosen;
**the long-ant pile is not visibly fixed** (the fix arm read worse), which is
still open; a follow camera ruins a colony card. **A card paired against a
census passes `rain=off`, and its `meta` is measured in the window it shows.**

## Round thirty, 2026-09-12 — the screen quiets, and both colony lanes were counting the wrong thing

*Record: [`../evolution-lab-round-30-2026-09-12.md`](../evolution-lab-round-30-2026-09-12.md).*
**What binds.** **Survival was in neither lane's sweep, and it is the variable
that moves.** 2 of 384 deaths are attributable to an attacker and two fifths
are a plant written into the ant's head (§Z16 — write site now diagnosed, the
digestion exit, repair taken by r29); the dig gate does not shrink the mound
but keeps the colony alive on **5 beds of 12 against 0** (#359). **Green CI is
not mergeability**, and the harness rules this round paid for are in
[`../session-programs.md`](../session-programs.md).

**Landed:** #344–#346, #352, #355, #359, #362, #363.

**Round thirty-one's brief is written:**
[`../evolution-lab-round-31-brief-2026-09-13.md`](../evolution-lab-round-31-brief-2026-09-13.md)
— five tasks, ordered, checked against `16bab295`. Read it before the list below.

**Round thirty-one, top two.** **Floating debris, §Z18** — named on two
unrelated cards, no lane owns it, in every picture of this bed. **The MENU page
reads as a list** — but the rows are **already clickable**; `Body::Choice` just
draws pixel-identical to an information row with an invisible tap target. Give
`Choice` a drawn treatment (fixes every page using it), then two columns.

**Before filing a bug run `python3 scripts/bugindex.py --branches`, not
`--check`.** `--check` reads one working tree, so it passes on a letter
already live on an unlanded branch — that is how §Z16 got filed twice.
`--branches` sweeps every fetched ref for the next free number.

## Rounds twenty-five to twenty-eight, 2026-09-09 → 2026-09-11

*Records and designs: the earlier-rounds table below.*

**What binds.** `nectar_only` stays: a plant specialist's mouth eats the plant
and no gut setting avoids it. **A card of a one-cell event is unreadable even
ringed and zoomed** — show the stand, the door or the colony over a long span,
or let the playtest judge. **Sonnet refuses a brief dense in genetics
vocabulary on a `[bio]` classifier**: write it in the world's words or run the
lane on Opus. Trophallaxis is a brain output the genome can evolve, never a
rule; **rest is the absence of a reason to act**; a queen is three authored
values over mechanisms that exist, **never a type the engine knows**;
**movement, not stills, is how animals are seen** — but a *follow camera* ruins a colony-level card (2026-09-12: two unreadable, "shaking gif"), and a scrubbable frame sequence plays for him where a GIF did not. The direction is *the
colony that gardens survives*; the breeding trade is graded (`e5792206`) and
ships graded only after `GRADED_MAX_SUPPRESSION` is swept. Ants are not the
pollinators; the eye is heritable (`TRAIT_SIGHT_RANGE`). **Conserve tokens**
(owner): a lane only for a build the owner asked for or a landing needs; a
question that needs no visual is asked in chat, not the queue. The `SUMMARY`
lines of `labforage`, `labstats` and `latecensus` are contested by every lane —
keep `main`'s fields first, append yours, never interleave.

## The earlier rounds

Rounds one to twenty-five are verbatim in
[`../evolution-lab-rounds-archive.md`](../evolution-lab-rounds-archive.md),
which prices each one and maps it to its owning report; twenty-six to
twenty-nine have their own records, linked above. **Read the one round,
not the file** — they are three concurrent lines braided into one sequence, and
knowing which is yours is most of the saving:

| line | rounds | design of record |
|---|---|---|
| the lab as an instrument — interface, shelf, rosters, persistence, soil, scenarios, forage | 3, 4, 5, 7, 9, 10, 11, 21, 25 | `evolution-lab-gui-physics-2026-08-30.md` |
| frame cost and the speed dial | 2, 6, 8, 17, 18, 19 | `evolution-lab-frame-cost-2026-09-01.md` |
| creatures — groups, kin, armour, castes, verbs, gates, lifespan, fission | 12, 13, 14, 15, 16, 20, 22, 23, 24, 25, 29 | `creature-signature-and-castes-2026-09-06.md`, `evolution-lab-late-game-design-2026-09-12.md` |
| the ecology — fruit, seed, nectar, flowers, the pollinator | 26, 27, 28, 29 | `evolution-lab-ecology-design-2026-09-10.md`, `evolution-lab-flight-design-2026-09-11.md` |

## Environment notes that cost time here

- **The container suspends between tool calls**, so a backgrounded job makes
  no progress while the session is idle. **Let CI be the gate on the full
  suite** — it runs on branch pushes as well as pull requests.
- **Every push cancels the in-flight suite and restarts a ~19-minute clock**,
  so batch commits. A run whose jobs all read "cancelled" two seconds in is
  the concurrency group superseding a push run with a pull_request run.
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
