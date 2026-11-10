# Toy feel: what makes touching the mould satisfying, and how to make it richer

Brainstormer: toy feel. Written 2026-11-08 from `src/mould/mod.rs`, `src/mould/render.rs`, `examples/mouldprobe.rs` and the probe report. I ran the binary that was already built (`target/release/examples/mouldprobe`, built 06:22 today) and looked at every picture myself. I did not edit the repo. Pictures are in `/mnt/project-files/game-design/mould-game-brainstorm-2026-11-08/toy-feel/`.

## The three changes I would make first, in order

1. **A pulse that travels along the tubes, outward from the food.** Today nothing on screen shows food flowing, and a starving branch looks almost the same as a healthy one. A beat drawn from the feeding search the game already runs makes the mould look alive at rest, shows which way food goes, and makes a death legible: the beat stops. It is drawing only, about 30 lines in `render.rs`, and costs well under 0.1 ms a frame.
2. **A "snap" when a bridge or loop closes: a flash at the exact join, and a sound.** This is the owner's favourite moment ("little bridges where there wasn't even food"). Today it happens in under a second and nothing marks it. One cheap pass at the census can find the exact cell where two pieces met or a loop closed. That turns his best moment into an event the game can see, reward and count.
3. **A cut that stays open long enough to mean something.** Measured today, a snip across a tube heals in about half a second, and nothing beyond it ever goes grey. The graded death the code was built for almost never shows. A short-lived wound that stays open about 4 seconds, drawn as a real slash with a recoil and a tearing sound, makes Snip a verb that delivers something.

Why these three: each fixes something the owner named or met (no sense of flow; the bridge he loved going unmarked; "I have no idea what the snip did"). All three are cheap. Each produces a discrete event (beat stopped, join made, wound opened) that a sound and a counter can hang on. Audio comes fourth because it needs those events first.

---

## What I measured and saw

All runs used the default parameters, seeds 1 to 3, about a minute of machine time each.

| Touch | What the player sees | How long it takes | Reads at phone size? |
|---|---|---|---|
| **Lamp push** | The tube dents away from the light, like a rope pushed with a finger | Dent in **0.05 s** (lamp to recoil, `lean=1`, 6 of 6 placements: 0.01 to 0.07 s). Held still, nothing more happens. Let go and the dent springs back in **about 0.5 to 1 s** | Yes, the bend is clear. The lamp's rings are 1 pixel thin |
| **Lamp sweep** | A swept lamp leaves the tube in its new shape. A V became a U and was still a U 6 s later | Lasting | Yes, best-reading verb in the game |
| **Oats pull** | A thin thread reaches out, touches the oats and runs on to the pile behind them, then fills in to a full tube | Lean **0.50 to 0.65 s**. First walker on the oats **0.30 to 0.45 s** (`lean=1`, 6 of 6). A pile 50 cells away was joined **1 s** after the click. The thread stage lasts about **0.25 s** | The tube does. The thread is gone before the eye finds it |
| **Bridge or join** | A thread becomes a tube | Under 1 s, then it thickens for **3 to 4 s** | The thickening does. The join moment does not: no mark, no sound |
| **Rewiring** | A new pile appears. A branch drops from the *middle* of an existing tube, makes a Y, the Y slides, and the old tube is gone | **About 5 s** from the pile landing to the route through it (seed 1, 55 to 60 s) | Yes. The most beautiful thing in the dish, and the old tube vanishes with no trace |
| **Cut (snip)** | A 1-pixel dotted line, then nothing. The gap closes | Healed by **0.5 s** (seed 1, snip at 25.0 s, gap gone by 25.5 to 26.0 s). No grey anywhere | No. The mark is one pixel per dab and fades in 1 s |
| **Cut (storm)** | The scar is a wall. The two cut ends pull back from it like a snapped rubber band | Recoil about **1 s**. They never try to rejoin in the 8 s watch | Yes. The recoil is a lovely, unadvertised moment |
| **Starving** | Pale grey tube, walkers dying | Each unfed walker dies at 1/600 per step, so half are gone in **about 3.5 s** and nearly all in 15 s | Rarely seen: see below |

**Two things were not what I expected.**

