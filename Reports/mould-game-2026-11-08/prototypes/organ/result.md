# Mould Organ: result (2026-11-08)

**Answer first.** The mould keeps time perfectly: left alone, the tube from the heart to each bell does not change length by a single cell in 60 seconds, on all 8 dishes. So step 0 passes by a wide margin and the organ got built. Every verb changes what you hear: the lamp drags a note late while you hold it, a bridge between two bells makes an echo, a salt wall silences a bell until you lead the mould round it. **The catch:** the mould straightens every tube again once your hand is off it. A lamp push is gone within 5 seconds of letting go, on 8 of 8 dishes, and a detour withers once its oats are eaten. So the lamp works like a pitch-bend wheel, and you can't use it to write a rhythm that stays. Whether that reads as fun ("I lean on it and it swings") or frustrating ("it won't keep my rhythm") is a feel question only playing it with sound can answer. I can't hear it, so that call is Scott's.

Picture page, with the GIFs, their matching sound and the counts: https://claude.ai/artifact/Np5fa71YVHe8gnZq7oVZNW (private until shared).

Branch `organ-proto`, commit `0022dbf14048be6fe77609bd32f0450048c5c0b8`. Run it with `cargo run --release --bin organ` (seed: `-- 3`). Sound plays on Windows and macOS. On Linux it needs `--features audio-alsa`; otherwise the window runs silent and the WAVs here are how to hear it.

## Step 0: does the timing wobble? (the gate)

Setup: a heart and three bells on the mould game's own model (`src/mould/organ.rs`). An oats-only scripted hand joins all three bells, then lets go. It waits until the last oats are gone and another 20 s have passed, then measures every step (120 a second) for 10 s, and separately for 60 s. Wobble means the most minus the least of the tube distance from the heart to the bell over that window. 8 seeds joined (1, 2, 4-9). **Seed 3 never got all three bells joined in 150 s with 14 oats**, so it is out and seed 9 is in.

| bell | n | wobble over 10 s, cells (min / p90 / max) | over 60 s (min / p90 / max) | steps the bell was briefly unreachable |
|---|---|---|---|---|
| LOW | 8 | 0 / 0 / 0 | 0 / 0 / 0 | 0 |
| MID | 8 | 0 / 0 / 0 | 0 / 0 / 0 | 0 |
| HIGH | 8 | 0 / 0 / 0 | 0 / 0 / 0 | 4 of 9,600 (10 s); 18 of 57,600 (60 s) |

Settled tube lengths: LOW 35-44, MID 45-54, HIGH 65-74 cells. At 50 cells/s that rings the bells about 0.75, 1.0 and 1.4 s after the heart's thump. Per-seed table: `numbers/step0-10s.txt`, `numbers/step0-60s.txt`. The picture `step0-settled.png` (2 seeds) shows the dish and its tube-length map, banded every 10 cells.

**Two things I checked before believing a zero.**
- **The brief's ruler cannot see a bend.** Counting breadth-first steps along tube, as the feeding search does, gives every staircase between two points the same length (dx + dy). A lamp that visibly bowed the high tube by about 10 cells moved that count by 0 on 7 of 8 seeds. On that ruler the zero would mean nothing. So the organ measures tube length in straight-line cells (16-neighbour Dijkstra, `heart_geodesic`). That ruler does see the push, and it also reads 0 wobble (table above, both rulers).
- **The instrument can report wobble when there is some.** Weight the same ruler toward a tube's thick middle (`core=3`) and the 10 s wobble becomes 13-17 cells (low), 16-21 (mid) and 19-26 (high). That's the brief's "mush" line. The trail inside a tube flickers every step; its outline does not. The organ uses the outline.

## What the verbs do to the notes

**Lamp bend test** (`numbers/bend-*.txt`, picture `lamp-bend-s4.png`). The lamp pushes the middle of the high tube sideways over 3-4 s, then lets go. This is compared against an untouched twin of the same dish. 8 seeds.

| push | high note later while held, ms (min / median / max) | still later 15 s after letting go, ms (min / median / max) |
|---|---|---|
| default (lamp edge to 4 cells past the tube, 3 s) | +79 / +122 / +245 | 0 / 0 / 0 |
| deep (to 8 cells past, 4 s) | +139 / +161 / +339 | 0 / 0 / +120 |
| same deep push, brief's step-count ruler | 0 / 0 / +40 | 0 / 0 / +40 |

**The scripted performance** (`numbers/play3.txt`). After joining and settling, one hand works through the verbs in order on 8 seeds. Notes are counted per phase, and the change is measured against the same seed left alone.

| phase (length) | LOW | MID | HIGH |
|---|---|---|---|
| alone (10 s): notes per bell per 10 s | 5 | 5 | 5 |
| lamp pushes high tube (6 s): change in ms | 0 | 0 | +56 / +65 / +138 |
| let go (10 s) | 0 | 0 / +35 / +69 | +38 / +114 / +170 |
| oats bridge LOW-MID, 3 oats (20 s): echoes, 80 beats per bell | **12 echoes**, +0 ms | **19 echoes**, +4 / +65 / +108 ms | 0 echoes |
| salt wall across MID tube + 1 oat to lead round it (20 s) | 0 ms | **silent, then back +262 / +401 / +452 ms late** on 6 of 8 seeds (1.8 notes per 10 s vs 5); silent all 20 s on 2 of 8 | 0 (one seed rerouted, +1,397) |

