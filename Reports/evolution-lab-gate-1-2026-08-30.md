# Gate 1, measured: the lab box lives, and what it costs

*Lane D of the evolution-lab program, 2026-08-30. The census and the frame
cost of `lab::scene::LabBox` — the first scene in this repo that runs plants
and creatures together. Design of record:
`evolution-lab-design-guide-2026-08-30.md`; the numbers it is downstream of
are in `evolution-lab-feasibility-2026-08-30.md`.*

**Read §1 if you read nothing else.** Everything after it is the evidence.

---

## 1. The finding

**The box lives, and it is a food web rather than two populations sharing a
bed.** Plants germinate, grow, set seed, breed to **generation 4** and turn
over — 285 organisms born and 266 dead across 90,000 frames from eight
founders. The ants do not breed, decline from 52 to 2, and **hold there
rather than going extinct**. And the two are coupled: the colony is the
single largest thing acting on the stand.

Five results, in the order they change a decision.

1. **The missing founders are neither germination failure nor invisibility.
   They are eaten.** All eight founders germinate; at frame 900 seven stand
   and one is dead, and by frame 5,000 only five. Run the identical bed with
   `colonies=0` and **all eight are alive at frame 20,000** with the stand at
   **95 organisms against 42**. PLACEHOLDER_SEEDS
2. **Gate 0 is not reachable in this bed, and the reason is foraging rather
   than economy.** The shipped ant's margin here is **−820** (ceiling 220
   against a 1,040 bar), which reproduces §162. Give it the matched gut and
   the margin goes **+500** — and births stay at **zero**, because the richest
   bank ever reached is **555**: no ant in 48,000 frames ever ate the flower
   the ceiling was computed from. #162 §5 names this case in advance.
3. **Fruit does stand, transiently, and then stops.** PLACEHOLDER_FRUIT
4. **Cost in this bed is the coarse field and almost nothing else.**
   PLACEHOLDER_COST
5. **The organism ceiling is not a live constraint here.** High water
   **66 slots of 4,095 — 1.6%** — and `organisms_refused` is **0** at every
   tile of every run. The 1,812–2,503 live organisms the guide quotes for
   `herb` are a *generated world* figure; this bed runs two orders of
   magnitude below it.

PLACEHOLDER_BODY
