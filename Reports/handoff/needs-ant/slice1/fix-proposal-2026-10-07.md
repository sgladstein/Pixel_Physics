# Slice 1 fixes: proposal for review

Redesign lane, 2026-10-07. **For review by another lane. Nothing here is built.** Scott reads this and the
review together before anything is built. This is the one round of fixes the design doc allows; after
it, the doc's kill rule applies.

The evidence is in [`results.md`](results.md), which this file cites by step number.

## The two faults, in one line each

Both faults are places where slice 1 took a shortcut the design doc does not take.

1. **Ants that are not diggers dig.**
   - Slice 1 hands the brain's own dig urge to foragers, idle ants walking home and resting ants.
   - They cut twice the shipped soil during the boom. The soil fills the mound's tunnels.
   - Food carriers stop getting in (steps 1-5).
2. **Foragers stand still outside.**
   - Slice 1 lets the brain decide whether a forager steps. For a fed, empty ant that is close to never.
   - The forage job also never ends outside. So 89% of the walk's empty ants outside sit still, and few
     reach the food (steps 6-8).

**How much of the collapse each fault causes is not measured.** The runs below split it.

## Fix 1: only diggers dig

**What changes.**
- Every drive except the dig job and escape hands the act no dig urge. This is the design doc's act
  table: the dig job digs at its face while its stimulus holds; escape digs at any energy toward the way
  out.
- The dig job gets the design's own stimulus, which slice 1 never built:
  - the stimulus is soil at the face plus bodies within 2 cells of the head;
  - it is lowered as space grows;
  - it ends when crowding falls below the ant's threshold with no fresh cut near, or when patience runs
    out.
- Today slice 1 puts an ant on the dig job only if it already remembers a face, which is why the dig
  job made 1% of cuts.

**Why.** 98% of the extra cuts came from drives that are not the dig job: foraging 63%, walking home
23%, resting 12% (step 1). In the mound's tunnels those ants face the unprotected walls above the
ground line, and cut 4.5 times as often per decision as shipped ants (step 2).

**Risks.**
- **The nest may stop growing.** Crowding-driven digging was a coin flip on room totals four times
  (dead-ends 1878). It never had many ants deep to act on (check C2, inconclusive for that reason).
  The new ant does put ants deep, so this is the first fair test. It is not a proven mechanism.
- **A carrier or homebound ant can no longer cut its way in through a closed door.** Escape covers an
  ant inside that wants out, not one outside that wants in. Two options:
  - **(a) A clearance cut** (recommended): an ant with food, or walking home, may cut only the one cell
    its own chosen step needs, and only after that step has been refused several decisions running.
    That is a blocked path, not an urge. Recommended because the trace shows carriers circling on the
    mound while the door is shut (step 5).
  - **(b)** Leave it to the dig job's crowding stimulus at the door, where bodies pile up. This
    depends on an ant inside taking the job.
- **The no-veto guard must stay green**: an ant at 30% of its energy must have eaten, reached air, or
  be digging if sealed. Fix 1 removes dig urges, so the guard's sealed-pocket rows are the ones to
  watch.

**Checks, by trace before any colony number:**
- the share of cuts made by the dig job, escape and clearance (aim: nearly all of them);
- next cut at the face (a gate item);
- soil holders in the mound's tunnels, against the stack's 58-142 and shipped's 47-82;
- door-shut maps;
- rooms at 100k, 200k and 300k against shipped.

## Fix 2: foragers keep walking, and give up outside

**What changes.**
- **An empty forager steps at the walk's own walking pace** while it holds the job: the walk's
  `P_HOME` (0.5), or the brain's chance if that is higher. The design doc lists "decide step or stay"
  as something the new walk must do itself; slice 1 passed it back to the brain.
