# Scout seat, step 1: adversarial review and feasibility

*2026-10-25. Reviewer did not write the design. I read `designs/seat-scout.md`,
`seat-scout-critique.md` and `engine-brief.md` in full, and skimmed
`seats-overview.md`, `teaching-plan.md` §2, §7 and §8 and the files in `shared/`. I checked
the claims against `main` at `9e32979e`. "Checked by running" means a
throwaway probe crate in my scratchpad that links `pixel_physics` by path. No
repo file was changed. The probe source is at
`/tmp/claude-0/-home-user-Pixel-Physics/450fdc25-ebaf-5a36-892f-b0e15c5babd7/scratchpad/steerprobe/`.*

## Verdict: feasible with changes

A playable faked slice is a few days of one agent's work, but only if it is
cut down from what the design specifies. As written, the slice has one
hidden blocker: the player's body cannot be steered in the lab's default
colony. Neither the design nor its critique saw this. Two of its key pieces
of evidence also come from a different ant from the one the demo would run.
Its A/B judges fun from GIFs, which cannot show fun. And its "Lead is a
skill" rests on a follower that, as scripted, never falls behind. Each of
these can be fixed. The list of changes is at the end.

## 1. What a scout demo can stand on today

**A steerable body, but only on the hand-built brain.** `sim::steer` has
five hooks, all inside `creature_tick`'s hand-built path:

- the output override at `creature.rs:8171`, applied through `steered_outputs` at `:27728`;
- the step at `:27763`;
- the `act` rewrites at `:20391`.

The always-on rules sit outside that path and apply to every body: no old
age (`creature.rs:7947`), no caste (`steer.rs:143`) and the queen rule
(`creature.rs:23030-23035`). **But an ant with a mind returns into
`mind::tick` at `creature.rs:8022`, before the output hook.** `mind.rs` has
no `steer` reference anywhere (grep). The lab opens on the trained mind ant
by default (`src/bin/lab.rs:297-307`, owner 2026-10-24).

*Checked by running* (`steerprobe`, `steady_income`, colony landed at frame
200, one ant marked at frame 1,200, then 1,500 frames):

- **Hand-built ant.** Unsteered she walked 115 cells. Steered with no key
  she walked 0. Steered east she walked 250 cells, to the east wall (x 257 to 507).
- **Mind ant.** Unsteered, steered with no key and steered east all gave the
  same trajectory, cell for cell: 295 cells, ending at x 69, *westward*. The
  keyboard is ignored. This confirms the enemy seat's finding (`shared/steer-mind.md`).
- **Mind stripped** (`mind::scratch_log::strip_mind`). The lab gave her a
  mind back within 15 frames, through `give_new_ant_minds`. That sweep runs
  every `NEW_ANT_SWEEP` = 16 frames (`src/lab/mod.rs:1089-1105`, `:1951`,
  `:4637`), and she walked off again (end x 189 while steered east).

So the cheap workaround, a hand-built body inside a mind colony, also needs
the player's body exempted from that sweep.

**Speed.** A steered ant moves about one cell per 6 frames: 250 cells in
1,500 frames, checked by running. 1x is 60 ticks a second: `PRESETS` is at
`src/lab/time.rs:740`, and 256x is 15,360 ticks a second at `:1147`. So the
player walks about 10 cells a second at 1x, and 200 cells takes about 20
seconds.

Tick cost without render, measured on 2 threads (checked by running, `pop`
probe):

- 0.2-0.5 ms with 10-90 mind ants;
- 0.3-1.4 ms with 52-326 hand-built ants.

So 4x to 16x walking is affordable here, and "drop to 1x at a tap" is a
dial move.

**Keys.** None are bound to steer. Nearly every letter is already a lab
control (`src/bin/lab.rs:568-800`). Every slice-0 verb letter collides:

| Steer verb | Lab control on that letter |
|---|---|
| E | the food tool (`:724`) |
| Q | fling (`:762`) |
| F | the framerate (header `:32`) |
| B | ADD (`:709`) |
| L | the life overlay (`:778`) |
| T | the clock |