- **Grey almost never appears.** I counted drawn pixels in a full scripted game (fresh player, seed 1) at 30, 60 and 82 s. There were 16,000 to 23,000 yellow pixels every time and **zero grey tube pixels**. The roughly 3,000 grey pixels are the salt line. Every piece of mould holds a pile, and the food store is shared across the whole dish, so cutting a tube between two piles costs nothing. Only a finger more than 70 cells from food ever goes grey. The theme brainstorm calls a cut-off branch "the strongest emotional beat in the game". It is, on paper. On screen it is missing.
- **Every tube looks the same.** A trunk feeding three piles and a spur to one oat are both about 12 cells wide and the same yellow. "It gets stronger where it is used" holds only for the 3 to 4 s after a join. The mould also sits exactly at its food cap (600 walkers for 2 piles, 900 for 3, 1,800 for 6: 300 per pile), so between events the dish is full and still.

**Lamp distance** (`bend=1`, lamp held 3 s beside the thickest tube against a no-lamp twin, seeds 1 to 3):

| Lamp centre from tube | Tube moved | Join lost |
|---|---|---|
| 6 cells | 6.6 to 7.7 cells | 0 of 3 |
| 12 cells | 1.8 to 2.8 cells | 0 of 3 |
| 20 cells (outside the 16-cell reach) | about 0 (-1.6 to -0.2) | 0 of 3 |

The push is strong and safe, but it only reaches as far as the light does. There is no feel of *pressure*: holding still does nothing new after 0.2 s.

**Pictures** (contact sheets, each frame labelled with its time):

- `toy-feel/verbs-sheet.png`: a lamp swept across a tube, then oats (seed 1). The lasting bend is frames 4.0 to 9.5 s.
- `toy-feel/push-sheet.png` and `toy-feel/push-s1.gif`: the lamp held 8 cells beside a tube for 3 s, then released. It dents in 0.2 s, holds, and springs back by about 3.5 s.
- `toy-feel/join-sheet.png`: oats as a stepping stone to the far pile. Note the thin thread at 31.5 s.
- `toy-feel/rewire-sheet.png`: the Y sliding to a new pile, 55 to 60 s.
- `toy-feel/snip-sheet.png`: a snip at 25.0 s, healed by 26.0 s.
- `toy-feel/storm-sheet.png`: the storm warning, the strike, and the ends recoiling.

**Response speed in general.** The hand-to-answer times are good. 0.05 s for the lamp feels direct, and 0.5 s for oats feels like the mould *chose* to come. I would not slow the simulation. The problem is the other way round: the best moments (the thread, the join, the snap-back) are over in a quarter of a second and leave nothing behind. **The fix is to make moments linger in the drawing and the sound, not to slow the mould.**

---

## The touches, one by one, with changes

Costs are against today's frame: mean about 2.6 ms, worst about 17 ms (the storm-aiming frame). Draw time is about half of the mean.

### 1. Lamp push and sweep (the owner's first love)

It works. Keep the push model as it is. What is missing is a trace and a sense of force.

- **A wake.** Where the lamp has passed, the bare agar stays faintly bleached for about 2 s and fades. *Lever:* a render-only `lit_at` stamp per cell, like `fed_at`, written in `update_light` and read in `cell_colour`. *Cost:* one array (160 KB), a few lines, under 0.05 ms. *Know it worked:* a GIF of a sweep. The path should read as a stroke, the way a finger in sand does.
- **Pressure that builds when held.** Today holding still gives nothing after 0.2 s. Draw the trail the push piles up at the lamp's edge as a brighter "bunched" rim, and let a held lamp slowly widen its reach (16 to about 20 cells over 2 s). Then holding is a choice: lean harder. *Lever:* `lamp_reach` grows with hold time in `update_light`. Edge brightness is already in the trail. *Cost:* trivial. *Know it worked:* `bend=1 offset=20` stops reading about 0 and starts reading a few cells after a 2 s hold.
- **A held shape that sets.** Today a dent made by holding still springs back in about 1 s (push sheet). A sweep's shape lasts. Let the shape "set": after the lamp lifts, the cleared area keeps a weak, fading shade for about 2 s that the sensors read lightly. *Lever:* a decaying `shade` field read in `sense`. **Caution:** every sensor weight tried on the light itself made tubes part (`Params::light_weight` doc, dead-ends entry). So this shade must be weak, only applied after release, and tested with the same `bend=1` join check. *Know it worked:* the tube offset 2 s after release against the twin, which needs one more print line in `push`.
- **Sound:** a soft wet squelch whose loudness follows how much trail the push moved this frame (sum the `moved` values in `push_light`), lower in pitch the more mass moves.

