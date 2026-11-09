# The mould game: brainstorm, prototypes, reviews and research (2026-11-08 to 11-09)

Everything written for the mould game (`cargo run --release --bin mould`) after the owner
called it the most promising of the eight probes, copied into the repo on the owner's ask
(2026-11-09: "All the sub-agent reports (and everything else you have done) should be saved
in the repo so their work is not lost"). The build itself and its measurements are in
[`../mould-probe-2026-11-07.md`](../mould-probe-2026-11-07.md); this folder is the thinking
around it. Pictures are not copied (about 60 MB of PNG and GIF); they live on the Artifact
pages listed below, and every one can be re-drawn with `examples/mouldprobe` (see
`Reports/instruments.md`).

**Status:** the brainstorm's picks are built and on `main` (PR #1054: three challenge dishes,
the Wanderer, the Organ, the Night look, two review rounds). The two depth reports
(2026-11-09) are proposals; nothing in them is built yet. The sources report is being
revised after a full read (its first version could not reach the sources; see its header).

## What is here

| Folder | What it is |
|---|---|
| [`brainstorm/`](brainstorm/) | The 2026-11-08 five-agent brainstorm after the owner's first play. `context.md` is the brief every agent got (his play notes, rules); then `theme.md` (what the game is about), `out-there.md` (big swings), `near-term-challenges.md` (dish ideas), `toy-feel.md` (making it feel like a toy), `research.md` (real *Physarum* and other simulations: the owner later found it changed little, see the depth reports), `bug-check.md` (a bug sweep of the shipped probe) |
| [`prototypes/`](prototypes/) | The overnight build briefs (`common.md`, `challenges.md`, `interface.md`, `organ.md`, `wanderer.md`) and each one's result with its counts. `interface/key.md` is the key to the blind A/B/C look pages (A Toy, B Glow, C Bench, N the built Night look) |
| [`reviews/`](reviews/) | The two adversarial bug reviews over all the demo work (13 and 12 findings, all fixed in PR #1054), and the second review's scratch harness |
| [`depth-2026-11-09/`](depth-2026-11-09/) | The owner's 2026-11-09 asks. `sources-research.md`: Sage Jenson's physarum post, Georgios Cherouvim's work and Jones 2010, compared with our model, with improvements and new ideas. `complexity.md`: how the tools, events, ground and the mould's responses could interact for depth and discovery (water, crust, fruiting, salt that dissolves, a 15x15 interaction matrix, three new dishes, a ranked build plan) |
| [`pages/`](pages/) | The HTML of every picture page shown to the owner, for the words and counts; the images they reference are not in the repo |

## The pages (open these for the pictures)

1. First look at the probe: https://claude.ai/artifact/R4Yjkt4bsozfZDV1CkpcDc (`pages/1-probe-first-look.html`)
2. Playtest fixes, before and after: https://claude.ai/artifact/52KtqyTGjyhKMb7L14Eqq6 (`pages/2-playtest-fixes.html`)
3. The lesson: https://claude.ai/artifact/PoyyzzGuRyqaEn4G8RidPK (`pages/3-lesson.html`)
4. Brainstorm synthesis: https://claude.ai/artifact/16qnDdYhzg3Q5kL3LnApYE (`pages/4-brainstorm-synthesis.html`)
5. The overnight build: https://claude.ai/artifact/TnWALC9vhyrkFQ2o52iWgY (`pages/5-overnight-build.html`)
6. The Organ: https://claude.ai/artifact/Np5fa71YVHe8gnZq7oVZNW (`pages/6-organ.html`)
7. The Wanderer: https://claude.ai/artifact/RrEJruyrAveXMGo6c7q5f4 (`pages/7-wanderer.html`)
8. Starving mould, three looks (blind; key A embers, B grey, C dried slime): https://claude.ai/artifact/GaadFXKa1YEzEE6EdnrfCP (`pages/8-starving-look.html`)

## Owner rulings that shaped it

- 11-08: the mould stays a toy in a dish; not bringing in the Pixel Physics world, plants or ants.
- 11-08: each piece eats its own food, on by default; build the look if the thread and the interface agent agree.
- 11-09: everything on screen must come from real simulation, no drawn-only effects (the drawn pulse along fed tube is now off, `render::DRAWN_PULSE`); food must really flow, all three levels, each behind a switch or number he can tune (`Params::flow` and the T panel, `src/mould/tune.rs`).
