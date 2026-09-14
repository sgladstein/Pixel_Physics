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
`src/bin/druid.rs` makes), one walked route of 143 cells:

| | before | after |
|---|---|---|
| trail still on the ground | **3.5 s** | **~14 s** |
| …in ant-cells walked | 35 | 135 |
| strength at t+3.5s | 0 | 88 of 255 |
| route cells still holding scent at t+3.5s | 0 of 143 | **143 of 143** |
| where the scent sits | 3 cells above the floor | on the floor |

A colony round trip is roughly 367 ant-cells, so the trail went from about a
tenth of one to about a third. It is longer, not long.

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

- **The scent was laid at his chest.** `lay_trail` marked `Player::center`, a
  median **3 cells above the floor** on this build. Once the swath made it
  wide enough to see, it read as green mist hanging at his waist over clean
  ground — and it is the half of the plane an ant can never reach, since an ant
  senses around its own head while walking the floor.
- **The readout had one colour and seven unreachable ones.** The band was
  `v * SCENT_BANDS / 256`, so everything under 32 fell in band 0 — and the
  plane never held more than about 31 along a route. Every mark of every trail
  drew in the same single dimmest colour.

## What shipped

1. **`druid::TRAIL_RADIUS = 3`** — a graded swath about him rather than one
   cell, strongest under his feet and fading to the rim. At the knee of the
   curve: 1.0 / 7.0 / 10.8 s for r = 0 / 2 / 3, and r = 5 buys 15.6 s for
   twice the per-tick write.
2. **`lay_trail` anchors at `Player::feet`.** Clearance to ground 3 → 0.
3. **`hud::band_of`** — bands off `sqrt(v/255)`, so the small values a cloud's
   edge is made of resolve instead of collapsing into band 0.
4. **The readout blends rather than stamps** — alpha carries strength
   alongside colour, so the core lands opaque and the rim fades into the
   world.

## What it costs, plainly

- **29 plane writes a tick while `G` is held**, against 1. Nothing in the
  sweep changed; a deposit is a `u8` write plus a `write_watch` mark.
- **The plane does now run close to saturation along his route** — peak 241 of
  255 against about 80 — because he re-marks each cell some ten times at 0.6
  cells/tick. That is what the extra lifetime is bought with, and it means an
  ant walking his road adds 14 rather than 40. Stated rather than tuned away:
  `TRAIL_DEPOSIT` is left at one ant's mark, because raising it pins the plane
  outright for 2.8 s more life and a pinned trail is one nothing can reinforce
  (`pheromone::DEPOSIT`'s P-14).

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

#432 was unmerged when this was written, so nothing here builds against
`set_channel_diffuse`, and `DIFFUSE` is untouched.

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
