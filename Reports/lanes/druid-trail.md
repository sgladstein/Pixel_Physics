# Lane D — the trail, 2026-09-14

Coordinator: `session_01TngZpRY8LoqpFWHuXjUTTD`. Branch `claude/druid-trail`,
cut from `main` at `9fd637a7`. Files owned and touched: the trail functions in
`src/druid/mod.rs`, the scent rendering in `src/druid/hud.rs`, and one new
instrument, `examples/druid_trail.rs`. Nothing else is touched — in
particular **`pheromone::DECAY_RHO` is untouched at 0.03**, and so is
`pheromone::DEPOSIT`, `DIFFUSE` and `PHEROMONE_INTERVAL`. The lab's foraging
work stands on exactly the constants it stood on this morning.

## The item

Owner, 2026-09-14 playtest: *"my pheramone trail should last way longer. it
dissapears so much faster than an ant could even move. I wonder if this is a
problem for ant laid trails too. The animation can be improved too. it should
be more diffuse looking, not like a bunch of dots."*

Scope ruling in the brief: *"you make the gnome laid trail last longer. The
evolution lab will explore the ant laid ones."*

## The headline

Real app (`Druid::update`/`Druid::draw`), one 143-cell walk over **soil**:

| | at the start | first pass | **now** |
|---|---|---|---|
| trail still on the ground | 3.5 s | 14 s | **30 s** |
| …in ant-cells walked | 35 | 135 | **300** |
| strength left after 10 s | 0 | 18 | **121** of 255 |
| height, cells above the surface | +3 (chest) | −4 (buried) | **+1** |
| marks buried, of 64 sampled | 0 | 64 | **0** |

**Two passes**, the second answering the owner's verdict on the first: *"Make
it last even longer (at least 2x more)... if it is fully underground, an ant
wont smell it either... just at/slightly above ground versus fully below."*
**2.1x** on his ask; **8.6x** on where the round started.

## What was wrong, and what it took

**Diffusion, not decay, removes a trail here.** `DIFFUSE` blends every cell a
quarter toward its own 3x3 mean each pass, and a one-cell line has six empty
neighbours of nine — so it sheds **~17% a pass** against `DECAY_RHO`'s **3%**.
Depositing harder barely helps (255 on a line = 4.8s); **width** is the lever
(`r = 3` at the *unchanged* deposit = 10.8s). The same fact is why it drew as
dots: a one-cell mark rounds to nothing a cell or two out, so there was no
cloud to draw.

**But width has a ceiling and it is not mine.** A cell laid once cannot pass
**~20s**: from a saturated 255, `DECAY_RHO` plus the LUT's forced strict
decrease is ~67 passes to 33 and 33 more at one-per-pass. Measured — `r = 12`,
wide enough that spreading costs its middle almost nothing, reaches **15.8s**.
So 2x needed a different mechanism, and the one it needed is what
`TRAIL_PER_SECOND`'s doc already called the verb: **a standing instruction**.
`Druid::step_trail` renews the remembered route once per pheromone pass toward
a target that **falls with each mark's age** — held, not added, so the death is
graded rather than a cliff, and at 28s the oldest half has expired while the 75
cells nearest him still stand.

**The anchor was wrong twice, and the second time my own number hid it.**
`Player::feet` is the surface only on bare rock: `wade_rows` is 4 of his 14
rows, so **a gnome on powder is sunk four rows into it by design**. The census
that blessed `feet` measured *"drop to the first solid cell below the mark"* —
**0 for a mark on the surface and 0 for one buried four cells inside it**. It
is signed now: old anchor **−4, 64/64 buried**, new **+1, 0 buried**, on a
route whose surface censuses `Powder` 64 of 64. `creature::colony_surface`
rises out of solid before taking the top solid row, so it answers from a
buried point and looks through canopy.

**The four changes and what they cost: `PR_BODY_LANE_D.md` on this branch** —
not repeated here; a lane note is for what another lane needs.

## The owner's aside — *"is this a problem for ant laid trails too"*

**Yes, identically**, and it was free: the shipped gnome trail *was* an ant
trail (`TRAIL_DEPOSIT` was defined as `pheromone::DEPOSIT`, laid one cell at a
time), so every "before" number above is also one ant laying one unreinforced
route. A mark a scout lays and does not re-walk is off the ground in ~3.5s
against a ~37s round trip. **What keeps ant trails alive is reinforcement, not
the mark's own life** — a working system, but it means a colony cannot recruit
to anything one ant found and left. **The lever is not `DECAY_RHO`**: at 3% a
pass it is not what removes the trail, so a decay sweep measures a small term.
Independently confirmed by PR #432 from the lab side, below. Measuring this was
mine; fixing it is not, and nothing ant-laid was changed.

## Instrument

`examples/druid_trail.rs`, with a row in `Reports/instruments.md` that says
what it answers and how it overlaps `pherolife`. Two modes: a plane-side sweep,
and `shot=`, which drives the real `Druid::update`/`Druid::draw` loop
headlessly and prints the plane's peak, live route cells, route geometry and
**clearance to the ground** per frame.

**Its `selftest`'s five positive controls earned their keep twice in one
session**, and both failures are recorded in the source:

- the decay loop advanced `frame` by 1 while `Pheromones::step` gates on
  `frame % 12`, so its "passes" were ticks and every lifetime printed **12x
  too long** behind a perfectly plausible decay curve;