The queen seat's proposed key layer (`shared/steered-ant.md`) handles this:

- arrows steer, and the dial moves to PageUp and PageDown;
- verbs read before the lab handler in `demo=` mode;
- Y U P J K R are reserved for seat verbs.

The scout needs two new keys, Tap and Pin. Wait is "no key held", which
already makes her stand (probe: 0 cells).

**Other pieces that exist:**

- **Tint and follow-cam.** Per-animal colour modes exist (`render.rs:1281`
  `CreatureColour`: Job, State, Energy). A "knows patch" tint is one more
  variant. Render has `follow()` (`render.rs:4494`), and the lab has
  follow-inspected (`ui.rs:4024`). `examples/labgif.rs` has a `follow=`
  camera (`:213`).
- **Ant pages and life records** exist (`Panel::Ants`; the engine brief).
- **Food placement.** The food tool (`lab.rs:724`) and scenario `Heap`
  placements. A heap is deterministic. A fruiting plant is not, so use a
  heap for the demo.
- **The last-food memory** exists as `OrganismState::last_food`, set by
  `mind::pick_up`, and read as `f::FOOD_WAY` (`options.rs:172`, `:692`,
  `:1136`). It is zeroed on load for the shipped pool (`mind.rs:539-552`).
  `std_pool.txt` line 5 is `layout=4`.
- **A brain-override precedent.** The mind already lets a scratch teacher
  replace its pick, which is exactly the slot a scripted follow needs:
  `PP_TRAIL_ACT` replaces the Walk target when the brain chose Walk, and
  `PP_OLDLAB_ACT` replaces `d.pick` (`mind.rs:2395-2404`, `:2359-2391`). The
  enemy seat is generalising this into a steer-for-minds slot
  (`shared/steer-mind.md`; the coordinator calls it `World::mind_force`).
- **Nothing makes one ant follow another.** No tandem, follow or leader rule
  exists in `src/` (grep `tandem`: none). `chain_follow` (`creature.rs:31271`)
  is body segments, not ants.
- **Headless showing.** `examples/labgif.rs` renders `Lab::draw`, the
  binary's own draw with the bar, into a GIF with no window and no GPU.
  `PIXEL_PHYSICS_SCREENSHOT_AFTER_FRAMES` gives one still under
  xvfb + lavapipe (`.claude/rules/running-the-app.md`). Playing needs a
  window, so the owner plays it on their machine. Note that `labgif` builds
  through `Lab::new`, which is the **hand-built** ant (`src/lab/mod.rs:657`
  `new_ant: false`) unless it calls `set_new_ant(true)`.

## 2. Hidden blockers

**H1. Steer does nothing on the shipped ant** (above, verified). This is
build step 2's real cost, and the design calls it "bind keys". There are two
routes:

- **(a)** Take the enemy seat's mind-steer slot (`shared/steer-mind.md`):
  keys pick the matching option. This is the right route, but the scout then
  waits on another seat's PR.
- **(b)** A stopgap. Strip the player's mind and exempt her from the 16-frame
  re-gift, so her body runs the hand-built path and today's hooks. That is
  about 10 lines, but her body rules then differ from her sisters': for
  example, `act`'s share against the mind's Feed.

Use (b) only to unblock prototyping, and switch to (a) before the owner plays.

**H2. The same override slot is the whole scripted slice.** Scripted follow,
"knows patch P", the scripted rival leader and arm B's column are all one
operation: *this ant's next step is the Walk option nearest a target point.*
Build one per-ant force map, e.g. `World::mind_force: Option<Map<id, Force>>`,
with Force = Toward(point) or Follow(id). Build it once and use it for all
four. The design specifies them as four separate fakes. `World::steered` is a
single `Option` (`steer.rs:103`), so the rival cannot be a second steered body.

**H3. The queen rule freezes breeding.** This is real. The line numbers
moved: `regime_of_colony` is now `creature.rs:23030-23035`, and the
colony-wide lock is `suppress_bar` at `:23592-23595` (`f32::INFINITY` while
any other member has `children > 0`). It also outlives her death
(`steer.rs:41-47`). The fix agreed in `shared/steered-ant.md`
(`SteerRole::Worker`) is right, and it keeps queenprobe's numbers unchanged.