### 2. Oats pull

- **Show the reaching.** Tubes are drawn from half the tube threshold. Below that, the first thread is invisible until it is nearly a tube. Add a third, faint "feeler" ramp from about 0.15 to 0.5 of the threshold, drawn pale and thin, so fingers show as they reach. *Lever:* `cell_colour`. *Cost:* none. *Risk:* trail spreads, so it may draw a haze round every tube. Look at a still before shipping, and keep it only if the haze is narrower than about 2 cells.
- **A gulp at contact.** When an oat or pile goes from no eaters to some (an event the census already sees, `Pile::eaters`), give it one bright expanding ring and a short brightening that runs back along the tube toward home. The second half comes free with change 1 below. *Sound:* a soft "plip", lower for big piles, higher for oats.
- **The oats arc and rings are 1 pixel wide.** At phone size they vanish. Draw overlays in cells (at least 1 cell thick), not in screen pixels.

### 3. Bridge and loop forming (the owner's favourite)

- **Detect the snap.** At each census, compute the local Euler count of the tube mask: one pass over 2x2 windows, which counts pieces minus holes. A cell whose change *lowers* the piece count is a join. A cell whose change *adds* a hole has closed a loop. That gives the exact cell where a bridge closed, food or no food. *Lever:* new code beside `census` in `mod.rs`. *Cost:* one pass over 41,000 cells every 6 steps, about 0.05 ms. *The catch:* tube edges wobble across the threshold (the old grey flashing was about 420 cells a second), so a raw count will fire on noise. Use a hysteresis mask (on above 1.0, off below 0.6), and only fire for holes or pieces bigger than about 30 cells that last 0.3 s. *Know it worked:* run the controls. On a settled dish with no hand (seed 1, 70 to 80 s) it should fire about 0 times. On the oats-to-far-pile join at 31 s, and on a hand-built loop, it should fire exactly once each. Print the count beside each picture.
- **The flash and the sound.** At the join cell: a white flash that spreads along the new tube for 0.5 s, then a warm glow that fades over 3 s while the tube thickens. Sound: a two-note chime, with pitch rising with the number of piles the piece now holds, and a lower, rounder tone for a loop.
- **A loop counter on the bar** ("LOOPS 2"). Loops are what survive the storm, and today a player cannot see that they built one.

### 4. A tube thickening, and what a tube carries

- **The pulse (first change).** The feeding search already gives every fed tube cell its distance from food (`Dish::fed`). Draw a brightness wave on fed tube: `bright = 1 + 0.25 * sin(2π(fed/λ − t/T))`, with λ about 24 cells and T about 1.5 s. Bands of light then travel outward from every pile the mould is eating, along every tube, at about 16 cells a second. Unfed tube gets no wave. *Lever:* `cell_colour`, reading `d.fed[i]`. Cells that only *look* fed (`looks_fed`) take the nearest fed neighbour's phase or stay steady. *Cost:* one table lookup per tube cell (about 6,000 cells), well under 0.1 ms, and no simulation cost. *What it buys:* food visibly flowing; the direction of supply; a mould that breathes even when the player does nothing; and a death you can see, because when a branch is cut off its beat stops. *Know it worked:* a 3 s GIF of a settled network, judged by eye on a phone. Then a snip on a long spur: the beat beyond it should stop within one census (0.05 s), well before the grey.
- **Real shuttle streaming, later and optional.** Real Physarum's flow sloshes back and forth on a cycle of a minute or two. For a toy, about 1.5 s reads as breathing. Reversing the wave's direction every 4 to 6 beats would echo the real thing. A model version, where walker speed itself rises and falls with the wave so fronts lunge and pause, would make growth surge in pulses. That changes every timing above, so do it only after the drawn pulse is liked. Re-run `lean=1` and keep the lean under 1 s.
- **Trunks look like trunks.** At the census, walk the fed cells from far to near and add up how many fed walkers lie beyond each cell. That number is the flow through it. Draw a bright core line inside each tube, with width growing with the square root of the flow. The trunk to home then reads thick and the spur to an oat thin. *Lever:* one extra pass in `census`, about 0.1 ms every 6 steps, plus `cell_colour`. *Know it worked:* a still where a stranger can point to the trunk.