- **The forage job ends outside when scouting patience runs out**, as the design says ("or patience
  runs out"). The ant turns idle, walks home at `P_HOME`, and rests at its own depth inside.
  - Slice 1 already keeps the patience (the shipped `scout_patience`). It never ends the job on it.
  - Patience only decays when the ant steps, so a frozen ant never gave up.
- **An empty ant rich enough to lay walks home to lay**, as the shipped ant does (the lay-home pull),
  instead of steering out on the forage drive. Today 81% of such ants outside are on the forage drive
  (step 8).

**Why.**
- At 80k-100k, empty ants outside stepped on 12% of decisions against 49% shipped (step 6).
- In both games, an empty ant below the egg bar barely steps (step 7). Shipped ants outside move
  because most of them are rich enough to lay; under the walk only 11% are.
- The walk's forage job keeps such an ant outside indefinitely.

**Risks.**
- **More ants walk home through the mound's tunnels**, where soil holders stand. With Fix 1 they no
  longer cut on the way.
- **Fewer ants stay outside.** Fed scouts must still leave a young colony. The forage stimulus has a
  floor for this, and the gate's 24-seed founding bed tests it.
- **The laying-bar proposal** (Nest race, `LAY_BAR=body`, also under review) changes who counts as
  rich enough to lay. The two do not conflict: this fix gives an already-ready ant the shipped home
  pull; that one moves the bar.

**Checks:**
- share of forage-job decisions outside that step;
- visits to the food per forager per 10k frames;
- forage jobs ended outside, and where those ants are 2,000 frames later;
- eggs laid by ants that walked home to lay.

## Fix 3: food keeps the purpose it was picked up for

**What changes.** Scott's rule, already in the design doc (rev 131), and agreed by Scott for the
storeroom at 01:26.
- Food taken to eat stays in the crop and digests until the ant's own energy, not its crop, is past
  the hunger onset. It is never put back down at home.
- A forager's cargo goes down at the nest on purpose, hungry or not.
- A nest worker's and a nurse's loads wait for the store and nurse jobs (later slices).

**Why.** 77 of 78 bites the walk's traced ants took at home were put back down, a median 15 frames
later, for 8 J each.

**Shared work.** The Nest race lane is tracing the same rule for the shipped walk (its storeroom part
B). It should be built once, as one helper both walks call. *(03:00: the coordinator assigned it to Nest race; the walk calls that helper once it lands and builds no copy.)*

**Checks:**
- bites taken at home and put back (aim: near zero);
- foragers' drops at the nest unchanged (Scott's condition).

## Fix 4: escape counts only real stalls

**What changes.** Escape's stall count rises only on a step the ant tried and was refused, never on a
decision where it chose to stay.

**Why.** Escape fired on a resting ant at hunger 0.06.

**Checks:**
- escape spells by hunger at onset;
- shut-in spells ending starved, at or under shipped's 0.5%.

## Not in this round

- **Where soil goes down.**
  - Pellets are held for thousands of frames in both games, because the drop is refused under the
    mound's roof.
  - Nest race's proposal A (drop on open sky, 2+ cells from any hole; Nest building is reviewing it)
    fixes the shipped haul target. The walk's haul job aims at that same target, so it inherits the fix
    when it lands. A second version here would duplicate it.
- **What closes the door is still untraced.** If Fix 1 brings soil holders back to shipped's level and
  the door still shuts more often than shipped's, that trace comes next.

## How the round is judged

1. **Runs.**
   - Each fix gets its own named part of the switch.
   - 4 seeds to 300k for each of: Fix 1 alone, Fix 2 alone, and all four fixes together. That is 12
     runs, the size of the gate round just done.
   - The walk as built, the stack and shipped are re-used from that round.
   - Running each fix alone is what splits the collapse between the two faults.
2. **Trace first.** Each fix's own behaviour is traced (the checks above) before any colony count is
   read.
3. **Then the full gate** on 4 seeds, plus the 24-seed founding bed.
4. **Kill rule** (design doc). After this round the new ant is dropped if any of these hold:
   - a late collapse;
   - less food carried in from outside;
   - deep time under twice shipped's;
   - deep time under the stack's.

## Questions for the reviewer

1. **Fix 1:** the clearance cut (a), or leave the door to the dig job (b)?
2. **Fix 2:** walking pace.
   - Is `P_HOME` (0.5) the right floor for a forager's step, or should it fall as the forage stimulus
     falls?
   - Shipped ants with home's pull step on about 65% of decisions.
3. **The runs:** is it worth running each fix alone (8 extra runs) to split the cause, given that
   after this round the kill rule decides?
4. **Anything in `results.md`** that does not support the step it is cited for.
