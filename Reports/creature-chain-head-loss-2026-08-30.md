# §R3 resolved: a chain longer than two cells overwrites its own head

**2026-08-30, lane K.** Diagnosis only — nothing in `src/` is changed by
this work. `src/sim/creature.rs` and `src/sim/brain.rs` are held by another
lane, and the whole of the measurement below was done through
`examples/creature_probe.rs`, which is not.

---

## The answer, in one sentence

**§R3 is neither cannibalism nor the hatch-site predicate: the colony never
dies at all.** A `Chain(n >= 3)` loses its `CellType::Head` marking to a
self-overwrite in `relocate_chain`, and §R3's `live 0` is a **head-cell
counter reading zero over a living, feeding, delivering population** — so
**the extent lever is recoverable**, and what stands between here and it is
one position-list bug plus a harness constant, neither of them an economy
and neither of them an ecology.

---

## 1. Cannibalism is ruled out, and both halves of the control were run

§R3 prescribes the experiment: *make ant flesh inedible to ants and re-run
`Chain(3)`*. `creature_probe` gained `kinfood=off` for exactly that. It also
gained `eatskin=on`, which is the **positive control** — the arm that forces
cannibalism to actually happen — because a null from the first arm is worth
nothing unless the instrument could have reported the other answer.

`terrain=world seed=0xA17 frames=12000 body=3`:

| arm | `meat_lost` | head cells | registry | verdict |
|---|---|---|---|---|
| shipped | 0 | 0 | 16 | the arm under test |
| `kinfood=off` | 0 | 0 | 16 | **byte-identical to shipped** |
| `eatskin=on` | **40,320** | 8 | — | positive control: the instrument sees it |
| `eatskin=on kinfood=off` | 0 | 0 | 16 | knob validity: identical to shipped |

Read the four rows together and the null is airtight. Row 3 says
cannibalism, when it happens, is enormous and unmissable — `meat_lost` goes
from 0 to 40,320 and every other account moves with it. Row 4 says
`kinfood=off` **suppresses** that, so the knob is not merely connected, it
is connected to the thing being controlled for. Row 2 is then a real null:
with kin flesh worthless the shipped colony behaves *identically*, down to
`moves 11440 blocked 816 falls 2446 | eats 62 ... deaths 18`.

**The knob is provably live**, and it is the very readout §R3 built its
hypothesis on that proves it: the frame-0 dump goes from
`food in reach: ant 480  ant 480  ant 480` to `food in reach: none` while
every behavioural number stays the same.

### Why the hypothesis looked so strong, and what the dump was really showing

Two instrument faults, both of the shape `CLAUDE.md` names first — *ask what
your number counts when nothing is wrong*.

- **The dump never applies the kin gate.** `report`'s "food in reach" line
  calls `creature::food_value` and nothing else. `adjacent_food` is the
  function that actually decides a mouthful, and it refuses living kin
  through `is_living_kin` unless `eats_kin` is set — which `ant.ron` does
  not set. So the dump reports flesh that *would* be food to a beetle, and
  cannot report whether an ant may take it. It was showing the menu of a
  different animal.
- **Three of those four entries are the animal's own tail.** The dump scans
  the head's 8-neighbourhood, and a `Chain(3)`'s head is permanently
  adjacent to its own two segments. `ant 480 ant 480 ant 480` on a
  three-cell ant is very largely the ant. That is also why the count grows
  with chain length, which read as "more contact surface with its
  neighbours" and is really "more of itself".

Both faults get worse exactly as `n` grows, which is what made the
hypothesis fit the data so well while being wrong.

---

## 2. Nothing was ever unaccounted for — the death counter was fine

§R3's second effect is *"peak 34 against deaths 12 leaves 22 animals
unaccounted for"*. The gap is entirely in the population counter, not in
the deaths.

`creature_probe` reports two different quantities that had never been
printed side by side. The summary's `live` counts **head cells standing in
the world**; `World::live_creature_count` counts **entries in the organism
registry**. A healthy colony has them equal.

| arm (12,000 frames) | bodies built | deaths | registry | head cells |
|---|---|---|---|---|
| world `Chain(2)` | 45 | 18 | **27** | 27 |
| world `Chain(3)` | 34 | 18 | **16** | **0** |
| world `Chain(6)` | 29 | 18 | **11** | **0** |
| slab `Chain(2)` | 55 | 31 | **24** | 24 |
| slab `Chain(3)` | 28 | 16 | **12** | **0** |

**`built - deaths = registry`, exactly, in every row.** The books close.
There are no missing animals, there never were, and `deaths` — the counter
`CLAUDE.md` warned not to trust here — was telling the truth the whole time.
What was wrong is the number it was being differenced against.

The survivors are not ghosts. At `Chain(3)` on the world the 16 of them
stand as **47 ant cells**, all owned by a live organism, and over the run
the colony logs `moves 11440 ... eats 62 ... deliveries 619`. They are
alive, embodied and working. They simply have no cell labelled `Head`, so
every instrument that finds ants by looking for one reports an empty world.

---

## 3. The mechanism: a chain that steps into its own tail

`body_after_step` builds a chain's next body as follow-the-leader:

```text
next = [head, chain[0], chain[1], ..., chain[n-2]]
```

`relocate_chain` then clears the old positions and writes the carried
`Cell` values into the new ones, in order — index 0 carrying the Head.

**If the head steps into a cell its own body already occupies, the same
position appears twice in that list**, and the two writes land on one cell.
Last write wins, and the last write is a Segment:

```text
from = [P0, P1, P2]     cells = [Head, Seg, Seg]
head steps back into P1
to   = [P1, P0, P1]              <-- P1 twice
  write to[0] = P1 <- Head
  write to[1] = P0 <- Seg
  write to[2] = P1 <- Seg        <-- overwrites the Head
```

