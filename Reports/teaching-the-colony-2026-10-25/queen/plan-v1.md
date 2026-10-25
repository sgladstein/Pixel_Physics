# Queen seat demo: implementation plan (v1, 2026-10-25)

*Step 2 of the seat-demo board. Built on the adversarial review
([`queen-feasibility-review.md`](queen-feasibility-review.md), verdict **feasible with changes**) and the design
(`designs/queen.md`, `designs/teaching-plan.md` §3-§6). Branch `claude/project-thread-epio3i`.*

## 0. What the owner will play

`cargo run --release --bin lab -- demo=queen` opens a lab box with one founding queen in it, and **you are her**.

- **Minutes 0-3, found.** Walk out to the food, eat, walk home, dig a chamber, lay. Everything is your own body.
- **Minutes 3-10, the first workers.** Your eggs hatch into workers that think with the trained brain. You keep your
  body. You feed the larvae you laid (drop food beside them and they pupate sooner) and you keep laying.
- **From the first ten workers on, choose whose line the colony becomes.** A *deeds ledger* in the corner lists your
  best diggers and foragers by name. Pick one and press **Y**: your next five eggs carry *her* mind, not yours. Adults
  are tinted by line, so you watch the colony change colour cohort by cohort, and the ledger shows whether her line
  out-digs the rest.
- **The gamble.** **P** lays a *wild egg*: its mind is a mutated copy. Most are worse; some are jewels. A wild ant
  carries a fleck, and the ledger flags any that do well, so you can take her line in turn.
- **If you neglect to lay**, your hold on the colony lapses: workers start laying eggs of their *own* lines (ones you
  never chose). Come back and lay, and your rule returns. This replaces the binary "colony dies if the player stops
  laying" (review must-fix 9) with a graded, visible consequence that is real biology (queen loss releases worker
  laying).

What it teaches, honestly: **who the colony is made of.** On today's ants that is the one lever with real teeth
(review §1). Everything else in the design that does not reach a trained ant is cut (§6).

## 1. M0: measure before building the genetic verb (go / no-go)

The review's central risk: the shipped pool's 30 minds are near-copies (members 2-30 differ from member 1 in 8-107
of 3,285 slots), so "choose whose mind" could make the colony *uniform* without making it *behave differently*.

**M0a, common garden of the pool.** `labsurvive founders=std member=K scenario=<demo bed> seed=S after=30000`
(the `member=` switch is added on this branch: every ant thinks with member K alone). K = 0..29, S = 1..3. Per run:
alive, lived (mean headcount), births, starved, digs, deliveries.
- **Go** for the whose-line verb as the core if the spread of member means (between-member SD) on digs or deliveries
  is at least the within-member seed SD, *and* the best and worst members differ by 2x on at least one of them.
- **No-go** (members do not differ): the wild clutch becomes the core verb and whose-line becomes its follow-up
  ("take the jewel's line"). The demo is still playable, the HUD order changes.
- Positive control: `founders=old` (the hand-built ant) must differ from the pool on the same seeds, or the census
  cannot see a brain difference at all.

**M0b, does a wild egg differ?** Same harness, `member=0` with every ant's mind mutated once at the wild clutch's
width (`wild=W` switch on the harness), 10 mutants x 2 seeds: the spread of mutants against the seed spread of
member 0. This sets the width W: the smallest width whose mutants spread at least 2x the seed spread, so a gamble is
a gamble.

**M0c, the trained founding.** `queenprobe mode=found play=1` founded 20 workers on the hand-built ant (slice 0). Add
`newant=1` (trained pool) and run 6 seeds on the demo bed: frames to 10 and 20 workers, deaths by cause. The demo bed's
food distance and reserve are tuned on this (target: 10 workers by ~15,000 frames under the scripted player).

Proof: a table in the report with each number's `n`, and the raw `RESULT` lines in the shared folder.

## 2. The demo bed: `assets/lab_scenarios/demo_queen.ron`

- 512 x 320 box, soil 80 deep, grass, mister off, no beetles at first.
- One ant lands at column 256 at frame 60 (`Colony(count: 1)`), and the demo grants her a founding reserve
  (`steer::grant`, 900 J, the slice-0 value; tuned by M0c).
- Food: a provisions heap 40 columns east at frame 60, then steady income (`Heap ... every: 1500`, sized by M0c so a
  colony of ~60 lives). Near food is deliberate: the trained pool starves on far food (a known failure the player
  must not mistake for their own).
- Later wave (optional, off in v1): beetles at frame 60,000, the "season" hook the enemy seat can use.

