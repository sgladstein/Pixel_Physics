# Lone scout seat: implementation plan to a playable demo (v2, revised after review)

*2026-10-25, scout thread. v1 ([`plan-v1.md`](plan-v1.md)) was independently reviewed
([`plan-v1-review.md`](plan-v1-review.md): buildable, seven blocking fixes). This version adopts every
blocking fix and every improvement; §12 lists them one by one. Feasibility: [`review-step1-feasibility.md`](review-step1-feasibility.md).
Step 0a's census is in [`step0/step0a-census.md`](step0/step0a-census.md). Base: `main` 9e32979e.*

## 0. The demo in one paragraph

You play one ordinary worker in a trained lab colony. A near trickle keeps the colony alive; a rich
heap lies about 120 columns out, past where unled ants usually go. You walk out, find it, eat, walk
home. You **feed** a sister who is too weak to go, then **tap** her. She follows **one cell behind
you, by touch**, walking in your footsteps. Now and then, at a landmark (a slope change, a tunnel
mouth, the heap's edge), she stops to learn it; you see her **drop back a cell** first. Walk on
through her stop and contact breaks; she sweeps the ground left and right looking for you. Bring her
in and she eats, turns and **carries food home along the route you showed her**, drawn as a fading
line, and the patch's income bar moves. A pupil you lost half-way **keeps what she learned**: she may
still find the heap, more likely the more landmarks she stopped at. In arm A a taught sister home
from the heap sometimes taps a sister of her own (a labelled scripted chance per trip), and a
teaching tree grows. **The question:** is persuading one ant at a time more fun than being followed?
Launch: `cargo run --release --bin lab -- demo=scout arm=1` (2, 3). Off by default.

## 1. Pieces and who builds them

| Piece | Owner | State | What scout needs |
|---|---|---|---|
| Launcher `demo=<seat>`, `Lab::demo`, key layer (arrows steer, E Q F G B L T, seat keys Y U P J K R, H, Tab follow-cam) | queen | agreed, building | **plus three asks (sent):** `arm=`/`c=`/`sealed=` passed through to the seat; a seat **draw hook inside `Lab::draw`** (so `labgif` GIFs show overlays); a seat **tick hook per simulated tick inside `tick_world`** (`lab/mod.rs:4875`), so the binary at 4x and the harness run the same sequence |
| `SteerRole::Worker` (no queen breeding rule, no old age), `steer::steered_id` | queen | agreed | |
| Steering minds + `World::mind_force` (`ForcedPick{dir, verb, once}`, applied in `mind::tick` after the mind decides) | enemy | agreed; "built, tests running" | followers are minds, so **no tandem runs without it**. If no PR within a day of now, I land a minimal slot (~30 lines, the reviewer's probe worked first try) in the agreed shape as its own PR and offer it to enemy, rather than a shim that never lands. Ask: forced picks update memory via `scratch_log::remember` as `PP_OLDLAB_ACT` does (`mind.rs:2377-2386`); and when holding Right meets a vertical face, fall back to the 90-degree (Up) option so the player climbs |
| Recall head (`recall::plant`, `recall::knows -> freshness`, `copies_by_generation`) | keeper | agreed, landing first | **one ask:** `plant` with a strength (`plant_with(world, ant, cell, s in 0..1)`), so a broken run plants a weaker memory (§3.6). Until recall lands, the HUD hides "second trips" |
| **Tandem** | **scout** | this plan | `shared/tandem.md` |

## 2. Step 0: measure first (0.5-1 day)

**0a. A colony that stays alive (done, first pass).** Near trickle of provisions 6 cells per 1,000
frames at 25 columns, colony of 52 landed at frame 200: alive on 4 of 4 seeds, 21-37 ants at 18k,
72-109 at 48k. 44 of 52 landers starve within 2,000 frames of landing (known), so the demo warms up
to **frame 18,000** at the dial's top preset (`TimeControl::set_preset`, `time.rs:832`), handing over on
the first tick at or past 18,000. If a seed comes in under 20 ants, top up with `Lab::grant_ants`
(`lab/mod.rs:1071`), demo-only. Re-run at the final far distance with mutation **on** (as shipped).

**0b. Far distance, by measurement.** Unled colony ants' reach vs a led ant's round trip on one tank
(~1,000 frames ≈ 200 cells at 5 frames per cell; instruments.md labsurvive row). Sweep the heap at 90,
120, 150 columns: pick the nearest where unled ants rarely work the heap (census "far" column under
~5) and a led pupil at the accept band's floor arrives with energy to spare. Expected 100-130.

**0c. Does a led trained ant deliver once?** Done with the forced slot (no teleport exists): force 10
ants to walk to the heap, release, count deliveries home in 6,000 frames and "pupils starved while
led". Positive control for the follower's-body channel.

**0d. Recall dose-response** (when keeper's piece is up): the same 10 with `plant` at gain 0 / g1 /
g2: trips back per ant in 12,000 frames. Gain 0 is the honest arm.

**0e. The nest exit.** Straight-line following stalls at tunnel corners and the door (review 6). Lead
10 pairs from inside the nest out with straight-line vs breadcrumb follow; count stalls. Picks §3.2.

0f (recorded, not acted on): is a hand-laid trail B read by minds? Keeper measured it evaporates in
~100 ticks, so skip unless cheap.

## 3. The tandem module, `src/sim/tandem.rs` (1.5-2 days)

Sim-side, lab-independent. `World::tandem: Option<Box<Tandems>>`, `None` = off.

**3.1 State.** `Pair { leader, follower, phase, crumbs: VecDeque<(i32,i32)> /* leader's recent head
cells */, landmarks_seen: u8, landmarks_route: u8, since, last_contact }`;
`Phase = Following | Tell{until} | Pausing{until} | Lost{since, centre, sweep} | Done`;
`tree: BTreeMap<pupil, teacher>`; `released: Vec<(id, frame, strength)>` for "broken, still arrived";
counters (§5). All draws `rng::stream(seed, id, frame, slot)`.

**3.2 Following (fixes review 1, 6, 8).** While `Following`, the follower is forced at **every gap up
to the break gap** (4), never left to her own mind: she steps toward the **oldest leader crumb within
reach** (breadcrumb follow; straight-line only if 0e shows it never stalls). At **gap ≤ 1 she Rests**,
so she sits one cell behind, never on top of the leader. Break: gap > 4 for 45 frames → `Lost`.

**3.3 The leader keeps the pupil's pace (review 8).** While the player leads, her step is held to no
faster than her slowest follower's decision interval (her ForcedPick becomes Rest on the ticks her
pupil cannot match). The player body is chosen at the colony's modal interval. Sniff shows "slow" or
"quick" (the heritable pace).

**3.4 Pauses: situational, few, and told (review 2).**
- **Where:** at landmarks on the leader's crumb path — a change of slope of 2+ cells, a tunnel mouth
  (roofed ↔ open), a junction, the heap's edge — plus a geometric clock with mean 50 steps. Expect
  4-6 stops per 120-column run, not 16.
- **The tell:** 12 frames before a stop she lags one cell (Rest for one decision), and her pip turns
  amber.
- **How long:** 60-90 frames, longer for a slow sister.
- **Resume:** at gap ≤ contact + 2 (4) she picks up; past that she is `Lost`. A leader who stops
  in time gains a landmark (`landmarks_seen += 1`, shown as a tick on her pip); one who half-stops
  (gap 3-4) keeps contact but loses the landmark. Graded, not pass/fail.

**3.5 Lost: a sweep on the ground (review 6).** She walks left, then right, along whatever she
stands on (surface or tunnel), widening each leg (8, 16, 32 cells) around the last contact cell, for
up to 360 frames. If the leader comes within 2 → `Following` (a re-contact, landmarks kept). Else
release. Overlay: a bracket on the ground, widening.

**3.6 Arrival and the graded lesson (review 2, 3).** Arrival = within 6 cells of a food cell in the
patch: `recall::plant_with(follower, food, 1.0)`, release, `tree[follower] = leader`. **A released
(lost) pupil** gets `plant_with(follower, patch_food, s)` with `s = landmarks_seen / max(landmarks_route,
1)` times 0.6 — a weaker, partial memory, so "broken runs still help" has a mechanism. Whether she
reaches the patch is counted for 6,000 frames ("broken, still arrived"). Without recall: arrival
gives the follower's-body channel only (she is at the food), and the HUD says so.

**3.7 Tap: hunger makes the choice (review 3, 9).** Candidates: same-colony adults within 3 cells,
not paired, not carrying. **No nest-bound filter** (it would make one in four sisters, by id,
untappable). Read `s = c::ENERGY` (the mind's own `e/(1+e)`, `mind.rs:1924`):
- `s < s_weak` (≈ a tank short of the round trip, from 0b): **refuses, "too weak to go"** — feed her
  first (F, share a taste) and tap again. Feeding is the recruiting verb.
- `s_weak ≤ s < 0.8`: accepts with chance falling from 0.9 to 0.4 as she fills.
- `s ≥ 0.8` (≈ 4 tanks): accepts at 0.15, "well fed".
- Every refusal is visible: a head-shake and the reason over her.
The bands are set from the 0a/0b bank distribution (warmed colony: p10 22-286 J, p50 116-1,039 J,
p90 300-1,282 J at 15k per the review probe), so roughly half of nearby sisters accept at first.

**3.8 The cascade (arm A; labelled scripted; review 5).** Keyed on the **first delivery of a trip**
(`returned = take(trip_load)`, `mind.rs:2995-2998`, or the `life.trip_deliveries` delta) **with her
recall memory inside the patch**, never on spoil (digging) or on a trickle trip. With chance *c* per
trip she taps by the same rule and leads toward her remembered cell (her forced pick is "toward my
remembered place"; her pupil's following is §3.2-3.6, the same as the player's). Cap 6 concurrent
pairs colony-wide. *c* = 0.25 per trip, second run at 0.12. *c* is printed only when not sealed.

**3.9 Arm B, the rejected design given equal terms (review 7).** Same key: **Y "calls"**: the tapped
sister and every eligible idle sister within 6 cells fall in, **same eligibility and hunger bands,
same cap (6)**, same arrival release and recall, a 3,000-frame timeout. No tells, no pauses, no
breaks, no cascade.

**3.10 Arm C, the escort.** A with *c* = 0.

**3.11 Tests** (`cargo test --release --lib sim::tandem`), all on **mind** worlds (`set_new_ant(true)`):
- off is off: `tandem: None` vs `Some(empty)` hash equal over 2,000 frames; one live pair differs
  (positive control). Same seed + same scripted input → same hash (pairing depends on it).
- follow holds at every gap ≤ 4 on flat ground; she never stacks (gap 0) for more than 2% of frames.
- a leader faster than her pupil is held to the pupil's pace.
- walking through a pause breaks; stopping keeps; a half-stop keeps contact and loses the landmark.
- the sweep re-contacts a still leader 6 cells off.
- refusal bands are monotone and every band shows a refusal reason.
- the cascade fires per trip, not per cell put down, and never on spoil.
- B's cap and timeout hold; B never pauses.
- `MindCounts::failed` stays 0 under forced picks.

## 4. Step 2: a headless player and the first picture (1 day)

`examples/scoutdemo.rs`, before any keys or HUD (review 10's build order). A scripted player: walk
out (pressing Up at faces), eat, walk home, feed a weak sister, tap, lead, stop at tells, eat to stay
alive. Modes: `census`, `play arm=A|B|C c= seed=`, `gif=1 out=`. Renders `Lab::draw` as `labgif` does,
with `set_new_ant(true)`, **mutation on** as shipped. Echoes its parameters. Per minute: the §5
counters.

**First result on an Artifact page** (no launcher needed): a GIF of one A run and one B run on the
same seed, plus the counters. If either looks wrong, fix before building more.

## 5. Step 3: the seat in the lab, `src/lab/demo/scout.rs` (1-1.5 days)

- **Keys:** arrows walk (**Up climbs**, on the help card); E eat; Q put down; F feed; G dig;
  **Y tap** (B: call); **U** pin the last pupil (Tab back); **P** sniff the ant under the cursor
  ("knows the heap (0.6) · led by you · 3 landmarks · slow · bank 2.1 tanks"); H help.
- **Overlays** (full replace on fixed colours): the player white with a ring; knows-the-heap tint by
  freshness (orange ramp); pips (green in contact, amber tell/pause, grey lost) with landmark ticks;
  the ground bracket while searching; **the pupil's route home as a fading line**; the teaching
  tree's lines; refusal head-shakes.
- **HUD:** your bank and your pupil's bank as two small bars; **the patch's income bar** (food
  delivered home from the heap per minute); `tandems started / done / broken (still arrived) · pupils
  starved while led · know the heap`. **Sealed mode** (the default for `arm=1|2|3`) hides *c*, the
  knobs and the arm letter; `arm=A|B|C` shows everything, for the maker.
- **Warm-up:** "the colony is settling…" at top speed to 18,000, then 1x, "you hatch"; the player is
  a modal-pace adult near the nest, `SteerRole::Worker`.

## 6. Step 4: the owner's hands, early and twice (review: the most likely way it ends dull)

- **A rough playable, A against B, at about day 2** (after step 3 runs in the binary, before the
  polish): the owner runs the branch, `demo=scout arm=1` then `arm=2`, and says which they would play
  again tomorrow. Posted with the GIFs on the Artifact page.
- **The final round:** a practice run first (arm order rotated), then arms 1-3 sealed. The page says
  in advance what each result would mean, including **"B wins" as a real result** (the owner's "too
  simple" meets "it has to feel satisfying"), and that **A and B differ in three ways at once** (one
  ant against a crowd, friction, consent), so "B wins" does not say which lost. The blind is real
  only between A and C; B is recognisable in seconds, and the page says so.
- **Meta on every picture:** *c* per trip; tandems started / done / broken / broken-still-arrived;
  pupils starved while led; pupils on a second trip; pupils who led; ants knowing the heap at 10 min;
  patch income; alive at start and end; **digs and foraging trips per ant-minute** (not raw digs:
  the colony digs 43-60 cells in 13.5k frames, so raw counts have no power).

## 7. Frame cost

`examples/ascii.rs` is the outdoor game and cannot move, so quote **the lab's own tick cost**, demo on
and off on the same warmed state (harness `scoutdemo mode=cost`), plus the overlay's draw cost per
frame. Gate: tandem tick under 2% of a lab tick with 6 pairs.

## 8. Order (a demo exists even if a dependency never lands)

1. 0a re-run, 0b, 0c, 0e (forced slot from enemy, or my minimal slot PR if none within a day).
2. `tandem.rs` + tests.
3. `scoutdemo` + the first GIF page (needs no launcher, keys or recall).
4. Seat module on queen's launcher (or, if it is a day late, a minimal `demo=scout` arg in my own PR,
   offered to queen; one launcher, whoever lands first).
5. Rough A vs B to the owner.
6. Recall wiring (keeper), the cascade, sealed arms, final page.

## 9. Landing

One PR (plus the minimal slot PR if needed): `src/sim/tandem.rs`, `src/lab/demo/scout.rs`,
`examples/scoutdemo.rs`, `assets/lab_scenarios/demo_scout.ron`. **Scenario page side-effect:**
`demo_scout.ron` would appear on every player's SCENARIO page (`ui.rs:3816`, `Scenario::list`) and
enter the scenario-wide tests (`scenario.rs:647`, `:688`): either keep it (labelled "DEMO: lone
scout") and pass those tests, or load it by path from the launcher only — decide with queen for all
six seats. Reports: step-1 review, plans v1/v2, the plan review, step-0 and results under
`Reports/teaching-the-colony-2026-10-25/scout-demo/` with a `Reports/README.md` line; a README lab
status paragraph for `demo=scout`; an `instruments.md` row for `scoutdemo`. Before the PR:
`cargo test --release --lib sim::tandem`, clippy as CI, `docscheck.sh`, `deadendindex.py --touching`.
Hand to the merge desk. **Off by default, and why:** it is a demo mode with labelled scripted fakes
(*c*, recall's gain); the tandem rule for every ant (design step 5) is what could ship on, and only by
beating no-tandem on paired seeds. Nothing changes training, so the Teacher lane is told only of the
findings (landing starvation, bank distribution).

## 10. What the demo cannot prove

That the engine can produce a cascade (A's tree is *c*); that memory is learned (recall's gain); the
fifth hour (minds do not mutate at birth in the lab; the pool is near-copies). If the owner likes A
only for its tree, the result is "a cascade would be fun", not "we can build one".

## 11. Estimate

Step 0: 0.5-1 day. Tandem + tests: 1.5-2. Headless player + first page: 1. Seat: 1-1.5. Rounds with
the owner + landing: 0.5-1. **5-6 days**, per the review.

## 12. What the review changed

| # | Finding | Change |
|---|---|---|
| 1 | Gap 3-4 unforced; stacking at gap 0 | Forced at every gap ≤ break; Rest at gap ≤ 1 (§3.2) |
| 2 | Pauses a 1-second metronome, pass/fail | Landmark + geometric(50) pauses, a tell, 60-90 frames, resume ≤ 4, landmarks as graded lessons (§3.4, §3.6) |
| 3 | Accept formula has no unit; pupils starve; player's hunger ignored | `c::ENERGY` bands with "too weak" refusals, F as the recruiting verb, both banks on HUD, starved-while-led in meta, far distance from 0b (§3.7, §2) |
| 4 | `extra_walls` seals the box; a step slows nobody; Right stalls at a face | No step claim; any obstacle via `Fill`; "Up climbs" + asked enemy for a 90° fallback (§1, §5) |
| 5 | Cascade per cell (c=0.25 → 82%/trip), on spoil | First delivery of a trip with recall in the patch; c per trip (§3.8) |
| 6 | Square spiral in a side view | Left-right widening sweep on the ground; breadcrumb follow, picked by 0e (§3.2, §3.5) |
| 7 | HUD unblinds; B unequal | Sealed mode; B uses Y with same bands, cap, release, timeout; page states what's blind and what isn't (§3.9, §5, §6) |
| 8 | Pace mismatch | Leader held to pupil's pace; modal-pace player; mutation on (§3.3) |
| 9 | Nest-bound filter = caste | Dropped (§3.7) |
| 10 | Dependencies unpushed; launcher lacks hooks | Minimal slot PR fallback; three launcher asks; build order guaranteeing a GIF first (§1, §8) |
| 11 | No teleport for 0c | Walk them with the forced slot (§2) |
| 12 | Hash test on the wrong ant; ascii wrong tool; scenario-page side effect | Mind worlds, None vs Some(empty); lab tick cost; scenario decision (§3.11, §7, §9) |
| 13 | Smaller | grant_ants lever; per-ant-minute digs; remember on forced picks; rng::stream; set_preset hand-over at ≥ 18k |
| dull | Escort chore, invisible payoff, owner read once | Route line + income bar; situational friction; owner's hands at day 2 and at the end (§5, §6) |
