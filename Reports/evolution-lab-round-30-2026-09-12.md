# The evolution lab, round thirty: the screen gets out of the way, and two lanes discover they were both counting the wrong thing

*The coordinator's record of one round, 2026-09-12
(`session_01NEcvi6sUugSuvmitBeeM4V`), written at the round's close and kept
out of [`lanes/evolution-lab-coordinator.md`](lanes/evolution-lab-coordinator.md)
so the note stays under its 12 KB cap. Status: **record, not a work order.**
Every number here was taken by the lane it is credited to and is on `main` or
on the branch named; what binds from the round stays in the note.*

**Read this if you want to know what the round overturned.** Seven lanes ran,
five of them on work the owner asked for in his own words that morning. Two of
them arrived, from opposite ends and without coordinating, at the same
conclusion: the two mechanisms everyone assumed were shaping the played bed —
the colony's digging and the colony's fighting — are shaping almost nothing.

## State at open, and what the owner asked for

The round opened with the owner's own list, given in five messages across the
morning rather than as a brief:

1. *"The tooltip that shows the material, moisture level, if creature is
   present should only show up when the look tool is open."*
2. *"All the text in the top left of the screen should be removed except for
   ticks."*
3. *"There are lots of hidden menus that can only be accessed by knowing the F
   key. There should be a master menu accessible from the main UI that leads
   to all the other menus."* — later widened with *"The menu could be revamped
   too."*
4. *"when I zoom out all the way, instead of looking crisp, it looks like
   pixels of plants and other foreground things are disappearing."*
5. *"what if we explore making zooming in look better, smoothing or upsample
   the resolution, or brainstorm better options."*

And a soil question asked at length, whose operative sentences were *"I don't
want to end up with a bunch of individual pieces of soil floating in the
air"*, *"They create cool towers, but it also engulfs plants and creates a
ground that new plants dont grow in"*, and *"Lets really think think
through"*.

Two framing corrections from him during the round changed what lanes were
told, and both are worth keeping because in both cases the coordinator had
over-fitted:

- On the soil direction: *"I wasn't saying loose everything, I was saying more
  fixed, less loose"*, then, when that was over-corrected the other way, *"I
  don't know if we want more loose or more fixed, but I want the agent to
  explore the options and down stream effects and layed out the issues that I
  had and wanted solved."* **The brief he wanted was options with consequences,
  not a direction.**
- On the pixel aesthetic: *"I am open to different visual styles. I don't know
  if I love the pixel aesthetic, even given the pixel simulation."* This
  withdrew a constraint the coordinator had imposed on two lanes unprompted.
- And on turning his preferences into rules: of the ant towers, *"This doesn't
  have to be a rule. It is a 'I like them, but don't know if they are worth the
  problem they cause. Consider'"*.

## Environment, learned this round — read before spawning anything

### The environment is inherited. The repository is not.

**This cost two lanes and it is one omitted argument.** Both were spawned
within three minutes of each other and both came up with an empty
`/home/user` — no clone, no `CLAUDE.md`, no push. One reported
`need_input: "no repository found"` and stopped. The other cloned the public
tree read-only through the git proxy, worked for seven minutes, and only then
discovered it could never push:

```
access denied by the git proxy: ... is not in this session's authorized
repository set                                                   (HTTP 403)
```

`create_session` with no `source_url` **does** inherit the caller's
`environment_id`, correctly, every time. It does **not** inherit the caller's
sources. Same coordinator, same minute, same environment:

```
working:  "sources":[{"git_repository":{"url":".../Pixel_Physics","revision":"main"}}]
dead:     (no `sources` key at all)
```

So "it inherits the environment" is true and is not the thing you needed.
**Pass `source_url` and `source_revision` on every build-lane spawn, then read
`sources` back out of the record the call returns.** The tool hands it to you,
so the check is free and it runs before the lane has burned anything.

