# The pheromone line: everything known, and what to build next

**Master report, 2026-09-17.** Supersedes nothing; it is the **entry point** to a
corpus of ~10,400 lines across thirteen reports and two lane notes. Read this
page, then read **one** source from §2 — never the directory.

**Who this is for:** a session picking the ant trail/homing line up cold. §7 is
an executable plan. §5 is the list of things that are wrong in the record and
must not be quoted. §8 is what has already cost days.

---

## 1. The one-page answer

**The colony forages and does not bring food home.** That is the problem; it has
been the problem since §T2 was filed, and none of the pheromone work has moved it.

Three facts, each measured independently, that between them say why:

1. **A laid trail is decisive.** Hand-lay one and 91.4% of ants reach the food
   against 3.7% unaided; 7 of 12 colonies survive against 0.
2. **The colony cannot lay one itself.** The `self` arm is indistinguishable from
   the `mute` arm (channel B zeroed): ~3.4 route cells of 90, in 2 of 12 seeds.
3. **Nothing steers a laden ant home.** Channel A — the homing plane — is laid by
   every ant on every successful move, so it integrates where ants *are*. In a
   colony that forages hard, that is the food, and the ramp inverts.

**The structural statement.** Channel A and channel B have the **same laying
rule** and need opposite ones. `wiki/ants.md` says it without noticing:
*"Nothing decides where a trail goes: it is just the leftovers of where ants have
been."* Correct for a food trail — ants have been at the food, and that is where
you want to go. Exactly wrong for a home trail.

**The biology says the same thing.** Real ants home by **path integration** — a
private home vector, run straight — and use the trail as a *contextual modulator*
("you are on the route"), weighted by how long the vector is. Trails are
**isotropic**: they carry no direction. The engine asks a concentration gradient
to encode direction, which real trails do not do. See §6.

**So the fix is a direction sense, not a weight.** Two sweeps (the odometer's
decay, and an emission floor) moved colony survival, trail length and amplitude
and left the correlation between "where ants spend time" and "which way the ramp
points" exactly where it was: `r = +0.64 to +0.91` in all nine arms.

---

## 2. The corpus — read one, not the directory

| Report | Lines | Status | Owns |
|---|---|---|---|
| **`pheromone-trail-direction-2026-09-16.md`** | 2,210 | live, this line's working record | §7.11–§7.18: the gap sweep, discovery vs homing, the inversion, both failed sweeps, the stale-binary incident. **§7.15/§7.18 carry errors — see §5** |
| `pheromone-lifetime-and-wiring-2026-09-14.md` | 746 | measurement, round 36 lane C | Trail life (144 frames vs a 2,200-frame round trip), `DECAY_RHO` inert, 4 of 7 reader slots unread, the `u8`→`u16` widening (§3c) |
| `nest-design-2026-09-14.md` | 648 | **design of record**, §13 owner ruling 2026-09-15, nothing built | What a nest is; §5.3 the `DIFFUSE` finding; **§8 option C = the home bearing this plan revives**; §9 the landing order |
| `stigmergy-research.md` | 438 | research, **implemented** | Deposit → diffuse → decay → follow. The colony is built on it |
| `foraging-range-measurement.md` | 447 | measured, instrument landed | The 19-cell bubble, the 2-cell-spacing gridlock |
| `colony-starvation-separated-2026-09-08.md` | 516 | measured; pheromone clause corrected 2026-09-09 | The colony dies twice |
| `decaying-gradient-quantization-2026-09-15.md` | 280 | survey | Does the `u8` root cause generalise — decay-plus-gradient in narrow storage |
| `colony-economy-design-2026-09-09.md` | 350 | design | Foraging returns less than it costs |
| `colony-food-economy-design-2026-09-14.md` | 170 | design | Food economy successor |
| `evolution-lab-nest-question-2026-09-14.md` | 131 | **research brief, nothing started** | Path integration named as the candidate |
| `evolution-lab-round-37-brief-2026-09-15.md` | 250 | current round | **Lane 3: "nothing steers a laden ant home"** and why every nest-research number is suspect |
| `creature-direction.md` | 1,772 | direction agreed 2026-08-17 | The origin document; most dead-end entries cite it |
| `creature-genome-flexibility-2026-09-02.md` | 2,471 | design, not built | Names homing-as-a-`since_nest`-odometer as one of four things "still spelling ant in Rust" |
| `lanes/evolution-lab-pheromones.md` | 170 | round 36 lane C | Nothing tuned; every default ships unchanged. The alarm fix |
| `lanes/evolution-lab-nest-research.md` | — | round 36 | Companion to `nest-design` |