### 5. A cut branch starving

- **A wound that stays open (third change).** Snip clears trail in a 2.5-cell disc. Trail and walkers refill a 5-cell gap in about 0.5 s. Make the snip stroke a short-lived wall (`Ground::Wound` with a heal time) that closes after about 4 s. The cut stays visible long enough for the far side's beat to stop, its tube to go grey, and the player to decide whether to rescue it with oats. *Lever:* `stroke_dab` and a timer cleared in `phase_tick`. Count down by refilling from the ends. *Cost:* trivial. *Know it worked:* snip a spur at 25 s against an unsnipped twin, and record time to grey and far-side walkers left at 5 and 10 s. Today: grey never, healed by about 0.5 s.
- **Draw the cut as a cut.** A slash 2 cells wide, red-white, fading over 1.5 s, not one pixel per dab. Both cut ends should recoil (they already do for the storm, and it is lovely). Sound: a short wet tear.
- **Make a cut-off piece actually go hungry.** Today one shared store feeds every piece, so a piece cut away from home lives on happily if it holds any pile. Give each connected piece its own share. A piece holding a small or nearly empty pile then shrinks over tens of seconds: a graded decline, not a binary. This is the bigger model change (piece labels at census, about 0.3 ms every 6 steps) and it changes the economy every star was tuned on. It is still worth doing, because without it the storm's cut is pure accounting.
- **Leave husks.** A dying walker leaves a brown speck that fades over 5 s. Death becomes a mark left behind, and its count (`Counts::starved`) is already kept.

### 6. Rewiring

- **A ghost of where the tube was.** Keep a slow `worn` field: the highest trail a cell has had, fading over about 20 s, drawn as a faint dark stain on the agar. When the Y slides to the new pile, the old route stays as a shadow for a while. The dish then remembers the player's work, and a swept shape can be compared with its old self. *Cost:* one field and one multiply per cell per step, about 0.05 ms.
- **A tube that remembers, as a model change (test it, don't assume it).** Let walkers sense `worn` faintly, so a healed cut regrows along its old line and old routes come back more easily. *Risk:* this could make the network stiff, and the 5 s rewiring is the best thing it does. *Know it worked:* time the 55 to 60 s rewiring (seed 1) with and without it. Ship it only if rewiring stays under about 7 s.

## Sound in one place

There is already an engine sound stack (`src/audio/`: `listen` reads the world and decides, `synth` makes granular textures, and the offline render is the same code). It never gives one voice per walker; texture density follows the count. A mould listener would read the dish and emit:

- **A low breathing drone** on the pulse's beat, louder with more fed mould.
- **A squelch** for the lamp, following trail moved.
- **A plip** when food is first touched.
- **A chime** for a join, and a rounder tone for a loop.
- **A tear** for a snip.
- **A beat that slows and fades** for a starving piece.

Cost: the audio thread only, with no simulation change. On Linux it needs the `audio-alsa` feature, as the lab does. Do this after changes 1 to 3, because they create the events it plays.

**One frame-budget note:** the HUD calls `measure()` (a full flood of the dish) every frame. If the pulse, flow and join passes are added, read the HUD's numbers from the census instead.

## Commands I ran (all on the prebuilt binary)

```
mouldprobe lean=1 seeds=1,2,3
mouldprobe bend=1 offset={6,12,20} seeds=1,2,3
mouldprobe verbs=DIR seeds=1
mouldprobe bend=1 offset=8 gif=DIR seeds=1
mouldprobe players=fresh seeds=1 gif=DIR every=15      # the whole dish, 4 frames a second; sheets cut from it
```
