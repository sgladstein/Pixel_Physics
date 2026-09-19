# The field's response to a walking colony — mechanism, two switches, and what they buy

*Lane P of the ant-survey follow-up round
(`ant-survey-followup-brief-2026-09-19.md`),
answering §8 of `ant-sim-research-review-2026-09-19.md`. **Both of those are
in flight on `origin/claude/ant-sim-research-review-eyuol2` (PR #475) and are
named rather than linked for that reason** — when it lands, the links work and
`Reports/README.md`'s in-flight section loses its entry.
All runs on one four-core container, `RAYON_NUM_THREADS=4`, release build of the
same commit, examples rebuilt in the same command as every run. Read the
counters and the hashes; the milliseconds are for this box only.*

## 0. The answer, stated once

**§8.2's mechanism is real and its premise is sound.** A chunk dirtied only by
creature steps does keep its field tile solving, and **no channel the field
solves reads a creature cell** — with one correction §8.2 did not make, which
turns out to be the load-bearing detail (§1.2).

**§8.5's step 2 splits into two halves, and only one of them is free.**

- **The write-seam half** — an ant's step stops marking its block stale, so
  `rebuild_blocked` inherits instead of re-reading 256 CA cells — is
  **bit-identical** on both the world hash and the field hash, under a guard
  that has been watched going red. It ships as `FIELD_CREATURE_WAKE=blocked`.
- **The solve-set half** — the tile is not woken at all — is **not
  bit-identical, and the reason is not a channel reading a creature cell.** It
  is the three momentum passes, which run over the solve set: an ant-woken tile
  was being pressure-stepped because an ant walked through it, and stopping
  that changes how a disturbance propagates. `field.rs`'s own reverted
  per-tile momentum subset is the same shape and carries the ruling — *"it is
  wind the player can see, and changing how it moves is the owner's call and
  not a side effect of a performance pass."* It ships as
  `FIELD_CREATURE_WAKE=0`, off, with the divergence measured per channel (§2.3).

**And both buy almost nothing on any bed this tree can build, for a reason
worth knowing.** The brief's falsifier (b) asked whether awake chunks at two
hundred ants fall back toward the no-ant figure. **They do not, and the field's
solve set does not move by a single tile** — 65.4 tiles/tick with the switch on
and 65.4 with it off (§2.4). The lab bed is 128 chunks and already solves ~65
of them for plants and sky, so every chunk an ant is standing in is inside the
one-tile halo of something the field was going to solve anyway. What the ants
*do* add is **block rescans** — 45.9/tick empty against 63.3 at two hundred
ants — and the write gate recovers 2.6 of those 17.4. On the clock that is
0.002 ms of a 0.117 ms field, inside the arms' own spread (§2.5).

So §8.3's headline — *the field's response to the ants costs more than the
ants* — survives as a **description of where the cost is** and not as a lever:
the response is the CA rescan, not extra tile solves, and most of the rescan is
charged to what the ants do (dig, drop, disturb soil water) rather than to
where they are.

**One unrelated bug fell out of the first run**, filed as
`open-bugs-handoff.md` §Z31 and given its own switch so it cannot confound this
one: `field::step`'s carry decision keys on `Chunk::is_settled()` on a premise
that measurably does not hold, and those tiles solve one frame against the
occupancy they had before the write (§3).

**And the numbers in §8.3 do not reproduce at that size on this box today.**
Same command, same bed: the field goes **0.081 → 0.117 ms** for 52 ants, not
0.090 → 0.188, and solves **29.1 → 30.4**, not 32.1 → 36.2. The direction is
the same and the ratio is 1.44x rather than 2.09x. Re-measured in this session
on this machine, as `CLAUDE.md` requires before quoting a delta.

## 1. Item 1 — the mechanism, verified in code

### 1.1 The wake path, by line

| step | site |
|---|---|
| an ant's step is a cell write | `creature.rs`, through `World::set` or `parallel::ChunkView::set` |
| the write marks the chunk | `World::write_cell` (`src/sim/world.rs:9144`) → `Chunk::set_world` (`src/sim/chunk.rs:734`) → `Chunk::mark_dirty` |
| the mark sets two things | `pending_dirty` (the CA's next sweep region) **and** `stale_blocks` (which blocks the field must re-derive) |
| the mark reaches `dirty` | `Chunk::end_sweep` (`chunk.rs`), which promotes `pending_dirty` |
| "awake" is exactly "has a sweep region" | `Chunk::is_settled` = `sweep_region().is_none()` |
| the field's global early-out cannot fire | `field::step`: `active_chunk_count() == 0 && fields_settled() && !amplitude_changed && !sky_temperature_changed` — a walking colony keeps the first term non-zero |
| **the solve set is seeded from it** | `field::step`: `chunk_awake = !c.is_settled()` → `awake.insert(coord)`, then a one-tile halo, then `read` = that plus another ring |
| the derived arrays are re-read from the grid | `rebuild_blocked` (`field.rs:2903`), over the `rescan` subset and its `stale_blocks` mask |
| the carry alternative | `previous.derived_valid && carry_derived() && c.is_settled()` → `inherit_derived`, else `rescan` |

So yes: **a chunk dirtied only by creature steps keeps its tile in the solve
set, and keeps its written blocks in the rescan mask.** Confirmed.

Worth adding, because it bounds the whole proposal: **`rebuild_blocked` is the
only place in `field.rs` that reads the CA grid at all.** One `Chunk::get_world`
(`field.rs:3049`) and one `materials.get` (`:3082`); the file's only other grid
read is in a test. Every one of the six channels is solved from the five arrays
that scan derives. Whatever an ant costs the field, it costs it through that
one scan and through the tiles the wake pulls in.

### 1.2 Does any channel read a creature cell? Per array

| derived array | reads a creature cell | how |
|---|---|---|
| `blocked` | **no** | `matches!(mat.kind, Solid \| Plant)` only. `Creature` is excluded *by name*, with its own paragraph: "a single mobile worm cell isn't a wall the way a stationary structure is" |
| `transmission` (`column_depth`) | **no** | the same `Solid \| Plant` branch, plus `Liquid`'s `water_opacity` for optical depth |
| `moisture_source` | **no, by content** | `Liquid` cells, and `mat.water_capacity > 0` soil. No creature material sets `water_capacity` |
| `glow` | **yes, and this is the correction** | `glow_level = glow_level.max(mat.glow)` runs for **every cell of every kind**, creature included. It returns 0 only because no creature material sets `glow` |
| `beam` | **yes, same shape** | `column_beam[dx] = column_beam[dx].max(mat.beam)` — unconditional |

And the traffic runs one way. Creatures **read** the field —
`LightHere` (`creature.rs:5077`), the temperature sense (`:5084`), the worm's
thermotaxis (`:525`, `:588`) — and nothing in `creature.rs` writes into it;
`add_heat` appears there only in tests.

**So §8.2's "a creature cell itself neither blocks a field block nor sources
moisture" is true, and is not the whole claim the switch needs.** Two of the
five arrays are read from a creature cell today and answer zero because of what
is in `assets/materials/ant*.ron`, not because of the cell's kind. A firefly, a
lit fungus gnat, a glowing lure — any of them makes the premise false, and it
would fail *silently*: the tile would stop being re-derived and the glow would
never reach the light channel.

That is why `Material::field_inert` is keyed on the emission rather than on
`MaterialKind::Creature`, so such a material simply drops out of the skip and
costs what it costs. And it is why the switch has a negative control keyed on
the kind alone (§2.2): the difference between those two predicates is exactly
the class of error that matters, and a guard that cannot tell them apart is
not evidence about either.

**One more hazard closed on the way.** The predicate was first landed as a
precomputed `bool` on `Material`, filled in by the one constructor — complete
by construction, one `Vec` index to read, which is what `CLAUDE.md`'s *guard
hot-path work at the call site* asks for. It was wrong: `lab::params::write`
and `app.rs`'s tunables panel both assign `glow` (and `water_capacity`) through
`MaterialRegistry::get_mut` on a live world, so the cached answer goes stale
the moment the owner drags the slider that makes the premise false. It is
computed per call now. The cost is four compares on a struct the caller has
already fetched, and the caller short-circuits on the switch before fetching
anything at all.

## 2. Item 3 — the switch, and the two falsifiers

Three settings, all off by default, on `FIELD_CREATURE_WAKE`:

| value | write-seam gate | solve-set gate | bit-identical |
|---|---|---|---|
| unset | — | — | the shipped engine |
| `blocked` | yes, keyed on `Material::field_inert` | no | **yes, measured** |
| `0` | yes, same predicate | yes | **no** — §2.3 |
| `kind` | yes, keyed on `MaterialKind::Creature` alone | no | a negative control, never a setting |

### 2.1 Falsifier (a): the field hash under a walking colony

`RAYON_NUM_THREADS=4 ./target/release/examples/lab_cost colonies=<n> frames=1500 every=1500 phases=1`

| bed | arm | world hash | field hash |
|---|---|---|---|
| `colonies=0` | off | `0xd33485e1280e26be` | `0x60a8741f9bfae2da` |
| `colonies=0` | `0` | `0xd33485e1280e26be` | `0x60a8741f9bfae2da` |
| `colonies=1`, 52 ants | off | `0xd2aaac85890342fc` | `0x3dbc2e53198b296f` |
| `colonies=1`, 52 ants | **`blocked`** | `0xd2aaac85890342fc` | **`0x3dbc2e53198b296f`** |
| `colonies=1`, 52 ants | `0` | `0xd2aaac85890342fc` | `0x534470711324db56` |

`blocked` is identical on both digests. `0` leaves the **world** identical over
1,500 frames — no creature decision changed — and moves the field.

### 2.2 …and the control that makes that green mean something

A matching hash is evidence about the hash until the guard has been watched
going red. `antcost antglow=<v>` sets the colony material's `glow` on the live
registry before the first tick, which makes the premise false by construction;
`FIELD_CREATURE_WAKE=kind` is `blocked` with the emission terms dropped from
the predicate, so the two arms differ in **one** term.

`RAYON_NUM_THREADS=4 ./target/release/examples/antcost ants=200 width=1024 colony_species=ant par=off rounds=60 frames=200 reps=1 grow=1500`

| arm | field hash | |
|---|---|---|
| off | `0x882079f387ccb96d` | |
| `blocked` | `0x882079f387ccb96d` | identical |
| `kind` | `0x882079f387ccb96d` | identical — the predicates agree when nothing emits |
| off `antglow=2` | `0x32376eebab435e6d` | the glow reaches the field at all: a second control, free |
| `blocked antglow=2` | `0x32376eebab435e6d` | **identical** — the sound predicate keeps it green |
| `kind antglow=2` | `0x583e2b32eb435e6d` | **different** — the guard goes red |

So the field hash can see exactly the error the switch could make, and the
shipped predicate does not make it.

### 2.3 Why `0` is not bit-identical, and it is not a channel reading a creature cell

One env switch, changing nothing else, settled it in a single run — with
`FIELD_MOMENTUM=0` in **both** arms the two field hashes are identical
(`0x7adc3e31e97c5edf`). The whole divergence is pressure, velocity and
advection, which run over the solve set.

Per channel, `colonies=1`, 1,500 frames, from `lab_cost fielddump=`:

| channel | max abs delta | its settle epsilon | ratio | largest value in the arm | cells differing |
|---|---|---|---|---|---|
| pressure | 0.00993 | 0.01 | **0.99** | 0.407 | 416 / 640 |
| vx | 0.00453 | 0.001 | 4.53 | 0.0065 | 358 / 640 |
| vy | 0.00416 | 0.001 | 4.16 | 0.0060 | 338 / 640 |
| temperature | 0.02601 | 0.02 | 1.30 | 26.0 | 123 / 640 |
| light | 0.01357 | 0.005 | 2.71 | 2.40 | 28 / 640 |
| moisture | 0.00579 | 0.005 | 1.16 | 4.00 | 254 / 640 |

Read the ratio column and then the one beside it. Only pressure comes in under
its own epsilon. The scalar channels are over it by 1.2x–2.7x but are tiny
against their own scale (temperature 0.1%, light 0.6%, moisture 0.14%). The
**velocity** channels are over by 4x *and* the delta is 70% of the largest
value in the field — the air's motion in a sealed box with no wind in it was
substantially the ants stirring it by walking, and the switch takes that away.

Which is the reverted per-tile momentum subset's finding arriving from the
other direction, and the same ruling applies: the old behaviour is the
accident, and changing it is the owner's call rather than a side effect of a
performance pass. Hence `0` and `blocked` are separate settings and `blocked`
is the one that is free.

### 2.4 Falsifier (b): the counters. It does not fire, and the reason is the halo

`RAYON_NUM_THREADS=4 ./target/release/examples/antcost ants=0,300,600,1000 width=1024 colony_species=ant par=off rounds=60 frames=200`

| arm | ants asked | awake chunks/tick | **field tiles solved/tick** | blocks rescanned/tick |
|---|---|---|---|---|
| off | 0 | 21.4 | 64.8 | 45.9 |
| off | 300 | 27.9 | 65.4 | 63.3 |
| off | 600 | 31.1 | 65.1 | 64.8 |
| off | 1000 | 27.9 | 65.4 | 63.3 |
| `blocked` | 0 | 21.4 | 64.8 | 45.9 |
| `blocked` | 300 / 600 / 1000 | 27.9 | 65.4 | **60.7** |
| `0` | 0 | 21.4 | 64.8 | 45.9 |
| `0` | 300 / 600 / 1000 | 27.9 | 65.4 | **60.7** |

Three things to read off it, in order of how much they change the proposal.

**The solve set does not move by one tile.** 65.4 with the switch on, 65.4 with
it off, and `0` is indistinguishable from `blocked` on every counter. The
brief's falsifier (b) — awake chunks at two hundred ants falling back toward
the no-ant figure — does not fire, and not because the mechanism is absent.
The bed is 128 chunks and the field already solves ~65 of them for the plants
and the sky, so every chunk an ant occupies is inside the one-tile halo of a
tile that was going to be solved anyway. **The halo is what defeats the
solve-set gate**, and it will keep defeating it on any bed dense enough to be
worth playing. (`awake/f` is the CA's own awake count, which the field switch
does not touch; it is in the table because the brief named it and because the
27.9 / 31.1 / 27.9 wobble is the harness reading a different rep of the same
bed, not three populations.)

**The write gate does fire, and recovers 15% of what the ants add.** 63.3 →
60.7 against a no-ant floor of 45.9: the ants add 17.4 block rescans a tick and
the gate gives back 2.6 of them. The remaining 14.8 are marks the ants earn
honestly — a dig writes `Solid`, a spoil drop writes `Solid`, and the soil-water
pass re-marks whatever ground they disturbed. Which is the finding that actually
bounds §8.5 step 2: **most of the field's response to a colony is charged to
what the ants do, not to where they are**, and no wake rule can reach the part
that is a real occupancy change.

**The stocking loop saturated**, exactly as the brief warned. `ants=300`, `600`
and `1000` produce byte-identical world hashes — one population wearing three
labels. The 1024-wide bed with `rounds=60` seats about two hundred, so this
table has two populations in it and not four.

### 2.5 …and on the clock

Paired and alternating, four reps of each arm round-robin in one sitting,
minimum per arm (the lower envelope — contention can only make a run slower).
`lab_cost colonies=<n> frames=1500 every=1500 phases=1`, the `field` column of
the per-phase table:

| arm | `colonies=0` | `colonies=1` (52 ants) | field solves/tick |
|---|---|---|---|
| off | 0.081 ms | **0.117 ms** | 29.1 → 30.4 |
| `blocked` | 0.081 | 0.115 | 29.1 → 30.4 |
| `0` | 0.081 | 0.113 | 29.1 → 30.4 |

Per-arm spread across the four reps at `colonies=1`: off {0.117, 0.117, 0.121,
0.121}, `blocked` {0.115, 0.115, 0.118, 0.118}, `0` {0.113, 0.115, 0.119,
0.122}.

Fifty-two ants cost the field **+0.036 ms, a 1.44x**. `blocked` gives back
0.002 of that and `0` gives back 0.004 — 6% and 11% of the ants' increment,
0.4% and 0.8% of the whole tick, and **both inside their own arm's spread**.
The counter says the mechanism fired; the clock says it bought nothing
measurable. That pair is the result, and it is why the counters were read
first.

## 3. The bug the first run found — `open-bugs-handoff.md` §Z31

The switch's first version bundled a third gate: a settled chunk holding an
un-taken stale mark rescans its derived arrays instead of inheriting them.
`field::step`'s carry decision asks `Chunk::is_settled()`, on the premise its
own comment states — *an awake chunk always seeds its own tile, so by the time
a chunk settles the field has taken its mask.* That gate moved the field hash
**on a bed with zero creatures in it**, which is how it was caught, because
that is not a thing a creature switch may do.

It is a real defect and it is not this switch's: `FIELD_CARRY_STALE=1`, alone
and with no creature switch set, moves `lab_cost colonies=0 frames=600`'s field
hash from `0x77761fab0c35bfce` to `0xd67e172d6d16b8a4`. Two candidates were
ruled out by ablation rather than by argument — the soil-moisture phase's
placement (`PIXEL_PHYSICS_MOISTURE=sweep` in both arms, still diverges) and
`Chunk::mark_moist_dirty`'s own stale mark (ablated in both arms, still
diverges). The mechanism that survives is the ordering of `end_sweep` against a
cross-chunk write: a write into chunk C from another chunk's sweep lands in C's
`pending_dirty` and its `stale_blocks`, but `dirty` is only promoted at C's own
`end_sweep`, which for a C swept earlier in the pass has already run — so at
`field::step` C reads settled while holding a live mark, inherits, and catches
up a frame later. Full entry, with what is and is not established, in the
register.

Left default-off and filed rather than fixed here, for the reason the brief
gives about scope: a one-frame lag in a coarse ambient channel may be
immaterial, deciding that is not this round's call, and bundling it would have
made every number above a measurement of two changes.

## 4. Item 2 — the per-phase stopwatch in the live app

`PIXEL_PHYSICS_PHASE_CLOCK=1`, off by default, woven through the eight phases
of `sim::frame::step` — **the one copy of the tick**, rather than a wrapper,
because a wrapper would need a second copy of the sequence and that module
exists to prevent exactly that. `examples/lab_cost.rs`'s own `PHASES` table had
to re-type it and is guarded for it; this does not.

The names, order and column widths match that table so a row from the owner's
session and a row from `lab_cost phases=1` are comparable without translation.
The times land in the CENSUS row of the chronicle — the file his session log
already writes — as a `ticks` column plus one column per phase, **and** as a
readable line in the addendum under it:

```
 | ticks ca_sweep liquid_b chunk_bo   player active_s particle    field pheromon
 |   600    0.356    0.000    0.000    0.000    0.063    0.000    0.117    0.053
        tick 0.590 ms over 600 tick(s), 6 awake chunk(s) | field 20% (0.117 ms)
        | active_sites 11% (0.063 ms) -- every creature decision and every plant
        tick is in there, nowhere else | dearest: ca_sweep 60%, field 20%
```

Four things about it that are deliberate:

- **It is in the same row as `awake_chunks`**, which the brief asked for and
  which is not cosmetic: the mechanism that makes the field track ant count is
  the chunks the ants wake, so a share that moved with an awake count that did
  not means the cause is elsewhere.
- **Each row is the window since the previous row**, not a session mean. A
  running mean over a hundred thousand ticks cannot show the field's share
  moving as a colony grows, which is the whole question.
- **It prints `--`, never `0.000`, when the clock was off** or when no tick has
  run since the last row — the distinction the perf columns already make,
  because a phase that costs nothing is a finding rather than an absence.
- **`active_sites` is named in the addendum as where the creatures are.** Every
  creature decision and every plant tick is inside `scheduler::step`; nothing
  else in the eight is. That sentence is there because the obvious guess is
  `ca_sweep`, and a reader who guesses wrong reads the whole split backwards.

`#374` declined stopwatches in the live loop and the objection was right for an
ungated one. Gated, the cost when off is one `OnceLock` read and eight `Option`
tests **per tick**, against a tick that costs hundreds of microseconds; no
`Instant::now` is called and no accumulator is locked. Measured in §5.

On the shipped lab bed at 52 ants it already answers the question §8.4 left
open for that bed: **the field is 20–21% of the tick and `ca_sweep` is 56–60%**,
with `active_sites` — the ants and plants themselves — at 11–14%. The owner's
bed is two orders of magnitude more populous and that is the number only he can
take.

## 5. Free when off

Two binaries differing only by this change — `origin/main` at `5f92c761` in a
separate worktree against this branch — run paired and alternating, six reps
each, one sitting, `RAYON_NUM_THREADS=4`, nothing set in the environment. The
`lab_cost colonies=1 frames=1500` bed.

First the output, because a timing that is free and wrong is not free:

| | world hash | field hash |
|---|---|---|
| `origin/main` | `0xd2aaac85890342fc` | `0x3dbc2e53198b296f` |
| this branch, nothing set | `0xd2aaac85890342fc` | `0x3dbc2e53198b296f` |

Then the clock, per phase, summed over the six paired reps and with each arm's
own minimum beside it:

| phase | main, min | branch, min | main, sum of 6 | branch, sum of 6 | delta |
|---|---|---|---|---|---|
| `ca_sweep` | 0.288 | 0.282 | 1.749 | 1.729 | −1.14% |
| `active_sites` | 0.056 | 0.055 | 0.346 | 0.340 | −1.73% |
| `field` | 0.118 | 0.116 | 0.719 | 0.711 | −1.11% |
| `pheromones` | 0.045 | 0.043 | 0.275 | 0.266 | −3.27% |
| **whole tick** | **0.507** | **0.496** | **3.089** | **3.046** | **−1.39%** |

Per-rep tick: main {0.514, 0.525, 0.518, 0.515, 0.507, 0.510}, branch {0.517,
0.505, 0.525, 0.501, 0.502, 0.496}. The branch is faster in four of six.

**Which is noise, and is reported as noise rather than as a win.** The branch
adds work and cannot be faster; the two ranges overlap almost completely, and
`CLAUDE.md`'s rule that a noise bar applies to both signs is exactly the rule
that forbids quoting −1.4% as a speedup. What the table establishes is the
claim that was wanted: **with nothing set, this change produces identical
output and no cost distinguishable from run-to-run spread.**

The mechanism behind that, for the record. When off:

- the write seam pays one `OnceLock<bool>` load and one perfectly-predicted
  branch per cell write, short-circuiting **before** any registry fetch or
  compare — the same budget `World::write_cell`'s own `write_watch.mark`
  already sits on;
- `field::step` pays two `OnceLock` reads per *step*, not per tile, and the
  two-question chunk lookup in the solve-set loop is the one lookup that was
  already there;
- the stopwatch pays one `OnceLock` read and eight `Option` tests per **tick**,
  and calls `Instant::now` zero times.

## 6. Item 4 — the GPU field

A design note only, nothing built:
[`gpu-field-design-2026-09-19.md`](gpu-field-design-2026-09-19.md). Its short
version is that §8.5 step 4's condition — *the field above about a third of the
tick at play population after step 2* — is not met on any bed measurable here
(20–21% at 52 ants, and step 2 moves it by under a percent), that the readback
is the structural problem rather than the solve, and that `PLAN.md`'s
*Simulation device* row would need a different argument than the one it
currently rests on.

## 7. What to do next, and what not to

1. **Land `blocked` as the default? Not on this evidence.** It is free and
   bit-identical, and it recovers 2.6 of 17.4 block rescans, which does not
   show above spread on a 52-ant bed. The number that would justify flipping
   the default is the same one the owner is the only person who can take:
   `PIXEL_PHYSICS_PHASE_CLOCK=1` with a colony of thousands, `blocked` set and
   unset, two sessions on his own bed. If the field's share there is a fifth
   and `blocked` moves it, flip it; if the share is what it is here, the switch
   is documentation of a mechanism rather than an optimisation.
2. **Do not tune the solve-set gate to try to beat the halo.** The halo is one
   ring and it is load-bearing — `step_advection`'s clamp is what makes one
   ring sufficient, and narrowing it is how a tile comes to sample a neighbour
   this frame never populated. The gate loses to the halo because the field
   already solves half a lab bed, and that is a property of the bed.
3. **The 14.8 residual block rescans are the part worth attributing next**, and
   the experiment is cheap: they are either the ants' own `Solid` writes (dig
   and spoil drop) or the soil-water pass re-marking ground they disturbed. An
   arm with the digging verb off would separate them in one run. Not run here —
   the brief puts creature behaviour out of this lane's scope — and named so
   the next session does not re-derive that it is the open question.