Proof: `every_shipped_scenario_loads_builds_and_places_what_it_says` (the existing guard over every scenario) plus a
new test that the demo binds her as the body when she lands.

## 3. Engine pieces, in build order

Each with the proof it must show before the next starts. "Off is off" for every one: with no `demo=` the lab is
byte-identical (the existing `a_steer_on_no_living_body_changes_nothing` plus `deeptrace` byte-compare once, at the
end).

### 3.1 Launcher and body (DONE on the branch, shared with every seat)
`lab::demo` (`demo=<seat>`), the key layer (arrows steer; E Q F G B L T; seat keys Y U P J K R; H help; Tab follow;
PgUp/PgDn speed), `steer::SteerRole::{Queen, Worker}`, `steer::steered_id`, and **a keyboard-held mind ant takes the
hand-built path** (`creature_tick`), because `mind::tick` reads no steer hook. Guards: `a_keyboard_drives_a_mind_ant`
(watched red with the hand-off removed: "the mind ant never took the keys"), `an_arrow_through_the_layer_steers_the_
bound_body`. Landed first as its own small PR so the scout seat can stack on it.

**Two checks the review asked for, done before 3.2:** (a) while steered, her eggs still inherit *her* mind (a test:
steered mind queen lays; the egg's mind `Arc` equals hers); (b) the 16-frame `give_new_ant_minds` sweep never replaces
her mind (it only gives minds to ants with none; asserted in the same test).

### 3.2 The queen's hold: graded laying rule (`steer` + `creature::regime_of_colony`)
- `Steered` gains `last_lay: u64`, written where her `Lay` output succeeds.
- `regime_of_colony`: her colony runs `Queen` while `frame - last_lay < HOLD` (HOLD = 6,000 frames, about two egg
  stages; a constant with its reason), and the process regime (`graded`) after it lapses, so workers may lay.
- The HUD shows her hold as a draining bar, and a notice when it lapses ("YOUR HOLD HAS LAPSED -- WORKERS ARE LAYING")
  and when it returns.
- Proof: a test (queen lays, then stops: a worker lays within N frames of the lapse, none before); a 3-seed headless
  run of the scripted player who stops laying at 20,000: the colony keeps births instead of the measured 41 -> 27.

### 3.3 Whose line an egg carries (the shared "who lays" piece, `shared/who-lays.md`)
- Per colony, a small father store in `World`: `Vec<(Source, weight)>` plus a mix knob `p`, as agreed with the colony
  mind seat. Source = a live ant id or a stored `Arc<MindGenome>`. Empty store or `p = 0` is today's behaviour exactly.
- The one hook: where an egg takes its mind from its layer (`creature.rs` ~6087, `mind::inherit`), draw from the
  store with probability `p`. The draw uses the world's own RNG stream only when the store is non-empty (so off stays
  byte-identical).
- The queen seat drives it as "next K eggs": `Y` with an ant selected pushes that ant's mind with a count of K = 5
  (a queue rather than weights; the store supports both).
- Proof: unit test (empty store = layer's mind; a queued source's mind lands in the next K eggs, then back to hers);
  an off-is-off hash test.

### 3.4 The wild egg (`P`)
- The next egg's mind = `neutral_mutate(source mind, W)`, W from M0b. The demo marks the ant (`wild: true` in the
  demo's own table keyed by organism id, not a new field on every organism).
- `neutral_mutate` is private in `lab::training`; expose a `pub fn mutate_mind(mind, width, rng)` wrapper there.
- This is **in-box mutation, demo only.** The lab rule "a mind does not mutate at a birth in the box" is unchanged
  outside the demo. (Flagged to the owner in the report: it is a liberty the demo needs if M0a says no-go.)
- Proof: test that a wild egg's mind differs from its source in > 0 slots and an ordinary egg's does not.

### 3.5 Lines: names and the tint
- A *line* is a mind identity: the hash of the mind's weights (equal minds = one line). Its name comes from
  `lab::names` seeded by the hash ("the Ember line"); the founder queen's own line is "the Crown line".
- Tint: a new `OrganismOverlay::Line` (render.rs, a contested file: land quickly) that paints each ant's cells with
  its line's palette colour, on a fixed palette (not a blend into the cell's colour, CLAUDE.md's overlay rule).
  In demo mode the overlay is on by default and toggled with `U`.
- Cost: draw-only, per creature cell; quote the worst-frame cost from `examples/ascii.rs`-style timing on the demo
  bed before and after.
- Proof: a screenshot with two lines visibly different, and a counter (cells painted per line) printed beside it.

