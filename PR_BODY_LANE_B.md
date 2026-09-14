# The world starts bare, and she is the one who plants it

Two owner playtest items from 2026-09-14, both on the held world
(`cargo run --release --bin druid`).

**What this does, in the world's words.** The druid walks out into a land with
nothing growing in it, carrying a pouch of seed. What comes up is what she put
there. And she can now switch off the sphere of running time she carries, so
the world holds still around her.

**Where it sits.** These are the first two of the playtest items that change
what the game *is* rather than how it looks — the held world had a verb for
planting but nothing that made planting cost anything, and no way to stop time
while standing in it. Lane A has the interface and the keys; Lane C has the
creatures. This lane is the world and the economy.

---

## Item 4 — "The world should not start with any seeds"

> *"The world should not start with any seeds. The druid has her own seeds to
> plant and that populates the world."*

### The world half is three asset lines

The seeds came from the worldgen `life_scatter` pass, which already early-outs
when all three of its densities are zero — the path `arid` and `flat` already
take. So the `druid` preset's `moss_density`, `tree_density` and
`grass_density` go to `0.0` and no code changes.

Measured rather than assumed, paired, one binary and two copies of
`assets/worldgen.ron` (`pass_ablation seeds=1 preset=druid`):

| pass | before | after |
|---|---|---|
| **`life_scatter`** | **993 cells** | **0** |
| `soil_blanket` | 1,239,668 | 1,239,668 |
| `ponds` | 94,493 | 94,493 |
| `soil_moisture` | 1,223,852 | 1,223,852 |
| `moisture_init` | 71,324 | 71,324 |

The before-arm is the positive control: the same instrument reads a known
non-zero case at 993. Every other pass is byte-identical across the two arms,
so nothing was displaced into a different pass rather than removed.

`Start::Bare` never grows and nothing else in the generator plants, so that is
the whole of it. Its doc claimed otherwise and has been corrected.

### The other half is that her supply was infinite

`Druid::plant_seed` read no resource and decremented nothing. `sown` was a
monotone counter for the readout, and no decrement existed anywhere in the
repo — so on a bare map the item as stated would have changed a wood into a
painting tool.

So: **a pouch per kind**. Eight of each to start, capped at 24. Sowing spends
one, and only on a seed that actually goes into the ground. An empty pouch
refuses and says so, like every other refusal in this game. And it refills
from **mature plants standing in the circle she carries** — 0.005 seeds per
plant per second, credited to the plant's own kind, and only above a 24-cell
maturity bar.

Three choices in that worth arguing with:

- **Per kind, not one number**, so *"eight grass and no oak"* is a state the
  game can be in. That is what makes cycling the seed kind a decision.
- **The carried circle, not a standing one.** Gathering is presence — the same
  thing the carried circle already is — so the way to be paid in seed is to
  walk your own wood. Crediting standing quickenings would pay a player who
  places a circle over a wood and leaves, which is the unlimited supply
  wearing a delay. It also makes the two items in this PR interlock: switching
  the sphere off stops the pouch filling, for free, out of the same `Option`.
- **A cell-count maturity bar** rather than the species' own `seed_maturity`
  fence, which is a plant's business and moves with the genome. This is the
  player's question, and the player's question is *"does this look like a tree
  yet"*.

**What I did not build, and why.** Seeds priced in power — `Setting::Unlimited`
already means power, so a seed bought with power would become free the first
time anyone pressed `U`, which is the mechanic deleting itself. Picking loose
seed cells up off the ground — more satisfying, and it is what the world
actually produces once plants reproduce, but a loose seed cell belongs to a
live organism, so harvesting one is surgery on the plant line's reproduction
path, and it would let her hoover up the regeneration she is there to
encourage.

**The assumption worth flagging**, since it is the owner's call and not mine:
*that a refill should exist at all.* A supply that can only go down makes the
outcome binary — you have seeds or the run is over — which is the failure law 1
names, so everything here assumes a refill. That is an inference from the
ethos, not from his words. If a run is meant to be bounded by what she sets
out with, delete `take_gathered_seed` and its caller and nothing else moves.

Every number here (8, 24, 24 cells, 0.005/s) is a first guess and says so at
its definition — the same footing as the rest of this economy, whose own doc
says as much. The one to sweep first is the maturity bar, because it is the
only one that decides whether the mechanic has a *middle*.

---

## Item 5 — "an easy way to full turn off the sphere"

> *"There should be an easy way to full turn off the sphere around the druid so
> no power is being used."*

**The power half of that premise is already true, and the code says so.**
`carried_cost` prices the area *added*, so the carried circle at its base
radius costs exactly zero, and there is a guard whose message is *"the circle
you already are must stay free"*. Nothing here is a saving unless the player
has widened it with `]`, and then it is the widening that stops being billed.
`carried_cost` is untouched and its guard still passes.

What was genuinely missing is the thing the words say: **a way to make the
world hold still where she stands.** That is what this builds.

