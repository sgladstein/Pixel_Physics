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

Scope ruling carried in the brief: *"For pheromone, you make the gnome laid
trail last longer. The evolution lab will explore the ant laid ones."*

## The headline

Measured in the real app (`Druid::update`/`Druid::draw`, the calls
`src/bin/druid.rs` makes), one walked route of 143 cells:

| | before | after |
|---|---|---|
| trail still on the ground | **3.5 s** | **~14 s** |
| ...in ant-cells walked | 35 | 135 |
| peak strength at t+3.5s | 0 | 88 |
| cells of the route still holding scent at t+3.5s | 0 of 143 | **143 of 143** |
| where the scent sits | 3 cells above the floor | on the floor |

**Both of the owner's complaints turned out to be one defect**, which is why
one constant answers both.

## What was wrong, in one paragraph

**Diffusion, not decay, is what removes a trail here.** `pheromone::DIFFUSE`
blends every cell a quarter of the way to its own 3x3 mean each pass, and a
one-cell line has six empty neighbours of nine — so it sheds **~17% a pass**
sideways against `DECAY_RHO`'s **3%**. Depositing harder barely helps (the 255
ceiling on a line reaches 4.8s; an `r = 3` band at the *unchanged* deposit
reaches 10.8s). The same fact is why it looked like dots: a one-cell mark
rounds to nothing a cell or two out, so the readout was drawing the spine of a
cloud that did not exist. Two more defects fell out of looking at the picture
rather than the numbers — the scent was laid at his **chest** (`Player::center`,
a median 3 cells above the floor, the half of the plane no ant can smell), and
the readout's band was `v * SCENT_BANDS / 256` while the plane never held more
than ~31 along a route, so **every mark of every trail drew in the same single
dimmest colour**.

**The full write-up, the four changes and what they cost is `PR_BODY_LANE_D.md`
on this branch** — not repeated here, because a lane note is for what another
lane needs.

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

`examples/druid_trail.rs` — new; `Reports/instruments.md` grepped first
(`trailfollow` answers *does a laid trail move a colony*, not duplicated).
Two modes:

- the default sweep, on the plane alone, printing the peak-on-route decay
  curve per arm in seconds **and in ant-cells**, against a colony round trip;
- `shot=` , which drives the real `Druid::update`/`Druid::draw` loop headlessly
  (no window, no GPU) and writes a column sheet with the plane's peak, live
  cell count, route geometry and **clearance to the ground** printed per frame.

`selftest` runs five positive controls. **They earned their keep twice in one
session**, and both failures are recorded in the source:

- the decay loop advanced `frame` by 1 while `Pheromones::step` gates on
  `frame % 12`, so its "passes" were ticks and every lifetime printed **12x
  too long** — a 1.0 s trail reported as 12.0 s, with a perfectly plausible
  decay curve;
- the repair then advanced by 12 from an *unaligned* frame and never hit a
  multiple again, so no pass ran at all and every arm reported a flat curve at
  its laid value. Control B caught that one; control E was added for it.

A third instrument bug was caught by asking what the number counted: the
`slope` column sampled the last cell of the *route*, which at a stride above
one the gnome never steps on, and so reported the newest end of the trail
*weaker* than the oldest — the exact reverse of the property it exists to
check.

**And the harness disagreed with the app by 3x even after both repairs**, so
every headline number above is the app's. A clean line on an empty plane is a
best case; a real walk is not.

## Gates

- `cargo clippy --all-targets --release --locked -- -D warnings` — clean.
- `cargo test --release` — the **full** suite, not `--lib`: 1,804 + 10 lib/bin,
  3 `tests/determinism.rs`, 44 `tests/worldgen.rs`, **0 failed**. Named
  because `CLAUDE.md` records that `--lib` cannot reach `tests/*.rs` at all
  and four merges were gated on it in one day.
- `bash scripts/docscheck.sh` — clean (it caught the missing
  `Reports/instruments.md` row and that row is now written).
- `python3 scripts/deadendindex.py --touching` — 4 files changed, **0 entries
  name an identifier this branch adds**. Silence is not evidence, per its own
  banner; `Reports/dead-ends.md` was also grepped directly for
  `TRAIL_DEPOSIT`, `lay_trail`, `SCENT_HALO`, `SCENT_BANDS`, `DECAY_RHO`,
  `decay_lut` and `PHEROMONE_INTERVAL` before any of this was built. The
  decay/interval entries are about `DECAY_RHO` and the LUT floor, neither of
  which is touched; nothing names the swath or the readout.

## Notes for whoever follows

- **`PR_BODY_LANE_D.md` was overwritten**, as the brief asked; what was there
  was the already-merged bubble-aura lane's body, which lives in that PR.
