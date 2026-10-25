# The colony mind: one seat's game design

*2026-10-25, revised the same day after an independent review (its items and
what changed are listed at the end of "Risks and open questions"). Game design
research only: no code changed, no runs. A deeper version of the sketch in
`perspectives.md` §2. Marks: [S] checked this session (from search summaries
and abstracts, not full papers; URLs in Sources), [K] from memory and
unchecked, [E] checked in the repo with file:line. **Numbers in the play scenes
("+6 out", cohort sizes) are illustrative, not measured.***

---

## The pitch

**You are not an ant and not a hand. You are how loud the colony's own signals are.**
You cannot make a signal from nothing. You can only swell or hush the ones the
ants are already giving: a forager coming home full, a larva begging, a bitten
ant's alarm, food passed mouth to mouth.

**The fantasy is being the hum inside the soil.** Press on a forager as she
comes through the door and a ring of news spreads from her; waiting ants turn
and file out. Pinch out an alarm and the nest keeps working while a beetle takes
one sister at the surface. Every swell costs the colony stored food, because
signals are expensive. So a starving colony has a quiet voice.

**The answer in one paragraph: what you amplify is what you breed.** Every ant
that answers a swelled signal is remembered, and the lines that answer get to
lay. The fast verb (seconds) and the slow verb (generations) are one gesture.
Over generations the colony needs fewer meetings to stir for the signals you
kept swelling. Real harvester colonies each set their own bar for how many
meetings send a forager out, and that bar is heritable [S: Gordon 2013]. You
see the lesson become instinct as a falling number. You never give an order.
You decide which of the colony's own voices win.

**Be clear about what exists.** On today's engine only the alarm verbs, a hush,
and a hook on the store are close to working. The anchor verb, swelling the
news, needs a recruit path for ants that have never foraged, and that path
(`PIXEL_PHYSICS_RECRUIT`) is off for a measured harm. Everything that makes it a
*teaching* seat (favour, policing, habits, the heritable bar) is new work.

**Why it might beat the queen and the keeper.** Its verbs scale with the
colony: one gesture moves dozens of ants. It acts on colony-level decisions,
where the main plan's review placed the unexpected (critique §5.8). And real
colonies do run on feedback loops; you are the gain on them.

**Why it might not.** It has no body, so no stake. `perspectives.md` §6 scored
it natural feel 1 of 5 and minute-to-minute fun 2 of 5. Its strongest use is
**the late-game seat of the life-cycle arc** (queen, then scout, then mind), not
the whole game. The test is a faked slice beside the other two seats.

---

## The biology it rests on

**There is no colony mind.** Hölldobler and Wilson's superorganism is a colony
working "as a single animal raised to a higher level", with no brain in it [S:
book reviews]. The accepted explanation is self-organisation: feedback from
local contacts, no overseer [S]. One 2019 paper argues even that should be
re-tested, not assumed [S]. **So the seat is a fiction, and should say so.** What
the player embodies is real: the strength of the feedback loops.

**Response thresholds make roles without castes.** In Bonabeau, Theraulaz and
Deneubourg's model (1996), each worker has a threshold for each task's stimulus.
Past it, she probably does the task, and doing it lowers the stimulus [S].
Low-threshold workers do most of the work; the rest join when the need is loud.
**This fits the owner's rule: one brain design, different numbers, roles emerge.**

**Thresholds that learn make specialists from clones.** In the 1998 version,
doing a task lowers your threshold for it and idling raises it; identical
workers split into specialists [S]. Remove the specialists and return them later,
and the colony does not regain its old state [S]. In the clonal raider ant,
identical workers made to succeed at foraging kept exploring a month later;
those made to fail moved to brood care [S: Ravary et al. 2007]. That took weeks.

**Variety in thresholds may steady the colony.** Honeybee colonies sired by many
fathers held brood temperature steadier, credited to spread in workers'
temperature thresholds [S: Jones et al. 2004]. A study of naturally mated
colonies found no effect [S]. A design hypothesis, not a law.

**The colony-level signals that really exist:**