**Open bugs:** §Z7 (the trail-following gate saturates the signal it gates) —
**OPEN**. §Z6 (every shipped bed starves its colony inside one play session) —
**OPEN**. §T2 (1,651 pickups, 4 deliveries) — OPEN, but `nest-design` §13 says it
closes with the site ruling. §Z5 (every homing odometer dead, charge below
`W_EPS`) — **closed**.

---

## 3. What is established

**Discovery is the binding constraint, not homing.** 0–8% of ants reach food
without a trail; 62–91% with one. §7.13.

**The odometer grades steeply — the opposite of what §7.15 assumed.** `brain.rs`'s
own fitting comment: *"the decay is dominated by `squash`, not by `w_rec`."*
Shipped emission runs **0.819 → 0.177 across a 141-tick trip, a 78% fall**, not
the 0.7% that `w_rec^141` suggests. Verified against the engine's own
`what_an_odometer_emits` readout to three decimals on three points.

**The inversion is real, and it needs a foraging colony to show.** On a
replenishing larder (`trailfollow refill=`), every foraging colony's ramp points
at the food — 4/4, 5/5, 7/7, 3/3 across settings, `r = +0.69…+0.85`. On the
one-shot default the colony starves at six ants and the question cannot be posed.

**The polarity metric is sound, and now calibrated.** `arms=homeA` paints a
perfect nest-ward ramp and reads **+0.115 on 6/6 seeds** — an independent
prediction from `lay_home`'s own arithmetic said ≈+0.12. Shipped foraging
colonies read **−0.076**. *(Run 2026-09-17; not yet in the working record.)*

**Neither weight lever works.** Across eight intervention arms — odometer decay
`recur` at 0.999/0.995/0.99/0.98 and emission floor `biasa` at
−0.05/−0.10/−0.18/−0.35 — **31 of 34 foraging colonies still point at the food**,
every arm's mean is negative, and the paired per-seed shifts go both ways with no
trend. The only thing a floor reliably did was shorten the trail: route coverage
**78 → 76 → 68 → 57 → 52** of 91 cells, with colonies dying alongside.

**A longer-lived plane is the one thing that has ever moved homing.**
`nest-design` §5.3: `DIFFUSE` on channel A alone (per-channel setter exists) at
0.02 raises laden-at-door on **3 of 3 seeds**, ninefold on the seed where homing
was dead, and deliveries on 2 of 3. **This is the cheapest untried lever in the
corpus and it is a runtime dial.**

**Correct homing alone is not sufficient and can hurt.** `homeA` — a perfect
homing ramp with no food trail — gives **0 alive, 0 delivered, reach `12 8 0 0 0`,
`AtNest` 27–50%**. A strong homing signal suppresses exploration. **Deliveries,
never polarity, is the success criterion.**

**Channel B is already laid correctly** — `(Carrying, EmitB, 2.5)`, only while
laden. That is the biologically right rule and it is not the defect.

**The `u8` → `u16` widening was a resolution fix and it worked.** Trail life
144 → 1,476 frames; steering 36 → 1,080; network 208–342 → 1,405–2,057 cells.
**Every measurement taken before 2026-09-15 is on the `u8` engine** where the far
half of every trail read exactly zero — round 37's Lane 3 brief says so in bold.

---

## 4. What is open

- **§Z7**: the trail-following gate saturates the signal it gates. The one direct
  repair is in `dead-ends.md` as rejected — but *on the bed*, and the rejection
  condition is *"what a food trail is worth in this bed, not the gate."*
- **Two unseparated causes for the same result.** §Z7's saturation, versus
  "channel B is laid only while laden so its gradient points at the **nest** and
  steers an empty ant home." `Reports/README.md` records: *"neither is
  established, and nobody has run the arm that separates them."*
- **The self-read** (§5, item 3) — never measured, only derived.
- **Which tree is right.** The reproducible tree grows a far sicker colony than
  the one §7.15's original numbers came from. Never resolved; see §8.

---

## 5. Errors in the record — do not quote these

**Mine, this line, all corrected in place but still readable in §7.15/§7.18:**