4. **§Z31 is a real defect with a reproduction and no owner.** It wants a
   decision (is a one-frame lag in a coarse ambient channel worth a gate?) more
   than it wants code.
5. **Not the parallel switch.** The owner is testing that on his own machine.

## 8. Commands, so every row above is re-derivable

```
cargo build --release --examples          # in the same command as the run

# §2.1, the guard
RAYON_NUM_THREADS=4 ./target/release/examples/lab_cost colonies=0 frames=1500 every=1500 phases=1
RAYON_NUM_THREADS=4 FIELD_CREATURE_WAKE=blocked ./target/release/examples/lab_cost colonies=1 frames=1500 every=1500 phases=1
RAYON_NUM_THREADS=4 FIELD_CREATURE_WAKE=0       ./target/release/examples/lab_cost colonies=1 frames=1500 every=1500 phases=1

# §2.2, the two-sided control
RAYON_NUM_THREADS=4 FIELD_CREATURE_WAKE=blocked ./target/release/examples/antcost ants=200 width=1024 colony_species=ant par=off rounds=60 frames=200 reps=1 grow=1500 antglow=2
RAYON_NUM_THREADS=4 FIELD_CREATURE_WAKE=kind    ./target/release/examples/antcost ants=200 width=1024 colony_species=ant par=off rounds=60 frames=200 reps=1 grow=1500 antglow=2

# §2.3, which pass and how far
RAYON_NUM_THREADS=4 FIELD_MOMENTUM=0 FIELD_CREATURE_WAKE=0 ./target/release/examples/lab_cost colonies=1 frames=1500 every=1500 phases=1
RAYON_NUM_THREADS=4 ./target/release/examples/lab_cost colonies=1 frames=1500 every=1500 phases=1 fielddump=off.bin
RAYON_NUM_THREADS=4 FIELD_CREATURE_WAKE=0 ./target/release/examples/lab_cost colonies=1 frames=1500 every=1500 phases=1 fielddump=skip.bin
# six f32 per field cell, channel-major: pressure vx vy temperature light moisture

# §2.4, the counters
RAYON_NUM_THREADS=4 ./target/release/examples/antcost ants=0,300,600,1000 width=1024 colony_species=ant par=off rounds=60 frames=200

# §3, the bug
RAYON_NUM_THREADS=4 FIELD_CARRY_STALE=1 ./target/release/examples/lab_cost colonies=0 frames=600 every=600 phases=1

# §4, the stopwatch (and this is the one the owner runs on his own bed)
RAYON_NUM_THREADS=4 PIXEL_PHYSICS_PHASE_CLOCK=1 ./target/release/examples/chronicle frames=1200 sample=600 colonies=1
PIXEL_PHYSICS_PHASE_CLOCK=1 cargo run --release --bin lab     # the CENSUS rows of the saved chronicle
```

