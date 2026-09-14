# Lane D — the trail

**What it does:** the scent the gnome lays now stays on the ground long
enough for the animals he laid it for to act on it, and it looks like scent —
a soft cloud lying along his route — instead of a hard green line that blinks
out behind him.

**Where it sits:** the held world's four-lane round on the 2026-09-14
playtest. This is the pheromone item: *"my pheramone trail should last way
longer. it dissapears so much faster than an ant could even move… it should be
more diffuse looking, not like a bunch of dots."* Both halves of that
complaint turned out to be one defect, which is why one constant answers both.

Measured in the real app (`Druid::update`/`Druid::draw`, the calls
`src/bin/druid.rs` makes), one walked route of 143 cells over soil:

| | at the start | first pass | **now** |
|---|---|---|---|
| trail still on the ground | 3.5 s | 14 s | **30 s** |
| …in ant-cells walked | 35 | 135 | **300** |
| strength left after 10 s | 0 | 18 | **121** of 255 |
| strength left after 20 s | 0 | 0 | **61** |
| height it sits, cells above the surface | +3 (his chest) | −4 (buried) | **+1** |
| marks buried in the ground, of 64 sampled | 0 | 64 | **0** |

**The owner reviewed the first pass and asked for two things**, both done here:
*"Make it last even longer (at least 2x more). I like the visual, but if it is
fully underground, an ant wont smell it either. I think you overcorrected and
it should be a little height, just at/slightly above ground versus fully
below."* That is **2.1x** on the lifetime and the anchor lifted out of the
ground; against where the round started it is **8.6x**.

## What was actually wrong

**Diffusion, not decay, is what removes a trail here, and nothing in the
codebase said so.** `pheromone::DIFFUSE` blends every cell a quarter of the
way toward its own 3x3 mean each pass. For a cell on a one-cell-wide line six
of its nine neighbours are empty, so that mean is about `v/3` and the line
sheds roughly **17% a pass** sideways — against `DECAY_RHO`'s **3%** to
forgetting. Five to six times the term the module's own write-up frames trail
lifetime around.

Three consequences, all measured on the plane:

- **Depositing harder barely helps.** A one-cell line at the shipped deposit
  stays legible **1.0 s**; pushing that deposit to the 255 ceiling reaches
  **4.8 s**.
- **Width is the lever.** At an *unchanged* deposit, an `r = 3` band reaches
  **10.8 s**.
- **The same fact is why it looked like dots.** A one-cell mark rounds to
  nothing a cell or two out, so there was no cloud to draw — the readout was
  drawing the spine of a cloud that did not exist.

Two more fell out of looking at the picture rather than the numbers:

- **The scent was laid at his chest.** `lay_trail` marked `Player::center`,
  about 3 cells above the floor, so once the swath made it wide enough to see
  it read as green mist at his waist over clean ground — and that is the half
  of the plane an ant can never reach, since an ant senses around its own head
  while walking the floor. Moving it to `Player::feet` then overshot into the
  ground; see *Why the anchor had to move twice*.
- **The readout had one colour and seven unreachable ones.** The band was
  `v * SCENT_BANDS / 256`, so everything under 32 fell in band 0 — and the
  plane never held more than about 31 along a route. Every mark of every trail
  drew in the same single dimmest colour.

## What shipped

1. **`druid::TRAIL_RADIUS = 3`** — a graded swath about him rather than one
   cell. At the knee of the curve: 1.0 / 7.0 / 10.8 s for r = 0 / 2 / 3.
2. **The anchor is the ground surface under him, one cell clear** —
   `creature::colony_surface(...) - 1`, not `Player::center` and not
   `Player::feet`. Both of the obvious ones were shipped first and both are
   wrong (below).
3. **`druid::TRAIL_LIFE_SECONDS = 30`, `TRAIL_HOLD = 180`,
   `Druid::step_trail`** — the route is a **standing** instruction he
   maintains, renewed once per pheromone pass toward a target that falls with
   each mark's age.
4. **`hud::band_of`** — bands off `sqrt(v/255)`, so the small values a cloud's
   edge is made of resolve instead of collapsing into band 0.
5. **The readout blends rather than stamps** — alpha carries strength alongside
   colour, so the core lands opaque and the rim fades into the world.

### Why the anchor had to move twice

`Player::center` is half a body up, so the swath drew as green mist at his
waist over ground he had walked clean. `Player::feet` is the bottom of his
*rectangle*, which is the surface **only on bare rock**:
`player::Tuning::wade_rows` is 4 of his 14 rows — "about knee-deep" by its own
doc — so **a gnome standing on any powder is sunk four rows into it by
design**, and his feet are four cells under the soil.