**H4. The trained colony is fragile, and the opening scene will kill it.**
Checked by running (`pop` probe, one seed, mutation off, `steady_income` with
its colony and heap moved to frame 200, food 30 columns out):

| Brain | Ants at landing | 3k frames | 36k frames |
|---|---|---|---|
| Mind | 52 | 10 | 90 |
| Hand-built | 52 | 59 | 326 |

This agrees with the link-price row: with fruit 200 columns out, the shipped
pool keeps 28 alive (`how-the-ant-works.md:1637`). The design's opening has
80 established ants, near food grazed bare and fruit 200 cells out. On
today's pool that box empties toward a few dozen. Most die before the player
has led anyone, and well-fed survivors idle.

`far_larder` is unusable as shipped. Built through `Lab::new`, every ant was
dead by 6k frames with either brain (51 to 0 hand-built, 0 by 3k frames
mind), because its heap first lands at 6,000.

The demo needs its own `demo_scout.ron`, started from an established,
**saved** colony rather than a fresh landing, with a near trickle that keeps
the colony alive but hungry. "Starving when food is far" must be a pressure,
not a countdown that ends before minute five.

**H5. Two key pieces of evidence describe the other ant.** The slice-0 trail
null (Mark "pulls nobody") and the recruit harm (442 against 298/323) were
both measured on the hand-built ant:

- slice 0 used `Lab::new`, which is the hand-built ant (`mod.rs:657`);
- `recruit_contacts` is called at `creature.rs:8189`, on the path mind ants
  never reach (`:8022`).

The mind ant reads trail B as an option feature, and nobody has measured
whether it follows one. The design's "Mark: now, pulls nobody" and its
Display risk are therefore unknowns on the demo's ant, not known results.
The ant reference adds to the confusion: its §16 describes steer for the
hand-built ant, and §13 still calls the mind "harness only" (line 1704).

**H6. The follower never falls behind.** In the scripted slice, a follower
"steps toward the leader within 2 cells", and a run breaks if the gap stays
over 3 for 60 frames. The leader and follower decide on the same clock,
about one step per 6 frames modulated by the heritable `pace` gene
(`creature.rs:7938`). So the gap opens only at obstacles: a fall, a climb,
water. Contact then never breaks on open ground, and the only failures are
terrain accidents the player cannot read. Either way there is no rhythm to
learn.

The biology the design cites supplies the fix. The follower stops to learn
landmarks, so script a stop-start: a pause of k decisions every m steps. The
player walking on through a pause is what breaks contact. "Pick a pupil" can
then rest on the real `pace` gene: a slow sister lags more. Body size does
not set speed, and the design admits the size match was a stand-in.

**H7. Smaller blockers.**

- **Pass on.** Re-`mark`ing another id is nearly free (`steer.rs:139`), but
  the old body loses no-old-age and the queen rule moves with the mark.
- **Lay (L).** On a mind body, L does nothing unless the steer-mind slot
  maps it: mind laying is the body's own bar (`how-the-ant-works.md` §13,
  "the mind passes Lay 0.0").
- **Named meals.** The crop holds energy, not a food identity. In the demo,
  the material of the cell last picked up can stand in for it (provisions
  against windfall), as a UI fake only.

**H8. The design's claims, re-checked against the trained ant.** This
widens H5, after the colony-mind seat's finding
(`status/colony-mind.md`, 08:40Z), which I verified. A mind ant returns
into `mind::tick` at `creature.rs:7998-8022`. That is before `sense`,
`act`, `chooser_step` and the walk's pulls. It reads only its option
features and context (`options.rs:160-215`; `mind.rs:72-150`). Re-read in
that light:

- **"A fixed-weight ant at food carries it home, laying trail B [§7]"**
  (channel table). §7's trip-load lay rule (`food_trail_lay`) belongs to the
  hand-built ant. A mind lays A, B and alarm through its own learned emit
  units (`mind.rs:66-69`, `:211-212`). Whether the shipped pool's B emit is
  live has not been measured. A mind does read B, as strength and two
  gradients per option (`options.rs:1021-1023`). So trail B can matter to a
  mind follower, but neither its laying nor its reading is known.
- **"Fed ants idle, so persuasion needs need"** and **"Tap accepts by her
  own state"**. The forage drive, the met-a-returner stamp, RECRUIT,
  MEET_WAY and the store rules are all `act` or walk machinery, and never
  run for a mind. A demo's accept rule must read mind-visible state: bank,
  crop and hunger.
- **Nest-bound for "not nest-bound".** This still exists for minds: the
  one-in-four caste stamps minds unless `PP_MIND_CASTELESS` is set
  (`how-the-ant-works.md:1638`).
- **"Playing on as a daughter: the egg inherits your genome."** For a mind
  colony, `mind::inherit` copies the *parent's mind* (`mind.rs:1730-1742`).
  Under the H1 stopgap the player has no mind, so her daughters are born
  mindless and the 16-frame sweep hands them a pool mind. "Your genome" is
  then lost. This is another reason to use the proper mind-steer slot.
- **The fifth hour: "a scouting lineage", "heritable lead gain", "born
  knowing".** These are dead in the lab as shipped:
  - minds do not mutate at a birth in the box (`src/lab/mod.rs:1026-1030`;
    `mind.rs:1735-1741` clones the `Arc` genome unchanged);
  - the 30 pool members are near-copies: each member after the first is
    stored as a delta from member 0 (`training.rs:94-96`), and members 1 and
    2 differ in 80 and 107 of 3,285 slots (checked by parsing
    `std_pool.txt`).

  Selection inside one played colony cannot move a gene. This does not
  touch the demo, but the design's long-game pitch rests on it.

## 3. The verbs, priced for a demo

The tags are (a) real engine work, (b) a scripted fake that is acceptable in
a demo, and (c) cut from the demo.

| Verb | Demo cost | Tag |
|---|---|---|
| Taste | E works on the hand-built path. On a mind body it needs H1. The crop glow is UI over "last material picked" | a (H1) + b |
| Share a taste | F feeds energy (H1 again). The learned smell is the memory layer | energy a; smell **c** |
| Tap | A new key. Nearest idle, non-nest-bound nestmate within 3 cells. The accept chance falls with fullness, so "persuasion needs need" shows. About half a day | b |
| Lead / Follow | `Follow(leader)` in the force map, a contact test, stop-start pauses (H6), break and spiral search. 1-1.5 days with the slot | b on top of a |
| Wait | Free: no key means she stands (probe: 0 cells) | exists |
| Pick a pupil | Tie it to the `pace` gene, not size, and show pace on Sniff | b |
| Display | Needs memory plus a read trail. The recruit evidence is the wrong ant | **c** |
| Mark (T) | The key exists. Whether mind ants follow B is unknown, so make no claim | exists, unmeasured |
| Carry nestmate / brood, Assess, Head-butt, Warn | Each is a subsystem, and none is needed for the one question | **c** |
| Open a door (G), Stock a cache (Q) | Exist (H1) | exists |
| Lay (L) | Not needed for the question | **c** |
| Sniff | The ant page exists. Add a one-line "knows P / led by you / trips left" read | b, small |
| Pass on | Re-`mark` plus a camera move. A stretch goal | b, stretch |

What a demo really has to build:

- the mind-steer slot (H1), which is shared;
- the force map (H2);
- Tap, Follow with pauses, break and search;
- "knows P" for n trips;
- the scripted cascade with chance *c*;
- a tint, a tree drawn as lines between ants, contact pips and a counter HUD;
- a scenario and an arm switch.

That is 3-5 agent-days, consistent with "a few days", if the rival and the
smell are cut.

## 4. The faked slice, as specified and as it should be

**Buildable as specified? Mostly. It answers the question badly in three ways.**