The lane cannot recover itself: `add_repo` with `access: "push"` is declined by
the permission classifier because it needs a human, and a lane spawned without
sources also has no `mcp__github__*` tools. Respawn and archive the original.

**The sharpest part of this failure is that the coordinator had written the
correct recipe into its own check-in fourteen minutes earlier and then did not
follow it.** Prose addressed to yourself does not survive. The check that
works is reading a value back out of a result.

Written up in [`session-programs.md`](session-programs.md).

### A lane being healthy is not evidence its work is durable

A lane three hours and $7.85 into a working build had **never pushed a
branch**. `git ls-remote` for it returned nothing. Context size, running cost,
task summary, a posted review card and two progress reports — every one of
those is a signal from inside the container, and **not one of them says the
work has left it**. The same coordinator had written that morning that
relaunching from the branch would beat waiting out a stall. There was no
branch.

**Run `git ls-remote origin 'refs/heads/claude/*'` for every live lane at every
check-in.** It fired twice in this round; both lanes pushed on request and both
survived.

### And a lane having delivered is not evidence it closed cleanly

The mirror, and the owner found it rather than the coordinator. The soil design
lane delivered its report, posted its card, then hit a five-hour limit at 05:52
and stopped. The limit reset at 07:10 and nobody woke it; it sat idle eight and
a half hours while the coordinator's notes recorded it as "closed".

Nothing was lost, because its report was already merged and the only remaining
work was acting on a verdict that had not arrived. That was luck. **Read
`status_bucket` and `post_turn_summary` for every lane the round has launched,
not only the ones still expected to report.**

### A same-file gate is a hypothesis, and one command tests it

**Three times in one morning** work was serialised because two lanes were "in
the same file" — twice by this coordinator, once by round 29 advising it. The
test is `git diff origin/main...<branch> -- <file> | grep '^@@'`:

| gate | claimed | measured |
|---|---|---|
| UI lane held for the chronicle lane, both in `ui.rs` | conflict | hunks at 49 / 6053 / 6074 / 9300 against targets near 855 / 3050 / 8068 — **disjoint** |
| ant lane held for two creature lanes | conflict | nearest approach **889 lines** |
| ant lane vs the who-kills lane | — | **21 lines** in `world.rs`, the one that was genuinely close, and it still merged |

**What predicts a painful merge is diff size, not file identity.** The lane
genuinely paying repeated conflicts in `creature.rs` had 174 lines across
eleven sites; the lane held out of the same file had a few at one.

Two riders. **A semantic gate is real where a textual one is not** — two lanes
changing what one shared input *means* must be serialised even though they
never touch the same line, and no diff finds that. And **a CI verdict quoted to
a lane has a shelf life of one push**: "9/9 green" went into a brief and was
stale within minutes.

### A partly-green PR carries almost no information, and I reported one as on track

**Read as "7 of 9 green, two long jobs outstanding" and relayed to the owner
that way. The two outstanding jobs were the only ones that could fail.** Run
34705723182 on lane P's `0953dffe` finished with `cargo test (debug)` red at
exit 101, and the lane caught it, not the coordinator.

The reason the reading was worthless is structural rather than unlucky. This
repo's nine checks split into two groups by cost: `branches`, `clippy`,
`docscheck` and `fmt` finish inside a minute, and **a lane that ran its own
gates locally has already cleared exactly those**. What is left running is
`cargo test` release and debug, `ascii`, `acceptance` and `worldgen` — the
slow ones, and the only ones with any chance of carrying a surprise. So the
green fraction climbs to about 4 of 9 for free on every PR and says nothing.

**A PR is green or it is unknown. There is no third state, and a count is not
a forecast.** The sibling rule below is about a green PR that still cannot
merge; this one is about a PR that is not green yet being described as though
it were nearly there.

### The second zero-conflict merge of the day, and this one broke the build

`lab::params::tests::no_page_is_longer_than_two_screens` caps a panel page at
20 rows. The ANTS page stood at **19** after round 29's landings; lane P's two
new dial rows made **21**.

