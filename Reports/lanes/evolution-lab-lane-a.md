# Evolution lab — lane A, the speed dial

Owner of `src/lab/time.rs` and `examples/labdial.rs`. Brief: *"I want the
speed up function developed; when the simulation is running at super speed
you should be able to watch everything moving quickly."*

---

## 2026-08-30 — the crossover is at 12 ticks per displayed frame

**Where motion stops being motion, measured rather than argued.** The
arithmetic the brief hands you — `60*M/D` ticks a frame, a falling cell
moving about a cell a tick, so the fastest thing jumps that many cells — is
an **upper bound**, and in a sealed lab box it is loose by about **14x**.
Nothing in the box is in free fall for long. The only things that translate
are the ants, and an ant covers **0.05–0.07 cells per tick**.

Censused by `examples/labdial.rs mode=census`, 6,000 ticks after a 3,000-tick
warm-up, on seeds 1, 2 and 7. Every number is a **counter**, not a clock, so
none of it moves when three other agents are compiling on the box.

| ticks between displayed frames | 3 | 6 | 8 | **12** | 16 | 24 | 48 | 192 | 768 |
|---|---|---|---|---|---|---|---|---|---|
| p90 ant displacement, cells (seed 1) | 1.00 | — | 1.41 | **2.00** | 2.24 | 3.04 | 5.02 | 11.28 | 21.38 |
| seed 2 | 1.00 | — | 1.41 | **2.00** | 2.24 | 3.00 | 5.00 | 11.07 | 20.06 |
| seed 7 | 1.00 | — | 1.12 | **2.00** | 2.06 | 3.00 | 4.61 | 10.12 | 20.06 |