1. **A blind GIF cannot judge fun.** The question is whether *persuading*
   is more fun, which is about the player's hands: the rhythm, choosing whom
   to tap, recovering a lost pupil. A GIF of a scripted player shows a
   spectacle. The board requires something the owner can launch and play.
   So the arms must be a playable switch, e.g.
   `cargo run --release --bin lab -- demo=scout arm=1|2|3`, with the
   arm-to-letter mapping shuffled per seed so it stays blind. GIFs go on the
   Artifact page as a supplement.
2. **Arm A can win by construction.** A carries a scripted cascade at chance
   *c*, so its "teaching tree" is whatever *c* makes it. A high *c* gives a
   beautiful tree that the engine may never produce (step 5 is ungated
   today). That is fine for a faked slice only if the card says so: *c* in
   the meta, and a second A at half *c*. Then the owner judges the mechanic,
   not the parameter.
3. **Arm B must be the rejected design at its best.** "Idle nestmates within
   6 cells fall in behind you" is fair, but it must get the same patch, the
   same "knows P" reward and the same HUD. Otherwise A wins on legibility,
   not on persuasion.

**The rival and the shared smell should be cut from the first slice.** The
rival is cheap once the force map exists. But the design's version is a
hand-picked colony ant given a scripted leading job from frame 4,000, which
is a hand-made role even in a fake. It adds a second opinion before we know
whether the first one is fun. The smell needs a food-identity fake and a
second steering bias. Both belong in slice 2 if A beats B.

**The smallest version that still answers the question:**

- **Scene.** A saved, established mind colony of about 60-100, hungry but
  alive (H4). A near trickle, and one heap 150-200 columns out, past an
  obstacle that slows a follower: a step or a gap, not necessarily water.
- **Body.** One steered worker via the mind-steer slot, with
  `SteerRole::Worker`: no queen rule, no old age.
- **Arms:**
  - **A**: Tap, Follow with stop-start and breaks, Wait, "knows P" for 3
    trips, and a cascade at *c*;
  - **B**: the rejected design, a column within 6 cells, same reward, no
    breaks;
  - **C**: A without the cascade.
- **HUD and overlays:** the knows-P tint, contact pips, the tree's lines, a
  counter line, and follow-cam.
- **Meta on the card:** *c*; tandems started, completed and broken; pupils
  on a second trip; pupils who led; ants knowing P at 10 minutes; adults
  starved; ants alive at the start and the end.

**If A does not beat B in the owner's hands, stop.** If A beats B but not C,
the friction is the fun, and the cascade, the whole engine roadmap of steps
4-5, is not what makes the seat.

## 5. Owner rules

- **One physics.** Kept. Nothing here touches cells or planes.
- **One brain, no hand-made castes.** A follow that overrides the mind while
  in contact is a *state-entered body rule*: any ant can be tapped, and it
  ends on a break. It is like a `Crossing`, which `creature_tick` runs out
  before the mind decides again (`how-the-ant-works.md` §13). It is
  acceptable in a demo and is not a caste.
  - Three things in the slice do cross the line. The *rival scout* is
    hand-picked for a scripted role. A cascade that only "flagged" ants may
    start is the same problem. "Knows P" overrides the brain's foraging for
    whole trips, which is a fake memory, not a body rule.
  - All three are acceptable **only** as labelled fakes in a non-default
    mode. To ship, accept and lead must be heritable-gain body rules seeded
    above zero, as the design's own "How each new piece gets weight" says.
    The demo cannot test that part.
  - A better demo rival: whichever colony ant first picks up at the seed
    patch becomes a leader by the same cascade rule. A state picks her, not
    a script.
- **Ants dig and forage all game.** Kept, if followers return to their own
  mind after a run. Watch "knows P" for 3 trips: it pulls ants off digging.
  Count digs per arm.
- **Graded outcomes.** Kept by design: breaks half-help, and the tree's reach
  is continuous. It is lost if H6 is not fixed, because contact is then
  all-or-nothing by terrain.
- **Every verb has a visible consequence.** Tap: the follower turns, and a
  pip appears. Wait: the search ring. A tap refused for being well fed must
  *show* the refusal (a shake, a word on Sniff), or it reads as a dead key.