### 3.6 The deeds ledger (HUD panel)
- A small panel at the top right (demo only): the top 6 living workers by the current deed, each row: line colour
  chip, name, the deed's count, age. `J` cycles the deed (digs / deliveries / trip deliveries / lived longest), `K`
  moves the selection down the list (wraps). `Y` takes the selected row's line. Clicking an ant with the LOOK tool
  also selects it (the existing inspect pin), so either route works.
- Under the list, the line mix: one bar split by line share of living adults, and per line "deliveries per ant" for
  this cohort against the last. That pairing is the evidence the player needs (design §7, "unattributable outcomes").
- Data: `LifeCounters` (exist). Text: `render::Hud::text` (exists).
- Proof: screenshot plus a test that the ledger's top row is the ant with the most of that deed.

### 3.7 Feed the young (works now; make it visible)
- Q already drops crop food; a larva eats food beside it (`brood.rs:976-990`).
- Add: the HUD counts "hungry larvae N"; a pulse mark on a larva the queen just fed (draw-only).
- Proof: a scripted feed shortens that larva's time to pupa against an unfed sibling (paired test).

### 3.8 Notices: the cohort report
- When ants of a queued line hatch: "3 DAUGHTERS OF THE EMBER LINE HATCHED". When a wild ant hatches: "A WILD ONE
  HATCHED". When a line dies out: "THE MOSS LINE IS GONE". All through `ui.say`; cheap and legible.

## 4. The scripted player and the pictures

- `examples/queendemo.rs`: builds `Lab`, calls `start_demo(DemoSeat::Queen)`, and drives the queen through
  `demo_key` with a script (slice 0's `play=1` script ported: walk to the heap, E, home, G, L, and after 10 workers:
  select the top digger, Y; one P). Every `every=` frames it draws the real UI with `Lab::draw` (the `labgif` route) and
  writes a GIF plus stills, and prints the event counts (eggs laid, eggs of each line, wild eggs, hold lapses,
  workers alive) so each picture has its number.
- The Artifact page: the GIF of the founding, a still of the ledger with two lines, the tint before and after taking a
  line, the M0 tables, and the key card.
- Frame cost: `labperf`/ascii-style worst frame on the demo bed at 60 ants, demo on vs off.

## 5. Order and size

| Step | What | Size | Proof gate |
|---|---|---|---|
| 1 | Launcher + body + mind hand-off (PR A, shared) | done | 2 guards, one watched red |
| 2 | M0a-c measurements | half a day of runs | tables |
| 3 | 3.1 checks, demo bed, 3.2 hold | hours | tests + 3-seed run |
| 4 | 3.3 who-lays store, 3.4 wild egg | half a day | tests |
| 5 | 3.5 tint, 3.6 ledger, 3.7, 3.8 | a day | screenshots + counters |
| 6 | queendemo GIF, Artifact page, report in `Reports/`, PR B | half a day | the page |

## 6. Cut, and why (all from the review's table)
Provision/castes (dead three ways on minds), walk-gene vocabulary (minds skip the walk), brood carry and gravity
(no verb; carry rule measured useless; minds not pulled to brood), tandem runs (memory weights zeroed), kin bite
(nearest_foe skips kin), presence damping beyond the hold (§3.2 is the cheap real version), brood temperature
(stage times are fixed), the dream replay (the ledger is the attention fix), crossover patrilines (the store is
clone-of-father; crossover later), scent language, rival colony, mutiny, shelf-as-father (the shelf drops the mind).
Each is a candidate for a v2 once the memory layer (plan §7) or the enemy seat's seasons exist.

## 7. Collaboration
- Scout stacks on PR A (worker role, steered_id, key layer). The arc queen -> scout: the scout demo can start from a
  queen-demo save once both exist.
- Colony mind uses the 3.3 store with weights instead of a queue.
- Enemy: the beetle wave at 60,000 is the hook for a "season".
- Who-lays is built here and announced in `shared/who-lays.md` with the merged SHA.

## 8. Risks
- M0a no-go is likely (near-clones). Handled: the wild egg becomes the core; the demo still plays.
- The trained colony boom-busts and starves on far food. Handled by a near, steady food bed; the ledger's
  per-line numbers are paired cohort-to-cohort, so a bust is not read as a bad choice.
- A keyboard-held queen runs the hand-built path, so her own walk differs from her mind's. Accepted: her outputs are the
  keys anyway; above 4x her mind drives her.
- render.rs is contested: the overlay change is small and lands with PR B quickly.
- The beetle that kills her: none in v1 until the colony exists.