The echo happened on 8 of 8 seeds once the bridge formed. My first try (a single oat between the bells) grew a spur off the trunk and made no loop. It took stepping stones: one oat beside each tube, then one between them.

Target rhythm (optional, built): ghost ticks at 0.75, 1.25 and 1.75 s, with a note counted on beat if it lands within ±60 ms. The scripted hand doesn't aim at it. On seed 1, 36 notes landed on beat and 56 off, which says nothing about fun. As built it's very hard to hit, because the mould takes back any added length.

## Pictures and sound (seed 1)

| file | what | discrete counts |
|---|---|---|
| `organ-lamp.gif` + `organ-lamp.wav` (18 s) | the lamp leans on the high tube, then lets go | 9 beats; HIGH rang 9 of 9; mean 1,434 ms alone → 1,499 ms pushed → 1,505 ms just after |
| `organ-echo.gif` + `organ-echo.wav` (20 s) | stepping-stone oats bridge LOW and MID; the loop echoes | 10 beats; LOW 10 notes + 1 echo, MID 10 notes + 4 echoes (at 1.3-2.4 s) |
| `organ-salt.gif` + `organ-salt.wav` (21 s) | salt cuts the MID tube; a lure leads it round; it withers | 10 beats; MID silent 4 beats, back at 1,293 ms (+389) for 3, then silent again |
| `organ-whole-run.mp4` (66 s, with sound) | all of the above in one go, picture and sound together | 33 beats, 130 notes in all (`numbers/organ-s1-notes.txt` lists every one) |
| `performance-s1.png` | 8 stills across the run, labelled | as captioned |

On screen: the pulse is a white-to-pink band riding the tubes. Echo fronts ride in the bell's own colour. Bells flash and throw a ring when they sound. Under the dish, the bar shows one beat: the heart's thump at the left, a coloured tick where each bell rang (hollow for an echo, short for a note that spilled into the next bar), ghost target ticks, and a playhead.

Sound: each bell is a struck bar (C3, G3, E4); the heart is a short falling kick. A thick tube rings louder, and a starving one fades. The WAV peaks at 0.46 of full scale, about -23 dB RMS. The lab's synth wasn't reused because it has no voice whose pitch a caller sets, and adding one would mean editing a shared file.

## Cost

Measured over 31,680 frames, each frame being 2 steps plus a draw at the window's scale: mean 2.86 ms, p99 7.5 ms, worst 17.4 ms. Frames with a beat in them (when the pulse measures every tube) average 5.6 ms, worst 13.9 ms. The mould game runs at 2.6 ms, worst 17 ms.

## What didn't work

- The brief's ruler (step count) is blind to shape, covered above.
- A pulse at 80 cells/s made a lamp push 46-122 ms, which sounds like sloppy playing rather than a choice. At 50 cells/s it is 79-245 ms.
- A pulse that keeps to the thick middle of the tube wobbles 13-26 cells: mush.
- With one bar of reach, a loop's long way round (about 120 cells) never rang. Pulses now run two bars and spill into the next one.
- A salt line across a tube cuts it rather than bending it. The mould doesn't grow round a wall on its own within 20 s; it needs a lure, and the detour fades once the lure is eaten.
- Not added to the arcade: its menu is laid out for 8 games with keys 1-8, which is more than one line to change.
- The Windows sound path compiles (`cargo check --target x86_64-pc-windows-gnu --bin organ`), but I haven't run it on a real speaker. The window itself runs and screenshots headlessly on Linux, silently.

## What I'd do next

1. **Scott plays it with sound for five minutes.** That is the question. The numbers say it keeps time and every verb is audible. They can't say whether a groove that drifts back is a toy or an annoyance.
2. If drifting back is the annoyance, give a lasting lever. For example, a bell or a tube that remembers its route after being held for a few beats ("set"), or salt that guides a tube as it grows rather than cutting it.
3. If drifting back is the charm, lean into it. Free play as an instrument you hold, like a whammy bar, where the puzzle is a rhythm you have to keep holding while the mould pulls back.
4. Cheap polish: snap arrivals to a sixteenth grid as an option, a pentatonic note per bell so more bells always fit, and more bells than three.

## Files in the repo (branch `organ-proto`)

- `src/mould/organ.rs`: the organ (dish, rulers, pulse, echoes, drawing, voices, offline WAV, joining hand), with 3 tests
- `src/bin/organ.rs`: the window, with live sound where a device is compiled
- `examples/organprobe.rs`: `step0=1`, `bend=1`, `play=1 frames=DIR`; row added to `Reports/instruments.md`
- `src/mould/mod.rs`: one line, `pub mod organ;`. Nothing in the mould game changed.

Checks: `cargo test --release --lib mould` 8 passed; `cargo clippy --release --locked --bin organ --example organprobe -- -D warnings` clean; `bash scripts/docscheck.sh` clean.
