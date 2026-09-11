# The evolution lab — coordinator note

*`cargo run --release --bin lab` is the second game: a sealed box of soil under
grow lights where the shipped plants and ants live. Design of record:
[`../evolution-lab-design-guide-2026-08-30.md`](../evolution-lab-design-guide-2026-08-30.md),
with [`../evolution-lab-feasibility-2026-08-30.md`](../evolution-lab-feasibility-2026-08-30.md)
under it.*

**Read this before picking the lab up.** Twenty-six rounds have been run here
since 2026-08-30. One to twenty-five are history and moved (2026-09-08, 2026-09-09, 2026-09-10) to
[`../evolution-lab-rounds-archive.md`](../evolution-lab-rounds-archive.md) —
verbatim, with a priced contents table and a per-round map of which round belongs
to which of the three concurrent lines. What stays here is what still binds.

## What binds on anything you do here

**The owner's standing direction, which reframes the whole programme** (round
three, and it has not been superseded):

> *"Your goals are not tweaking and optimizing evolution now. **Give me the
> tools, data, access to the parameters that need to be tweaked and I do that
> testing myself in the game. That is the game.** If I have access to food,
> water, can cull, can create plants, and creatures, I can figure it out."*

**So: stop balancing, start exposing.** A default that looks wrong is something
to **register and report**, never to tune.

**Ship new behaviours as default** (round twenty). Reach is not behaviour:
nothing born swinging or listening is `CLAUDE.md`'s second law failing quietly.

**Read the bed at a session, not at a minute.** The owner's own framing — a
session is a few hundred thousand frames and a million is several sessions. At
that length **every shipped bed starves its ant colony**, on every seed, with
births plentiful: [`../open-bugs-handoff.md` §Z6](../open-bugs-handoff.md)
carries the table and the bar a fix has to clear. **§Z6 is upstream of the
castes question, the kin drift and Gate 2 alike** — a channel cannot be *found*
by a line that does not outlive a session; **round twenty-one answers its open
question.** Every creature result in archived
rounds twelve to eighteen was taken at a few minutes of play at most and is
true at that length only — **the owner's machine is the ruler, not this
container's**: a full box runs at 1–4x there and one plant at 40x, so 24,000
frames of the full box is two to seven minutes and 300,000 is a long session
(the container's 6x is where "one minute" came from, and it is wrong for the
owner). A session census costs six to eight minutes a bed on one core, so it
is the cheap default from here on, not the expensive exception. **And run it
on the played bed** (`ants_at=`, round twenty-five): the frame-0 bed is the
harness talking.

**Deliberately not being built yet:** the score and the economy, the guide's
Gate 5. **Gate 2 — does selection have teeth in *this* bed — passes for
creatures**, and this paragraph carried the opposite claim for weeks:
`creature_arena arm=lethal`, a zeroed brain against the shipped one, 24,000
frames, six seeds, on the harness default bed **and** a fed one, puts the
zeroed brain at **0.0% of animals on 12 of 12 seed-runs** — the harness prints
its own verdict, *the bed has teeth*. It is a maximal-effect test and licenses
only that: the bed turns a *large* fitness difference into a population
difference. It cannot resolve a small one, and the arena's own 2.42–3.12x seed
noise — not the ecology — is why the flight races nulled, so those nulls are a
statistical-power problem. **Gate 2 for plants is untouched**, a different
harness on a different kingdom, and `selection_arena`'s finding stands there:
a null is a statement about the world rather than about the genome.

**Re-derive file ownership from the open PR list, never from a table in a
note.** Round one's ownership table was landed whole by PR #170 and every later
round edited those files; `CLAUDE.md`'s own rule says the roster you were
handed is a claim about the past. `sim::frame::step` is the tick sequence
shared by both binaries, and its guard
`frame_step_matches_the_sequence_app_update_ran_before_extraction` holds a hash
taken from the other side of the extraction — **if it goes red either a phase
moved deliberately (re-take the number and say what moved) or a phase was added
to one binary's loop and not to `frame::step`**, which is the failure the module
exists to prevent.