### It is a flag, not a radius of zero

`World::carried_radius`'s own doc refuses to let a dial reach off by accident,
and that comment is load-bearing: `frame.rs` reads a zero radius as the
*default size*, and even a genuinely zero radius would still run time for the
cell underfoot, because `Quickening::contains` is `<=`. Both are asserted
directly, so a later session that "simplifies" either will be told.

`World::carried_off` is therefore its own word, default `false` — the sandbox
and the lab see no change at all.

### Off is a trade, not a free button

With the circle off, the colony under her feet stores no charge, a seed she has
sown does not germinate, the wood she is in stops growing, and her pouch stops
filling. Every one of those falls out of `World::time_runs_at` and needed no
code — the same way the rule that a colony must be founded inside running time
does.

The delivery, per law 2, is that **the world visibly goes still**: with
`world.carried` at `None` the held look reclaims the ground she is standing on,
so the bubble does not dim, it disappears. The HUD says `YOUR CIRCLE OFF` in
warning colour — a word, because no number could say it — which is what stops
an idle power bar reading as a fault.

The key belongs to `src/bin/druid.rs`, which is Lane A's file.
`Druid::toggle_carried_circle()` is the call, and
`PIXEL_PHYSICS_DRUID_CIRCLE=off` is a headless control arm for judging the two
states off one binary in the meantime.

---

## Guards, and the two that were blind

Both new guards were written after the code, so neither gets `CLAUDE.md`'s
already-watched-it-go-red exemption. Putting the faults back found the first
version of the pouch guard **blind three ways**:

| fault put back | first version | now |
|---|---|---|
| the decrement deleted | **passed** | fails |
| the empty-pouch refusal deleted | fails | fails |
| credit the selected kind, not the plant's | **passed** | fails |
| drop the carried-circle gate | **passed** | fails |
| drop the maturity bar | **passed** | fails |
| `frame.rs` ignores `carried_off` | fails | fails |

Two causes, both general:

1. **The guard never reached a successful sowing.** Its world had no ground, so
   `plant_seed` returned `false` from its first line and every assertion was
   about the refusal path — *a scene that contradicts the code will look like a
   bug in the code*, arriving as a false **pass**. A second scene error sat
   under it: standing still, the second sowing is refused for *"the cell is not
   empty"*, so a guard that did not move him between sowings would have been
   measuring the ground.
2. **The crediting rule was inline in an organism walk that needs a grown
   world**, so the one claim a player would notice going wrong was unreachable
   by any test running in under a minute. It is now `seed_credit`, a function
   over plain values whose four `None`s are the rules, with a guard that opens
   on the positive control before asserting any of them.

---

## Files, and the lane boundary

`assets/worldgen.ron`, `src/worldgen/` (unchanged in the end — the pass already
supported this), `src/druid/mod.rs`, and the carried-quickening lines of
`src/sim/world.rs` and `src/sim/frame.rs`.

`src/druid/hud.rs` is Lane A's and is touched under a narrow licence, in one
commit, entirely within `Readout` (`:185-258`) and ~140 lines clear of
`Interface::draw`: two fields, two lines gaining a branch, and the coverage
sweep extended so both branches of both are actually formatted. Two fields
rather than the one the licence named — `carried_off` cannot be inferred from
`carried_radius` (the radius keeps its value while the circle is off) and the
seed count cannot be inferred from `sown`.

`Druid::charged_animals` is untouched and still there for Lane A to delete.

Full lane note, including the seed-economy reasoning and what the brief got
right: `Reports/lanes/druid-seeds-and-sphere.md`.

## Judged by eye, not described

Both items are visible, so both went to the review queue as real captures of
the shipped binary (`xvfb-run` + lavapipe + the app's own screenshot hook, so
the HUD is in the picture — which is where half of each mechanic lives).

- **`20260914T044639296Z-dd68c2`** — the bare world as she walks out into it.
  Asks whether it reads as somewhere worth planting or as an empty map.
- **`20260914T044811000Z-26d062`** — a blind A/B of the sphere on against off,
  over a grown wood, same world and same frame. Asks whether off reads as the
  world holding still or just as the ring being hidden. Its context says
  plainly that the sphere already cost nothing, because both panes read
  `OUT 0.0/S` and a card that did not explain that would read as a broken
  mechanic.

Neither was waited on. `python3 scripts/review.py inbox` collects them.

## Gates

- `cargo clippy --all-targets --release --locked -- -D warnings` — clean
- `cargo test --release` (the full suite, not `--lib` — the worldgen guards
  that sweep `presets()` live in `tests/*.rs` and `--lib` cannot reach them)
- `bash scripts/worldgencheck.sh`
- `bash scripts/docscheck.sh`
- `python3 scripts/deadendindex.py --touching`

🤖 Generated with [Claude Code](https://claude.com/claude-code)

https://claude.ai/code/session_01CbyvwaFsrCF6jnA87mYnky
