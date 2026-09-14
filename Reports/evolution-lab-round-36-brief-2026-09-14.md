# Round 36 brief — make the food economy readable, and find out where an ant's cost really goes

*Rewritten 2026-09-14 by round 35's coordinator **after the owner answered the
round-35 cards**. The first version of this brief led with performance and was
written before those verdicts existed. It was wrong at the top: round 35's
headline deliverable — the food-economy instruments — **failed on his eye**, and
by this repo's own ethos that outranks everything else queued behind it.*

*Read [`evolution-lab-round-35-2026-09-14.md`](evolution-lab-round-35-2026-09-14.md)
for what landed and [`lanes/evolution-lab-coordinator.md`](lanes/evolution-lab-coordinator.md)
for what binds. **Do not read the whole archive.***

---

## The verdicts this round is built on

Quoted rather than paraphrased, because three of them overturn something.

| card | his words |
|---|---|
| The FOOD page | ***"Nope. I don't understand what these visuals are trying to tell"*** |
| The harvest map | ***"No i cannot really tell. the amber hatch it bad. is it too much to track actual paths and make trail?"*** |
| The same, other bed | ***"Why is the amber hatch drop like a huge box?"*** |
| Empty-handed walking | ***"both is interesting"*** |
| The zoom ladder | ***"get rid of stop 3"*** |
| Rivalry on by default | ***"On"*** |
| Colonies fighting | ***"There have been at least 4 seperate cards asking if I can tell if two colonys are fighting… I want the fighting on. If I watch close I can tell that some ants are dying and the groups are not happily eating next to each other. I am fine with this."*** |

**That last one carries a process finding, and it is the coordinator's fault
rather than any lane's**: four cards asked one question because neither lane
looked at what was already queued. **Before posting a card, read the open
queue.** A second card asking what the first already asked spends the owner's
attention and buys nothing.

---

## Lane A — the food economy, made readable (the lead)

**He asked for this in his own words and what we built does not answer it.** The
instruments are correct — the joules balance, the counters fire — and they are
**not legible**, which is the failure mode this repo's ethos names first.

**Start by reproducing his reading, not by redesigning.** There are two channels
and he almost certainly saw them at once: a **road** drawn per cell where laden
ants walked, and a **harvest map** drawn per **8-cell tile** shaded by face
value taken. *"The amber hatch drop like a huge box"* is the map's tile grid;
*"track actual paths and make trail"* is asking for the road. **Render them
separately before changing anything** — it is entirely possible the road works
and the tiles were burying it, in which case the fix is what to draw rather than
what to build.

Then answer his question directly: **is it too much to track actual paths?** The
road is already per cell and already decaying — say what a true per-ant path
would cost over and above it, in whole-frame terms, and post the comparison
rather than arguing it.

**What the FOOD page has to survive**: a reader who has not read its commit
message. *"I don't understand what these visuals are trying to tell"* is not a
complaint about a chart type, it is a complaint that no row says what it is
**for**. Consider whether the page should answer one sentence — *is this colony
feeding itself?* — before it answers nine.

**And the page now has a headline it was not built to deliver, which may be the
sentence it should lead with**: the colonies forage **2.6%** of what they are
granted and eat **84–95% corpse**
([round 35 §4](evolution-lab-round-35-2026-09-14.md)).

**Keep both arms of the empty-handed walking** — *"both is interesting"*.

**Post early and post often.** This lane is judged entirely by eye, its first
attempt was rejected, and a second rejection costs the round. **Read the open
queue before posting** so you are not the fifth card asking one question.

## Lane B — where an ant's cost actually goes

Round 34 killed the premise the old plan rested on: **there is no knee**. What
replaced it is measured directly rather than by subtraction
([`evolution-lab-knee-2026-09-14.md`](evolution-lab-knee-2026-09-14.md) §4):

> **About half of what an ant costs is not in the creature pass at all.** That
> phase is **45%** of the frame's growth from 0 to 428 ants; the other **55%**
> is the CA sweep over the **29.4 cells per ant per frame** an ant's movement
> leaves dirty.

**This lane is a census first and a fix second.** An ant is a few cells; 29.4 is
several times its own body, and **nobody has looked at what the multiplier is
made of** — its own moves, the cells it wakes by being adjacent, the chunk it
keeps awake, the pheromone it writes. Produce that breakdown before proposing
anything. A fix aimed at the wrong term is the expensive outcome here.

**Pin bed age and the plant bill** in any harness or it measures those instead
of the ant — that is exactly how round 32 produced a knee that was not there.
`examples/antcost.rs` already separates the phases.

**Two traps, both already paid for.** A cost that *vanishes* rather than
shrinking usually means the **work** vanished — find the quantity that says the
subsystem still does its job. And **removing work is not removing cost**: a gate
that skipped 91% of the field's momentum passes made the frame *slower* in 7 of
8 paired runs, because the arithmetic went away and the memory traffic only
moved. **The phase a change is made cheaper against is the whole frame**,
measured paired and alternating.

## Lane C — the pheromones: do they fade too fast, and is all of it wired?