**The perf line's handed-forward list** (archived round nineteen, and
[`../evolution-lab-frame-cost-2026-09-01.md`](../evolution-lab-frame-cost-2026-09-01.md)
§18.5, in order): the **~21% in the kernel and rayon**, the largest block left
by a wide margin; then the moisture pass, where what remains is per-cell
arithmetic; then the pheromone `roundf`, which is cheap and is **not**
behaviour-free. `step_organisms`' three pure levers were priced at under 1%
between them and are closed. **Rebuild the baseline binary after every merge** —
a hash gate is worthless against a stale one.

## Round twenty-five, 2026-09-09 — the instrument turned toward the player

*Verbatim in [`../evolution-lab-rounds-archive.md`](../evolution-lab-rounds-archive.md)
(moved 2026-09-10); its design of record is
[`../evolution-lab-direction-2026-09-09.md`](../evolution-lab-direction-2026-09-09.md).
What still binds from it is the rulings paragraph below; the two claims it
overturned (the frame-0 bed is the harness talking; the trail circuit is
wired and inert, §Z7) are in the archive, and its open list has been worked
by round twenty-six.*

**Owner's rulings, all the same day, all binding here:** trophallaxis is a
brain output the genome can evolve, shipped on, never a rule; **rest is the
absence of a reason to act, not the presence of a full stomach** — the ant
must not become an animal that only acts when hungry, so the bias comes
down and never off, and digging is conditioned on a crowded nest; a queen
is built as three authored values over mechanisms that exist (a founding
rule, a founder who rests because she is full, sterile workers through the
caste channel), **never as a type the engine knows**, and the eusociality
lane's first deliverable is the generation-clock measurement, not a
feature; staggered founder reserves are approved; the marker overlay was
rejected on sight; and **movement, not stills, is how animals are seen** —
a visibility claim is judged on a moving sequence, never a contact sheet.

## Round twenty-six, 2026-09-10 — the box gets its first relationship

*Coordinator `session_01NGdywxc1ACg3L5scK7xBTc`. The full record, with every
number, is [`../evolution-lab-round-26-2026-09-10.md`](../evolution-lab-round-26-2026-09-10.md);
the ecology design of record is
[`../evolution-lab-ecology-design-2026-09-10.md`](../evolution-lab-ecology-design-2026-09-10.md).
This is the pointer and what still binds.*

**The direction.** Phase one made the box legible; it still had no
*relationship* — one organism eating another was the entire ecology. Close
the loop fruit → animal → nest → seedling and the colony becomes the plants'
distribution network and the plants the colony's renewable larder: **the
colony that gardens survives.** Graded by construction, visible at play
zoom, and the patchy larder §Z7's recruitment finding said no bed had.

**Landed on `main` (#296, #297, #295, #298, #300, #301, #302, #304), in one
line each:** the ecology design (fruit is budget-limited, not
pollinator-limited; nothing has ever eaten a flower; `labshot` ignored
`seed=` in scenario mode); the fruit loop censused (2 of 3,089 germinations
from windfall; windfall arrived ownerless); the breeder index and the
recycled-slot fix (the breeding rule was blind to every animal in a reused
slot); rain, OFF by measurement, on key `8`; the severed fruit keeps its seed
(§Z8); the pip (a bitten fruit leaves its seed at 0.6 — and an ant bites a
fallen fruit about twice in 360,000 frames on the played bed); the thicket
bed (fruit on the floor 9.6x, bites 5 → 32 across the sweep, no pip yet a
plant); the chronicle exported and a HISTORY page on F5.

**Landed 2026-09-11** on the owner's ruling: *"Go with A, but the long ant
should be an option that I can place."* PR #303/#311/#315/#316 land with
the shipped ant kept at `Chain(2)` — both reds named above were about the
body, not the tuck/founding/flip mechanics, and both clear once it
reverts — and the seven-cell body moves onto `longant`
(`assets/species/longant.ron`, `assets/lab_scenarios/played_bed_longant.
ron`), a species placed rather than started with. Tuck and the on-by-
default flip ship for every body; founding stays spine-only for the
shipped ant. `longant`'s own costs (a 5-segment body seating 12 of 52
where the shipped ant seats 39; its own body opening the plate the swarm
test needs closed) are named, not chased. Report §13 on the branch has
the numbers; superseded there is "a width-2 colony cannot dig a roofed
chamber" — it was the body, and the shipped ant's chamber scene roofs
again.

