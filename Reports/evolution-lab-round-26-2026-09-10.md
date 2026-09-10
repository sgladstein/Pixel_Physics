# The evolution lab, round twenty-six: the box gets its first relationship

*The coordinator's record of one round, 2026-09-10 (`session_01NGdywxc1ACg3L5scK7xBTc`),
moved out of [`lanes/evolution-lab-coordinator.md`](lanes/evolution-lab-coordinator.md)
the same day because the note had reached 30 KB against its 12 KB cap — the
"report living in the channel" failure `lanes/README.md` names. Status:
**record, not a work order.** Every number here was taken by the lane it is
credited to and is on `main` or on the branch named; what binds from the round
stays in the note.*

**Read this if you want to know what the round overturned.** Nine lanes ran
under one coordinator in about nine hours, seven landed on `main`, one is in
flight, one was killed and salvaged. The design of record for the ecology is
[`evolution-lab-ecology-design-2026-09-10.md`](evolution-lab-ecology-design-2026-09-10.md);
for the bodies, `Reports/creature-articulated-body-2026-09-09.md` §7e–§7f, which
lives on branch `claude/creature-evolution-engine-67lhjp` (PR #303) and is not
on `main` yet.


*Coordinator `session_01NGdywxc1ACg3L5scK7xBTc` (tags `evolution-lab`,
`coordinator`). State at open: phase one landed (round twenty-five); PR #293
measured the breeding clock (queen-only is a thirteen-fold collapse and is
off the table; individual against graded is the owner's taste call, §6.1 of
that report); articulated creature bodies are in flight on
`claude/creature-evolution-engine-67lhjp` (Opus, running — it owns
`creature.rs`, `organism.rs`, `render.rs`, `ant.ron` and `hopper.ron` until it
lands); the eusociality branch holds one unlanded scenario. Both were poked
with the handover the recorded way and both fires landed (`cse_<lane>`).*

**The direction, in one paragraph.** Phase one made the box legible; what it
still lacks is a *relationship*. One organism eating another is the entire
ecology, a flower is colour with no function to any animal, and the seed
inside a carried fruit dies on the trip. The owner's own long-horizon
complaint — *over a long enough time everything dies out* — has an
ecological answer nobody has built: close the loop **fruit → animal → nest →
seedling** and the colony becomes the plants' distribution network and the
plants the colony's renewable larder. **The colony that gardens survives.**
It is visible at play zoom (a ring of herbs around a nest, flowers turning
to fruit after visits, fruit carried home in a line, a bed whose plant map
records where the colony foraged), it is graded by construction (a bonus to
setting seed, never a requirement — `dead-ends.md`'s seed-limited trap), and
it is the patchy larder that §Z7's recruitment finding said no bed had.
This is the owner's own first thought (*flowers and fruits and interactions
between plants and creatures*) with the player's reason for it attached.

**Lanes this round** — separate worktrees, file-disjoint, the coordinator
merges on CI green and runs `docscheck` after each:

| lane | delivers | owns |
|---|---|---|
| measure (Sonnet) | `scenario=` on `windfall_probe` and `chronicle`, both echoing their parameters — this coordinator ran `chronicle scenario=played_bed` and got the frame-0 bed with no warning, the silent-ignore gotcha; then the fruit loop censused on the played bed, three seeds × 120,000 frames: set, dropped, on the floor, eaten / carried / rotted, and where fruit-borne seedlings stand relative to the nest | `examples/windfall_probe.rs`, `chronicle.rs`, world-side counters only |
| design (Opus) | the ecology design of record: gut/midden dispersal, nectar and pollination as a graded bonus, pollen as gene flow (an owner ruling, posted as a card with the BRUSH hand version), palatability co-evolution, a niche by height; build order with the creature-side hooks specified as one call each and placed last | `Reports/` |
| legends (Sonnet) | the chronicle written out on reset and quit (the direction report's first big swing), and a HISTORY page of the lines that ended — the graveyard the owner rated 5, given a place | `ui.rs` (new panel), `mod.rs` reset path, `world.rs` log region |
| rain (Sonnet) | measure whether the played bed dries over a session, then `RAIN OFF / LIGHT / STEADY / HEAVY` from the lid through the water tool's own placement, default set from the measurement, a counter beside it (Arc B3, owner-asked) | `mod.rs` water tool, `ui.rs` BOX rows, a scenario `Setting`, one `labstats` line |

**Ruled since round twenty-five, and where the ruling lives:** the breeding
trade is **graded suppression** — the owner answered it in chat, and it is
recorded as commit `e5792206` on the eusociality branch (PR opened by this
round), not as a card. **That is a rule, not an accident: the owner's
standing instruction is *"if it does not require a visual, just ask
questions here"*** — a text question goes to the owner in chat, and the
queue is for things judged by eye. Two conditions travel with the ruling
before graded ships as the default: the breeder lookup must scale (today it
scans every organism in the world, plants included, once per tick for every
animal that can afford a child — `crowded_bench.ron` is the pessimal case
built to price it), and `GRADED_MAX_SUPPRESSION` is a provisional 6.0 that
nothing has swept. The extra-breeder anomaly (#293) has one live
hypothesis: a non-finite `bank`/`reachable` passing `bank + reachable <
f32::INFINITY`; the instrument is a finiteness `debug_assert` at the
precheck and a print at the `children` increment under `queen`.

**Landed or opened so far, and what each overturned** (kept current as the
round runs):

- **design → PR #296**, `evolution-lab-ecology-design-2026-09-10.md`, filed
  under *Creatures and ecology* by the index's own rule. Three measurements
  that change the build: **the fruit pipeline is budget-limited, not
  animal-limited** — a ripe fruit went unfilled 18,867 times in 40,000 frames
  against 56 that dropped, and the refusals double with no colony, so a
  pollination bonus goes on the ripening *price*, never the clock;
  **nothing has ever eaten a flower** — the best mouthful in 40,000 frames
  was a 960 J fruit, so the 1,440 calibrates a printed ceiling and a comment
  and nectar is cheap; and **`labshot scenario= seed=` silently ignores the
  seed**, so every played-bed contact sheet to date is seed 1 (the same
  defect `labforage` fixed on itself; three lines). Build order **A0 → A1 →
  A2 → B1 → B2 → C on the ruling → D → E**, plant side first, every
  creature-side change one call at a named line (`creature.rs:4954` the
  bite, `:5001` the clear, `:5137` the drop) after the bodies branch lands.
  Its recommendation on pollen: **the player's BRUSH first**, animal-carried
  pollen only on an owner reversal, because it costs the
  cluster-in-genotype-space definition that is the lab's only operational
  test for plant speciation. Cards `…6dfed9` (pollen) and `…cba50c`
  (dispersal form) are the two rulings, asked in chat as well.
- **measure → PR #297**, `Reports/lanes/evolution-lab-ecology-measure.md`.
  `windfall_probe` and `chronicle` now take `scenario=` and echo it. On the
  played bed, three seeds × 120,000 frames: **32 windfalls produced, standing
  stock zero at every 30k checkpoint** (a fallen fruit lasts 354–518 frames),
  fates eaten-or-carried 19 / rotted 14 / unclear 17 (the positive control
  `handout=1000` moved eaten-or-carried 2 → 81, so the instrument reads),
  and **2 of 3,089 germinations came from windfall**. The loop the round is
  named for is not closing through fruit, and the first-order reason is not
  reach (89% / 7% / 17% of standing organs within ground reach by seed — a
  property of the draw, not a bottleneck). **The finding under it: windfall
  cells arrive on the floor already ownerless** (`organism_id = 0`, credited
  to neither producer, 7 and 15 unexplained departures on seeds 1 and 3),
  and an ownerless windfall can never germinate because `germinate()` is
  reached only through organism-scheduled dispatch. Eight causes ruled out
  by direct check; the line was not found. Reproduction lives behind
  `WF_DEBUG=1` in `windfall_probe` (frame 1,184, cell (456,158)). **This sits
  under the whole arc**, so it got its own lane at once.
- **A1 → PR #301.** `labshot` seeds a scenario at last (seeds 1/2/3 on the
  played bed went from digit-identical to different); `pip.ron` at 40 J with
  the arithmetic in its own comment (40 × 0.25 = 10 < 12: the shipped gut
  cannot farm it, a plant specialist can — seed predation as a niche in one
  number); `plant::seed_survives_bite` at `seed_gut_survival: 0.6` on herb
  and scrambler, the organism kept so the pip germinates on the ordinary
  path; five counters; the one-line hook at **both** clear sites as its own
  commit; six tests, suite 1,553 / 0. **And the finding that reorders the
  arc: an ant bites a fallen fruit about twice in 360,000 frames on the
  played bed** (spills 0 / 2 / 0 over three seeds, with the ownerless
  counter at ~0 after #300, so it is not the bug) — `dead-ends.md` line
  1636 reproduced on a different bed and gut. The mechanism is right and
  the event does not happen: the herb's fruit stands 22–40 rows up, drops
  rarely (budget-limited, §296), and is gone from the floor in a few hundred
  frames. The ablation was not run (null against null at n=2) and
  `reproductive_allocation` was not re-derived, both by the brief's own
  fork. **So A2 (the seed rides home) is not the next build; making fruit
  reach the mouth is.** Two routes, cheapest first: a low fruiting plant in
  the bed (the `scrambler` is exactly that and has never been in the played
  bed — **lane M2**, Sonnet, running: the played bed with four scramblers
  against the played bed, one binary, three seeds × 120,000, bites / spills
  / pips / seedlings and whether the thicket feeds the colony), then B2
  (pollination as a ripening-price discount) if fruit is still scarce.
- **W → PR #300, §Z8 closed.** The ownerless windfall was a **third
  windfall-creation path neither counter saw**: a fruit organ severed by
  ordinary structural failure (a branch snapping under its own hanging
  weight) rides `fell_severed_tissue → promote → settle()` like any felled
  limb, and `rigid.rs`'s `settle()` writes `Cell::new(into, shade)` for
  every `severs_into` target — right for `wood → log` and `leaf →
  deadleaf`, and for `fruit → windfall` it dropped the organism id and the
  `Seed` packing. Confirmed on the measure lane's own frame and cell
  (`src_organism_id=4136 fruit → windfall` at frame 1,183, (456,158)).
  Fixed narrowly on the windfall target only — keep the id, stamp `Seed`,
  let `World::set`'s existing re-anchor seam and a scheduling call the
  landing was missing do the rest — with a guard watched red first
  (`a_severed_fruit_lands_as_windfall_still_carrying_its_organism`).
  Ownerless appearances **15 → 0 and 3 → 0** on two seeds over 40,000
  frames; windfall-sourced germinations 1 → 2 and 0 → 0 (one seed each, so
  noise, recorded as such). It narrows `settle()`'s deliberate "must not
  silently re-attach" rule and says so in the code; the rule's stated harm
  (a structural body reappearing load-bearing) cannot apply to a `Powder`.
  `plant.rs` untouched. A1 was told to merge it before its deciding sweep.

- **rain → PR #298**, `src/lab/rain.rs`. Measured first: the played bed loses
  **5.7% / 4.1%** of its soil water over 120,000 frames on two seeds, against
  a bare-soil control that lost exactly nothing — so `RAIN` ships **OFF**,
  with a live `LIGHT / STEADY / HEAVY` ladder (50 / 150 / 400 cells per 1,000
  frames) on key `8` (every letter was bound), placed through the WATER
  tool's own call, jittered from the world's RNG, counted by
  `World::rain_cells`. A "water still in the air" instrument read 30 with
  rain off — the lid's own condensation — and was pulled for that reason.
  Card `…f2fb5b` (STEADY against OFF, a GIF) asks whether the rate reads.
  Also built: `examples/labgif.rs`, the lab's missing headless GIF capture.
- **Landed to main this round so far:** #296 (design), #297 (measure), #295 (breeder index and the recycled-slot fix), #298 (rain), #300 (the windfall keeps its seed), #301 (the pip), #302 (the thicket bed and `windfall_bitten`), #304 (the legends) — merged in that order 04:12–12:16, `docscheck` clean after each; the trunk run on the first four is green. #299 (the ablation switch) is merged into the bodies branch, not main.
- **The eusociality lane, un-poked, kept going and found a shipped bug**
  (four commits on PR #295, CI running): an organism id is
  `(generation << 12) | slot`, and both breeder-scan loops iterated bare slot
  indices, so **the breeding rule was blind to every animal in a recycled
  slot** — which is the whole of #293's extra-breeder anomaly (a queen in a
  reused slot was invisible, so a second animal bred). Found by running the
  old scan and its replacement, a **per-colony breeder index (33x fewer
  organisms visited at 12,000 frames on the crowded bench, 11.7x at
  40,000)**, as two arms of one binary over 40,000 frames: identical for
  26,100 frames, then one birth apart. Fixed; all six queen seeds now report
  exactly one breeder; the thirteen-fold collapse is unchanged and now tight.
  PR #295 is therefore code, not docs; it landed before the bodies branch, which must merge `main` before anything else happens to it.
- **The articulated bodies are built and they do not walk** — the creature
  lane's own §7, pushed and then the session went idle at ~$145. Ant 5
  segments / 7 cells, hopper 7 / 8, expressed from four heritable rules
  each; **blocked on 43.9% of moves on `flat` and 96.8% on `rolling`**
  (hopper 74.6% / 91.9%) against a plain `Chain(6)` control at 2.5% / 12.4%;
  `ascii` red ("the colony has gone sessile"); 4 of 52 ants founded on
  `scene=colony`. One real deadlock fixed (the lateral sat on the segment
  ahead after any upward step) and it moved the number barely; three
  hypotheses recorded as moving nothing. The clue: the hopper has one
  lateral and is blocked more than the ant's two — **spine length**, not
  laterals, is the suspect. Also fixed on that branch: `corpse.ron`'s
  `food_energy` 480 → 120, a 3.5x energy creation on the burnt-corpse path
  the re-pricing exposed. **Not landable; no PR.** Its §7d names the one
  next step — an env switch that places a `Segmented` body's laterals or
  not, inside one binary — and **lane B ran it (PR #299, stacked on their
  branch): the laterals are the whole cause and the segmented spine is
  exonerated.** Same binary, `PIXEL_PHYSICS_BODY_LATERALS=0`: ant **43.9% →
  2.0%** flat, **96.8% → 14.8%** rolling (the `Chain(6)` control 2.5% /
  12.4%); hopper 74.6% → 5.5%, 91.9% → 40.7% with a residual that is not
  plain length either (`Chain(7)` 13.7%). A `Segmented` body of length 6
  with no laterals is **byte-identical** to `Chain(6)` on every counter, so
  `segmented_body_after_step` degenerates to `chain_follow` exactly. The fix
  is a design decision — what a lateral does when its cell is not placeable
  (`lateral_for`, `landing_is_placeable_through_tissue`) — so the creature
  session was poked with a bounded brief: a lateral may never block a move
  the bare spine could make; it takes the other side or **tucks** when there
  is no room (a body that squeezes, the ethos' middle); re-run the table,
  `ascii` green, the moving card, then merge `main` and open the PR. **It
  asked instead whether to build (2–3 hours of Opus) or hand over; the
  coordinator answered by poke under the owner's cost policy — Opus designs,
  Sonnet builds — and it wrote §7f, the rule as a build spec** (the spine
  decides the move and a lateral never votes; authored side, other side, or
  tuck; colour keyed on (segment, is-lateral) so a re-emerged cell returns
  the colour it left; five sites named to the line; three tests; the §7e
  table as the bar, laterals-on landing on laterals-off). **Lane T** (Sonnet,
  running, cut from the bodies branch) builds exactly that and opens the PR
  against `main`.
  The ingestion question is answered in its lane note: **two clear sites**
  (the mouthful into the crop, `creature.rs:5001` on main; brood
  provisioning, which clears and credits in one block) and the drop site
  where a carried load becomes a whole cell again (`:5137`) — A1 was told to
  hook both clears.

- **M2 → PR #302, merged.** The played bed with four scramblers against
  the played bed, one binary, three seeds × 120,000: **fruit on the floor
  9.6x** (118 dropped a run against 12), **real bites 5 → 32 across the
  sweep** (a `windfall_bitten` counter that counts only fruit — its first
  form counted every bare-seed bite, 384 against 0 spills, and was caught
  by asking what it counts when nothing is wrong), spills 2 → 8, **and not
  one of the eight surviving pips grew into a plant on either bed.** So
  the thicket moves fruit to the mouth and the seedling half of the loop
  has never yet been seen live: the event is still rare (13 bites in
  360,000 frames on the thicket bed, of ~355 fruit dropped — a fallen
  fruit lasts a few hundred frames and the colony rarely finds one), and
  the ownerless counter spiked to **19** on one scrambler seed (0–1
  everywhere else): a *fourth* ownership path, likely the scrambler's
  heavier structural churn, not covered by #300. Colony intake trended up
  on the thicket bed inside a spread wider than the gap. Card `…c8709b`
  asks whether the thicket belongs in the default bed.
- **T → PR #303 (open, not landable yet).** The tuck rule built to §7f:
  ant **43.9% → 1.9%** blocked on flat and **96.8% → 22.1%** on rolling
  (chain 2.5% / 12.4%), hopper 74.6% → 6.8% and 91.9% → 15.4% — the hopper
  beats its own laterals-off arm on both presets, the ant lands on the
  chain on flat and 4.4x better on rolling with a residual tuck cost;
  `ascii`'s forage guard **0 → 32 round trips**; the moving card `…0180fc`
  is up. Three things stop it landing, and **lane K** (Sonnet, running,
  pushing to the same branch) has them in order: **founding is still 4 of
  52** (placement refuses a site unless the whole authored footprint is
  empty — the spine-only rule must apply at placement too; an owner
  dropping a colony today gets four ants); the §9 guard is still red (one
  articulated ant opens the plate at frame 101 — length or width, to be
  diagnosed with the switch, then re-scene or fix the accounting); and
  `ascii` now reaches the chamber scene and fails `roofed > 0` under
  width-2 bodies (6 ants dig 96 cells, no roofed void, 5 die; the bare
  spine passes with roofed 14) — fix if accounting, write if geometry.
- **L (legends) hung at 04:13 with 861 uncommitted lines** (transcript
  stopped mid tool-call, no process alive); the coordinator committed its
  worktree as an unverified WIP (`a59e118f` on `claude/lab-legends-r26`),
  killed it, and **lane L2** (Sonnet, running) finishes from there — merge
  `main` first, since seven landings moved `ui.rs`, `mod.rs` and
  `world.rs` under it. **L2 → PR #304, merged**: the salvage compiled and
  passed as found; the export writes on reset and quit through
  `format_log_line` (header, LINES view, a LEGENDS paragraph per ended line,
  the counts); the HISTORY page is on **F5** (every letter and the digits
  the rain lane checked were bound) and from the LOG page's own row; the
  brief's own verification step — inject `~#~`, watch it go red — found two
  independently hardcoded kingdom labels and fixed them at the root. Card
  `…c0b68b` shows the page at frame 5,752 with 47 ended lines.

- **K → pushed to PR #303 (`499b64ec`), and the bodies are still not
  landable — for three reasons that are now design decisions, not
  defects.** (A) **Founding is fixed to the spine-only rule** (placement
  calls the movement rule's own `lateral_for`; a guard watched red first)
  and it lands exactly on the width-free control on both scenes — colony
  scene **4 → 5** of 52, played bed at frame 6,000 **11 → 12** of 52 — so
  the width gap is closed and **what remains is body length: a 2-cell ant
  seats 39 of 52 on the played bed and a 5-segment one seats 12.** An owner
  dropping a colony gets a third of it. (B) **The §9 guard stays red, and
  the cause is length, not width**: with laterals off the lone attacker's
  median breach barely moves (101 → 112, against the 900-frame timeout the
  scene was built on), through the pre-existing whole-body bite scan; and
  re-scaling the plate makes it *worse* (swarm-to-lone ratio 0.76 at armour
  1 → 1.41 at armour 8) because eight wide bodies crowd each other 1.2–3.7x
  more than one does. (C) **A width-2 colony cannot dig a roofed chamber**:
  both widths dig the same open crater, only the width-free colony reaches
  the deep gallery that holds a roof (14 cells), the wide one digs 28% less,
  is blocked 63% against 54%, and 5 of 6 starve outside the bank before a
  chamber forms. Report §10–§12 carry the numbers. `cargo test --lib` 1,562
  / 1 (§9) / 69 ignored; clippy clean; `ascii` green through "a double
  bridge" (forage trips 14, bar 6) and red at the chamber scene.
  **Reading:** the articulated body is a change to every constant that was
  calibrated on a two-cell ant — founding sites, bite adjacency, tunnel
  geometry — and each of the three is the shared-budget trap `CLAUDE.md`
  names; none is a lane's to settle. The owner's call: accept the bodies
  with those three costs re-derived as a follow-on programme (a founding
  rule that seats a long body — curl, or found along the surface; a bite
  that is the head's, not the body's; a dig that a wide body can roof), or
  keep the branch as the design and the measurement and ship the
  two-cell ant. PR #303 stays open with CI red until that is ruled.

**Put to the owner this round** — in chat, per the rule above, with the
priced readings on cards as a second copy: pollen as gene flow (the animals
carry it, the player's BRUSH carries it, or both with the animals off); the
seed inside a taken fruit — left on the spot, or carried home and put down
where a laden ant puts things down (a ring of herbs around the nest, at about
three times the work); the rain rate and default; the HISTORY page. Still
unanswered from round twenty-five: the tree in the bed (the owner's mix was
grass, herb and shrub, so a *no* closes it), the marks three-way, and *is the
box empty*.

**Inherited from the phase-one coordinator, kept or dropped here:** the
food-trail half of §Z7 (a decay sweep via `labforage bdecay=`);
`scent_spread > 0` as the condition under which castes are adaptive; `CROSS`
on the shelf; outdoor-to-lab collection. None is started this round.

**Outside the box, offered and not started** — each is a question for the
owner before it is a lane: **BRUSH**, pollinate by hand — the shelf's `CROSS`
verb realised physically in the bed, on the same mechanism the animals would
use, so the breeding fantasy arrives through the ecology; **sound** — the
ethos lists *no sound* among what leaves an event unfinished, the lab has no
audio dependency, and it needs a ruling on the dependency before anything
else; **a nectar-feeding hopper** that lives up the stems — a niche by height
(every food is worth the same at every height today), the first animal that
is not a ground ant in body, verb and food, after articulated bodies land;
**palatability co-evolution** — expensive dark leaves defended, the colony's
gut evolving against the bed's defences, two kingdoms evolving against each
other in colour on screen; **the bed as a record** — do not sweep it between
experiments; **wild collection** (Arc C3).

**Environment, learned this round:** a poke's fire response names where it
landed — `cse_<lane id>` both times here, so the session-programs correction
of 2026-09-09 holds. **A trigger's prompt bound to another session cannot be
edited afterwards** (`update_trigger` refuses), so every new message is a new
trigger, and the old one is deleted the moment it has fired so a stale prompt
cannot be re-fired. And a lane whose status reads *"awaiting … results"* is
usually in the sub-agent trap the note above names — it woke, resumed its
build lane, and went idle again waiting for a turn that will never end.

