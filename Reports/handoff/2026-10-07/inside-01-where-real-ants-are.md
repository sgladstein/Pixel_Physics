# Where real ant workers spend their time

Lane 20 (Nest race), 2026-10-05 23:50. Written **before** reading the new traces (how-we-test §3 step 1). Citations are *recalled* from the literature, not re-checked this session: a PubMed search returned nothing for these. Where a claim is already in the project's memory with a doi, that is noted.

| # | Rule in real ants | Evidence | What it predicts in our box |
|---|---|---|---|
| 1 | **Most workers are inside the nest.** Foragers are a minority, often 10-30% of the workers. The rest (nurses, nest workers, and a large share that are simply idle, the "reserve") stay inside. | Gordon's harvester-ant task work; Charbonneau & Dornhaus on inactive workers (*recalled*). | A healthy colony has most ants underground most of the time, not 2-14%. |
| 2 | **Each worker keeps to its own zone.** Nurses stay with the brood, cleaners stay in the middle, and foragers stay near the entrance and outside. Workers move between zones as they age, inside first. | Mersch, Crespi & Keller 2013, *Science*, doi 10.1126/science.1234316 (in memory). | Young ants start inside and stay there. They go out later in life, not on their first walk. |
| 3 | **Ants that go out are the ones the colony needs out.** How many forage follows how much food comes back (returning foragers met at the entrance), and leaner workers start foraging first. | Gordon 2013, *Nature*, doi 10.1038/nature12137; Bernadou et al. 2020, *J Exp Biol*, doi 10.1242/jeb.219238 (both in memory). | Fed, idle ants have no reason to leave. Only hungry ants, or ants that are signalled, go out. |
| 4 | **The nest itself is the cue for "inside".** Darkness, humidity, CO2 and the smell of the colony are all strongest deep inside. Nest odour marks the inside of the nest, not the ground around it. | Cox & Blanchard 2000, doi 10.1006/jtbi.2000.2010 (CO2 gradient, in memory). | "Home" is a gradient that keeps rising into the chambers, not a point at the door. |
| 5 | **Resting happens inside.** Idle workers rest in chambers, often in clusters near the brood. | General observation (*recalled*). | An idle ant should drift deeper and settle, not stand in the doorway or on the spoil heap. |

**Prediction before looking.** Our ant breaks rules 1, 4 and 5 by design, judging from the lanes' traces so far:
- "Home" is the doorway.
- Nest scent is flat across the whole mound.
- No rule makes an idle, fed ant go anywhere in particular.

So I expect the trace to show that every ant is born inside and walks out within its first few hundred frames. I expect that because nothing makes inside a place to stay, not because something pushes ants out. Rule 2 (ants start inside and stay until older) is the cheapest place to check this, because every ant is born in the brood room.
