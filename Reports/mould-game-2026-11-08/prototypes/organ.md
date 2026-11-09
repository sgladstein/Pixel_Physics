# Prototype: Mould Organ

Read `common.md` first, then `../out-there.md` sections "What the fun is, exactly", "3. Mould Organ", and "What I'd prototype next: Mould Organ".

**The question:** is steering the network fun when the reward is a *sound* rather than food joined?

**Step 0 comes first and can end the prototype.** On a settled dish with a heart (one pile) and three nodes (piles) joined by tube, measure how much the tube distance (breadth-first along tube cells, as the feeding search in `src/mould/mod.rs` does) from the heart to each node wobbles over 10 s with nobody touching it. Report min/max/p90 of the wobble per node over 8 seeds. Under ~5 cells: timing reads, continue. Over ~15 cells: stop, write result.md with the numbers, and report that the medium is too noisy (that is a real, useful answer). Between: continue, but say so.

If step 0 passes:
1. A new binary `--bin organ` (`src/bin/organ.rs`, model reuse from `src/mould/` — make things `pub` there if needed, no behaviour changes to the mould game). Same dish, tools lamp and oats (salt/snip optional). One pile is the heart, the others are bells.
2. A pulse leaves the heart every beat (pick a period, ~1-2 s) and travels along tube at a fixed speed in cells per second; draw it as a bright band riding the tubes. When it reaches a bell, that bell rings a note (each bell its own pitch) and flashes. Tube length = timing, so pushing a tube with the lamp changes the rhythm; a loop gives two arrivals (an echo).
3. Sound: the repo has a synth (`src/audio/`, used by the lab, key F12) — reuse it if it fits; otherwise a simple sine/pluck. Also render an offline WAV of a scripted run so the page can carry sound (check how `src/audio` or its examples render offline).
4. Optional if cheap: a target rhythm shown as ticks, and a count of notes that land within ±X ms of it.

Measure: notes per bell per 10 s, and for a scripted hand that bends one tube with the lamp, how far that bell's note moves in ms. Count notes on beat vs off beat if you built the target.

Files you may change: new `src/organ/` or `src/mould/organ.rs`, `src/bin/organ.rs`, a new `examples/organprobe.rs` (or a mode of mouldprobe), visibility-only edits in `src/mould/*`, and the arcade list if one line. Do not change mould game behaviour.

Deliver per common.md, folder `organ/`: the step 0 table first; then a GIF of the pulse running with the matching WAV beside it, and the note counts.
