# Why the bed's water stands in columns

**2026-09-11. Measured, diagnosed, not changed — the default is the owner's
to rule on and the lever is shipped as a switch.** The owner, with the soil
moisture overlay on:

> *"I want you to understand why water in the soil builds up in these
> columns. Is this by design, or should it be fixed?"*

**Short answer: the mechanism is deliberate, its application to the sideways
face is not, and what a player sees during normal play is nothing at all.**

---

## 1. The picture, and the control that identifies it

The report came with an overlay screenshot of the lab bed: a bright, jagged
band of near-saturated soil standing on the stone floor with individual
columns spiking up out of it, and fine vertical streaks running down through
the bed above.

The first question is whether that is biology — roots drawing water, a
rooting pattern printed into the soil — or transport. `examples/waterstand
founders=0` answers it in one run: **an empty box, no plants, no animals,
nothing alive at all, reproduces the picture exactly** at the mister's
shipped LIGHT over 24,000 frames. Nothing biological is involved.

The matching negative control, the same empty box with the mister **off**,
holds flat at field capacity — every column reading exactly 620, widest
standing gap 0 — at every stop to 24,000 frames. So the instrument is quiet
when nothing is happening and loud when something is, which is
`CLAUDE.md`'s two-sided check on a number run both ways in one command each.

## 2. What actually holds the columns apart

`update::update_soil_water`'s capillary exchange rests on a threshold, and
there are two of them:

```rust
let rest = if wetter > SOIL_FIELD_CAPACITY { SOIL_SATURATED - SOIL_FIELD_CAPACITY }  // 380
           else { SOIL_CAPILLARY_REST_UNSATURATED };                                 //  60
```

A pair of neighbouring cells whose difference is at or under their `rest`
is declared level and never exchanges again. Above field capacity that
licence is **380 units — more than a third of the whole 0..1000 scale.**

Measured on the empty box at 24,000 frames:

| | |
|---|---|
| soil cells over field capacity (where the wide threshold applies) | 43,023 of 48,384 — **89%** |
| side-by-side pairs still exchanging | **0** |
| side-by-side pairs silenced by the **wide** threshold | 46,249 |
| **widest standing gap between neighbouring columns** | **380 — exactly the threshold** |
| column means across the bed | 600 … 682 |

The bed is not mid-way through levelling. It is **at rest, and this is what
at rest looks like.** The widest gap landing precisely on the constant is the
tell.

On the played bed the same reading is starker, because the plants make
infiltration patchy: column means **616 … 905**, a 289-unit standing spread
between neighbouring columns, with every pair at rest.

So the shape is: rain infiltrates at scattered points, each plume descends
as a **column** (drainage moves water *down* and only down), and the
sideways threshold then forbids those columns from ever levelling into each
other.

## 3. Why the threshold exists, and why the sideways half is an accident

The wide threshold is load-bearing and its derivation is in the source. Its
argument is a **pump**: drainage is at rest only at or below field capacity,
capillary is at rest only under its own threshold, so a cell in the
drainable band with room below it drains — and capillary refills it from the
saturated side, for ever, keeping every chunk at every water-table boundary
awake. Only a threshold spanning the drainable band stops that.

The argument is sound and it is about a pair of rules that disagree over the
**same** pair of cells. **Drainage only ever moves water down.** The face it
can fight capillary over is the vertical one; on the sideways face there is
no drainage term at all. The wide threshold is applied to every neighbour
regardless, and nothing in the derivation asks for that — it is the shape the
code happened to take.

## 4. What narrowing it would buy, and what it costs

`PIXEL_PHYSICS_SOIL_CAPILLARY=level` narrows the **sideways** face back to
the churn guard (60) and changes nothing else. It ships inert: the default is
the current rule bit for bit, confirmed by the shipped arm reproducing this
lane's pre-switch numbers digit for digit.

Empty box, seed 1, mister LIGHT, 24,000 frames, `RAYON_NUM_THREADS` pinned
and the arms alternated:

| | shipped | sideways narrowed |
|---|---|---|
| widest standing gap between columns | **380** | **0** |
| column means across the bed | 600 … 682 | 609 … 656 |
| pairs silenced by the wide threshold | 46,249 | 0 |
| soil-moisture writes per tick | 1,701 | **2,455 (+44%)** |
| soil cells walked per tick | 10,562 | 11,419 (+8%) |
| median ms/tick | 0.754 | **0.828 (+9.8%)** |
| mean ms/tick | 0.766 | 0.829 (+8.2%) |
| p90 ms/tick | 1.064 | 1.142 (+7.3%) |

