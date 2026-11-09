# Prototype: The Wanderer

Read `common.md` first, then `../theme.md` section 1 ("The Wanderer") and "The bet, plainly".

**The question:** does *leading the mould somewhere* feel better than *scoring a dish*? 

**The cheapest test (from theme.md):** in the current mould game, replace the 90 s clock with an **exit on the right edge**, a **Go key**, and **"<name> arrived with N of M"** in place of the stars. The food under the mould already drains (about 2 min 20 s per 20,000 pile at a full table) and its scent fades; let that be the clock. Moving on costs a graded amount: walkers left behind on tube that loses its food go grey and starve.

What to build, smallest first:
1. A Wanderer mode of the existing game (a key or a CLI arg on `--bin mould`, your call; keep the existing dishes and the lesson working unchanged). Draw an exit zone on the right edge. When the player presses Go (G) — only allowed while some mould touches the exit — the dish ends: count walkers that are in or connected-by-fed-tube to the exit as arrived (decide the exact rule and say why), show "MABEL ARRIVED WITH N OF M" (M = walkers at their peak this dish, or at start; pick and say).
2. Food drains with no clock. If the whole mould starves, show that too ("MABEL DIDN'T MAKE IT").
3. Carry-over: the next dish starts with N walkers placed at the left edge. Two or three dishes in a run is enough (vary seed). This is the "it's your mould" feeling.
4. Optional if cheap: the mould starts restless (scent fading) as food runs low; nothing more.

Measure with scripted hands in `examples/mouldprobe.rs` (add a `wander=1` mode or similar): at least do-nothing, a lamp-and-oats leading hand, and a careless hand (Go as soon as possible). The distribution of "arrived N of M" across 8+ seeds per hand is the number that matters (law 1: a spread, not a pass/fail). A do-nothing hand should arrive with much less, or nothing.

Files you may change: `src/mould/*`, `src/bin/mould.rs`, `examples/mouldprobe.rs`, new files under `src/mould/`. Note the lesson (`src/mould/lesson.rs`) and briefing card must still work; extend the briefing card text for the Wanderer mode.

Deliver per common.md, folder `wanderer/`: a GIF of one leading run from start to Go, a still of the arrival card, and the table of arrived N of M per hand per seed.