**The owner's question, 2026-09-14, and it is the lane's number one.** There are
three planes in `src/sim/pheromone.rs` — `Channel::A` and `Channel::B` carry
**no semantics** (which is which lives in a species' instinct weights), plus an
**alarm** plane.

**There is a hard ceiling and the margin is thin, which is why the question is
live.** `build_decay_lut` forces every nonzero value strictly downward, so a
cell loses **at least 1 per pass** whatever `DECAY_RHO` says — **255 passes is
the longest any trail can survive unreinforced**, by construction. At
`PHEROMONE_INTERVAL = 12` that is **~3,060 frames against a colony round trip of
roughly 2,200**. About 1.4x, and nothing has re-measured it since the round trip
itself moved.

**It has already failed once, exactly this way.** At interval 4 (~1,000 frames
of ceiling) the colony reached **zero deliveries** with total channel A across
the whole world sitting at **100**. The module's own doc records it.

**And the instrument that set the constants cannot answer his question**, which
is the part to fix first. `trail_following_sweep` runs a **single follower
re-laying its trail every pass**, so decay never has to be *survived* — and
`DECAY_RHO` was set from it at **0.03, deliberately below the literature band of
0.1–0.5**. A real colony lays a cell once and comes back minutes later. **Build
the harness that lays a trail and then leaves it alone**, and read the trail's
life against the round trip.

**The other half he asked for: make sure they are all wired up properly.**
Audit, end to end, and say what you find rather than assuming: which species
author instinct weights against which channel; whether both trail channels have
a live reader and a live writer; whether the alarm plane's readers fire.
**One known-unmeasured site**: `contest.rs`'s `DISPLAY_DEPOSIT` (40, against 240
for a wound) is documented as *"a deposit that decays at `ALARM_RHO`… this verb
has not been measured"* — that arrived with round 35 and nobody has looked at it
since.

**Constants for reference, all in `pheromone.rs` with their derivations**:
`PHEROMONE_INTERVAL 12`, `DIFFUSE 0.25`, `DECAY_RHO 0.03`, `DEPOSIT 40` of 255;
`ALARM_RHO 0.25`, `ALARM_DEPOSIT 240`. **Read those doc comments before touching
a value** — each records what it trades and what was already tried. `DIFFUSE`
in particular is not a free knob: a full mean filter flattens a shared trail's
peak from 153 to 63, and the *height* of a well-used trail against a
lightly-used one **is** the path-selection algorithm.

**If a trail is pinning at 255, halve `DEPOSIT` before touching anything else**
(the module's own P-14).

## Lane D — the economy a live rivalry now sits on

**A stranger is food now, and the constants that price a colony were all
calibrated on a bed where no ant was.** #423's commit names three: **the birth
bar, `colony_ants`, and the starvation balance**. *A correct mechanism at
inherited constants is a regression* — this repo's own rule, pointing straight
at these.

What the switch already moved, paired off against shipped over 12 seeds: deaths
**+99 median**, up on 11 of 12 and down on none; starvation share **−4.3
points**, down on 10 of 12, because killing displaces starving; **population and
births unmoved, medians exactly 0.** The bed absorbs it today — which is why
this is a re-derivation rather than an emergency, and also why it is easy to
leave undone.

Questions worth the lane: is a colony that can eat its neighbours meant to need
the same **birth bar**? Starvation fell because killing replaced it — does that
make the **starvation balance** right, or reveal it was never doing the work?
And `colony_ants` sets how many founders stand together, which at a live dial is
also how big a war party is.

**Gate any change on an order statistic over seeds and keep an `off` arm.
Six seeds is not a sweep.** Two traps this exact work sprang once already, both
in [round 35 §8](evolution-lab-round-35-2026-09-14.md): the `spread=` override
*added* its offset so **`spread=0` was not an off arm**, and **the founding draw
is not stable across engine changes** — one seed's gap moved 0.907 → 2.170 at an
unchanged setting. **Tune on the threshold argument, never on a table of
particular seeds.**

**§Z23 folds in here or into Lane C**, whoever takes the creature line:
`nearest_foe` counts a plant as a foe. **The obvious repair is the wrong one** —
do not test whether the target's species has a `creature` def; gate
`cry_alarm`'s two feeding call sites on the victim being an animal.

## Not a lane — the zoom edit, for the coordinator

***"Get rid of stop 3."*** One-line ladder edit, and it settles a question that
has been open across three rounds. It also **overturns**
`Reports/dead-ends.md`'s `rendering:049`, whose re-test clause reads *"a report
of the soft rung as a problem in play rather than on being asked"* — he has now
reported it. Update the entry, its `screened.tsv` verdict, and
`held-world-zoom-plan-2026-09-13.md` §6, which still records the question as
**ASKED AND NOT YET ANSWERED**.

---

## Standing facts that will cost you time otherwise

- **Read the open review queue before posting a card.** Four cards asked one
  question in round 35.
- **There is no delivery signal for a poke.** `fire_trigger` success,
  `last_run`, and the session's `updated_at` are all uninformative. **Check the
  branch head.** Put anything load-bearing in the repo as well as in the poke.
- **`cargo run --release --example ascii` is a CI gate and is the one a lane's
  local gate list forgets.** Round 35's behavioural default change landed red
  there and nowhere else.
- **`bugindex.py --branches` before filing a bug, never `--check`** — two
  branches filed the same letter on one day.
- **A new `dead-ends.md` entry needs a `screened.tsv` verdict**, or `docscheck`
  goes red and nothing else notices.
- **`cargo test --release` exceeds the 600 s Bash cap** — split `--lib` then
  `--test worldgen --test determinism`. **`cargo build --release` does not
  rebuild examples**; use `--examples` with `set -o pipefail`.
- **Timings in this container are untrustworthy across runs.** Pin
  `RAYON_NUM_THREADS`, or compare arms **inside one run** with the order swapped
  each round. Quote the ratio; the milliseconds do not transfer.
- **Post the picture, do not describe it**, and `review.py get <id>` is the only
  authoritative read.
