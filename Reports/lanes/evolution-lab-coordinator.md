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
listening is the second law failing quietly.

**Read the bed at a session, not at a minute** — a few hundred thousand frames,
and a million is several sessions (owner's framing). **The owner's machine is
the ruler, not this container's**: a full box runs at 1–4x there, and every
creature result in archived rounds twelve to eighteen was taken at a few
minutes of play and is true only at that length. `latecensus scenario=…
frames=N` grows the bed, so it is **played by construction**. **Read it as an
order statistic over seeds, never one trajectory** (§Z14) — adjacent
20,000-frame stops of one run read 3,099 and 16 ants.

**Deliberately not built yet:** the score and the economy, the guide's Gate 5.
**Gate 2 — does selection have teeth in *this* bed — passes for creatures**
(`creature_arena arm=lethal`, 12 of 12 seed-runs; a maximal-effect test
licensing only that, and the arena's 2.42–3.12x seed noise is why the flight
races nulled). **Gate 2 for plants is untouched.**

**Re-derive file ownership from the open PR list, never from a table in a
note.** `sim::frame::step` is the tick sequence both binaries share; its guard
hashes the sequence, so **red means either a phase moved deliberately (re-take
the number) or a phase was added to one binary's loop and not to
`frame::step`**.

**From rounds twenty-five to twenty-eight.** **A one-cell event is unreadable
on a card even ringed and zoomed** — show the stand, the door or the colony
over a long span. **Movement, not stills, is how animals are seen** — but a
*follow camera* ruins a colony card ("shaking gif"), and a scrubbable sequence
plays for him where a GIF did not. **Sonnet refuses a brief dense in genetics
vocabulary on a `[bio]` classifier**: use the world's words or run Opus.
**Conserve tokens.** **`labforage`, `labstats` and `latecensus`
`SUMMARY` lines are contested** — keep `main`'s fields first, append yours.

**Not to be re-litigated:** `nectar_only` stays; trophallaxis is a brain output
the genome evolves, never a rule; a queen is three authored values over
existing mechanisms, **never a type the engine knows**; the breeding trade
ships graded only after `GRADED_MAX_SUPPRESSION` is swept; ants are not the
pollinators; the eye is heritable (`TRAIT_SIGHT_RANGE`). **Rest is the absence of a reason to act — and standing still long enough is
now one of those reasons** (`brain::BrainInput::Stillness`): *"If they never
ask to move that is still stuck."* Direction: *the colony that gardens
survives*.

**The perf line's handed-forward list**
([`../evolution-lab-frame-cost-2026-09-01.md`](../evolution-lab-frame-cost-2026-09-01.md)
§18.5): the **~21% in the kernel and rayon**, the moisture pass, then the
pheromone `roundf`, which is **not** behaviour-free. **Rebuild the baseline
binary after every merge.**

## Rounds twenty-nine to thirty-four, 2026-09-12/14 — archived

*Narratives in
[`../evolution-lab-rounds-archive.md`](../evolution-lab-rounds-archive.md).
**What still binds is here.***

**THERE IS NO KNEE** (#407,
[`../evolution-lab-knee-2026-09-14.md`](../evolution-lab-knee-2026-09-14.md)),
which overturns round 32. Pin the two things that were moving under the ant
axis — **bed age, and plants paying the ants' bill** — and one line fits **0 to
793 ants at 3.4–4.3 µs/ant**, right across the supposed threshold. **Pin them
in any creature-cost harness or it measures them instead.** **The redirect,
and the owner's #1: about half of what an ant costs is not in the creature
pass** — that phase is **45%** of the frame's growth to 428 ants, and the CA
sweep over the **29.4 cells an ant dirties per frame** is the other **55%**.
The cost is **diffuse, no term over 31%**, so no lever halves it. Relatedly,
**the creature pass runs across cores and is not worth switching on here**
(#398, ships `ParMode::Off`, 3–4% of the frame): a speculation pays only if
`hit rate x parallel speedup > 1`, here `0.35 x 2.1` — **both terms belong to
the box and the bed, not the code.**

**Nobody here can build the owner's bed** (1000+ long ants against a shipped
median of 111), so **a card is a picture of a different world** — but the
defect in a box beats a replica. **A scene can fail to contain the defect you
are removing**, and a working fix and a dead one then look identical.

**`review.py get <id>` is the only authoritative read; `inbox` is a filtered
view** — cards vanish from it entirely, and one with three marker notes read as
having none. **A card can be archived carrying no stored response even after he
answers**, so write the verdict into the record, not a pointer to it. **The
queue is for visual evaluations ONLY** (owner ruling).

**A pooled rate cannot tell "everyone does it briefly" from "a few are stuck" —
census per individual.** That hid #396 (every degree of "would rather not"
clamped to exactly 0.0; **18.5–21.9% of a bed went quiet for good**) and it is
the same shape as round 35's forager distributions. **A repair can remove the
picture and leave the mechanism: census the mechanism, not its consequence.**
**Read a conservation failure as a question about the ruler before the
engine.** **`life_half_life: 40000` survives on a floor, not population**; **the nest is
not where the colony lives** on two beds in three (#350).

**A *look* ships default-off pending his eye; *ship everything on* governs
behaviours.** **Verify live** — the zoom buffer panicked in the real app under
xvfb while 1,687 tests passed, because every test applied the budget before
drawing and `main.rs` did not.

**A stalled lane's finished work is the coordinator's to land.** Every conflict
across six landings was a **generated** file — **regenerate it, never
`--theirs` a whole file**, which drops its prose.

## Round thirty-five, 2026-09-14 — the live round

*Record:
[`../evolution-lab-round-35-2026-09-14.md`](../evolution-lab-round-35-2026-09-14.md).
Landed #416, #417, #419, #420.*

**A STRANGER IS ALREADY FOOD.** `ant` material is `food_class: 1.0` against the
shipped neutral gut, so two colonies outside each other's tolerance eat each
other through the **ordinary mouth** — `eats` **54 → 750** from nothing but kin
recognition. **Turning rivalry on produces predation, not war.**

**Only `Behavior::scent_spread` is binding, and it is a threshold, not a
slope**: the acceptance radius is `tolerance + 1`, so `spread = 1` makes
strangers of **9.3%** of ordered pairs and **one seed in four never meets**. It
saturates by **2**, so **a dial topping out at 1 ships a mechanism a third of
beds never show.**

**An encounter is no longer a bite** (`src/sim/contest.rs`): escalation
**1.000 → 0.520**, the rest withdrawals that *display* into the alarm plane.
**No setting makes an animal unattackable** (`COMMIT_FLOOR`).

**Food has a face** — per-colony books in joules (#419), a per-cell road and
per-tile harvest map on `F7` (#420). **The two food numbers disagree on
purpose**: the books price a mouthful by the eater's gut, the map prices what
left the world at that tile. **Both overlays decay every tick, so they defeat
the dirty-rect skip and the SETTLED bed is the price** (+88–100%).

**Open, and he ruled on it:** *"You can ship it on"* — **nothing in the round
implements it**, and it is now a choice of *value*: the constants it
reallocates want re-deriving, on a seed sweep gating an order statistic.
**§Z23** is open, repair designed.

## The earlier rounds

All of one to thirty-four is in
[`../evolution-lab-rounds-archive.md`](../evolution-lab-rounds-archive.md),
priced per round. **Read the one round, not the file** — they are concurrent
lines braided into one sequence, and knowing which is yours is most of the
saving:

| line | rounds | design of record |
|---|---|---|
| the lab as an instrument — interface, shelf, rosters, persistence, soil, scenarios, forage | 3, 4, 5, 7, 9, 10, 11, 21, 25 | `evolution-lab-gui-physics-2026-08-30.md` |
| frame cost and the speed dial | 2, 6, 8, 17, 18, 19 | `evolution-lab-frame-cost-2026-09-01.md` |
| creatures — groups, kin, armour, castes, verbs, gates, lifespan, fission | 12, 13, 14, 15, 16, 20, 22, 23, 24, 25, 29 | `creature-signature-and-castes-2026-09-06.md`, `evolution-lab-late-game-design-2026-09-12.md` |
| the ecology — fruit, seed, nectar, flowers, the pollinator | 26, 27, 28, 29 | `evolution-lab-ecology-design-2026-09-10.md`, `evolution-lab-flight-design-2026-09-11.md` |

*Cross-cutting rather than on one line: 30 colony survival, 31 the playtest
gap, 32 the creature cost, 33 parallelism and rest, 34 the knee that was not
there.*

## Environment notes that cost time here

- **The container suspends between tool calls**, so a backgrounded job makes
  no progress while the session is idle. **Let CI gate the full suite** — it
  runs on branch pushes as well as pull requests.
- **Every push cancels the in-flight suite and restarts a ~19-minute clock**,
  so batch commits. Jobs reading "cancelled" two seconds in is the concurrency
  group superseding the push run with the pull_request run.
- `rust-toolchain.toml` pins 1.98 and CI has a build cache, so plain
  `cargo clippy --all-targets --release --locked -- -D warnings` matches CI.
- **The lab window captures with no display** —
  `PIXEL_PHYSICS_SCREENSHOT_AFTER_FRAMES=N` under `xvfb-run` with lavapipe.
  `labshot` renders the *world* and no interface; `examples/labui.rs` renders
  the bar headlessly and scripts clicks.
- **`/tmp` is shared between agents in this container.** Set a private
  `TMPDIR`, or one lane captures another's frame.
- **`review.py inbox --mark-seen` marked all 199 cards seen**, not just the
  caller's. Use `get <id>` rather than trusting an empty inbox.
- **Several agents in one container makes every timing untrustworthy** —
  pin `RAYON_NUM_THREADS` or compare arms inside one run.
- **A cloud lane is reached by poke, never by `SendMessage`**, and a poke
  lands at the lane's *next turn boundary*. Delete every poke at the close,
  **including ones a lane armed itself** — one fired into an archived session
  and errored where the owner saw it.
  [`../session-programs.md`](../session-programs.md).
- **THERE IS NO DELIVERY SIGNAL FOR A POKE.** Round 33 said to read `last_run`
  and the session's `updated_at`; round 35 measured that **neither is
  evidence** — `last_run` is absent for pokes that did arrive, `updated_at`
  moves for the lane's own work. **Say a message was sent, never that a lane
  was contacted**; the only check that works is the **branch head**. Put
  anything load-bearing in the repo as well as in the poke.