- the repair then advanced by 12 from an *unaligned* frame and never hit a
  multiple again, so no pass ran at all and every arm reported a flat curve at
  its laid value. Control B caught it; control E was added for it.

A third was caught by asking what the number counted: the `slope` column
sampled the last cell of the *route*, which at a stride above one the gnome
never steps on, and so reported the newest end of the trail *weaker* than the
oldest — the reverse of the property it exists to check.

**And the harness disagreed with the app by 3x even after both repairs**, so
every headline number above is the app's. A clean line on an empty plane is a
best case; a real walk is not.

## Gates

All green on the merged tree, and **CI green on `b8b31edc`: all 9 checks**
(clippy, `cargo test` release *and* debug, `ascii`, structural acceptance,
worldgen interference, branches, docscheck, fmt).

Locally: `cargo clippy --all-targets --release --locked -- -D warnings` clean;
`cargo test --release` — the **full** suite, not `--lib`, because `--lib`
cannot reach `tests/*.rs` at all — 1,810 + 2 + 10 lib/bin, 3
`tests/determinism.rs`, 44 `tests/worldgen.rs`, **0 failed**;
`scripts/docscheck.sh`, `scripts/acceptance.sh` and `scripts/worldgencheck.sh`
clean. `deadendindex.py --touching`: **0 entries** name an identifier this
branch adds, and `dead-ends.md` was grepped directly for `TRAIL_DEPOSIT`,
`lay_trail`, `SCENT_HALO`, `SCENT_BANDS`, `DECAY_RHO`, `decay_lut` and
`PHEROMONE_INTERVAL` before any of this was built.

**`branchcheck.sh --gate` exits 1 in this container and it is not a defect** —
it says so itself: the clone is shallow, so ancestry is unknowable here. CI
checks out at full depth and its `branches` job passes.

## Notes for whoever follows

- **`PR_BODY_LANE_D.md` was overwritten**, as the brief asked; what was there
  was the already-merged bubble-aura lane's body, which lives in that PR.
- **Widening the swath further is not a lever.** Real-app lifetime by radius
  was r=3 **14s**, r=4 15s, r=5 ~17s, r=7 ~18s, and every radius above 3 pins
  the plane at 255 outright. The renewal is what moved it to 30s.
- **`TRAIL_LIFE_SECONDS` is the dial if the owner wants more or less.** It is
  a clean knob: the trail holds its level for that long, dimming, then goes.
  Nothing else needs re-deriving with it.
- **If a third instrument is ever wanted here, it is a radius dial on
  `pherolife`**, not a new binary — see the run-drive gap above.

## Reply to the coordinator's two corrections

**First** (19:40Z): `DECAY_RHO` is inert, the lever is `DIFFUSE`, the new
per-channel setter is shared, so *"is there a druid-only way?"* — with "the
only lever is shared" offered as an acceptable answer.

The two measurements agree, arrived at independently: diffusion **16.7%** a
pass against decay's **2.9%** (#432) and **~17% / ~3%** here; ant-laid trails
have the same defect either way.

**The answer is yes, and the lever was not on its list: the mark's *width*,
then its *renewal*.** Diffusion drains a line into *empty* neighbours — that is
the whole of the 16.7% — so a cell mid-band sheds almost nothing. Neither the
swath nor the renewal touches `DIFFUSE`; both are laid by `Druid::lay_trail`
and `Druid::step_trail` alone, and nothing ant-laid moves.

**Why #432 could not have found it.** `pherolife` sweeps `rho`, `diffuse` and
`deposit` over a trail it lays **one cell wide**: width is a constant of the
harness, not a variable. See the proposed rule below.

**Second** (21:35Z, with the owner's verdict): `wade_rows` sinks him four rows
into powder, so `feet()` is under the surface — **correct, confirmed to the
cell**, and it named the right repair (`colony_surface`) before I measured it.
Its 2x target needed the renewal, which its own suggestions (re-lay while held,
deposit harder, a separate mark) gestured at; the ceiling measurement above is
why none of the cheaper ones reach it.

**What this lane still has not measured.** `pherolife` carries a **run drive**
counter and reports the shipped trail **stops steering at frame 48** with 77
cells standing — a better measure than presence. This lane measured presence
and strength. The standing trail holds 61 of 255 at 20s where the old one was
at 0, and the slope guard passes, so the gradient should be readable far
longer — but that is an **inference**, and a radius dial on `pherolife` settles
it in one run against its own run-drive column.

## One proposed rule, for the coordinator or the owner to place — not edited in

> **Ask what your harness is holding still, not only what it sweeps.** An
> instrument that fixes the answer as a constant reports that the levers it
> *does* vary are the only ones there are — and it looks exactly like a
> thorough sweep. The tell is a sweep in which every arm fails the same way,
> which `CLAUDE.md` already has — but that rule says *suspect the rider*, and
> this one says *suspect the constant*.

**Deliberately not added to `CLAUDE.md` by this lane** — the most contested
file in the repo, loaded before every session in all three games. Placing it
is the owner's call.

## Head

PR [#438](https://github.com/sgladstein/Pixel_Physics/pull/438). Opened by the
coordinator from an early draft of `PR_BODY_LANE_D.md`; its body has been
updated in place twice since, and carries the current numbers.

Review cards: `20260914T202113978Z-80ad3b` (the swath — answered, visual
approved, two changes asked) and `20260914T222659700Z-f1e8e1` (this pass —
posted, `owner_can_see_it: true`, all 8 frames verified on the remote).

Head `ce0382da` plus the doc commit that carries this line.
