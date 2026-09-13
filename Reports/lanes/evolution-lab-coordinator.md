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

**So: stop balancing, start exposing.** A default that looks wrong is something
to **register and report**, never to tune. **Ship new behaviours as default**
(round twenty; the owner in round twenty-nine: *"ship everything on"*). Reach is not behaviour: nothing born
swinging or listening is `CLAUDE.md`'s second law failing quietly.

**Read the bed at a session, not at a minute.** A session is a few hundred
thousand frames and a million is several sessions (owner's framing). Every
creature result in archived rounds twelve to eighteen was taken at a few
minutes of play and is true at that length only — **the owner's machine is the
ruler, not this container's**: a full box runs at 1–4x there. A session census
costs six to eight minutes a bed on one core, so it is the cheap default, and
**`latecensus scenario=… frames=N` grows the bed, so it is played by
construction** — it does *not* parse `ants_at=` (that is `labforage`/`labshot`,
and an unknown argument here is silently ignored; this note said otherwise
until r31). **Read it as an order statistic over seeds, never as one
trajectory** (§Z14): adjacent 20,000-frame stops of one run read 3,099 and 16
ants, so a single-seed series cannot be compared across any change that
perturbs behaviour at all.

**Deliberately not being built yet:** the score and the economy, the guide's
Gate 5. **Gate 2 — does selection have teeth in *this* bed — passes for
creatures**: `creature_arena arm=lethal` puts a zeroed brain at **0.0% of
animals on 12 of 12 seed-runs**. A maximal-effect test, licensing only that;
the arena's own 2.42–3.12x seed noise is why the flight races nulled. **Gate 2
for plants is untouched.**

**Re-derive file ownership from the open PR list, never from a table in a
note.** `sim::frame::step` is the tick sequence shared by both binaries, and
its guard `frame_step_matches_the_sequence_app_update_ran_before_extraction`
holds a hash taken from the other side of the extraction — **if it goes red
either a phase moved deliberately (re-take the number) or a phase was added to
one binary's loop and not to `frame::step`**.

**From rounds twenty-five to twenty-eight** (records in the table below). **A
one-cell event is unreadable on a card even ringed and zoomed** — show the
stand, the door or the colony over a long span. **Movement, not stills, is how
animals are seen** — but a *follow camera* ruins a colony card ("shaking gif"),
and a scrubbable frame sequence plays for him where a GIF did not. **Sonnet
refuses a brief dense in genetics vocabulary on a `[bio]` classifier**: use the
world's words or run on Opus. **Conserve tokens** (owner): a lane only for a
build he asked for or a landing needs. **`labforage`, `labstats` and
`latecensus` `SUMMARY` lines are contested by every lane** — keep `main`'s
fields first, append yours, never interleave. And, not to be re-litigated:
`nectar_only` stays (a plant specialist's mouth eats the plant, no gut setting
avoids it); trophallaxis is a brain output the genome evolves, never a rule;
**rest is the absence of a reason to act**; a queen is three authored values
over existing mechanisms, **never a type the engine knows**; the breeding trade
ships graded only after `GRADED_MAX_SUPPRESSION` is swept; ants are not the
pollinators; the eye is heritable (`TRAIT_SIGHT_RANGE`). Direction: *the colony
that gardens survives*.

**The perf line's handed-forward list** (archived round nineteen,
[`../evolution-lab-frame-cost-2026-09-01.md`](../evolution-lab-frame-cost-2026-09-01.md)
§18.5): the **~21% in the kernel and rayon**, then the moisture pass, then the
pheromone `roundf`, which is **not** behaviour-free. **Rebuild the baseline binary after every merge.**

## Rounds twenty-nine and thirty, 2026-09-12 — archived

*Both records moved to [`../evolution-lab-rounds-archive.md`](../evolution-lab-rounds-archive.md)
on 2026-09-13. Round 29 is 18 PRs and three designs; round 30 is colony
survival. **What still binds is here; the narrative is there.***

