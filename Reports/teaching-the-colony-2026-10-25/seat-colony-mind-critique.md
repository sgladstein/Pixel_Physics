# Critique: colony-mind.md (2026-10-25)

Checked against `main` at 1c8b7a3b. Most serious first.

1. **BLOCKING. "Swell the news ... now": the swell reaches ants that cannot answer it.** `colony_drive_level` returns 0 unless the ant has foraged before (`state.foraged`), or RECRUIT has recruited it, and RECRUIT is off. It also returns 0 for any nest-bound ant [creature.rs:26095-26103]. RECRUIT's own trace says the mound's loafers are *exactly* never-foraged ants and nest workers [creature.rs:26350-26364]. A wider stamp mostly marks ants that read 0. Ants who have foraged and still hold a fresh memory are already at the plateau. So "+6/+7 out" has no engine path. Letting never-foraged ants answer is RECRUIT itself (442 starved vs 298/323). Fix: mark the verb **new**. Say it needs a recruit path for newcomers, and name the harm it must beat. Step 0(a) has to count only foraged, non-nest-bound ants within 12 cells.

2. **BLOCKING. Nurses cannot be "sent" (1:30 scene; "the voice"; the biology framing).** Nurses are the nest-bound caste: `NurseMode::Workers` has shipped since 2026-10-12 [brood.rs:145-166]. "A nest worker is never driven" [how-the-ant-works.md §6d]. The graded cost "loud calls pull nurses off the brood" cannot happen. Fix: give the voice a cost the engine can actually deliver, or list "drive reaches nest workers" as new work, and rewrite the 1:30 beat.

3. **BLOCKING. "The storeroom's one-in-k caste switch ... must stay off in this seat."** It is **on** by default: `Storeroom::SHIPPED` has `caste: 4`, chosen for measured stability (fall from peak 59% -> 7%) [creature.rs:4008-4026]. The shipped nurse rule depends on it. Turning it off reverts a shipped decision, and only nest-bound ants feed larvae from their banks. Fix: state that the caste ships on and conflicts with "roles emerge". Make "turn it off" an owner decision with its cost: the nurse rule has to change too.

4. **BLOCKING. The hearing chart has no heritable substrate.** The doc's bloodline knob is `TRAIT_RETURN_MEMORY`. It scales how *long* the met drive lasts [creature.rs:26131-26137], not how *many* ants a swell sends. The stamp is binary and the pace term is a constant, so no gene sets "volume needed". The faked slice's payoff ("doubled gene, the same swell sends twice the ants") is therefore wrong on the engine. Fix: add a heritable gain on the drive, or on the stamp's reach (new work). Or redefine the chart as "time stirred after one swell", which the gene does move.

5. **BLOCKING. "Shift the nursery ... brood is carried to the pile site: now (to verify)".** Brood carrying is off by default (`PIXEL_PHYSICS_BROOD_CARRY`, [brood.rs:454-474]). Eggs are *placed* on the pile when laid, not carried [brood.rs:367-373]. `pile_site` has no warmth term. The "daily brood march" surprise also needs memory or a clock that fixed-weight ants lack. Real *C. mus* follows a circadian rhythm, not a single demonstration (Falibene 2016). Fix: mark **new** (brood carry on, plus a warmth term in pile choice). Recast the march as a liberty, or cut it.

6. **IMPROVE. "Echo a cry: alarm deposit is doubled."** A bite already lays 240 of 255 [pheromone.rs:347]. The plane is u16 8.8 fixed point, so doubling saturates at about +7%. Fix: echo by area (lay at more cells) or by slowing the decay, not by amount. Also, nobody has measured whether today's pool mobs, so the 4:30 kill should not be scripted as a sure outcome.

7. **IMPROVE. "Leave the dead: nobody carries it to the midden".** There is no corpse-to-midden rule. The midden code is for dig spoil, and corpses are carrion with food worth [creature.rs:1961]. Holding off the dead therefore means forbidding the colony to eat them. Fix: rename the verb to what it suppresses. "Fence of the dead" also depends on the `dead` feature's weight in the shipped pool, so check that before promising ants route round bodies.

8. **IMPROVE. "Lean on a habit: mem (habit form)".** Plan §7 contains no task thresholds. The habit layer is this doc's invention, so it is **new**. The ant also has no "task" for a pull to point at: it picks verb+target each tick. A dig or nurse "pull" has no walk-level target apart from nurse-seek. Fix: label it new, and say what a "forage/dig threshold" acts on concretely.

9. **IMPROVE. Prior art not cited.** `PIXEL_PHYSICS_MEET_WAY` (a carrier stamps the ants it passes; off) is the closest built relative of the swell [creature.rs:26159-26180]. Its trace found loafers stand 5-17 rows above where deliveries land, so even a 12-cell ring may miss them. It was "never run as its own arm" [brakes-and-nest-life-2026-10-08-13.md:298]. Fix: cite it, and make step 0(a) run MEET_WAY's arm first.

10. **IMPROVE. Misattributed biology.**
    - Greene & Gordon 2007 used beads coated with *patroller* hydrocarbons. The cue of foragers returning with food is Greene, Pinter-Wollman & Gordon 2013.
    - "Colonies that held back in drought left more daughters" is Gordon 2013, *Nature* 498:91, not a 2014 abstract. That paper also says each colony has its own interaction threshold, which is better support for this seat.
    - Removing marks made the choice random in Franks et al. 2007 (*Proc R Soc B*), not Stroeymeyt 2011. That 2011 paper is about knowledgeable individuals.
    - "One *Lasius* donor feeds ~100 workers" could not be found (Planckaert 2019 used subcolonies of 50+). Downgrade it to [K].

11. **IMPROVE. Fun: the slow clock is still mostly watching.** Each minute has one real verb, the swell, and it may send nobody (items 1-2). Everything that makes the seat a teaching game (Bless, policing, habits, the chart) is new or scripted. "Bless" ("gather the glow, flick onto the pile") is a selection menu drawn as a gesture. Fix: the faked slice should script the swell's response too (so it tests feel, not item 1). It should also ask a second question: does Bless feel like play or like ticking boxes?

12. **IMPROVE. Unsourced numbers.**
    - "+6", "+7", "eight ants", "14 young: 9 ...", "one ant's meal per second" and "30+ generations in hour five" have no measurement behind them.
    - "Hunger is the strongest sense ... the brain weights heavily" has no [E].
    - The cohort at 9:00, 2.5 minutes after the Bless, assumes an egg-to-adult time the doc never states.

    Fix: mark them as illustrative, or cite them.

13. **IMPROVE. "Swells will reproduce that harm" sits in Risks, but the verbs table says "now".** This is an internal contradiction. Fix: make the table's code agree with the risk.

**Verdict.** The biology framing (response thresholds, rate of contacts, colony personality) is good and fits the owner's rules. But the anchor verb, the nursery verb and the readout of instinct do not work as described on today's engine.
Of the 15 verbs, only echo/swallow and spend-the-reserve are near "now". Re-code the table, fix the caste claim, and give the hearing chart a gene that actually moves it before the faked slice is built.