| Signal | What is real | Source |
|---|---|---|
| **Meetings with returners** | Red harvester foragers inside the nest go out when they meet returning foragers carrying both forager odour and seed odour; either odour alone did nothing | [S: Greene, Pinter-Wollman & Gordon 2013] |
| **Each colony its own bar, inherited** | Colonies differ in how many meetings send a forager out; colonies that held back outside good conditions had more offspring colonies; the response is heritable from parent to daughter colony | [S: Gordon 2013, *Nature* 498:91] |
| **Colony personality** | Colonies differ consistently in speed and responsiveness across situations | [S: Pinter-Wollman et al. 2012] |
| **The shared stomach** | In *Camponotus*, foraging falls roughly linearly as the colony's crops fill, and foragers unload more slowly to fuller nestmates | [S: Greenwald et al. 2018] |
| **Hunger spreads wider when starved** | A search summary of a *Lasius niger* study said a starved donor spreads her crop to about 100 workers. The reviewer could not find this in the paper (subcolonies of 50+). Treat as unconfirmed | [S, summary only] |
| **Brood demand** | Larval odour alone raised protein foraging in *Temnothorax*; larvae raised foraging in the clonal raider ant; in *Formica* the larval hunger signal varies by species | [S] |
| **Queen pheromone** | A conserved class of non-volatile hydrocarbons stops workers' ovaries developing, across ants, wasps and bees | [S: Van Oystaeyen et al. 2014] |
| **Worker policing** | Workers eat eggs laid by other workers in many species | [K] |
| **Nest temperature** | *Camponotus mus* nurses move brood to a warmer spot each afternoon and back about eight hours later, on a daily rhythm | [S: Falibene et al. 2016] |
| **Colony memory in marks** | *Temnothorax* colonies avoided a poor nest they had inspected; with the marks removed, the choice became random | [K: Franks et al. 2007, *Proc R Soc B*; the 2011 Stroeymeyt paper is on informed individuals] |

**The liberties this seat takes, stated plainly:**

1. **A mind exists.** It does not. We draw the feedback gain as a presence.
2. **Signals spend a shared pool.** Real signals cost the signaller; there is no
   colony budget. The pool makes loudness a choice.
3. **The mind decides who lays.** Real reproduction is set by queen pheromone,
   dominance and policing. In queenright colonies workers can usually lay only
   sons [K]. "Lines that answer get to lay" is a liberty.
4. **Time is squeezed.** Threshold learning that took weeks happens within one
   ant's life here.

---

## Your verbs

Every verb is a gesture on a living ant or a place: press, hold, stroke, pinch.
**Pressure and hold time set the gain**, so every verb is graded. Every swell
costs food from the store. Codes: **now** = the ant's sense and response exist
today and the verb needs only a small hook; **mem** = needs plan §7's memory
layer; **new** = new engine behaviour. Where "now" depends on a response nobody
has measured, the cell says so.

