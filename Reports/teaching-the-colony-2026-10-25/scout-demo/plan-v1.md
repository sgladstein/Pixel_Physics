# Lone scout seat: implementation plan to a playable demo (v1)

*2026-10-25, scout thread. Follows the step-1 review
[`review-step1-feasibility.md`](review-step1-feasibility.md) (verdict: feasible with changes) and
adopts all seven of its required changes. Base: `main` at `9e32979e`. To be independently reviewed
(step 3), then revised (step 4).*

## 0. What the demo is, in one paragraph

You play one ordinary worker in a hungry, trained lab colony. Near food is a thin trickle; a rich heap
lies about 200 columns away, past a step that slows anyone who follows. You walk out, find it, walk
home, and **tap** a hungry sister. She follows you **by touch**: she pauses now and then (learning the
way), and if you walk on while she pauses, contact breaks and she circles to find you. Bring her to
the heap and she **remembers it** (the keeper seat's recall head, a labelled gain), so she commutes on
her own. In arm A, a taught sister home with food sometimes taps a sister of her own (a labelled
scripted chance *c*), and a **teaching tree** grows on the overlay. The owner plays three blind arms
against each other: A (the scout), B (the rejected "walk to food and they follow", given the same
reward and HUD), and C (A with no cascade, the honest escort). The demo answers one question: **is
persuading one ant at a time more fun than being followed?** It does not answer whether the engine
can make a cascade by itself; that stays the seat's real risk (§9).

Launch: `cargo run --release --bin lab -- demo=scout arm=1` (or 2, 3). Off by default; the shipped
lab is byte-identical without `demo=`.

## 1. Pieces and who builds them

| Piece | Owner | State | What I need from it |
|---|---|---|---|
| Launcher `demo=<seat>` + `Lab::demo` enum + key layer (arrows steer, E Q F G B L T, seat keys Y U P J K R, H help, Tab follow-cam) | queen (`shared/launcher.md`, `steered-ant.md`) | agreed, building | `DemoSeat::Scout`; my code in `src/lab/demo/scout.rs`; seat keys reach my handler |
| `SteerRole::Worker` (no queen breeding rule; no old age, per "My player shouldn't die of old age") + `steer::steered_id` | queen | agreed | the player body is an ordinary worker; her colony keeps its own laying |
| Steering a **mind** ant + `World::mind_force: BTreeMap<OrganismId, ForcedPick{dir, verb, once}>` applied in `mind::tick` after the mind decides | enemy (`shared/steer-mind.md`) | agreed 08:02Z, building first as its own PR | (1) arrows actually move my trained body; (2) I fill entries for followers |
| Recall head: `World::recall`, `gain * FOOD_WAY` on Walk options, `recall::plant(world, ant, cell)`, per-ant "knows place P" | keeper (`shared/recall-head.md`) | proposed | the taught follower returns to the heap on her own; I call `plant` on arrival |
| **Tandem**: tap, follow by contact, pauses, break, search, arrival, cascade, tree | **scout** (new `shared/tandem.md`) | this plan | keeper may use "follow" for a carried ant later; mind may use the cascade rule |

**If a dependency is late**, the stopgaps (§7) let me build and measure in parallel without landing
anything that depends on them.

## 2. Step 0: look and measure before building (half a day)

Every number below is printed by a harness and posted, before any tandem code exists.

0a. **A colony that stays alive but hungry.** Write `assets/lab_scenarios/demo_scout.ron`:
512x320 box, trained colony (the lab's default), 52 ants landed at column 128 at frame 200; a near
trickle of provisions 25 columns east, small; a far heap of provisions at column ~330 (about 200
columns out), refilled on a timeline; a raised step of `extra_walls` stone between, 4-6 cells high,
so a follower must climb. Warm-up: the demo fast-forwards (time dial at its top preset) to frame
~15,000, then drops to 1x and hands control to the player. Tune the trickle so that, **with no player,
on 4 seeds**, the colony is alive at 15k (≥ 30 ants) and still alive at 15k + 36k (10 minutes at 1x),
and its median ant is hungry (bank below half). *Proof:* a table of alive at 15k / 51k and hunger, 4
seeds, printed by `examples/scoutdemo.rs mode=census`. Review H4 measured the trained colony crashing
52 → 10 by 3k frames on `steady_income`, so this is the step most likely to need iteration.
*No saved-world format exists in the lab* (grep: no save/load), so "a saved colony" is this warm-up.

0b. **The 1x rate and walking speed.** Review measured 60 ticks/s and ~1 cell per 6 frames (10
cells/s): a 200-column run is ~20 s alone. Re-measure on the demo scene with the trained body under
mind-steer once it lands. *Proof:* one printed line.

0c. **Does a mind ant put at far food deliver once?** Teleport 10 hungry trained ants onto the far
heap (positive control: the "follower's body" channel). Count deliveries home within 6,000 frames.
*Proof:* count printed, and a GIF strip. If zero, arrival must also plant the recall memory before
releasing (it will anyway), and the demo leans entirely on recall.

0d. **Does recall make a commute?** Same 10 ants, with keeper's recall at gain g ∈ {0, g1, g2}:
trips back to the heap per ant over 12,000 frames. Gain 0 is the honest arm. *Proof:* table by gain.
If keeper has not landed recall yet, I run this on keeper's branch, or skip the arm until it lands
(the demo can be built and played with "arrives, delivers once" as the reward; that is arm C's
honesty anyway).

0e. **Is trail B read by a mind?** Not needed for the demo; measured only because the review says
every trail claim in the design is unmeasured on the trained ant. 10 ants beside a hand-laid B line
toward the heap vs none. *Proof:* count of ants reaching the heap. Recorded, not acted on.

## 3. Step 1: the tandem module, `src/sim/tandem.rs` (1-1.5 days)

Sim-side, no lab dependency, so headless harnesses and tests use it. Off is off:
`World::tandem: Option<Box<Tandems>>`, `None` by default, every hook one `is_none()` branch.

**State.** `Tandems { pairs: Vec<Pair>, tree: BTreeMap<OrganismId, OrganismId> /* pupil -> teacher */,
patch: Option<(i32,i32,i32)> /* demo's target, x,y,r */, knobs: Knobs, counts: Counts }`.
`Pair { leader, follower, phase: Phase, since: u64, last_contact: (i32,i32), steps: u32 }`.
`Phase = Following | Pausing{until} | Lost{since, centre} | Done`.

**Knobs (all in one struct, printed on the HUD's help card):** contact radius 2 cells (Chebyshev,
from the follower to the leader's body); break gap 4 cells held for 45 frames; pause every m = 12
follower steps, lasting k = 40 frames (scaled by the follower's heritable `pace` gene: a slow sister
pauses longer, so "Pick a pupil" rests on a real gene, review H6); search 360 frames, an expanding
square spiral around the last contact cell; accept chance from hunger (below); arrival radius 6
cells of a food cell inside `patch`; max concurrent pairs 6.

**Per tick** (one call from the lab's step, before `world.step`, so the forced pick is fresh):

1. For each pair: compute `gap` = Chebyshev distance follower → leader.
2. `Following`: if gap ≤ contact → write `mind_force[follower] = ForcedPick{dir: toward leader, verb:
   None, once: true}`; count steps; every m steps enter `Pausing`. If gap > break gap for 45 frames →
   `Lost{centre: last_contact}`.
3. `Pausing`: `ForcedPick{dir: None}` (she stands: Rest). On expiry, back to `Following` if gap ≤
   contact + 1, else `Lost`. **This is the skill:** a leader who walks on through a pause loses her.
4. `Lost`: force a spiral walk around `centre`; if gap ≤ contact → `Following` (count a re-contact);
   after the search time → release (remove the force; her mind takes over) and count a break.
5. Arrival: follower within arrival radius of food in `patch` → `recall::plant(follower, food cell)`
   if recall is on; release; mark `Done`; add `tree[follower] = leader`; count a completion.
6. A broken pupil who later reaches the patch by herself (review: "broken runs still help") counts as
   "broken, still arrived" — tracked by remembering released pupils for 6,000 frames.

**Tap** (`tandem::tap(world, leader) -> TapResult`): candidates are same-colony adult nestmates
within 3 cells, not in a pair, not nest-bound (`nest_bound_until`), not carrying. Accept chance,
from mind-visible state only (review change 6): `p = clamp((0.75 - energy_frac) / 0.5, 0, 1) *
0.9 + 0.05`; a full sister almost always refuses. **A refusal is visible**: she gets a brief "shake"
mark and the HUD says "refused: well fed". Result: Accepted(id) | Refused(id, reason) | NoOne.

**Cascade (arm A only; labelled scripted).** When an ant in `tree` with a fresh recall memory of the
patch delivers home (her crop/spoil drops inside the nest radius), with chance *c* per delivery she
taps (same rule) and becomes a leader. **Her walk to the patch is a forced pick toward her
remembered place**, so the scripted part is exactly "a taught ant may lead, and walks to what she
remembers"; her pupil's following uses the same contact rule as the player's. Default *c* = 0.25,
on the HUD and in every card's meta, with a second run at *c* = 0.12 (review change: *c* in meta and
a half-*c* run). The rival scout is cut (review change 4).

**Arm B (the rejected design at its best).** No tap: while the player walks with nothing in her
jaws, every idle, non-paired nestmate within 6 cells joins a column: `Following` with **no pauses,
no breaks**, one leader. Same arrival, same recall plant, same tint, same counters. B is given every
legibility advantage A has (review §4.3).

**Arm C (the escort).** A with *c* = 0.

**Tests** (in `tandem.rs`, `cargo test --release --lib sim::tandem`):
- off is off: a world stepped 2,000 frames with `tandem: None` hashes identical to main (the
  `steer.rs:281` pattern). Positive control: set an empty `Some(Tandems)` and confirm the hash still
  matches; then one pair and confirm it differs (the test can fail).
- contact holds on flat ground for a leader walking at the follower's pace with no pauses.
- a pause plus a leader walking on causes a break; the same leader who stops does not break.
- a lost follower's spiral re-contacts a stationary leader 3-5 cells off within the search time.
- refusal rate rises with fullness (graded: 3 fullness levels, monotone).
- arm B never breaks; arm A on the same path breaks at least once when the leader ignores pauses.

## 4. Step 2: the seat in the lab, `src/lab/demo/scout.rs` (1 day)

- **Keys** (queen's layer): arrows walk; E eat/pick up; Q put down; F share (feed); G dig;
  **Y tap**; **U pin the last pupil** (follow-cam on her, Tab back); **P sniff** the ant under the
  cursor (one line: "knows the heap · led by you · trips 2 · fullness 40%"); H help card.
  Wait = hold no key (she stands).
- **Overlays**, drawn over the lab view, a full replace on fixed colours (CLAUDE.md overlay rule):
  - tint: ants that know the heap (recall fresh within r) orange; the player white with a ring;
  - contact pip above each follower: green in contact, amber pausing, grey lost, with the search
    spiral's square drawn faint;
  - the teaching tree: a thin line from each pupil to her teacher while both live, fading 30 s after;
  - a refusal shake (two-frame jitter + "no" glyph).
- **HUD line** (always on): `arm 2 · tandems 3 started, 2 done, 1 broken (1 still arrived) · pupils
  on a 2nd trip 1 · pupils who led 0 · know the heap 4 · alive 41 · c=0.25`.
- **Arm mapping is blind**: `arm=1|2|3` maps to A/B/C by a permutation drawn from a seed written to
  `target/scout-demo-seal.txt`; the HUD shows only the number. `arm=A|B|C` also works, for the
  maker's own testing.
- **The warm-up**: on load the seat sets the time dial to its top preset until frame 15,000, shows
  "the colony is settling…", then 1x and "you hatch". The player body is the youngest adult nearest
  the nest centre, marked with `SteerRole::Worker`.

## 5. Step 3: a headless player, `examples/scoutdemo.rs` (half a day)

A scripted player so every claim is reproducible without a window: walk to the heap, eat, walk home,
tap the nearest hungry sister, lead her back, waiting when her pip turns amber; repeat for 10 minutes
of game time. Modes: `mode=census` (step 0a), `mode=play arm=A|B|C c=0.25 seed=N`, `gif=1 out=x.gif`
(renders `Lab::draw` as `examples/labgif.rs` does, with `set_new_ant(true)` — review: `Lab::new`
defaults to the hand-built ant). It echoes its own parameters (CLAUDE.md gotcha). Output: the HUD
counters per minute, plus digs per arm (ants dig and forage all game: review §5), adults starved,
alive at start and end.

*Proof for the whole demo:* 4 seeds × 3 arms (+ A at half *c*), paired by seed, one table; and a GIF
per arm on the same seed.

## 6. Step 4: show it (half a day)

- An **Artifact page** "Lone scout demo": the GIFs as Arm 1/2/3 (blind, mapping in a collapsed
  section at the bottom), the counter table with *c* and alive-at-start/end beside each, the launch
  line and the keys, and the one question in plain words. The owner cannot see images in chat.
- The owner plays it locally: `cargo run --release --bin lab -- demo=scout arm=1`, then 2, then 3,
  and says which felt best. That verdict is the demo's result.
- **Frame cost:** `cargo run --release --example ascii` worst frame on main vs branch (must be
  unchanged: the module is off) and the demo's own tick cost with 6 pairs, both quoted on the page.

## 7. Order, dependencies and stopgaps

1. Step 0a/0b/0c now (needs nothing; 0b re-measured after mind-steer).
2. `tandem.rs` + tests against a local `mind_force` shim with the agreed shape (`ForcedPick`), kept
   in my branch only, rebased onto the enemy's PR when it lands. Nothing landed depends on the shim.
3. If mind-steer is more than a day late: prototype the player body by stripping her mind and
   exempting her id from `give_new_ant_minds` (`src/lab/mod.rs:1089-1105`), review H1(b); for
   **prototyping only**, never landed, never shown to the owner.
4. `scout.rs` after the queen's launcher lands (rebase).
5. Recall: arms work without it (arrival = one delivery); with it, pupils commute. Wire it when the
   keeper lands it; if it doesn't land in time, run 0d on the keeper's branch and say so.

## 8. Landing

- One PR from `claude/project-thread-x1ms83`: `src/sim/tandem.rs`, `src/lab/demo/scout.rs`,
  `examples/scoutdemo.rs`, `assets/lab_scenarios/demo_scout.ron`; the step-1 review, this plan, its
  review and the results as `Reports/teaching-the-colony-2026-10-25/scout-demo/` with a line in
  `Reports/README.md`; a README `Lab` status paragraph for `demo=scout`; `Reports/instruments.md`
  row for `scoutdemo`; `how-the-ant-works.md` untouched (no change to the shipped ant).
- Before the PR: `cargo test --release --lib sim::tandem`, clippy as CI runs it, `bash
  scripts/docscheck.sh`, `python3 scripts/deadendindex.py --touching`. Hand to the merge desk.
- **Default off, and why** (project rule: features default on unless there is real uncertainty): the
  demo is a separate mode with scripted, labelled fakes (*c*, the recall gain); it is not a feature of
  the shipped lab. The tandem rule for every ant (design step 5) is the thing that could one day ship
  on, and only by beating no-tandem on paired seeds.
- Tell the Teacher lane only if anything changes training results (nothing here should: all off).

## 9. What the demo cannot prove, said in advance

- **That the engine can produce a cascade.** Arm A's tree is the scripted chance *c*. Whether a
  taught ant leads by her own genes is design step 5 (lead as a heritable body rule, gated on
  far-food deliveries beating no-tandem on paired seeds). If the owner loves A only for its tree,
  the result is "a cascade would be fun", not "we can build one".
- **That memory is learned.** The commute rides the recall head's labelled gain.
- **Anything over hours.** Minds do not mutate at birth in the lab, and the pool is near-copies, so
  the design's fifth hour (scouting lineages, born knowing) cannot appear (review H8).
- **If B wins**, that is a real result: the owner's "too simple" meets "it has to feel satisfying".
  The page says so before the owner plays.

## 10. Collaboration

- queen → scout → mind arc: the scout demo starts from an established colony; a later "you were
  her daughter" hand-off from the queen demo's "first workers hatch" point is noted, not built.
- keeper: we share recall; keeper's "carry an ant to the station" and scout's "lead her there" are
  two verbs for the same lesson and could be one demo with two hands (the keeper's from outside, the
  scout's from inside). Offered in both status files.
- colony mind: if the mind seat wants "send never-foraged ants", the cascade rule (a taught ant taps
  a hungry one and leads her) is a candidate that sends ants **with** a route, unlike RECRUIT.

## 11. Estimate

Step 0: 0.5 day. Tandem + tests: 1-1.5. Lab seat: 1. Headless player + runs: 0.5. Page + PR: 0.5.
**About 4 days**, consistent with the review's 3-5, plus waiting on three other seats' pieces.