1. **§7.18's floor argument is self-refuting.** §7.16 *pre-registered* that
   `a_peak_amt` would not move under an emission floor ("a floor does not change
   what an ant fresh from the nest lays"); §7.18 read that confirmation as proof
   the floor never bit. It is a route-wide **max** quoted as a min across seeds,
   set by the nest-end deposits the floor deliberately does not touch. Predicted
   shift ~1.5%, below seed spread.
2. **§7.15's mechanism is wrong** (its *finding* survives). Step 1 derives the
   odometer's per-trip decay from `w_rec` alone and concludes the charge is flat.
   It falls 78% — see §3.
3. **"1,813" is derived, not measured.** It is `0.177 × DEPOSIT` off the odometer
   curve, presented as an observation. `ant.ron:1764-1783` is emphatic about not
   quoting a curve without naming the weights it belongs to.
4. **"−418 net homeward cells over 2.1M carrying ticks"** could not be found in
   `Reports/` by an independent check. Re-derive before quoting.
5. **The polarity metric has a second gate nobody fixed.** It appears at
   `examples/trailfollow.rs:976` **and `:1121`**. Its `here > 0.0 || ahead > 0.0`
   admits blob-edge cells worth **−0.97** against interior cells' ±0.03–0.07 —
   one edge cell is worth ~20 interior cells. The +0.115 control shows the metric
   is not *dominated* by this, but it is unquantified on the `hand` arms, where
   coverage is 57–78 of 91 cells and edges therefore exist.
6. **"Integral over occupancy" is the wrong noun.** Deposits happen **only on a
   successful move** (P-11, `creature.rs:4223`). The 21:1 food-end ratio is
   ant-*ticks*; moves-per-band has never been measured. A congested ant at the
   larder burns ticks without laying.

**Inherited, in source comments:**

7. **`creature.rs`'s odometer comment named the dead pre-fix weights** as "the
   weights `ant.ron` actually authors" — `w_in = 0.0005` is below `W_EPS` and
   `brain.rs`'s own readout labels that row `authored (dead)`. Corrected
   2026-09-17.
8. **`ant.ron` quoted a curve its own weights do not produce** — `0.992 → 0.072,
   saturated 85 ticks` is the fitting test's chosen fit at `w_out 900, bias −0.2`;
   the file ships `w_out 32.0` and **no bias**, which gives `0.819 → 0.010`,
   saturated 0. Corrected 2026-09-17.

---

## 6. The biology, and what to take from it

Established in the literature, and it corrects an earlier claim of mine that ants
do not use pheromones to home — **they use both**:

- **Path integration supplies direction.** A home vector, built from a sky
  compass and a stride integrator, run *straight* rather than retraced.
  Wittlinger 2006 proved the odometer directly: *Cataglyphis* on stilts overshot
  the nest, on stumps undershot, by the ratio of leg length.
- **Its weight scales with the vector's length.** A long path integrator
  *dominates* the trail and ants readily leave it; a short one makes them hesitate
  and retreat into the trail rather than overshoot the nest. In *Veromessor
  pergandei* the integrator wins outright in conflict.
- **Trails are isotropic** — "non-directional and axi-symmetric". The chemical
  trail does not encode which way the nest is. Direction comes from vision, path
  integration, and trail geometry.
- **So the pheromone is a contextual modulator**, not a compass: it says *you are
  on the route*, and adjusts how far the ant trusts its own vector.
- **The pioneer/recruit split is real**: a scout searches, homes by path
  integration, lays trail on the way back; recruits use that trail as a route and
  still resolve direction themselves.

**It does not have to match.** The owner's framing, 2026-09-17: *"It does not have
to perfectly match how real ants, but we should take inspiration when we can."*

---

## 7. The plan

Ordered so each step is attributable. **Steps 4 and 5 must not land together** —
both add a term to a shared weighted sum, and `CLAUDE.md`'s *"a correct mechanism
at inherited constants is a regression"* makes the joint result unreadable.

### Step 0 — baseline seed sweep at HEAD

There is no control arm otherwise. `brain.rs:2046-2052`: once `live_slots`
changes, every breeding scene's numbers move from birth 1, and *"the remedy is a
seed sweep, not a diff."*

### Step 1 — readouts only, no engine change

Fixes the meter everything else is judged on. Running a mechanism before fixing
the instrument is the §8 failure that already cost a day.

- **Both polarity gates** — `trailfollow.rs:976` **and `:1121`**. Report the
  boundary-cell count and a figure restricted to `&&`.
- `probe_in[I::PheroAAlong]` and `[I::PheroBFront]` per ant, split laden/empty,
  beside the existing `AtNest` probe at `:1093`. `probe` is non-mutating by
  construction (`creature.rs:4505-4519`).
- **Moves-per-band, not ticks-per-band.** Plumb `deposits_a`
  (`src/sim/pheromone.rs:1028`).
- **Re-baseline §7.18's numbers against the fixed meter.**

### Step 2 — `DIFFUSE` on channel A, the dial that already exists

**Do this before writing any engine code.** `set_channel_diffuse` landed with
round 36 lane C; `nest-design` §5.3 measured 0.02 lifting laden-at-door on 3 of 3
seeds. It is a runtime dial, costs nothing, and is the only lever in the corpus
with a positive homing result. Weigh what it costs the B trail — Lane C's number
is `DIFFUSE` 0.10 scoring 0.623 on-trail against 0.25's 0.817 — which is why it is
per-channel.

### Step 3 — deposit on the vacated cell

The ant moves P→Q; deposit at **P**, not Q. Next tick it senses at Q, which it has
not written. `(x, y)` is in scope at `creature.rs:4223`. No new state, P-11
untouched (still only on a successful move), trail shape unchanged but registered
one cell back.

**Why:** within one tick `sense` reads `here` at the dispatch cell (`:4615`),
`step_chain` moves (`:4209`), and the deposit lands on the **new** head (`:4284`).
Next tick the ant is dispatched there, so `here` holds its own deposit. With
`ahead` empty that gives `along = −0.876` and **−4.20** on `Move`'s pre-squash
sum — the same magnitude as the entire swing that pair can produce (±4.29
derived). **This repo already paid for this on channel B** (`brain.rs:1256`:
*"the strongest thing they could smell was the food trail they were themselves
laying"*).

**Two caveats.** It is **not** a standing homeward bias — only where `ahead` is
empty; walking back down its own trail both are high and `along ≈ 0`. So the real
effect is a **penalty on stepping onto virgin ground**, pushing `Tumble` over
`Move` — which bears on discovery, not homing. And *"subtract your own deposit"*
is intractable: the amount is discarded, the plane decays and diffuses between
write and read, and a nestmate on the same cell breaks it.

**Cost:** this shifts channel A's spatial registration for every ant, so **every
number in §7.11–§7.18 must be re-taken.** Determinism holds (P is a pure function
of state); verify `sense_read_rects` (`:5422`) with `ParMode::Verify`
(`:3877-3896`) rather than assuming.

### Step 4 — `HomeBearing` / `HomeDistance`

**This is `nest-design` §8 option C, designed and priced 2026-09-14, never built.**

**The state already ships.** `forage_anchor: (i32, i32)` (`organism.rs:5888`) and
`forage_max` (`:5891`) are anchored at spawn (`creature.rs:1509`), updated per
move (`:9505-9512`) and **re-anchored at every nest contact** (`:9516`). The
inputs are `(anchor − head)` — **zero new state, zero accumulation.**

Do **not** accumulate displacement at the deposit site: `step_crossing` and
`step_flight` return early at `:3863-3868` *before* it, so a flying or
trunk-crossing ant displaces without incrementing and the integrator drifts.
`step_chain` returns `bool`, not a delta. `since_nest` is not reusable —
`:9522-9532` records its visit guard firing exactly once per lifetime.

**Drive `Turn`**, as the four existing bearings do (`PreyBearing`,
`creature.rs:4901`). That keeps it off `Move`'s crowded sum.

**`HomeDistance` is not decoration** — it is how the animal weights the two cues
(§6), and the term that makes trail and vector cooperate rather than compete.

| must change | site |
|---|---|
| `BRAIN_INPUTS` 30 → 34 (both bearings + both `*Here` from step 5, one bump so `mutation_rate` is re-derived once) | `brain.rs:43` + doc `:35-42` |
| `INPUT_NAMES` (compile error if missed) | `brain.rs:245` |
| `INPUTS` (compile error if missed) | `brain.rs:1302` |
| `genome_manifest()` pin | `brain.rs:2184` |
| `live_slots()` pin **870 → 966** | `brain.rs:2055` |
| `mutation_rate` → **0.0032919** in **six** files | `ant.ron:718`, `ancestor.ron:620`, `beetle.ron:225`, `flitter.ron:809`, `hopper.ron:743`, `longant.ron:862` |
| the two "measurement only" docs — both say reading `forage_anchor` means the homing model changed | `organism.rs:5871`, `creature.rs:9504-9506` |

`live_slots = 16·I + 8·I + 8 + 16·8 + 14`, which reproduces the current **870** at
`I = 30`. **Both derived numbers rest on a derived numerator** — confirm `3.18`
against `brain.rs:2007`'s doc history and take the real count from the failing
pin. At `I = 32` (step 4 only) the values are **918** and **0.0034641**.

Adding inputs **at the end shifts no existing slot** (`io_slot` is
`output · INPUT_SLOTS + input`, `INPUT_SLOTS = 64` fixed). Nothing in `assets/`
carries a `genome_manifest` stamp, so neither load site fires. Off-repo
`specimen.rs` jars with `layout: None` hard-fail `StaleGenome`.
`plainspeak.rs:1595` passes with 2 characters of slack — **any new input name
≥ 15 chars fails it.**

### Step 5 — a trail-concentration sense

**Recruitment, not homing.** Judge on reach and discovery, **never on delivery
counts**, and keep it out of any arm measuring step 4.

- `*Front` reads one cell at `sensor_offset: 6` **along the heading**
  (`creature.rs:4581-4593`) — directional, not the isotropic "am I standing on
  it" the theory wants, and its designed partner `*Lateral` is dead for surface
  walkers. `here` is already fetched at `:4615`; expose it as
  `BrainInput::PheroAHere`/`PheroBHere`, ~2 lines, folded into step 4's bump.
- **Unsigned input → a single gated unit**, not the ± pair: `Bias −45,
  Carrying +45.5, <concentration> +w`. The ± idiom at `ant.ron:1793-1798` is
  antisymmetric because `along` is *signed*. `ant.ron` uses hidden units 0–6 with
  `BRAIN_HIDDEN = 8`, so **unit 7 fits exactly.**
- **Derive the weight.** Normalised by `Scent::MAX`, realistic trails read
  **0.059–0.216**, so `w ≈ 30`, not 6. Side effect: `GATE_DOMINANCE = 3.0`
  (`plainspeak.rs:493`) — the lab's cell page stops calling it a conditional.
  Cosmetic; note it in writing.
- **In its favour:** `PheroAFront`/`PheroBFront` are written every tick and read
  by **no species** — the dead-weight half of this repo's writer/reader rule.

### Step 6 — docs

§7.19 in `pheromone-trail-direction-2026-09-16.md` correcting §5's items 1–6 in
place (**do not delete them**). `wiki/ants.md`: *"a colony paints its own map
outward from home"* is the premise being retired — with a real date.

---

## 8. Traps, each of which has cost real time here

- **The stale binary.** §7.15 and the odometer sweep measured a
  `target/release/examples/` binary matching no commit; a clean worktree build of
  the same SHA reproduced *different* numbers. **`cargo build --release
  --examples` in the same command as the run**, always.
- **`refill` was not echoed in the header.** A replenishing larder and a one-shot
  larder printed byte-identical parameter lines; six hypotheses (determinism, a
  six-value thread scan, contention, comment edits, `SPOIL_IS_CARGO`, corpse
  isolation) were spent before the *scene* was suspected. Fixed — the header
  prints it now. **A knob nobody can see the value of cuts both ways.**
- **A control that validates the knob does not validate the scene.** The rider
  controls before the odometer sweep all passed and were correct.
- **Unknown arguments are silently ignored** — `gaps=`, `seed0=` and `onlyfood=off`
  were each dropped at least once, and each time the run looked fine.
- **`cargo test --lib` reaches the brain and species guards** (they are in-lib);
  `tests/` holds only `determinism.rs` and `worldgen.rs`. Run full `cargo test`
  anyway for `determinism.rs:344`, which runs live creatures.
- **Every pre-2026-09-15 measurement is on the `u8` engine.** Round 37 Lane 3.

## 9. Instruments

`trailfollow` (can this animal read a trail, does a laid one move the colony —
`mode=gap`, arms `hand`/`hmute`/`self`/`mute`/`homeA`, `refill=`, `onlyfood=`,
`recur=`, `emita=`, `biasa=`) · `nesthome` (what homing rests on; `arm=noemit`,
`arm=nosteer`, `diffuse=`, `width=`) · `pherolife` (how long a trail lives) ·
`pherocost` (what one pass costs) · `pherowire` (which species read and write the
planes) · `onetrail` · `quantgrad` (does a decaying scalar still read as a
gradient) · `ant_ablation` (is the authored brain doing anything) · `colonybooks`
(what the colony lives on, in joules).

## 10. Commands

```
RAYON_NUM_THREADS=4 cargo build --release --examples && \
  RAYON_NUM_THREADS=4 ./target/release/examples/trailfollow \
  mode=gap gate=b2 gaps=90 seeds=12 arms=hand onlyfood=on larder=fruit \
  food=200 frames=24000 refill=2000
cargo clippy --all-targets --release --locked -- -D warnings   # unpiped, read the real exit code
cargo test                                                     # not --lib, for determinism.rs
bash scripts/docscheck.sh
python3 scripts/review.py serve --open                         # the owner judges deliveries by eye
```

**The bar is deliveries, judged by eye.** For creatures `filmstrip gif=1` is the
only instrument — an ant is two dark cells picked out by motion, so a grid of
stills cannot answer any question about it. Post with the delivery count in
`meta`.