- **The next lever, if 14s is judged short**, is the trail as a *standing*
  instruction — he keeps paying `TRAIL_PER_SECOND` and the remembered route
  keeps being re-laid, age-graded from the oldest end so the slope survives.
  Measured on the plane it reaches **105s**. It is not built here because it
  is a design decision with an economy behind it rather than a number, and
  `Druid::step_economy` is shared ground. Posted to the owner as the question
  on card `20260914T202113978Z-80ad3b`.
- **Widening the swath further is not the lever.** Real-app lifetime by
  radius: r=3 **14s**, r=4 15s, r=5 ~17s, r=7 ~18s, and every radius above 3
  pins the plane at 255 outright. Five times the per-tick write for four
  seconds.

## Reply to the coordinator's correction (received 19:40Z, after this was built)

The correction said `DECAY_RHO` is inert, the lever is `DIFFUSE`, the new
per-channel setter in PR #432 is shared with the ants, and therefore *"the
question your item actually turns on is: is there a druid-only way to make her
mark persist?"* — listing re-laying, a higher deposit, or a non-pheromone mark,
and adding that **"if the honest answer is still 'the only lever is shared',
that is a complete and correct result."**

**The two measurements agree, arrived at independently**, which is worth more
than either alone:

| | #432 (`pherolife`) | this lane (`druid_trail`) |
|---|---|---|
| diffusion's share, per pass | 16.7% | ~17% |
| decay's share, per pass | 2.9% | ~3% |
| ant-laid trails have the same defect | yes | yes |

**But the answer to its question is yes, and the lever is not on its list: the
mark's *width*.** Diffusion only drains a line *into empty neighbours* — that
is the whole of the 16.7%. A cell in the middle of a band has a 3x3 mean of
roughly its own value and sheds almost nothing, so widening the mark defeats
the dominant term **without touching `DIFFUSE` at all**. Nothing is shared: the
swath is laid by `Druid::lay_trail` and by nothing else, ant-laid marks are
untouched, and no per-channel dial is needed. Real app, 3.5s → ~14s.

**Why #432 could not have found it.** `pherolife` sweeps `rho`, `diffuse` and
`deposit` over a trail it lays **one cell wide**, so width is not a variable it
has — it is a constant of the harness. See the proposed rule at the end.

**The correction's own suggestion, measured, is the weak one.** Deposit is
listed as promising on the grounds that the loudest cell anywhere is 98 of 255
so there is headroom. There is, and it does not buy much: a one-cell line at
the **255 ceiling** reaches 4.8s against the swath's 10.8s at the *unchanged*
deposit. Depositing harder moves the exponential's starting point; widening
changes its rate.

**And the headroom is now spent** — the swath peaks at **241 of 255** along his
route, because he re-marks each cell some ten times at 0.6 cells/tick. So #432's
"the plane is three-quarters empty at its peak" is true of a shipped ant trail
and is no longer true of the gnome's. Already stated above as the trade.

### What this lane did *not* measure, and which instrument answers it

`pherolife` carries a **run drive** counter — frames until the ant's steering
contribution at mid-trail falls under an absolute bar — and reports that the
shipped trail **stops steering at frame 48** while 77 cells are still standing.
That is the right question and a better one than presence.

**This lane measured presence and strength, not steering.** The swath's peak
runs 3x higher over the same window (241 against 80) and the along-route slope
guard `a_laid_trail_slopes_toward_the_newest_end` still passes, so the absolute
gradient an ant reads should be several times larger for several times longer —
but that is an **inference, not a measurement**, and it is exactly the kind this
repo keeps having overturned. Whoever owns #432 can settle it in one run by
giving `pherolife` a radius dial and re-reading its own run-drive column; that
is a smaller change than either instrument.

**Instrument overlap, so nobody builds a third.** `druid_trail`'s plane-side
sweep and `pherolife` overlap, and `pherolife` is the better of the two there —
it has the run-drive counter and the `DIFFUSE` setter. What it lacks, and what
`druid_trail shot=` is for, is the **real `Druid` loop**: route geometry, and
the clearance-to-ground column that caught the trail being laid at his chest.

**#432 was not merged when this was written**, so nothing here builds against
`set_channel_diffuse`, and `DIFFUSE` is untouched.

## One proposed rule, for the coordinator or the owner to place — not edited in

The reusable half of the above is a *measurement* rule and reads universally:

> **Ask what your harness is holding still, not only what it sweeps.** An
> instrument that fixes the answer as a constant will report that the levers it
> does vary are the only ones there are — and it looks exactly like a thorough
> sweep. `pherolife` varied `rho`, `diffuse` and `deposit` over a trail it laid
> **one cell wide**, and concluded the only lever was a shared constant; width
> was not a variable it had. The tell is a sweep in which every arm fails the
> same way, which `CLAUDE.md` already has — but that rule says *suspect the
> rider*, and this one says *suspect the constant*.

**Deliberately not added to `CLAUDE.md` by this lane** — the most contested
file in the repo, loaded before every session in all three games. Placing it is
the owner's call.