## 9. What this rests on

Source read at the lines cited: `src/sim/field.rs` (`step`'s early-out, the
solve set, the carry decision, `rebuild_blocked`, `field_hash`,
`field_channels`, the settle epsilons), `src/sim/world.rs`
(`write_cell`, `touch_neighbours`, `mark_dirty_at`, `take_stale_blocks`),
`src/sim/chunk.rs` (`set_world`, `mark_dirty`, `end_sweep`, `is_settled`,
`stale_blocks`, `mark_moist_dirty`), `src/sim/material.rs` (`Material`,
`MaterialRegistry::get_mut`), `src/sim/frame.rs` (the tick order),
`src/sim/parallel.rs` (`ChunkView::set`), `src/sim/creature.rs` (the field
reads), `src/lab/params.rs` and `src/app.rs` (the live `glow` writes),
`src/lab/census.rs` (the chronicle row), `assets/materials/*.ron` (every
`kind: Creature` material's `glow`, `beam`, `water_capacity`).

Reports: `ant-sim-research-review-2026-09-19.md` §8;
`ant-survey-followup-brief-2026-09-19.md` (Lane P);
`evolution-lab-playtest-2026-09-13.md` §1;
`evolution-lab-creature-cost-2026-09-13.md` §2, §5;
`evolution-lab-frame-cost-2026-09-01.md` §2, §11.3, §15.2;
`measurement-under-contention.md` §7; `instruments.md`; `PLAN.md`'s decision
table.

`branchcheck --who-touched`, run before writing into either contested file:
`src/sim/world.rs` — 39 changes landed on `main` in seven days, the most recent
`339f0ad8` six hours before this branch was cut; `src/sim/field.rs` — quiet, two
changes in seven days, the most recent `82429040` five days before. Both quoted
from the tool rather than from memory, and `world.rs` is touched in exactly one
place here for that reason.