**Every played-bed figure past ~100,000 frames taken before #347 is stale**
(nest scent drift 0.15), and **a control shorter than the mechanism's onset
proves nothing**. **The nest is not where the colony lives** on two beds in
three (#350) — budding, cohesion and "time away turns enemy" all wait on that.
**The flitter is caged by the canopy, not broken in flight** (§Z15). **The
long-ant pile is not visibly fixed** — owner's verdict, and the fix arm read
*worse*. **A card paired against a census passes `rain=off`, and its `meta` is
measured in the window it shows.** **Survival was in neither colony lane's
sweep and it is the variable that moves.** **Green CI is not mergeability**
([`../session-programs.md`](../session-programs.md)).
## Round thirty-one, 2026-09-13 — the bed the complaints come from does not exist here

*Record: [`../evolution-lab-round-31-2026-09-13.md`](../evolution-lab-round-31-2026-09-13.md);
brief `../evolution-lab-round-31-brief-2026-09-13.md`. Landed #370–#376.*

**What binds, and the first governs the rest.** **Nobody here can build the
owner's bed, so a card is a picture of a different world.** He plays 1000+ long
ants; the shipped bed's median at 200,000 frames is **111**, max over 75 runs
**408** (whole sweep 1,067, once). His verdict on the most developed nest
anyone has rendered — 74 ants — was *"none of this reads as an ant hill... just
herbs growing in dirt"*. **Floating debris, the stripped ground that never
recovers and the resting-ants complaint are all reports from a bed never
reproduced here**, so a null from a card means nothing until one exists.
`CLAUDE.md`'s *check the scene still contains the situation* has a second half:
**a scene can fail to contain a defect you are trying to remove**, and then a
working fix and a dead one look identical. **Round 32's task 1**; everything
visual waits on it.

**The chronicle is the way in** (#374) — it records the player's own actions
and whether the box was slow, and autosaves. **One played session, the newest
`.txt` under `assets/chronicles/`, is what a reproducing bed is built from.**

**`life_half_life: 40000` survives on a floor, not on population** (#376, 75
runs): against an immortal colony it moves nothing (p ≥ 0.39 everywhere),
halving to 20,000 kills 4 of 12 colonies. **§Z6 overturned as written** — 26 of
27 runs hold a colony at 200,000 frames — left OPEN narrower. **The dig gate is
rewritten**: not #359's *the colony lives* (12 of 12 both arms now) but *the
bed stays green*, 10 of 12 seeds, p = 0.039. **Removing death makes the colony
hungrier, not larger:** starvation +58%.

**Adding a material silently breaks every census that names materials.**
`spoil` broke **five**; the lane found three, CI the rest, and **both misses
were in `examples/`**, where every measurement here comes from. Grep the
*pairs* — any identity naming a material set on both sides of an equals sign —
and **read a conservation failure as a question about the ruler before the
engine**: both readings fit the number and prescribe opposite work.

**`review.py inbox` is not a listing of the queue — it is a filtered view, and
reading a verdict off it gives wrong answers.** Measured 2026-09-13: three
cards, two of them posted that day, are **absent from `inbox` entirely** while
`get <id>` returns them in full. Off `inbox` the round-29 resting card reads as
having no annotations; `get` returns **three marker coordinates with notes**.
**So `get <id>` is the only authoritative read** — this is the second
independent reason for that rule, after the `--mark-seen` incident.
**A card can also be archived carrying no stored response at all** even after
the owner has answered it: round 31's idle card (`20260913T034419970Z-34d562`)
is archived with no comment and no annotations, though he gave a verdict and
placed three markers. **When that happens the verdict he relayed in chat is the
only copy, so write it into the register rather than pointing at the card.**

**Ask "did it fire at all" of a negative verdict, not only of a harness.** All
three idle animations read as failures and the mechanism had mostly not run —
its delay counted draw calls, so **8 of 22** long ants animated in the window
he judged (21 of 22 fixed), live in the real game too. **The check is not "is
the effect too subtle" but "how many animals did it reach".**

**The review queue is for visual evaluations ONLY.** Owner ruling, 2026-09-13,
on a card that asked him to send a chronicle file: *"This is a bad use of the
review tool. This is just for needed visual evaluations. General questions or
requests should be sent to the coordinating agent to tell me."* So a lane that
wants a file, an answer, a preference or a decision **routes it through the
coordinator, who asks in chat**. A card is for something he has to *look* at.

**Before filing a bug run `python3 scripts/bugindex.py --branches`, not
`--check`**: `--check` reads one working tree and passes on a letter already
live on an unlanded branch, which is how §Z16 got filed twice.

## The earlier rounds

Rounds one to twenty-five are verbatim in
[`../evolution-lab-rounds-archive.md`](../evolution-lab-rounds-archive.md),
which prices each one and maps it to its owning report; twenty-six to
thirty-one have their own records, the last three linked above. **Read the one
round, not the file** — they are three concurrent lines braided into one
sequence, and knowing which is yours is most of the saving:

| line | rounds | design of record |
|---|---|---|
| the lab as an instrument — interface, shelf, rosters, persistence, soil, scenarios, forage | 3, 4, 5, 7, 9, 10, 11, 21, 25 | `evolution-lab-gui-physics-2026-08-30.md` |
| frame cost and the speed dial | 2, 6, 8, 17, 18, 19 | `evolution-lab-frame-cost-2026-09-01.md` |
| creatures — groups, kin, armour, castes, verbs, gates, lifespan, fission | 12, 13, 14, 15, 16, 20, 22, 23, 24, 25, 29 | `creature-signature-and-castes-2026-09-06.md`, `evolution-lab-late-game-design-2026-09-12.md` |
| the ecology — fruit, seed, nectar, flowers, the pollinator | 26, 27, 28, 29 | `evolution-lab-ecology-design-2026-09-10.md`, `evolution-lab-flight-design-2026-09-11.md` |

*Rounds 30 and 31 are cross-cutting rather than on one line: 30 is colony
survival, 31 is the playtest gap and the instruments for it.*

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