**It needs three cells.** At `Chain(2)` the list is `[head, chain[0]]`, and
those two are distinct however the animal turns — which is exactly the
length threshold the measurements show.

Nothing downstream notices. `reconcile_chain` compares the chain against
the cells the organism still owns; with `chain = [P1, P0, P1]` every entry
is still owned and `chain.first()` is unchanged, so it books **no death, no
injury and no `meat_lost`** — which is precisely the silent disappearance
§R3 described.

### Verified, not derived

The probe now censuses chains directly. `terrain=slab seed=0xA17 body=3`:

| frames | chains | with a repeated position | non-Head at `chain[0]` | cell types |
|---|---|---|---|---|
| 20 | 28 | 1 | 2 | Head 26, Segment 57 |
| 100 | 28 | 2 | 10 | Head 18, Segment 64 |
| 500 | 28 | 2 | 26 | Head 2, Segment 80 |
| 3,000 | 17 | 1 | **17** | **Segment 50** |

and the control, same seed, same 3,000 frames, `body=2`:

**`55 chains | 0 contain a repeated position | 0 have a non-Head cell at
chain[0]` — `Head 55  Segment 55`.**

The duplicate is rare at any instant (one or two chains) and its damage is
**permanent**: a head once overwritten is never re-marked, so the count of
headless animals only ever rises. That is the signature of a transient event
with an irreversible consequence, and it is why the effect looks total by
12,000 frames and invisible at 600 — the horizon
`creature-appearance-design.md`'s decoy measurements were taken over.

### What the relabel actually costs today

Less than it looks, which is worth saying plainly so the repair is not
over-scoped. Movement, feeding and delivery all key off `chain[0]`'s
*position*, not the cell's type, so a headless ant behaves normally — the
totals above are a working colony. The consumers of `CellType::Head`
outside `creature.rs` are `render.rs:4345`, which treats `Head | Segment`
alike, and `render.rs:4913`, a debug overlay. So the standing cost is:

- **every population instrument reads zero**, which is the whole of §R3;
- the debug overlay mislabels a live animal;
- and the duplicated chain entry makes `live_body_cells` (`chain.len()`)
  bill the animal for a cell it does not occupy — a small, real
  over-charge that rides along.

The repair is not this lane's to make and is not attempted here.

---

## 4. Placement is real, and on the slab it is the harness

§R3's effect 1 — placement roughly halving at three cells — reproduces, and
splits by terrain.

**On the hand-built slab it is `creature_probe`'s own founder loop.** Both
scenes plant 55 founders at `x = base + i * 2`, a **two-cell pitch**, and a
`Chain(n)` is laid out as *n* cells running left from its head. At `body=2`
the bodies tile the row exactly; at `body=3` every consecutive pair overlaps
by a cell. The pitch was calibrated against the shipped two-cell ant and
silently became a body-size filter the moment `body=` existed.

Adding `pitch=` settles it — slab, `body=3`, 12,000 frames:

| pitch | bodies built (of 55 attempts) |
|---|---|
| 2 (the shipped harness) | **28** |
| 4 | **46** |

So the slab's 55 -> 28 is arithmetic about the scene, not about the engine —
`CLAUDE.md`'s *a scene that contradicts the code will look like a bug in the
code*.

**On the generated world it is not the pitch.** Same knob, `terrain=world`:
`body=3` places 34 at pitch 2 and **35** at pitch 4, and `body=2` places 45
at pitch 2 and *40* at pitch 4 — wider spacing walks the founder row off the
nest into worse ground and makes things slightly worse. So the world's
45 -> 34 is a genuine interaction between a longer body and real terrain,
and it is the part of §R3's effect 1 that survives.

**Neither is why the colony reads dead**, which is the thing to carry: with
placement fully repaired on the slab (46 founders instead of 28) the arm
still reports `live 0`, because that number is counting heads.

---

## 5. What this does not settle

- **No fix is proposed or measured.** The duplicate-position write is one
  lane's to repair and one lane's to test; this document only shows it
  exists and how to see it.
- **The world-terrain placement drop is unexplained.** It is smaller than
  the slab's and it is not the pitch. It has not been chased.
- **`start_energy` remains flat while burn is per cell**, so an *n*-cell
  animal still has a `1/n` starvation horizon. Lane D flagged this and
  deliberately left it; nothing here touches it, and it is a real cost of a
  longer body that will still be there once the head bug is gone.
- **Whether a three-cell colony is *healthy* once counted correctly is a
  separate question.** It survives — 16 of 34 on the world against 27 of 45
  at two cells — but survival fraction is not viability, and `births 0` in
  every arm means nothing in this scene reproduces at all, at any body size.

---

## Reproducing

`creature_probe` gained five knobs for this work, all defaulting to the
shipped behaviour so every number already filed still reproduces:

```
kinfood=off     living-ant materials' food_energy -> 0 (corpse untouched)
eatskin=on      force CreatureDef::eats_kin, the positive control
pitch=N         founder spacing; 2 is the shipped harness
seed=0xA17      hex is now accepted -- see below
```

and prints, unconditionally: the energy-ledger accounts, `bodies ever
built`, the head-cell/registry population gap, chain integrity, and the
standing cell-type histogram.

**§R3's own headline command could not run.** It quotes
`creature_probe terrain=world seed=0xA17 frames=12000`, and the seed parser
took decimal only, so the argument panicked. The probe echoes `seed={:#x}`,
which is where the `0xA17` in the report came from — a readout that could
not be pasted back in. `seed=` now accepts both forms; the runs above use
`0xA17` literally.