**Neither side was wrong and the two never touched the same line.** Round 29
added rows, lane P added rows, `git merge-tree` reported no conflict, and the
sum went over a bar that neither branch could see from where it stood. The
test fires on whoever lands last, which is arbitrary.

This is the case `CLAUDE.md` already names — *two merges scoring 132 and 96,
comfortably "safe", were zero-conflict by `git merge-tree` and still broke the
tree* — and **it happened twice in one day** on this round. A conflict count
predicts whether a merge will be laborious. It cannot predict whether a merge
will be *wrong*, and the failure mode is specifically a **shared budget**: a
row cap, a pixel width, a byte cap on a lane note. Every one of those is a
resource two branches spend independently.

**The remedy is a full local suite after the merge, not before it.** Lane P's
`cargo test --lib` ran before bringing main in; afterwards it ran only the
filtered guards for its own tests and let CI carry the rest, which is what its
brief told it to do. That is a reasonable rule and **it is wrong across a
merge that spends a shared budget**. It fixed this by dropping a row rather
than raising the cap, on the argument that an ablation switch is not a
constant the *expose every constant* ruling covers — the right call, and the
next lane to add an ANTS row now has to move something first.

### An instruction at the point of use was already there, and was still missed

Lane P hit a merge conflict in the register's generated index, resolved it
correctly — take either side whole, re-run `bugindex.py` — and reported that
it had got it right *by luck rather than by knowing*, suggesting the rule be
put in the register's header.

**It is already in the register's header, in bold, immediately above the
table**, and has been. So the cheap fix does not exist: the instruction is at
the point of use and a careful lane still did not read it, because a merge
shows you a conflicted hunk rather than the top of the file, and the block's
header can be a hundred lines above the markers.

**What actually protects that file is `bugindex.py --check` in `docscheck`**,
which fails on a stale index and names the command. Lane P's docscheck was
clean, which is how we know its resolution was right. That is this file's own
removal criterion running forwards: *machinery now enforces it, so the prose
is a pointer at best*. Do not add a third copy of the sentence.

### Green CI is not mergeability, and they are checked separately

`#359` was 9 of 9 green and GitHub refused it. The checks had run against a
trunk three merges old. **Before quoting "green, ready to merge", the second
half of that claim is `mergeable_state`.** The cheap follow-up is to
test-merge in a throwaway worktree and hand the lane the exact conflict rather
than the news that it has one.

### `review.py sync` pushes new cards only

A card whose last line had lost two key names to a shell substitution was
edited and synced; sync reported success with `"pushed": []` and the remote
kept the broken text. **An amendment needs a manual push of the `review-queue`
branch.** Never amend a card that has been answered — the verdict is stored
against it, so editing afterwards points the owner's words at content he never
saw. Amend for a defect in the writing; repost for a defect in the artifact.
Written up in [`../.claude/skills/review/SKILL.md`](../.claude/skills/review/SKILL.md).

### Which bug letter is free is a question about every branch, and the command we told people to run could not answer it

Lane P filed **§Z16**. The register's highest was §Z15, `bugindex.py --check`
was green, and §Z16 was already live on another unlanded branch — so the
letter had to be renumbered after the PR was up. **`--check` reads one working
tree**, and `CLAUDE.md` sent the lane to exactly that command, calling it the
thing to use instead of checking by eye.

The manual sweep that found the truth took a morning:

```
for b in $(git branch -r); do git show $b:Reports/open-bugs-handoff.md \
  | grep -oE '^### Z[0-9]+'; done
```

**Fixed as a command rather than as another rule** (6f9f7a52):
`python3 scripts/bugindex.py --branches` reads every fetched ref's copy of the
register and prints the next free number in each series — **§Z18 over 75 refs
in 1.2 s**, the same answer. `CLAUDE.md`'s instruction now names it and says
plainly that `--check` is a confident wrong answer for this question.

Three things the first version got wrong, each of which is a rule in this
repo already and was walked into anyway:

- **Reporting every cross-branch disagreement gave 8 collisions of which 7
  were superseded letters on branches that landed weeks ago.** The live §Z16
  was one line in a wall of noise — a number that is arithmetically correct
  and answers a different question.
- **The filter for that needs `--merged`, which a shallow clone cannot
  answer**: 7 refs of 75 came back merged and the other 68 read as unlanded
  whether they were or not. That is a confident wrong answer, not a degraded
  one, so the collision report now **suppresses itself and says why**. The
  next-free line needs no history and always runs.
- **A sweep that read nothing reports no collisions, which reads as a pass.**
  The branch count is printed unconditionally and an empty sweep exits 2.

It is deliberately **not** gated by `docscheck`: its answer moves with what
you have fetched, and a gate whose verdict depends on your clone teaches
people to ignore it. `--selftest` is the positive control, and it goes red
when the comparison is broken on purpose.

### Posting the review card caught two bugs that no gate would have

Lane R2's own report, and it is the sharpest evidence this round for the
`review.py` habit being a *test* rather than a courtesy. Building the card for
the master menu meant driving the real UI, and driving it found two things
`cargo test`, `clippy`, `ascii`, `acceptance` and `docscheck` were all green
through:

- **`examples/labui.rs`'s own navigation was broken at runtime.** Every
  "reach this panel" idiom assumed PLANTS/ANTS/BOX/PARAMS always had a bar
  chip. The harness that renders the pictures could not reach the pages.
- **The MENU page silently overran its row budget by 1 px**, because generic
  panel pages never call `fit_rows`.

Neither is visible to a gate: the first is in a harness nothing asserts
against, the second is one pixel. Both are obvious the moment a human looks
at the page. **The rule this supports is already in `CLAUDE.md` — post rather
than describe — and what this adds is that the cost of posting is negative.**
It is not a tax on finishing; it is the only thing that ran the code the way
the owner will.

## Lane S — the thin things stop disappearing at full zoom-out (#345, merged)

The owner's complaint 4. Diagnosed from source before the lane ran: zoom-out
was point-sampling with a stride, so at the widest setting one cell in sixteen
decided the pixel and a one-cell twig had a fifteen-in-sixteen chance of
vanishing. **134 of 512 columns had lost their plant entirely.** Shipped as a
coverage filter behind a selector, with the arms named on screen.

## Lane T — what zooming *in* should look like (#344, merged)

The owner's complaint 5, as an exploration rather than a build: six candidate
looks over one frozen frame, costed per pixel, put to him as a card. His
verdicts picked the direction the next lane built, and rejected two by name —
material texture *"bad"*, and smoothing *"Looks blurry; very very sudtle, I
like it but probably not worth extra cost?"*, which is a rejection on cost by
his own words.

## Lane U — two soils (#346, merged)

The soil question, answered as a design report with `examples/soilfork.rs` and
no material changes: **five files, zero behaviour.** Deliberately so — the
owner asked for options and downstream effects, not a direction.

**Its card was rejected on its premise** (see the verdicts below), and **the
lane itself died on a rate limit before it could act on that.** Its report is
on `main`, so the reasoning is durable and a fresh lane can start from it.

## Lane V — the look he picked, as a mode (#352, merged a1d67819)

`render::MagnifyStyle`: five looks behind one key, today's the default and
byte-identical to it, ink weight and the corner rule as dials. The owner had
asked to combine two of lane T's arms — *"Could we maybe combine C and D? I
like D the best ... but D/B look blurry. Maybe adding C or something else could
crisp it?"* — and that combination is what shipped.

**The gate that decided it was a one-cell twig's survival to the screen**, and
it produced two method findings:

- **3x is a blind zoom.** The middle pixel of an odd block sits exactly on its
  cell centre, so plain bilinear hands a twig one full-strength pixel free:
  peak reads 1.00 while a tenth of the twig is gone. The coordinator's brief
  had specified 3x only. **A magnification gate reads 2x and 4x, or it reads
  nothing.**
