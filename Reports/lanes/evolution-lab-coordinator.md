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

**Read the bed at a session, not at a minute** — a few hundred thousand frames
(owner's framing), and **his machine is the ruler, not this container's**.
`latecensus scenario=… frames=N` grows the bed, so it is **played by
construction**. **Read it as an order statistic over seeds, never one
trajectory** (§Z14) — adjacent 20,000-frame stops of one run read 3,099 and 16
ants.

**Deliberately not built yet:** the score and the economy, the guide's Gate 5.
**Gate 2 — does selection have teeth in *this* bed — passes for creatures**
(`creature_arena arm=lethal`, 12 of 12) **and is untouched for plants.**

**Re-derive file ownership from the open PR list, never from a table in a
note.** `sim::frame::step` is the tick sequence both binaries share; its guard
hashes the sequence, so **red means either a phase moved deliberately (re-take
the number) or a phase was added to one binary's loop and not to
`frame::step`**.

**On cards:** **a one-cell event is unreadable even ringed and zoomed** — show
the stand, the door or the colony over a long span; **movement, not stills, is
how animals are seen**, but a *follow camera* ruins a colony card ("shaking
gif") and a scrubbable sequence plays where a GIF did not. **Sonnet refuses a
brief dense in genetics vocabulary**: use the world's words or run Opus.
**`labforage`, `labstats` and `latecensus` `SUMMARY` lines are contested** —
keep `main`'s fields first, append yours.

**Not to be re-litigated:** `nectar_only` stays; trophallaxis is a brain output
the genome evolves, never a rule; a queen is three authored values over
existing mechanisms, **never a type the engine knows**; the breeding trade
ships graded only after `GRADED_MAX_SUPPRESSION` is swept; ants are not the
pollinators; the eye is heritable (`TRAIT_SIGHT_RANGE`). **Rest is the absence of a reason to act — and standing still long enough is
now one of those reasons** (`brain::BrainInput::Stillness`): *"If they never
ask to move that is still stuck."* Direction: *the colony that gardens
survives*.

**Rebuild the baseline binary after every merge.** The perf line's older
handed-forward list is in
[`../evolution-lab-frame-cost-2026-09-01.md`](../evolution-lab-frame-cost-2026-09-01.md)
§18.5; round 36's section below supersedes its ant half.

## Rounds twenty-nine to thirty-four, 2026-09-12/14 — archived

*Narratives in
[`../evolution-lab-rounds-archive.md`](../evolution-lab-rounds-archive.md).
**Round 36 supersedes the ant-cost half** — the knee is still gone, but 29.4
was never an ant's number. What still binds:*

**THERE IS NO KNEE** (#407). Pin **bed age and plants paying the ants' bill** in
any creature-cost harness or it measures those instead. **The creature pass runs
across cores and is not worth switching on here** (#398, ships `ParMode::Off`,
3–4%): a speculation pays only if `hit rate x parallel speedup > 1`, here
`0.35 x 2.1` — **both terms belong to the box and the bed, not the code.**

**Nobody here can build the owner's bed** (1000+ long ants against a shipped
median of 111), so **a card is a picture of a different world** — and **a scene
can fail to contain the defect you are removing**, at which point a working fix
and a dead one look identical.

**`review.py get <id>` is the only authoritative read; `inbox` has lost cards
outright**, and a card can be archived carrying no stored response even after he
answers — **write the verdict into the record, not a pointer to it**. **The
queue is for visual evaluations ONLY** (owner ruling). **A card whose OPTIONS do
not match its QUESTION does not answer it** — round 36 asked "what should an
alarm mean?" over before/after panes of a different change; the click endorsed
the change and settled nothing.

**A pooled rate cannot tell "everyone does it briefly" from "a few are stuck" —
census per individual.** **A repair can remove the picture and leave the
mechanism.** **Read a conservation failure as a question about the ruler before
the engine.** **`life_half_life: 40000` survives on a floor, not population**;
**the nest is not where the colony lives** on two beds in three (#350). **A
*look* ships default-off pending his eye; *ship everything on* governs
behaviours.** **Verify live** — the zoom buffer panicked in the real app under
xvfb while 1,687 tests passed.

**A stalled lane's finished work is the coordinator's to land**, and every
conflict across six landings was a **generated** file: **regenerate it, never
`--theirs` a whole file**, which drops its prose.

## Round thirty-six, 2026-09-14/15 — the live round

*Record: [`../evolution-lab-round-36-2026-09-14.md`](../evolution-lab-round-36-2026-09-14.md).
Next: [`../evolution-lab-round-37-brief-2026-09-15.md`](../evolution-lab-round-37-brief-2026-09-15.md).
Landed #431, #432, #433, #436, #440, #441, #442, #444, #445, #446, #447.*

**ANTS WERE EATING THE FOREST AND THE JAW WAS 100% OF IT.** Grazing a plant
raised an alarm; alarm is the only route to attacking; `nearest_foe` counted
the plant as the foe. Both owner rulings shipped (#440), reopening condition in
`dead-ends.md` `creatures:082`, and **`PIXEL_PHYSICS_PLANT_FOE=on` restores it
byte-identically** so the A/B is one binary.

**TWO NUMBERS WERE RETRACTED BY THE LANES THAT MADE THEM, after being quoted
onward.** *"29.4 cells per ant"* is **per-bed** (1.75x across three seeds; 21.7
vs 13.3 on one seed at two bed ages). The plant fix did **not** raise plants
standing: 243→275, 161→**97**, 190→187, **median −3**. **It removes the jaw's
pure loss; what happens next is set by what the colony does with the freed
energy** — seed 1 stays flat and recovers to the unhunted control, seed 2
nearly doubles and grazing replaces the jaw. **Seed 2 is the birth bar's case.**

**A STANDING CENSUS CANNOT MEASURE A FLOW**: the paired standing-plant count
moved **−84, +3,463, +3,484**, one arm reading *more* plant with the colony on.
Count where the cell leaves the world.

**PERFORMANCE: THREE ROUNDS, NO SHIPPED WIN, AND A KILL CONDITION NOW EXISTS.**
Single-rect reach narrowing is **1.9%** because at reach 24 on a 64-wide chunk
**the rect is already full width** — the prize is a **shape** prize, never
priced at whole-frame. **Round 37 is the last attempt; under ~5% whole-frame
and the line closes** (owner, 2026-09-15). §E2 is now **frame 237, four soil
cells, the soil-moisture channel**. Located, not diagnosed.

**THE TRAIL IS DIAGNOSED, NOT FIXED.** `DECAY_RHO` is **inert**; `DIFFUSE` is
the lever and is not free (a mean filter flattens a shared trail's peak 153→63,
and that height *is* path selection). **Channel A is 3,421 at frame 10,000 and
0–9 from 20,000 on; laden ants are 0–10% at the food; cutting the homing
circuit out of the genome leaves deliveries inside the noise.** Every trail
constant still ships unchanged. **The ALARM plane did change** (#442): a shout
is not a substance, so it spreads as an active space (`ALARM_RHO` 0.35,
`ALARM_FALL` 12) — its audible radius had been **two cells**.

**THE NEST IS RESEARCHED, NOT BUILT** (#446). **A site, not a material** — the
gap is one function. **NOT a blob**: a wider footprint **kills the colony
monotonically on 3 of 3 seeds**, and the case it defends never arises (0/0/4
cells lost over 120k frames). **Real ants home by path integration**, corrected
at short range by nest odour — which is what `AtNest` already is. **`nest`'s
resistance 6.0 against every `dig_force` 1.0 means a colony cannot dig its own
doorstep.** **Do not quote "414 deliveries"** — that scene places 15 of 55
ants and has no channel A by frame 6,000.

**UNRULED, AND THE OWNER'S:** the alarm semantics. Shipped as *a living animal
bitten, whichever verb*, **inert on his bed**. Not his decision.

**A POKE CROSSED A LANDING** — the reassignment handing `creature.rs` to Lane E
fired eleven minutes after Lane D opened a PR on the same fix. **Read the branch
head before you write a poke, not after.**

**Model tally**: five Opus lanes, two retracting their own headline unprompted;
one Fable research lane that argued against the owner with a measurement and
found **two errors in its own brief**. With round 30's one bad Fable point:
one each way. Keep counting.

## The earlier rounds

All of one to thirty-five is in
[`../evolution-lab-rounds-archive.md`](../evolution-lab-rounds-archive.md),
priced per round and **mapped there to the concurrent lines they braid**.
**Read the one round, not the file**; the archive's own table says which
concurrent line each belongs to.

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
