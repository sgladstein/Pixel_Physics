# The evolution lab, round thirty-one: the brief

**Handed over by round 30's coordinator, 2026-09-13, main at `16bab295`.**
Every file path, constant and section reference below was checked against that
trunk before this was written — see *What was verified* at the end, including
one item that is already done.

Round 30's own account is
[`evolution-lab-round-30-2026-09-12.md`](evolution-lab-round-30-2026-09-12.md);
its **Open at close** is the longer carry-forward and this brief is the ordered
subset worth a lane.

## Task 1 — re-derive the ant lifespan and every played-bed number

**Why.** Until #366 (merged 2026-09-12) two fifths of colony deaths were an ant
being overwritten by the seed it had just eaten. Everything measured on the
played bed before that carried the cull: the 40,000-frame lifespan
(`life_half_life: 40000`, `assets/species/ant.ron:47` and
`longant.ron:141`), the seed-cargo census, round 30's room-gate survival
result, and **§Z6** (*every shipped bed starves its colony inside one play
session*). Paired at 120,000 frames the fix moved ants alive from **0 / 8 / 29
to 52 / 10 / 135** on seeds 1–3. None of those numbers is known any more.

**Do.** `examples/latecensus.rs scenario=played_bed`, `RAYON_NUM_THREADS=1`,
**at least twelve seeds** (six is not a sweep), runs of 200,000 frames or
fewer, both arms from one binary. Report **order statistics** — p10, median,
p90 — of ants alive, seed bank, plants, and starved against old-age deaths, at
120k and 200k.

Then answer three questions: does the lifespan constant still hold, does the
room gate still earn its place, and is §Z6 still true. **Re-set constants only
from the sweep**, and put the before and the after in the commit message.

Do **not** re-derive the hazard-interval arithmetic;
[`lanes/evolution-lab-lifespan.md`](lanes/evolution-lab-lifespan.md) explains
why.

## Task 2 — the chronicle carries the playtest

**Why.** The owner plays sessions with hundreds of ants and wants to hand the
log to an agent for review. The chronicle (`Lab::write_chronicle`,
`src/lab/mod.rs`; files under `assets/chronicles/`) already carries a census
every `Lab::CHRONICLE_CENSUS_EVERY` = **10,000** frames
(`src/lab/mod.rs:508`) and the lineage story. It does not carry the playtest.

Three gaps, three pieces:

- **Player actions as frame-stamped events.** A `LogKind` variant in
  `src/sim/world.rs` for what the player did — placed a jar, walled, poured,
  changed a dial, changed speed, rebuilt — pushed from `Lab::act`, printed
  through `format_log_line` in `src/lab/ui.rs` so the LOG page and the file
  agree, and included in the chronicle's story. Today an agent can only *infer*
  an intervention from a jump in the census.
- **Autosave.** Write the chronicle on every census row as well as on rebuild,
  quit and `9`, with a timestamp in the filename. Today a crash writes nothing
  and two saves in one day overwrite each other.
- **The story must not age out.** `RUN_LOG_CAP` = **2048**
  (`src/sim/world.rs:308`) holds births and deaths in the same ring as line
  events, sized at ~640 events per 90,000 frames on the shipped bed. A big
  colony pushes its own history out. Give line events and player actions their
  own ring, or cap births and deaths separately.

**Guards watched red first.** `examples/chronicle.rs` prints through the same
formatter and must show the new lines. Say what the autosave costs per frame
(`ascii` worst-frame). Update
[`lanes/evolution-lab-chronicle.md`](lanes/evolution-lab-chronicle.md). Then
ask the owner for one real session file and read it back to him.

## Task 3 — the MENU page reads as a list, not a menu

The owner's verdict, 18:45 on 2026-09-12, and round 30 already resolved the
ambiguity in it.

**Do not rebuild click handling. The rows are already clickable** —
`Row::choice` carries an `Action` and drawing one pushes a full-width tap
target. `Body::Choice` in `src/lab/ui.rs` simply draws **pixel-identical to
`Body::Value`**, an information row, with an invisible target. The source says
so itself: *"it draws like one and only the tap target under it differs"*.

So: give `Choice` its own drawn treatment first — borrow the bar chips' button
idiom; this fixes **every page that uses `Choice`**, not only MENU — then two
columns, which buys back the `PAGES` and `VIEW & TOGGLES` headings the lane cut
at 211 px of a 228 px budget.

**Post a before-and-after card at play zoom and wait for the eye.** This is the
category where description has failed here repeatedly.

## Task 4 — floating debris, §Z18

Dug spoil standing in open sky. The most reported visual defect of round 30,
named by the owner on **two unrelated cards**, and no lane owns the repair. The
design of record is
[`evolution-lab-soil-design-2026-09-12.md`](evolution-lab-soil-design-2026-09-12.md)
— start there, **not from weathering**. Grep
[`dead-ends.md`](dead-ends.md) for the mechanism before building.

## Task 5 — smaller, take as lanes free up

- **§Z17** — `World::ground_datum` is wrong inside a sealed box. The repair is
  small; what the right answer *is* for a box with a lid is deliberately open,
  because a lid is genuinely a roof.
- **§Z13** — a resting ant reads as stuck at play zoom. A colour or mark
  question, not a mechanic.
- **§Z12** — bred one-cell morphs that cannot flip. Inheritance, not injury.
  With §Z13 it is why the long-ant pile still reads as stuck to the owner's eye
  after the expiry landed.
- **The water-level line** from lane T's zoom-in exploration is unbuilt, and
  lane T recommended it whatever the style verdict.
- **Splitting or renaming `DeathCause::Killed`**, on the test that two lanes
  re-derived the same wrong reading from it in one day.

## What was verified against `16bab295`, and the one item already done

| claim | verdict |
|---|---|
| `life_half_life: 40000` in `ant.ron` and `longant.ron` | present, lines 47 and 141 |
| `Lab::CHRONICLE_CENSUS_EVERY` = 10,000 | `src/lab/mod.rs:508` |
| `RUN_LOG_CAP` = 2048 | `src/sim/world.rs:308` |
| §Z6 exists in the register | yes |
| next free bug letters §Z19 and §W8 | confirmed by `bugindex.py --branches` |
| **"correct 0.4% to 1.31% in the late-game design §1.2"** | **already done** — that file's line 182 reads *"1.31% at 2.5x (this line said 0.4% until 2026-09-12; lane M measured the hazard as shipped and the design's figure was wrong)"*. Nothing to do. |

**And two branches the handover called unowned are not.**
`claude/gallant-fermat-dqvtm4` and `claude/plants-124-crown` both carry **open
pull requests** (the dead-ends sweep and the felled-tree work). They are
someone's live work, not orphans — do not touch their files, and check the PR
list before concluding otherwise.