- **The chamfer was silently erasing thin things** — 0.03 of a twig at 2x —
  by cutting all four corners of a cell with no orthogonal neighbour, which at
  2x is the whole block. Fixed; 1.000 at every zoom after. Nothing but that
  gate would have caught it: it looks fine on a still of a thick object.

Cost +25.2 ns/px for painted-plus-ink, **under** the 48 ns lane T had estimated
for painted alone, because the neighbourhood is hoisted per cell rather than
per pixel — lane T's figures were an upper bound on an implementation, not a
forecast. **0 settled pixels recomputed under every style**, so the dirty-rect
render skip holds.

It also caught a disconnected knob in its own instrument: `notch=` was parsed,
echoed on the parameter line, and never applied. Reconnected, with the positive
control it had been missing.

## Lane R1 — the box stops talking over the bed (#355, merged 1417f725)

The owner's complaints 1 and 2. The hover readout now draws only under the Look
tool; a pinned cell page is left alone, because the player asked for that one.
The Look tool's own help had promised the readout was always on *"tool or no
tool"*, and that sentence changed with the code.

The top-left corner went from up to six lines to two. Removed: the run state,
the asked-versus-got rate, simulated time per real second, ticks per frame, and
the motion-versus-fast-forward line — all of it either duplicated on the bar
and window title or commentary derived from those.

**It left exactly one thing, and left it deliberately**: `MIN {}HZ`, the
display floor that `F` cycles, was the only readout of that value anywhere. It
searched the bar, the help page and the window title before concluding that.
Deleting it would have stranded a control the player can change and then never
see. It flagged it for the menu lane instead — and the owner has since asked
for it to go, which the menu makes possible.

Its guard draws a real frame under the Look tool and six others and checks the
docked pixel, verified sensitive by removing the gate and watching it fail.

## Lane R2 — the lab stops hiding its pages (open at close)

The owner's complaint 3, plus the revamp licence. **The bar was measured full,
twice, before this round**, and the lane re-measured it: row 0 and row 1 both
at exactly 508 of 508 px with zero slack, and only at the tightest of three
spacings.

Shipped: a MENU chip replaces three pure-navigation chips, and PARAMS moves
onto the menu. **Both rows now fit at all three spacings including the loosest**
— real headroom for the first time since the shelf chip landed. Buttons 29 to
26. The jar chip stays, on the correct distinction that it carries live state
the pure-nav chips never did.

Costed and rejected: bare-adding a MENU chip and keeping everything. It passes
the guard, but only at the single tightest spacing with zero slack on both
rows. Both options are in `dead-ends.md` with their numbers.

The menu carries every page, the view toggles, the five magnify dials from
#352, and R1's display floor — which is what lets R1's corner line finally go.

**It watched the bar guard go red with a deliberately oversized label before
citing its green.** Most lanes skip that.

## Lane P — the anthill, and the variable nobody was measuring (#359)

**The gate does not shrink the mound. It keeps the colony alive on five beds
of twelve where today's behaviour leaves none.**

| colony alive at frame 300,000 | |
|---|---|
| crowding at the door (today) | **0 of 12** |
| room at the door | **5 of 12** — 427, 222, 181, 178 and 46 ants |

Five discordant pairs, all one way: **p ≈ 0.03** one-sided by McNemar, which is
0.5⁵. Every control colony is extinct. The sweep this lane ran all round
tracked cells dug, spoil, chambers and bare share, and **did not track whether
anything was still alive** — so the finding came out of a re-read of the same
twelve paired runs, not a new one.

It also rescues the owner's blind verdict from the obvious objection, which was
the coordinator's. He was shown one bed where the control had died and the room
arm had not, so *"A is bad, B is good"* could have been that bed's luck. It is
5 of 12 against 0 of 12.