**The criterion**: apparent motion holds while a mover's net displacement
between two displayed frames stays inside **its own body**, because past that
the eye can no longer match a feature to itself. An ant is **2.00 cells**
(measured, mean over 227k–251k organism-frames — the guide's *"two dark cells
at play zoom"*, confirmed). The fastest tenth of ants crosses two cells at
**exactly 12 ticks a frame on all three seeds**.

**The positive control fired.** At *one* tick between frames the largest
displacement any ant managed was **1.41 cells** — one diagonal step, the hard
bound. 0.00 there would have meant the probe never ran, and `CLAUDE.md` is
explicit that a null and a dead probe look identical.

**The corroborating instrument says the box is nearly still.** Cells whose
*material* differs between two displayed frames: **2 at 3 ticks, 68 at 12,
258 at 768**, out of 163,840. Under 0.16% of the box changes even at the top
of the dial. So the visible motion in a lab box **is** the ants; there is
nothing else moving to set the bar.

### → coordinator: the consequence for Gate 3's display-rate call

**A lower display rate buys tick throughput and spends the motion half of the
dial to do it.** The crossover is fixed in ticks per frame, so the multiplier
it becomes is `M* = 12 * D / 60`:

| display rate | 60 Hz | 30 Hz | 20 Hz | 10 Hz |
|---|---|---|---|---|
| highest multiplier that still reads as motion | **12x** | 6x | **4x** | 2x |

Gate 3 says 20 Hz roughly triples the tick multiplier. It does — and it also
drops the top of the *watchable* range from 12x to 4x. Both halves are real
and they pull opposite ways, which is an argument for the rate being a player
control rather than a constant somebody picks once.

---

## 2026-08-30 — what the box actually achieves, and what the display rate buys

**Wall clock, and therefore the only numbers in this note that move when the
machine gets loud.** `labdial mode=rate`: a fresh lab bed per configuration
warmed 6,000 plain ticks (53 organisms), then the real `Lab` frame loop driven
for 3 s with the draw gate and the idle sleep in place. Taken twice on purpose,
because three other agents were building on this box.

| display rate | loud box (load 16) | quiet box (load 5) |
|---|---|---|
| 60 Hz | 1.7–2.0x | 3.4–4.0x |
| 30 Hz | 3.2–3.7x | 4.8–5.3x |
| 20 Hz | 4.5–5.0x | 5.2–5.7x |
| 10 Hz | 5.3–5.5x | **5.6–6.2x** |

**The display rate is worth most exactly when the machine is worst.** Loud, 60
Hz → 20 Hz is **2.6x** the world per second; quiet it is **1.4x**. That is the
render's share of a contended frame, and it means the control earns its keep on
the machine that needs it.

**Every row asked for 64x, 256x or 1024x and every row got the same answer**,
which is the honest reading: this bed's throughput is ~350 ticks a second and
the dial is entirely machine-limited above about 6x. On a box that is not
shared four ways it will be far higher, and the readout will say so without
anyone re-deriving anything. `orgs 53` in every row — population is the
performance budget, exactly as §2 of the guide says.

**One thing to watch, from the cross-check built into the harness.** `mode=rate`
prints the readout's own `achieved` beside a count the harness took
independently. They agree closely at 60 Hz (3.9 against 4.0) and the readout
runs **10–25% low at 10 Hz** (4.6 against 6.0). It is a lag rather than a
disagreement — the readout is the last completed 500 ms window and a 10 Hz
window holds only about five passes — but if the gap ever widens or flips sign,
that is the dial starting to lie and the two columns are there to catch it.

---

## → coordinator: three changes I need in files I do not own

`src/lab/time.rs` is written so that **a caller which ignores all of this
still works** — it just draws every pass, exactly as today, and the top of the
dial is slower. Nothing below is load-bearing for correctness; all three are
load-bearing for the feature.

### 1. `src/bin/lab.rs` — gate the draw and honour the idle

This is the mechanism that buys the top of the dial. `Advance` gained two
fields, `draw: bool` and `idle: Duration`; `Lab::advance` already returns it
and `src/lab/mod.rs` needs no change at all.

In `Handler::frame`, replace

```rust
        self.lab.advance(elapsed);

        let render_error = match &mut self.pixels {
```

with

```rust
        let advance = self.lab.advance(elapsed);

        // The display rate is decoupled from the frame loop: in Running,
        // `advance` says which passes draw, and the ones that do not spend
        // their whole budget on ticks instead. Gate 3's call, and the
        // mechanism behind the top of the dial.
        let render_error = if !advance.draw {
            None
        } else {
            match &mut self.pixels {
```

closing the new `else` with the existing `};`, and add immediately after the
`if let Some(message) = render_error` block:

```rust
        // Skipping the draw also skips the vsync that was throttling this
        // loop, so a dial the box can easily meet would spin a core between
        // displayed frames. `idle` is non-zero only when no tick and no frame
        // is due, and is capped at `time::MAX_IDLE` (2 ms) so a keypress is
        // still answered promptly.
        if !advance.idle.is_zero() {
            std::thread::sleep(advance.idle);
        }
```

**And `fps` now means something different.** It is computed once per pass
through the loop, and with draws gated the loop runs many passes per drawn
frame — so the title bar would report a number the player never sees. Either
move it inside the draw branch against a new `last_drawn: Instant` field, or
relabel it; the phase half of the title (`running {:.1}x` off
`time.achieved`) is honest either way.

### 2. `src/bin/lab.rs` — a key for the display rate

`TimeControl::cycle_display_rate()` steps 60 → 30 → 20 → 10 → 60. `F` is
free:

```rust
            KeyCode::KeyF => self.lab.time.cycle_display_rate(),
```

...and a row in the doc-comment key table: ``| `F` | display rate: 60 / 30 /
20 / 10 Hz |``.

### 3. `src/lab/mod.rs` — one row in `HELP`

```rust
    "F        DISPLAY RATE",
```

`every_help_line_is_drawable` will check it; every character is in the font.

---

## What the dial does, for anyone reading the lab cold

- **The tick is never scaled.** `clock.rs` measured the alternative — the same
  number of organism ticks at 4x `growth_slowdown` gave a median 0.61x final
  cells across 8 seeds. More ticks is exact; faster subsystems is a behaviour
  change wearing a speed control. Nothing in `time.rs` reads or writes a
  cadence.
- **The catch-up loop is bounded by wall clock, never by tick count**, so a
  dial past what the box can do produces what the box can do and says so.
- **The debt never exceeds one displayed frame's worth.** A brief hiccup is
  caught up, as a fixed timestep is meant to; a *sustained* shortfall is
  discarded rather than banked. Banking it compounds — the request climbs for
  ever and the world sprints the moment the load lifts.
- **The achieved rate is measured over real seconds**, including the render
  and the event pump, over a 500 ms window. An earlier draft divided by the
  tick loop's own time, which flatters the number by exactly the quantity
  Gate 3 is about.