**The census that said `feet` was right could not have said otherwise.** It
measured *"drop to the first solid cell below the mark"*, which is **0 for a
mark resting on the surface and 0 for one buried four cells inside it** — the
scan stops immediately either way. It is signed now and reports the buried
count; putting the old anchor back reads **−4, 64 of 64 buried**, against
**+1, 0 buried**. `CLAUDE.md`'s metric trap in its excavation shape: a number
that cannot tell two opposite states apart reports the one you expected.

`colony_surface` rather than a hand-rolled scan because it rises out of solid
to open air *before* taking the top solid row, so it answers from a buried
point, and it looks through a canopy rather than stopping on leaves.

**And the scene was checked, not assumed.** The route's surface censuses as
`Powder` 64 of 64, so this is measured on exactly the ground the defect exists
on — a run over bare rock could show neither the bug nor the fix.

### Why lasting longer needed a different mechanism

**No width and no deposit can do it.** A cell laid once has a hard ceiling from
`DECAY_RHO` plus the decay LUT's forced strict decrease: from a saturated 255
that is ~67 passes to fall to 33 at 3% a pass and 33 more at one-per-pass, so
**~20 s is the most a single mark can survive** before diffusion is even
counted. Measured against that: a swath at **r = 12**, wide enough that
spreading costs its middle almost nothing, reaches **15.8 s** against r = 3's
10.8 s — five times the per-tick write for 1.5x, and still short. The ceiling
is made of a constant this game does not own.

So the trail became what `TRAIL_PER_SECOND`'s own doc already called it, *"a
standing instruction to the colony"*. `Druid::step_trail` renews the remembered
route once per pheromone pass — **the core cell only, not the swath**, because
`DIFFUSE` spreads a standing mark outward by itself (the case its own profile
sweep measures), which is 29x cheaper than re-laying the disc.

**Held toward a falling target, not topped up by a fixed amount.** Adding pins
the cell at the ceiling for the whole life and then drops it off a cliff at
expiry — the binary outcome this project's first law is named for. Renewing
toward a target that falls with age means the trail dims along its whole length
as it ages, and at 28 s **the oldest half has expired while the 75 cells
nearest him still stand**: it retreats toward him, which is the direction its
slope already pointed.

**Not the treadmill `Reports/dead-ends.md` records twice** on the plant line:
re-laying what decay removes was a dead end there because construction was
*charged* both times. Renewal here is charged once, when he walks the route.

## What it costs, plainly

- **29 plane writes a tick while `G` is held**, against 1, plus **one write per
  remembered mark per pheromone pass** (≤900 marks / 12 frames = ≤75 a frame)
  while a trail stands. A deposit is a `u8` write plus a `write_watch` mark;
  nothing in the sweep changed.