Two honesties the lane put in the record rather than smoothing away. **The bed
pays for a living colony**: where the room arm survives, the seed bank and the
standing stand come out lower and starvation deaths are far higher, because
there are ants alive to starve — though pooled over twelve neither moves
reliably, at 7 of 12 each way. And **the mechanism is unmeasured**: whether a
colony that stops digging when it has room spends the saving on foraging is a
plausible story and nothing here tested it.

The rest of the lane's account stands, and is what it was briefed to find.

Briefed to repair a dig gate whose input was believed pinned at its ceiling —
a claim resting on a 2026-09-02 census and restated in `dead-ends.md`. The lane
measured it on this bed for the first time and **the premise is false**.

Pooled over 12 seeds at 300,000 frames, n = 835,536 at-nest ticks, buckets from
0.0:

```
271,185 / 85,255 / 128,946 / 60,817 / 0 / 54,015 / 36,760 / 33,166 / 30,344 / 135,048
```

A third of every read in the bottom tenth and a sixth at the ceiling. **An ant
at its own door is very often standing alone.**

**The zero is not an artifact and the reason is worth keeping.** `CROWDING_SCALE`
is 8.0, so the input can only take k/8, and bucket 4 is the one decile that
eighths cannot reach. The hole is exactly where quantisation requires it and
nowhere else — a miswired probe could not have produced that. **An exact zero
is the tidiness tell unless the quantity is quantised and the zero sits where
the quantisation forbids a value.**

**And then the round's best method finding, which only a lane measuring across
a whole round of landings could have produced.** It ran the identical 12-seed
paired sweep three times, on three trunks of the same day:

| trunk | cells dug, room against crowding |
|---|---|
| before #343 | a coin flip — 6 of 12 each way |
| before #354 | a lean — 9 of 12 mounds bigger |
| after #354 (today) | one-directional — **11 of 12 digging more, p ≈ 0.006** |

**A sweep measures one trunk, not a mechanism.** A lane measuring across a
round of landings has to re-take it after each one rather than average them,
and a sweep quoted without the trunk it was taken on is a number about a world
nobody has any more. Final figures: cells dug median 1.90x (room lower on 1 of
12), cemented spoil median 2.16x (lower on 3 of 12).

**The premise held on all three trunks, which is the only reason to trust it** —
the bottom tenth of at-nest crowding reads 24.8%, then 32.5%, then 37.5%. Three
worlds, one answer.

**It shipped off on the numbers and then on, when the owner picked it by eye.**
That sequence is the point. It first went out switched off against the standing
"ship new behaviours on by default" ruling, flagged rather than buried, because
it costs +4.0% of a frame and builds a *bigger* mound. The owner's blind verdict
then chose the room arm, and the lane flipped the default **without softening a
single number** — the sweep it reports under that decision is 11 of 12 rather
than the 9 of 12 it had shipped off on, which is a worse result for the gate.
He chose it, the counters say what they say, it is a dial, and all three are in
the record.

The coordinator had told the lane to hold the default pending a survival
measurement, on the grounds that the card's two panes differed by a colony that
lived and one that died. **That was the wrong call**: it weighted a real
confound above the owner's eye, the standing ruling, and the fact that the goal
the gate was failing had been retired by the lane's own finding.

**The instruction was wrong and the question underneath it was the round's best
one**, and both halves are worth keeping because they came apart. Asking what
the owner might have been looking at is what produced the survival re-read; the
answer made the hold unnecessary and the difference the coordinator called a
confound turned out to be the signal. The lesson is not "trust the eye and stop
asking" — it is that **the question a doubt points at should be measured, and
the doubt should not gate the ship while you measure it**.

Two more traps it paid and recorded: a single seed read first gave a tidy
"+27% digs, +13% mound" that twelve seeds erased; and it had *already seen* the
two roofed rules disagree (422 against 289 on one world) and filed it mentally
as a definitional difference rather than chasing it — the drift was real, the
room datum was freezing lazily at the colony's arrival, and it is now frozen in
`begin_step` with the two rules asserted equal on the selftest and agreeing to
a median 0.97 on the real bed.