**What binds from this round.** The breeding trade is **graded**, ruled in
chat and recorded in commit `e5792206`; **a question that needs no visual is
asked in chat, not the queue** — the owner's standing instruction. Ship
graded as the default only after `GRADED_MAX_SUPPRESSION` (a provisional
6.0) is swept; the breeder lookup now scales. The ecology's next lever is
the owner's to pick with the numbers in hand: fruit residence (expose the
windfall's rot half-life on the parameters page) or fruit production (B2,
pollination as a ripening-price discount) — not A2 (the seed rides home),
which builds on an event that happens twice a session. A fourth
windfall-ownership path (19 ownerless on one scrambler seed) is open and
unowned. Open cards: pollen as gene flow (`…6dfed9`, the design recommends
the player's BRUSH first), the dispersal form (`…cba50c`), the thicket in
the default bed (`…c8709b`), the rain rate (`…f2fb5b`), the HISTORY page
(`…c0b68b`), the moving bodies (`…0180fc`), and from round twenty-five the
tree, the marks and *is the box empty*.

**Offered and not started:** BRUSH (pollinate by hand); sound (needs a
ruling on the dependency); a nectar-feeding hopper up the stems; palatability
co-evolution in colour; the bed as a record; wild collection.

**Environment, learned this round:** a poke's fire response names where it
landed (`cse_<lane id>`); a trigger's prompt cannot be edited once bound, so
each message is a new trigger, deleted after it fires; **a sub-agent that
ends its turn to wait for a build never resumes** — one hung two hours with
861 lines uncommitted and was salvaged by committing its worktree and
killing it; the container suspends while the coordinator idles, so keep a
check-in armed.

## The earlier rounds

All twenty-five are verbatim in
[`../evolution-lab-rounds-archive.md`](../evolution-lab-rounds-archive.md),
which prices each one and maps it to its owning report. **Read the one round,
not the file** — they are three concurrent lines braided into one sequence, and
knowing which is yours is most of the saving:

| line | rounds | design of record |
|---|---|---|
| the lab as an instrument — interface, shelf, rosters, persistence, soil, scenarios, forage | 3, 4, 5, 7, 9, 10, 11, 21, 25 | `evolution-lab-gui-physics-2026-08-30.md` |
| frame cost and the speed dial | 2, 6, 8, 17, 18, 19 | `evolution-lab-frame-cost-2026-09-01.md` |
| creatures — groups, kin, armour, castes, verbs, gates | 12, 13, 14, 15, 16, 20, 22, 23, 24, 25 | `creature-signature-and-castes-2026-09-06.md` |

## Environment notes that cost time here

- **The container suspends between tool calls**, so a backgrounded job makes
  no progress while the session is idle. `cargo test --lib` does not fit in
  one foreground call. **Let CI be the gate on the full suite** — it runs on
  branch pushes as well as pull requests.
- **Every push cancels the in-flight suite and restarts a ~19-minute clock**,
  so batch commits. A run whose jobs all read "cancelled" two seconds in is
  the concurrency group superseding a push run with a pull_request run, not a
  failure.
- `rust-toolchain.toml` pins 1.98 and CI has a build cache, so plain
  `cargo clippy --all-targets --release --locked -- -D warnings` matches CI.
- **The lab window captures with no display** —
  `PIXEL_PHYSICS_SCREENSHOT_AFTER_FRAMES=N` under `xvfb-run` with lavapipe.
  `labshot` renders the *world* and shows no interface; `examples/labui.rs`
  renders the bar headlessly and scripts clicks.
- **`/tmp` is shared between agents in this container** and the screenshot
  hook writes `$TMPDIR/pixel_physics_lab.png` — one lane captured another
  lane's frame. **Set a private `TMPDIR`.**
- **`review.py inbox --mark-seen` marked all 199 cards seen**, not just the
  caller's. Use `get <id>` rather than trusting an empty inbox.
- **Several agents in one container makes every timing untrustworthy** — two
  byte-identical `ascii` runs have disagreed 2.42x here. Pin
  `RAYON_NUM_THREADS` or compare arms inside one run; the general rule is in
  `CLAUDE.md`.
- **Cloud lanes cannot be messaged.** `SendMessage` does not resolve a
  `create_session` child, so a brief cannot be narrowed once it is running.
  Write briefs that degrade well: say what to land first and to report the
  rest.