- **`TRAIL_HOLD` is 180 of 255, deliberately below the ceiling**, so an ant
  walking his road still adds a readable 40 rather than clipping flat
  (`pheromone::DEPOSIT`'s P-14). The walk's own swath does still peak near
  saturation for its first seconds; the standing phase, which is most of the
  life, does not.
- **`trail_laid` is parallel to `trail` rather than folded into it** — the
  element type `src/bin/druid.rs` reads stays put, and that file is another
  lane's this round. Kept in lockstep in two places, `debug_assert`ed, and read
  through `zip` so a desync degrades to renewing fewer marks rather than to a
  panic in the player's game.

## Scope

**`pheromone::DECAY_RHO` is untouched at 0.03**, along with `DEPOSIT`,
`DIFFUSE` and `PHEROMONE_INTERVAL`. Per the owner's ruling — *"you make the
gnome laid trail last longer; the evolution lab will explore the ant laid
ones"* — nothing ant-laid changes, and the lab's foraging work stands on
exactly the constants it stood on this morning.

**The owner's aside, measured — *"I wonder if this is a problem for ant laid
trails too."* Yes, identically**, and the answer was free because the shipped
gnome trail *was* an ant trail: `TRAIL_DEPOSIT` was defined as
`pheromone::DEPOSIT` and laid one cell at a time, so every number in the
"before" column is also one ant laying one unreinforced route. A mark a scout
lays and does not re-walk is off the ground in about 3.5 s, against a 37 s
round trip. What keeps ant trails alive is **reinforcement**, not the mark's
own life — which is a working system rather than a bug, but it does mean a
colony cannot recruit to anything one ant found and left. **And the lever is
not `DECAY_RHO`:** at 3% a pass it is not what is removing the trail, so a
decay sweep will measure a small term. Aim at reinforcement rate or mark
width. Measuring this was ours; fixing it is not, and nothing was changed.

## Instrument

`examples/druid_trail.rs`, with a row in `Reports/instruments.md`
(`trailfollow` answers *does a laid trail move a colony* — the other half, not
duplicated). Its `selftest` runs five positive controls and **two of them
fired in anger during this session**: the decay loop advanced `frame` by 1
while `Pheromones::step` gates on `frame % 12`, so every lifetime printed
**12x too long** behind a perfectly plausible decay curve; the repair then
advanced by 12 from an unaligned frame and never hit a multiple again, so no
pass ran and every arm reported a flat curve at its laid value. A third bug
was caught by asking what the number counted. All three are recorded in the
source. The harness still disagreed with the app by 3x afterwards, so **every
headline number here is the app's**.

## The lab reached the same root cause independently

PR #432 (`claude/evolution-lab-pheromones`) measured this from the lab side
while this was being built. **The two agree**: diffusion 16.7% a pass against
decay's 2.9%, here ~17% against ~3%; and ant-laid trails have the same defect.

It concluded the only lever is a **shared** per-channel `DIFFUSE` setter, and
offered "the only lever is shared" as a complete result. **It is not the only
lever.** Diffusion drains a line into *empty* neighbours — that is the whole of
the 16.7% — so a cell in the middle of a band sheds almost nothing and widening
the mark defeats the dominant term **without touching `DIFFUSE` at all**.
Nothing is shared and no new dial is needed.

`pherolife` could not have found it: it sweeps `rho`, `diffuse` and `deposit`
over a trail it lays **one cell wide**, so width is a constant of the harness
rather than a variable. That generalises, and it is written up as a proposed
rule at the end of the lane note — **not** edited into `CLAUDE.md` by this
lane, since placing a rule in the file every session in all three games loads
is the owner's call.

**What this lane did not measure:** `pherolife` carries a **run drive** counter
and reports the shipped trail stops *steering* at frame 48 with 77 cells still
standing — a better effect measure than presence. The swath's peak runs 3x
higher over the same window and the slope guard
`a_laid_trail_slopes_toward_the_newest_end` still passes, so the gradient an ant
reads should be larger for longer; **that is an inference and is named as one.**
A radius dial on `pherolife` settles it in one run.

**#432 has since landed and this branch carries it** (merge `6100c053`).
Checked rather than assumed: it adds the setters and leaves **every shipped
constant where it was** — `DIFFUSE` 0.25, `DECAY_RHO` 0.03, `DEPOSIT` 40,
`PHEROMONE_INTERVAL` 12 — and re-measuring on the merged tree returns the same
numbers to the cell. Nothing here calls the new setter. The one conflict was
`Reports/instruments.md`, where both lanes added a row in the same place;
resolved by keeping all three and cross-referencing.

## Gates

All four green on the **merged** tree (`main` landed 40 commits touching
`src/druid/mod.rs`, `src/druid/hud.rs`, `src/render.rs` and `src/bin/druid.rs`
mid-session; merged clean, and it turns out `main` never touched the scent path
in either file). Re-measured after the merge: identical numbers.

- `cargo clippy --all-targets --release --locked -- -D warnings` — clean.
- `cargo test --release`, the **full** suite rather than `--lib`: 1,810 + 2 + 10
  lib/bin, 3 `tests/determinism.rs`, 44 `tests/worldgen.rs` — **0 failed**.
- `bash scripts/docscheck.sh` — clean (it caught the missing
  `Reports/instruments.md` row, now written).
- `python3 scripts/deadendindex.py --touching` — **0 entries** name an
  identifier this branch adds. `Reports/dead-ends.md` was also grepped directly
  for `TRAIL_DEPOSIT`, `lay_trail`, `SCENT_HALO`, `SCENT_BANDS`, `DECAY_RHO`,
  `decay_lut` and `PHEROMONE_INTERVAL` before any of this was built.

Judge-by-eye, per `CLAUDE.md`: review card `20260914T202113978Z-80ad3b`
(`owner_can_see_it: true`), a before/after frame pair stepping 0 / 3.5 / 8 / 14
seconds, asking whether 14s is enough or whether a trail should outlast a full
colony round trip.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

https://claude.ai/code/session_01CCsDdkXqh6ANtpkN8ff94C