It also found [§Z17](open-bugs-handoff.md): `World::ground_datum` is built and
wrong inside a sealed lab box.

## What the round overturned

**The two mechanisms everyone assumed were shaping the played bed are shaping
almost nothing, and two lanes found it independently from opposite ends.**

Round 29's who-kills lane asked who is killing the colony. The answer is
**nobody**: `DeathCause::Killed` is booked whenever a creature's deciding cell
goes away, whatever took it. Over three seeds at 500,000 frames, **216 / 98 /
70 killings booked and 2 / 0 / 0 attributable to an attacker.** About two fifths
of the rest are a plant standing in the dead ant's head. The colony is being
overgrown, not fought.

This round's anthill lane asked whether a better dig question shrinks the
mound. It does not, and the question it was replacing was never being asked
badly in the first place.

**Both lanes measured the wrong variable, and the round only found that out on
its last afternoon.** Round 29 counted killings and found nobody kills. This
round counted mound size and found the gate does not shrink it. Both numbers
are right. Neither is the variable that moves: **the colony's survival was in
no sweep either lane ran**, and when the anthill lane finally read it off the
runs it already had, the gate it had just declared a null keeps the colony
alive on **5 beds of 12 where the control leaves 0**.

So the corrected statement is narrower and more useful than "nothing matters".
Fighting does not shape the bed, and the dig question does not shape the
mound — **but the dig question decides whether there is a colony at all**, and
nothing in either brief asked. The open question is no longer whether the gate
earns its place; it is *by what route*, since whether a colony that stops
digging spends the saving on foraging is untested.

What also remains is the undiagnosed half of those deaths — something vacates
the vital cell and leaves nothing behind.

A correction worth recording because it arrived before the brief was written:
round 29's first reading was *"the repair is in the plant growth path"*. That
was withdrawn on inspection — `plant::growable` already refuses an occupied
creature cell. The suspects are three **in-place writes**, and the right first
step is a measurement, not a rule.

## The owner's verdicts (2026-09-12, after the round's cards)

- **The painted-plus-ink look, moving**: *"Need to playtest"*.
- **The chamfer, moving**: *"need to playtest"*. Neither is a rejection, and
  both already ship as modes he switches to — which is what the verdict asks
  for. In the lab there is still no route to them, which is what the menu lane
  is for.
- **The readout and the corner**: *"Much better, remove Min 10hz too"*. The
  sole-route exception can now go, because the menu re-homed it.
- **The soil futures**: *"These all look relatively similar. lots of small
  hunks floating in the air (bad). With a big pile on the left."* **This is a
  rejection of the card's premise rather than a choice among its options.** He
  could not tell the four apart, and the defect he reported — floating soil —
  is his original complaint, present in every pane and addressed by none of
  them. The lane asked the wrong question and the coordinator approved the
  card.
- **And he said it twice, about two unrelated cards.** On the anthill lane's
  superseded shot: *"Both look bad and have lots of stuff floating in the
  air."* **Two lanes, two subjects, the same complaint** — which makes floating
  debris a property of every picture of this bed rather than a question about
  soil design. It is the most consistently reported visual defect of the day
  and the top of the carry-forward below.
- **The anthill, blind**: *"A is bad. B is good"*, with `blind_was: [1, 0]`, so
  he preferred the new gate over today's behaviour. **The map was checked
  against a property only one arm has** — the pane he called bad carries 0 ants
  alive and 128 of 129 bare columns — because with two panes a reversed map is
  indistinguishable from a correct one.

  **But the verdict is true about a quantity the card did not ask.** Those
  panes differ by a colony that lived and a colony that died, on one bed. That
  is the single-seed artifact the lane itself had documented an hour earlier,
  and the coordinator reviewed the card without catching it. The sweep tracked
  digs, spoil, chambers and bare share — **not survival** — so whether the gate
  helps a colony live is unmeasured and open.

## What round 29 handed over at its close

Its own cards were answered the same afternoon, and two of the verdicts are
round 30's problem now:

- **The long-ant pile is not visibly fixed.** Round 29's traffic-deferral
  expiry improves the counters — wedged long bodies better on 7 of 9 seeds —
  and the owner's verdict on the card was *"both have lots of stuck, A looks
  worse"*, with **A the expiry arm**. **It ships on counters and fails the
  eye.** That is this repo's own standing rule landing on a shipped change:
  the owner's eye is the bar, counters are constraints. The causes it did not
  repair are §Z12 and §Z13.
- **A follow camera ruins a colony-level card.** Two cohesion cards came back
  *"shaking gif, probably following a creature"* — unreadable. Re-shoot from a
  fixed frame. Folded into the coordinator note's movement rule, beside this
  round's finding that a scrubbable frame sequence plays where a GIF did not.

Chosen and confirmed: the new flight reads as flying and uncaged, seed cargo
keeps the bed alive, and the 40,000-frame lifespan arm.

## Open at close

- **Floating debris is the most reported visual defect of the day** — filed
  by lane P as **§Z18** (dug spoil standing in open sky), named by the owner
  on two unrelated cards, and no lane owns the repair. It is not the soil
  design question it was filed under. **This is the first thing round 31
  should look at**, because it is in every picture of this bed regardless of
  what the picture is of.

  Note the letter: lane P filed it as §Z16 and had to renumber, because
  round 29's §Z16 was already on `main` and `bugindex.py --check` cannot see
  a letter claimed on another branch. Run `--branches` before filing.
- **By what route the room gate keeps a colony alive.** The survival result
  is measured (5 of 12 against 0 of 12, p ≈ 0.03); the mechanism is not.
  Whether a colony that stops digging when it has room spends the saving on
  foraging is a plausible story and nothing has tested it. **This is the
  successor to the anthill brief** and it is a measurement, not a build.
- **What a living colony costs the bed.** Where the room arm survives, the
  seed bank and the standing stand come out lower and starvation deaths are
  far higher — because there are ants alive to starve. Pooled over twelve
  neither moves reliably (7 of 12 each way), so this is a question, not a
  finding.
- **Lane R2's PR** — the menu, plus deleting R1's corner line on the owner's
  instruction.
- **Floating soil is the soil brief**, not weathering. The design report is on
  `main` and the lane that wrote it is gone; a fresh lane starts from the
  report.
- **The long-ant pile**, inherited from round 29 on the owner's verdict rather
  than on a measurement — its expiry landed and the pile still reads as stuck.
  §Z12 and §Z13 below are the two causes it left.
- **§Z13** — a resting ant reads as stuck at play zoom. A colour or a mark
  question, not a mechanic; the same shape as this repo's own note that a dead
  creature is unfindable by the very channel that makes a live one findable.
- **§Z12** — bred one-cell morphs that cannot flip. Inheritance, not injury.
- **§Z15 and §Z16 are probably one repair** — a living animal blocked by plant
  cells and a dead animal whose head became a pip are the same collision from
  two sides. Measurement first: an attack-in-progress bit on
  `World::note_vital_loss`, one run, which splits the undiagnosed column.
- **Splitting or renaming `DeathCause::Killed`** — round 30 took this from
  round 29's three follow-ups, on the test that two lanes re-derived the same
  wrong reading from it in one day.
- **§Z17** — `ground_datum` in a lab box. The repair is small; what the right
  answer *is* for a sealed box is deliberately left open, because a lid is
  genuinely a roof.
- **The water-level line** from lane T's exploration is unbuilt, and lane T
  recommended it whatever the style verdict.
- **The odour timescale question** — the owner's model is *"if he spends too
  much time away from home then he turns enemy"*. Measured: an ant's smell is
  set at birth, home resets it, and away from home it does not move. What does
  move is the nest's, about one tolerance radius per 120,000 frames. So the
  mechanic is half-built and running at session timescale rather than trip
  timescale. A costed repair sits unbuilt in #349 because it is his call.