- **Frame cost.** The contact checks touch only ants in a tandem, and ants
  cost 0.2-1.4 ms per tick here. No risk.
- **The shipped lab unchanged by default.**
  - Use the `demo=scout` launcher (`shared/launcher.md`).
  - The force map and mind-steer are `None`, so the default path stays
    byte-identical. Prove it as `steer.rs:281`'s "off is off" hash test does.
  - The regime change goes behind `SteerRole::Worker`, so queenprobe keeps
    the queen rule.

## 6. Top risks that the demo will be dull or will not show its point, ranked

1. **It is an escort mission with a fake ending.** C, the honest arm, runs
   about 20 seconds out per run plus the walk back: 3-4 runs in ten minutes,
   each ending in one delivery. A's tree is the scripted *c*. If the owner
   likes A only because the tree grows, the demo has proved that a fake
   cascade is fun, not that the seat is buildable. Mitigation: *c* in the
   meta, a half-*c* run, and C timed.
2. **Leading has no skill (H6).** Same-pace followers never lag, and breaks
   come only from terrain. The core claim, "a good run is a rhythm", is then
   absent. Script the stop-start before anything else, and tune it by play.
3. **The colony dies or idles before the player matters (H4).** On today's
   pool, a far-food box loses most of its ants in the first few thousand
   frames, and fed ants refuse taps. Start from a saved, hungry but stable
   colony. Put alive-at-start and alive-at-end in the meta.
4. **Losing the pupil among dozens of identical two-cell ants.** An ant is
   readable by motion only (`Reports/instruments.md` row 67). A pupil needs a
   tint plus follow-cam plus pip, or A and B look the same.
5. **B is too good, or too weak.** A column is immediately legible and
   satisfying, while tandems are slow. If B wins, the owner's "too simple"
   ruling meets the owner's "it has to feel satisfying" ruling. That is a
   real result, and the plan should say in advance that it is one.
6. **Blocked on another seat.** H1's proper fix is the enemy seat's slot.
   Use the strip-and-exempt stopgap to prototype in parallel, and land
   nothing that depends on it.

## Changes required before an implementation plan

1. **Re-scope build step 2.** "Bind keys" becomes "make the player's body
   steerable on a *mind* ant". Use `shared/steer-mind.md`, or as a stopgap
   strip her mind and exempt her from the `NEW_ANT_SWEEP` re-gift
   (`src/lab/mod.rs:1089-1105`, `:1951`). Use the queen seat's key layer and
   `SteerRole::Worker` for the regime.
2. **Build one per-ant force map** (`Toward(point)`, `Follow(id)`) on the
   mind's pick-override slot (`mind.rs:2395-2404` precedent). Follow, "knows
   P", arm B's column and any later rival all use it. Share it with the
   enemy seat's mind-steer work rather than building it twice.
3. **Give the scripted follower a stop-start rhythm** (pause k every m),
   with the break caused by the leader walking on, and tie Pick a pupil to
   the `pace` gene.
4. **Make the slice playable and blind** (`demo=scout arm=` with shuffled
   labels), with GIFs as a supplement. **Cut** the rival, the shared smell,
   Display, carrying, Assess, Head-butt, Warn and Lay from the first slice.
   Put *c* and a half-*c* run in the meta.
5. **Write `demo_scout.ron` from a saved, established mind colony** that
   survives the opening. Measure alive-at-start and alive-at-end and the
   1x rate first. Do not use `far_larder` or a fresh landing (H4).
6. **Relabel the evidence.** The slice-0 trail null, the 442/298/323
   recruit harm, §7's trail-B lay rule and the forage drive are all the
   hand-built ant's. On the mind ant, all are untested (H5, H8). Write the
   demo's accept rule against mind-visible state (bank, crop). Mark the
   fifth-hour lineage claims as dead in the lab until minds mutate at birth.
   Fix the stale line references: `creature.rs:23030-23035`, `:23592`.
7. **Record what the demo cannot prove.** It does not show that the engine
   can produce a cascade. That is step 5's paired-seed gate, and it stays the
   seat's real risk.