And on the played bed (`played_bed_scrambler`, the same settings):

| | shipped | sideways narrowed |
|---|---|---|
| widest standing gap between columns | **380** | **0** |
| column means, every 16th column | 616 … 905 | 636 … 699 |
| soil-moisture writes per tick | 1,323 | **2,212 (+67%)** |
| median ms/tick | 1.554 / 1.573 | 1.579 / 1.590 |
| mean ms/tick | 2.008 / 2.032 | 2.142 / 2.153 |
| p90 ms/tick | 3.844 / 4.035 | 5.319 / 5.343 |

**The columns go away completely** — the widest standing gap falls from the
threshold to zero on both beds, and the water table stops being a comb of
spikes and becomes the flat sheet a water table is.

**The churn guard is doing real work.** +44% to +67% more soil-moisture
writes a tick is not a rounding error on a phase the lab's own cost work
names as the largest block left after the kernel. The median frame moves
about 10% on the empty box and about 1.5% on the played bed; the p90 moves
7% and 35%.

**One number here must not be read as a result.** The stand on the played
bed reads 6,844 plant cells shipped against 5,940 narrowed, one seed. That is
**not** evidence of an effect: `labsoil` measured **2.32x** spread in plant
cells across twelve seeds at fixed settings with nothing varying but the
seed, so a 13% difference on a single seed is inside the noise by a wide
margin. Anyone who wants the biological question answered has to sweep it.

## 5. The recommendation

**It is a side effect, not a design, and it is nearly invisible.** Three
things decide it:

1. **A player does not see it.** Rendered in the shipped material colours,
   the empty box with the full striping in it is a uniform brown slab —
   the columns only exist on the soil-moisture overlay. Both renders are in
   `examples/waterstand png=`, from the same run.
2. **It has almost no biological reach.** The wide threshold only applies
   *above* field capacity. Below it — the whole plant-usable band, where a
   seed decides whether to germinate and a root decides whether it is thirsty
   — the narrow 60 threshold is in force and the bed does level. A column at
   905 beside one at 616 is two cells that are both comfortably wet.
3. **The one place it has real reach is the water table**, which under the
   shipped rule is a comb of spikes rather than a level surface. What stands
   on that is whether an ant's gallery floods, which is
   `update::wet_collapse_line`'s own subject: *"a tunnel dug below the water
   table filling up is a hazard"*. A spiky table means neighbouring columns
   flood at arbitrarily different depths.

So: **not worth paying 8–10% of the lab's tick to fix the overlay.** Worth
reopening the moment the water table becomes something the game reads — a
flooding hazard, a moisture-seeking dig rule, a species that wants wet feet.
The switch is there so that day costs a run rather than a re-derivation.

`CLAUDE.md`'s standing instruction for this line applies and is why nothing
was tuned here: *a default that looks wrong is something to register and
report, never to tune.*

## 6. Two things the measurement nearly got wrong

**A cost A/B that could not have moved.** The first cost reading was taken on
`examples/lab_cost`, whose bed never rises above field capacity — so the wide
threshold never applies, the switch is inert, and both arms came back with
**identical field hashes, identical cell counts and identical `sw` counters.**
The tell was exactly `CLAUDE.md`'s: identical output across a change that must
have moved something. The switch is not dead; the *bed* was degenerate for the
question. Every number in §4 is taken on a bed that is actually over field
capacity, which means a bed with the mister on.

**A census that has to mirror the switch.** `waterstand`'s "declared level"
column computes the threshold itself. Left reading the shipped rule, it would
have reported pairs as silenced under an arm where the engine no longer
silences them. It reads the same env var now.

## 7. The instrument

`examples/waterstand` was built for the throughfall half of this session
(`canopy-throughfall-2026-09-07.md` §7) and grew the soil half here. What it
answers that nothing else did: **`soil_drawdown`'s horizontal profile is a
mean over an eighth of the bed, so a stripe one or two cells wide is averaged
away before it can be seen.** The column read has to be per column, and the
"is this pair at rest" read has to be per pair.