| Verb | What you do | What the ant perceives | What the player sees | Works today? |
|---|---|---|---|---|
| **Swell the news** | Press and hold on a laden forager as she enters | Her "met a returning forager" stamp reaches up to 12 cells, not 3 [E creature.rs:25412, 26144]. Only ants that have foraged and are not nest workers read the drive [E creature.rs:26093-26103] | An amber ring leaves her; stirred ants file out; a count at the door | **new**: the mound's idlers are mostly never-foraged ants and nest workers, who read 0 [E creature.rs:26350-26364]. Needs a recruit path (RECRUIT, off for harm) |
| **Hush the door** | Draw a slow hand across the door | Deliveries there stamp nobody for a while; drives fade on each ant's clock | Door traffic falls over a minute; foragers drift back in | now (hook; reaches only ants that have foraged) |
| **Echo a cry** | Tap a bitten ant | Alarm laid over a wider patch, or decaying slower. Not a bigger deposit: a bite already lays 240 of 255 [E pheromone.rs:347] | A red bloom spreading; ants converge or scatter | now (hook; whether today's pool mobs or flees is unmeasured) |
| **Swallow a cry** | Pinch the red out at a spot | Less alarm laid there, or faster decay | The red fades; workers carry on through a raid | now (hook) |
| **Make the brood beg** | Stroke a clutch of larvae | Its hunger scent is louder; nurses walk up it [E brood.rs:1391] | A pale pulse over the brood; nurses walk in | new (the nurse-scent rule is built and off; needs a gain) |
| **Open the shared stomach** | Sweep a chamber with an open palm | Ants there pass crop food to hungrier nestmates; energy is a context sense every ant reads [E mind.rs:73-74] | Amber flowing ant to ant in a crop view; hunger evens out | new (a gain on sharing; how strongly the pool weights energy is unmeasured) |
| **Close the shared stomach** | Sweep with a closed fist | Ants there keep what they hold | A few stay fat, others go lean; lean ants go out on their own hunger (the lean rule) [E how-the-ant-works.md §6d] | new (small) |
| **Forbid the carrion** | Hold over a corpse | Nobody eats it for a while. Corpses are food with a worth [E creature.rs:1961] | A body left lying; it rots to soil; ants route round it, or not | now (hook; avoidance depends on the `dead` feature's weight, unmeasured) |
| **Shift the nursery** | Trace a warm line through the nest | Warmth is a sensed feature [E options.rs:279] | A column of nurses carrying brood to the new spot | new: brood carry is off [E brood.rs:454-466]; eggs are placed on the pile when laid [E brood.rs:367-373]; the pile choice has no warmth term |
| **Spend the reserve** | Squeeze a store pile | Nest workers fill crops at the store and feed larvae (an existing store rule) | Store food shrinks; larvae fatten; a bigger cohort later | new (hook on an existing rule) |
| **Lean on a habit** | Tap an ant while it forages or nurses | Its threshold for that task falls (see notes for what it acts on) | A task tint that deepens; it returns to the task unasked | new (this document's invention, not part of plan §7) |
| **Bless the answer** | After a swell, gather the glowing answerers and flick the glow onto the brood pile | Those lines gain favour; favour sets who may lay | Eggs flecked in the answerers' line colours | new |
| **Police an egg** | Flick an egg off the pile | Workers eat eggs of unfavoured lines | An egg carried off and eaten | new |
| **Call a quorum** | Tap two chambers when one floods or starves | Scouts inspect; past a head count, the colony moves | Ants carrying brood in a column to the new chamber | new (emigration) |
| **Step down into one ant** | Double-tap any ant | Nothing: you are her for a minute | The camera dives; you walk her (SimAnt's yellow ant) | new (a steerable body exists for a queen only) |

**The voice: what loudness costs.** Each second of swelling spends store food
(about one meal a second, to be tuned). The hum's brightness shows the pool. The
other cost is the colony's own: over-sent foragers outrun the food, starve on the
surface or meet beetles. That is the harm RECRUIT measured (adults starved 442
against 298 and 323). Here it is a choice the player makes, and it must be graded.
**Nurses cannot be pulled off the brood.** Nest workers never read the forage
drive [E how-the-ant-works.md §6d], so the first draft's "loud calls take the
nurses" cost was wrong.

**Swell the news is the anchor verb, and it is new work.** Ants who have foraged
and hold a fresh meeting are already at the drive's plateau; ants who never
foraged read 0. A wider ring mostly reaches ants that cannot answer. The closest
built relative is `PIXEL_PHYSICS_MEET_WAY`: carriers stamp the ants they pass on
the way home, off [E creature.rs:26159-26180]. Its trace found the mound's
loafers stand 5 to 17 rows above where deliveries land, so even a 12-cell ring
may miss them. It was never run as its own arm
[E Reports/brakes-and-nest-life-2026-10-08-13.md:298]. The swell must sit on a
recruit path that reaches newcomers, and must beat RECRUIT's harm.

**What a habit acts on, concretely.** The ant picks a verb and target every tick,
so it has no "task" for a habit to point at. Two habits have a real lever:

- **Forage:** a per-ant multiplier on the forage drive it feels. A low threshold
  means fewer meetings stir it, and a stirred ant steps out sooner.
- **Nurse:** how long feeding a larva keeps an ant a nest worker. The engine has a
  built version: under `NURSE_STAY`, a nest worker that feeds a larva stays one
  2,000 frames longer [E creature.rs:16812-16824]. It is off: with other switches
  on it measured far fewer live ants [E how-the-ant-works.md §12].

Dig and guard habits would be additive biases on the Dig and Attack scores.
They are a later step, with no built relative.

**Bless the answer is what makes it a teaching game, and it is the riskiest
gesture.** Drawn badly it is a selection menu with a flick. The alternative is no
gesture at all: answerers gain favour automatically, and the player selects only
by what they choose to swell. The faked slice should test both.

---

## How a lesson travels

| Channel | What carries the lesson | Clock | On today's ants | With new work |
|---|---|---|---|---|
| **Place** | A body left lying; a hushed door; the store spent | seconds to hours | Works: it is the world | Same, plus the nursery moved |
| **One ant's habit** | Its forage and nurse thresholds | one life (about 40,000 frames, critique §1) | Absent (`NURSE_STAY` is a built, off relative) | Specialists form |
| **Ant to ant** | Meetings with returners; alarm spreading; food passed mouth to mouth | seconds | Alarm and sharing work; meetings reach only ants that have foraged | A recruit path reaches newcomers |
| **The young** | Which lines lay; which larvae are fed from the store | a brood cycle | New: favour and policing | Same |
| **The bloodline** | How many meetings stir an ant; how long it stays stirred | lineage (a generation about 12,000 frames) | One knob exists at mutation width 0 | A heritable meeting bar |

**On today's ants, the mind is a lever, and the lesson lives in two places
only.** The verbs that work act on drives and planes, and when you let go nothing
stays in any head. What stays is the place and, once favour exists, the bloodline.

**What the hearing chart measures, and which gene moves it.** The first draft
plotted "the smallest swell that sends ten ants", and claimed the return-memory
gene moved it. That was wrong. `TRAIT_RETURN_MEMORY` scales how *long* the drive
lasts after a meeting [E creature.rs:26131-26137]; the stamp is all-or-nothing,
so no gene sets how many ants a swell sends. Two honest versions:

- **Cheap: "time stirred".** How long an ant keeps going out after one meeting.
  The return-memory gene moves it, once its mutation width (0 today) is opened
  [E how-the-ant-works.md §6d].
- **The target: "meetings needed".** A heritable per-ant bar on how many meetings
  in a window stir it. This is Gordon's colony threshold, inherited by daughter
  colonies [S: Gordon 2013]. It is new work, built on RECRUIT's contact count.
  Selection then trades the bar against RECRUIT's starvation harm, which is
  exactly the balance a colony should find.

The chart plots that bar, by signal, against generation. Be honest about the
speed: selection in one colony is slow (plan §7). Favour is selection by the
player, the case where it can show in a session (critique §4). Measure the slope
on paired seeds before promising it.

**The habit layer must not ship dead.** A pull added at gain 0 does nothing, and
every live link is billed. Habits enter with a hand-seeded gain and ship on only
if habit ants beat frozen ants on paired seeds (plan §7 item 5).

---

## Playing it

### The first ten minutes (the mind wakes)

The mind is the late seat. In the arc it wakes when the colony passes about 100
workers. Standalone, it starts from a saved colony of 150. This assumes the
recruit path and favour exist (or are faked):

- **0:00.** The camera lifts from the scout you were walking. The soil fills with
  a faint hum: amber threads where foragers walk laden, red flecks of old alarm,
  a pale glow over the brood. A ring of light at the door is the voice, full.
- **0:30.** A laden forager comes in. You press on her. An amber ring opens; a
  handful of waiting ants turn and file out. A count floats at the door.
- **1:30.** You hold too long on the next one. A crowd goes, more than the patch
  can feed. The store drops from the cost; minutes later some come back thin, and
  a beetle takes two stragglers. Loudness has a price on both ends.
- **2:30.** You hush the door. The trickle stops; the thin ones eat.
- **3:30.** A beetle bites a forager at the surface. Her red flares. Echo it, or
  swallow it and keep the nest working.
- **4:30.** You echo. You find out what your colony is: either it converges and
  the beetle is dragged home as food, or the door empties. Nobody has measured
  which, so the game must make both readable.
- **6:00.** The answerers glow faintly: the ants your first swell stirred, and
  any mobbers.
- **6:30.** You flick the glow onto the brood pile (or, in the gesture-free
  version, it settles there itself). Their line colours fleck the next eggs.
- **7:30.** The voice is low. You close the stomach in the top chamber, and a
  few lean ants go out on their own hunger.
- **Later.** When those eggs emerge, a cohort card reads how many came from
  lines that answered. The hearing chart gets its first point.

### The minute-to-minute loop

1. **Read the hum.** Too loud where, too quiet where? An idle mound, a begging
   brood, an unanswered cry.
2. **Choose a voice and a loudness.** Swell, hush, echo, swallow, stomach.
3. **Watch the trickle or the flood.** The most easily stirred go first.
4. **See who answered.** They glow for a minute.
5. **Bless, or let it fade.**
6. **Pay.** The store ticks down.

**Hands:** one hand presses and holds, hold time setting the gain; the other
runs time control, because brood cycles need fast-forward.

**The colony must run without you.** The known problem "fed ants idle on the
mound" becomes the player's to manage. If idling kills an unattended colony,
the seat is babysitting, which the critique ranks among the worst failures. The
baseline colony survives alone; the mind shapes and rescues.

### The first hour

- The budget is learned: swell when the store is full, hush before dark.
- Once built, habits show their first specialists, each with a deep tint.
- Two or three generations of favoured lines. The chart has a slope, or none,
  and the player starts asking why.
- **The first Goodhart moment:** a knot of ants loitering by the door, because
  that is where swells land and answerers get credit.

### The fifth hour

- A colony of a few hundred with a personality you made: it stirs at one meeting
  and mobs everything, or it ignores news and forages on hunger.
- The work shifts from causing behaviour to correcting excess: hushing a colony
  that pours into the rain, swallowing alarms a jumpy colony cries at nothing.
- A rival colony (the two-colony bed exists, `examples/rivalbed.rs`) with another
  personality meets yours.

---

## What the colony might do

### What the player will aim for

- **A colony that reaches far food.** Swells send bursts of foragers; favour
  breeds lines stirred by fewer meetings. Far-food starvation becomes a problem
  the player can work on.
- **Restraint before bad weather.** Hushing before rain, and favouring lines that
  stayed home, mirrors Gordon's colonies that held back and left more daughters.
- **A nest that defends itself.** If the pool mobs, echoed alarms and blessed
  mobbers breed a colony that converges on a beetle unprompted.
- **Nurses who stay.** Nurse habits, and policing eggs of lines that leave brood
  unfed, give steady larval care.

### What will surprise them

- **The courtiers at the door.** Lines that loiter where returners arrive,
  because that is where swells land and favour is credited. A useless crowd the
  player bred by rewarding presence at the signal, not the work.
- **The carry-out cheat.** If favour credits deliveries, expect ants carrying food
  out past the 16-cell trip reach and back in. The engine blocks the short version
  [E how-the-ant-works.md §6d]; selection will look for the long one.
- **The jumpy colony.** Echo often and bless mobbers, and you breed ants that
  answer alarm at a whisper. A stranger-touch alarm at the edge cascades into a
  fight with outlying sisters whose odour has drifted.
- **The deaf colony.** A player who hushes a lot breeds ants with a high meeting
  bar that forage on hunger alone. In a famine it may outlive the obedient colony.
- **Bodies that become a hedge.** Forbid the carrion on the far side and the
  bodies rot where they lie. If rot feeds soil there, the spot greens: a hedge
  the colony never meant to plant. If the pool weights `dead` at zero, ants simply
  walk over them, and the player learns that too.
- **Fat few or thin many.** In a famine, a closed stomach keeps a few fat and
  lets the rest starve; an open one keeps everyone thin until all fall together.
  The player chooses the shape of the decline: the graded death the owner wants.
- **A brood march that lags the lamp.** Once the pile choice reads warmth (new),
  brood would follow the lamps each light period, a reaction to heat. Real
  *Camponotus mus* moves brood on an internal daily rhythm, ahead of the heat.
  Ours would move after it. A liberty to show honestly, not a hidden clock.
- **The nurse who never left.** Habits make one nurse do much of the feeding.
  When she dies the brood glow spikes. The colony re-sorts, but into a different
  shape, as the threshold model predicts.
- **Your belief overruled.** You swell news from a patch where a spider waits.
  The answerers die, their lines shrink, and the colony slowly stops answering
  news from there. With place memory (plan §7) it happens within a lifetime.

---

## Fun and boring

### What kills this seat, ranked

1. **Sliders by another name.** If the verbs become a panel (forage 60%, guard
   20%), it is a management screen and teaches nothing. Every verb must be a
   gesture on a living ant or place, answered on the ants.
2. **A swell that sends nobody.** On today's engine it would: the stirred are
   already stirred and the idle cannot hear. That is the slice-0 trail again.
3. **The lesson nobody can see.** If the chart stays flat, the slow clock is dead
   and the seat is a lever box.
4. **Babysitting.** A colony that idles unless swelled turns the seat into chores.
5. **No stake.** Nothing of yours can die. Black & White's creature worked partly
   because it was one body you cared about [K].
6. **One recipe.** Swell when fed, hush when hungry. The voice cost and the
   Goodhart surprises must punish that, gradually.
7. **Bless as box-ticking.** If favour is a gesture done after every swell, it is
   a chore. The gesture-free version may be better.

### What keeps it alive past hours

- **Goodhart's colony.** Whatever you reward, the colony finds the cheapest way to
  produce it. Each surprise is a lesson about your own habits.
- **A personality that is yours.** Like Black & White's creature, whose habits
  came from how it was treated [S: low-quality summaries], but a whole colony.
- **The hearing chart.** Progress with no skill tree: lessons graduate into
  instinct, freeing the voice for the next one.
- **Indifference.** As in Rain World, creatures live without you and owe you
  nothing [S: Jakobsson, via PlayStation Blog].
- **Stepping down.** Diving into one ant, as in SimAnt, ties the giant scale to a body.

**Games this seat borrows from, and what to avoid.**

- **Dwarf Fortress:** workers choose jobs; the player only permits labours [S].
  The mind should feel like this, minus the menus.
- **Pikmin:** one gesture, a crowd's answer, at once [S]. The swell should feel
  like the whistle.
- **Creatures:** reward and punishment must not unlearn needs ("One Hour
  Stupidity", plan §7). No verb acts on the hunger-to-eat path.
- **Black & White 2:** its designers let players buy creature miracles instead of
  teaching (critique §4). The hearing chart has to show repetition paying off.

---

## Building it

### Fit with the owner's rules

| Rule | Fit |
|---|---|
| One brain, no castes, roles emerge | Thresholds and habits are numbers in one design. **But a caste ships today:** one ant in four is a nest worker for life, by its id (`Storeroom::SHIPPED`, `caste: 4`), chosen for measured stability (fall from peak 59% -> 7%) [E creature.rs:4008-4026]. The shipped nurse rule depends on it: only nest workers feed larvae from their banks [E brood.rs:154-166]. This seat's natural replacement is "nest worker by habit", but that is an owner decision with a cost (below) |
| Dig a multi-room nest and forage all game | No verb stops digging; quorum and nursery add rooms |
| Graded, visible, satisfying | Hold time sets the gain; every verb has an on-ant answer and a counter |
| Frame cost | Swells are events on a few ants; no new scent plane |
| Realistic | Every signal is real; four liberties are named |
| Features default on | Habits, favour and recruitment ship on only if they beat their absence |
| Not "walk to food and they follow" | The player never leads. The colony's own voices win or lose |

### Build steps, in order

| Step | What | What it proves |
|---|---|---|
| **0. Look and trace** | (a) Run `MEET_WAY` as its own arm first. Then widen the delivery ring to 12 cells, counting only ants that have foraged and are not nest workers, and trace who steps out. Then the same with RECRUIT on. (b) Widen an alarm's patch around a bitten forager: does this pool mob or scatter? (c) Hold corpses uneaten: does anything route round them? (d) Print the pool's weights on energy and `dead` | Whether the cheap verbs move anything, and which recruit path the swell needs. Pictures with counters, as review cards |
| **1. The faked slice** | Ten minutes, below | Whether the seat is fun before any of it is built |
| **2. Alarm and hush verbs, and the voice** | Echo and swallow by area or decay; hush; the store cost; the hum overlay | The response grades with hold time; the store falls with use |
| **3. A recruit path for the swell** | Newcomers stirred by a meeting rate, on the base of RECRUIT, with a heritable bar | Exits per swell grade with hold time; adults starved no worse than with RECRUIT off, paired seeds |
| **4. Favour, laying and policing** | Credit answerers from the decision trace; favour sets laying; open mutation on the meeting bar and return-memory gene | The bar falls under favour and stays flat without it, paired seeds |
| **5. Stomach and brood** | Sharing gain; nurse scent on with a gain; brood carry with a warmth term | Hunger spreads, nurses answer, brood follows warmth |
| **6. Habits** | Forage and nurse thresholds, hand-seeded gains | Habit ants beat frozen ants on paired seeds |
| **7. Quorum and the arc** | Emigration; queen-to-scout-to-mind handoffs | A flooded chamber is left; handoffs keep the player |

### Frame cost

- **Swell:** one 25-by-25 scan per held delivery, against today's 7-by-7.
- **Echo and swallow:** writes to the existing alarm plane over a small patch.
- **Favour, the meeting bar, habits:** a few numbers per ant, one pull term.
- **The hum overlay is the real risk.** It animates, so it would defeat the
  dirty-rect skip on a settled world. Show it while a key is held, or redraw it
  slowly. Quote `examples/ascii.rs` worst-frame cost before and after.
- **No new scent plane.** The voice works through stamps, the alarm plane and
  crops.

### The cheapest faked ten-minute slice

Load a saved colony of about 150 ants. Script what is not built:

- **Swell (scripted response):** on a press, a hand-set share of ants in a
  12-cell ring step out, graded by hold time, whatever their foraging history.
  This tests the feel, not the engine's recruit question (step 0 answers that).
- **Echo and swallow (near-real):** widen or shrink the alarm patch near the press.
- **Stomach (scripted):** sweep a chamber; move crop food evenly or not, and show it.
- **Habit (scripted):** a tapped ant gets a hand-set pull toward its task, with a tint.
- **Favour, two versions:** the flick gesture, and the automatic one.
- **Instinct (scripted):** at minute 8, load a hand-made "many generations later"
  save where the scripted response needs half the hold time. The chart shows it.

Post it blind beside the queen and keeper slices, with ants sent per swell, store
spent and answerers credited in the card's meta. **Two questions:** does pressing
on a forager and watching the colony answer feel like teaching? And does
blessing feel like play, or like ticking boxes?

### The arc: queen, then scout, then mind

The mind is the late seat of `perspectives.md` §7, combination 2.

- **Founding, the queen's body.** Survival and the first chambers, the template
  the colony digs from.
- **Growing, a scout's body.** At the first workers: persuasion by tandem runs and
  shared meals. The routes the scout teaches are the signals the mind later swells.
- **Mature, the mind.** At about 100 workers the colony is too big to persuade one
  by one, and the hum opens.

**The critique's warning applies:** taking a body away just as the colony gets
interesting is the worst moment to change genre (critique §1). So nothing is
taken: the mind is added, and the player can step down into any ant, the queen
included. The voice grows with the colony (one held swell at 100 workers, three
at 300), which is why it goes last.

---

## Risks and open questions

### The biggest risks to fun

- **Abstraction.** No body, no stake. The step-down verb and the arc carry this;
  if the faked slice reads as a dashboard, stop.
- **The first swell sends nobody.** True of today's engine (above). Step 0 and the
  recruit path come before any swell is offered to a player.
- **More foraging can kill.** RECRUIT was shipped off when more adults starved,
  442 against 298 and 323. A recruit path for the swell inherits that harm. As a
  player trade-off it is fine only if the decline is graded and recoverable.
- **A colony hunger slider was already tried as an engine rule, and failed.**
  Sending foragers by the nest's mean hunger or its store shortfall did not track
  what the colony lacked, which was growth, and was worse in the lab box
  [E Reports/dead-ends.md:1217]. That is measured evidence against sliders.

### The biggest risks to feasibility

- **Credit assignment.** "Who answered" must come from the decision trace. Ants in
  the ring who went out anyway will be credited; noise slows selection.
- **Selection may be too slow.** If the bar moves only over 100 generations, the
  slow clock is invisible. Measure the slope on paired seeds first.
- **Habits can ship dead,** and their nearest built relative (`NURSE_STAY`) is off
  for a large harm. Hand-seeded gain, and the learners-beat-frozen gate.
- **Genes at width 0.** Opening mutation on the return-memory gene and walk gains
  is a re-baseline for training, not a free switch.

### Decisions only the owner can make

1. **Standalone or late stage?** Recommended: the last stage of the arc.
2. **May the mind decide who lays?** Favour and policing are the liberty that
   makes this a teaching seat. Without them it is a lever box.
3. **Do signals spend colony food?** Without the cost, loudness is free and one
   recipe wins.
4. **May ants form habits in life?** Two thresholds per ant, shipped only if they
   beat frozen ants.
5. **Should the shipped caste become a habit?** Today one ant in four is a nest
   worker for life by id, which the owner's "roles emerge" rule argues against.
   Replacing it with nest work by habit means the nurse rule changes too, and the
   caste's measured stability (fall from peak 59% -> 7%) must be matched.

### What the review changed

1. Swell the news: **fixed.** Now new; needs a recruit path for never-foraged ants, must beat RECRUIT's harm; step 0 counts only eligible ants.
2. Nurses cannot be sent: **fixed.** The 1:30 beat and the voice's cost rewritten.
3. The caste switch: **fixed.** It ships on (`caste: 4`); stated with its measured reason and the nurse-rule link; owner decision 5.
4. The chart's gene: **fixed.** Return memory moves "time stirred"; the target is a new heritable meeting bar (Gordon 2013).
5. Shift the nursery: **fixed in part.** Now new. The brood march is kept as a reaction to the lamp (current heat suffices, no clock), labelled as lagging real *C. mus*.
6. Echo by amount: **fixed.** Echo widens the patch or slows decay; the 4:30 outcome is unscripted.
7. Leave the dead: **fixed.** Now "Forbid the carrion"; the hedge is conditional on the `dead` weight.
8. Habits as mem: **fixed.** Now new, with concrete forage and nurse levers; `NURSE_STAY` cited.
9. MEET_WAY prior art: **fixed.** Cited; step 0 runs its arm first.
10. Citations: **fixed, one partial dispute.** Greene 2013, Gordon 2013 *Nature*, Franks 2007 corrected. The *Lasius* "about 100" was in this session's search summary, so kept as unconfirmed [S], not [K].
11. Slow clock and Bless: **fixed.** The slice scripts the swell and asks whether blessing is play; a gesture-free favour added.
12. Unsourced numbers: **fixed.** Marked illustrative; the "weighted heavily" claim and the cohort timing removed.
13. Table versus risks: **fixed.** Codes now agree.

---

## Sources

Web sources (search summaries and abstracts read this session; no full papers):

- Bonabeau, Theraulaz & Deneubourg 1996, fixed thresholds: https://pmc.ncbi.nlm.nih.gov/articles/PMC1688885 ; https://ideas.repec.org/p/wop/safiwp/98-01-009.html ; https://www.doi.org/10.1006/BULM.1998.0041
- Theraulaz, Bonabeau & Deneubourg 1998, threshold reinforcement: https://ideas.repec.org/p/wop/safiwp/98-01-006.html
- Ravary et al. 2007: https://antkey.myspecies.info/en/content/individual-experience-alone-can-generate-lasting-division-labor-ants ; https://pmc.ncbi.nlm.nih.gov/articles/PMC2686371
- Jones et al. 2004: https://pubmed.ncbi.nlm.nih.gov/15218093/ ; https://kops.uni-konstanz.de/entities/publication/bc28b042-0c37-4370-9b62-794dbfdcaebd/full
- Greene, Pinter-Wollman & Gordon 2013: https://pmc.ncbi.nlm.nih.gov/articles/PMC3540075
- Gordon 2013, *Nature* 498:91: https://www.nature.com/articles/nature12137 ; https://link.springer.com/article/10.1038/nature12137
- Harvester-ant colony personality and regulation: https://ideas.repec.org/a/oup/beheco/v23y2012i5p1022-1029..html ; https://ideas.repec.org/a/oup/beheco/v22y2011i2p429-435.html
- Trophallaxis and crop loads: https://www.weizmann.ac.il/complex/feinerman/research/food-dissemination ; https://wis-wander.weizmann.ac.il/node/9305 ; https://elifesciences.org/articles/77659.pdf ; https://link.springer.com/article/10.1038/s41598-019-52019-6
- Larval signals: https://blog.myrmecologicalnews.org/2024/05/08/larval-chemical-cues-induce-rapid-changes-in-foraging-preferences-of-ant-workers/ ; https://www.frontiersin.org/journals/ecology-and-evolution/articles/10.3389/fevo.2019.00398/pdf
- Van Oystaeyen et al. 2014: https://sciencedaily.com/releases/2014/01/140116150808.htm ; https://researchportalplus.anu.edu.au/en/publications/conserved-class-of-queen-pheromones-stops-social-insect-workers-f/
- Brood thermoregulation: https://www.frontiersin.org/articles/10.3389/fnbeh.2016.00073/full ; https://pmc.ncbi.nlm.nih.gov/articles/PMC3962001/
- Superorganism and decentralised control: https://corte.si/posts/books/superorganism/ ; https://www.frontiersin.org/articles/10.3389/fpsyg.2019.02653/full
- SimAnt: https://en.wikipedia.org/wiki/SimAnt ; https://www.giantbomb.com/games/3030-15236/
- Black & White: https://en.wikipedia.org/wiki/Richard_Evans_(AI_researcher) ; https://www.itechguides.com/how-black-whites-creature-learning-ai-and-morality-system-work/
- Rain World: https://en.wikipedia.org/wiki/Rain_World ; https://destructoid.com/?p=194993 ; https://gamedeveloper.com/design/crafting-the-complex-chaotic-ecosystem-of-i-rain-world-i-
- Dwarf Fortress: https://dwarffortresswiki.org/DF2014:Labor ; https://dwarffortresswiki.org/index.php/Job
- Pikmin: https://www.nintendo.com/ph/interview/ampy/index.html ; https://nintendoworldreport.com/feature/12736/nintendo-world-reports-greatest-gamecube-games-pikmin

Repo files relied on (read only):

- `Reports/teaching-the-colony-2026-10-25.md` §1, §2, §4, §7, §8, §10; its folder's `engine-brief.md`, `perspectives.md` (§0, §2, §6-8), `critique.md`, `queen.md` (§0, §1, §4c-d, §5, §6); `colony-mind.critique.md` (the review of this document)
- `Reports/how-the-ant-works.md` §4, §6d (forage drive, `met`, trip reach, lean rule, nest workers, `TRAIT_RETURN_MEMORY`), §12 (`NURSE_STAY`)
- `Reports/dead-ends.md:1217-1218`; `Reports/brakes-and-nest-life-2026-10-08-13.md:298`
- `src/sim/creature/mind.rs:72-123, 1897`; `src/sim/creature/options.rs:236-283`
- `src/sim/creature.rs:1961` (corpse worth), `:4008-4026` (`Storeroom::SHIPPED`), `:16812-16824` (`nurse_stays`), `:25412` (`RETURN_MEET`), `:26093-26137` (drive and `met`), `:26144` (the stamp), `:26159-26180` (`MEET_WAY`), `:26350-26364` (RECRUIT's trace)
- `src/sim/brood.rs:145-166` (`NurseMode`), `:367-373` (egg pile), `:454-466` (brood carry, off), `:1391` (`larva_scent`); `src/sim/pheromone.rs:347` (`ALARM_DEPOSIT`)
