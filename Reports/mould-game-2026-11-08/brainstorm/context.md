# Mould game brainstorm: shared context (2026-11-08)

Read this first. It is the only context you share with the other brainstormers.

## The ask, in the owner's words (Scott, 2026-11-08 06:39Z)

> So I think I want to elevate this demo. I think it is inherently the most fun mechanic, which is probably the hardest thing and there is something I like you can connect to more about a slime mold than a gas. I'd like to spin up a handful of agents, to explorer brainstorming agents for how we can take the enjoyable mechanic of kind of controlling the slime, mold and build a game out of it. One should especially focus on the actual theme of it like what's going to be enjoyable and going to be able to connect to as a player like mi as the player a caretaker for this slime mold. Do I have to get it to spread and take over something? Or what's the motivation in the theme of all of this? The second agent should more focus on the game aspect, mechanics game design of it all and how to actually make something that is fun and increases in difficulty and challenge and complexity throughout the game. One really out there agent who takes what's fun about this game mechanic and suggests it for maybe a surprising different direction to take. It doesn't even have to be slime mold. But what's fun about this is it you know does actually seem connected to the pixel physics simulation, you really do feel like you can control it and move it to make new networks and patterns which is fun. One of the fun things was trying to like get connections between interfood nodes are like make little Bridges where there wasn't even food. Anyways I'm just saying an out-of-the-box brainstorm agent. One agent specifically focused on challenges in the near term for the demo.

His first play, an hour earlier (same day):

> This demo has the most promise so far in that the core mechanism is inherently. Just kind of fun to play around with and looks cool. We definitely want to dig deeper. The game needs better instruction of what you're supposed to do. What your actual options are. The interface could use a lot of work and then actually understanding like what the challenge is. I didn't really know what I was doing. I just kind of moved the lamp to play around with the networks it was making which was cool and then I realized I could place oats and was pushing it around. I have no idea what the [salt] the snip did, but the core mechanism is kind of fun, but there needs to be a challenge. There's a weird red rectangle in the middle [the storm's scar]. ... sometimes things were flashing gray a bit.

**What he found fun, said plainly:** pushing the mould around with the lamp and watching the networks it makes; pulling it with oats; making connections between food, and building little bridges where there was no food at all. It feels like a real physical thing you can move. **What was missing:** knowing what to do, a reason, and a challenge.

## The game as it is today (`cargo run --release --bin mould`)

Code: `/home/user/Pixel_Physics/src/mould/` (`mod.rs` the model, `play.rs` scripted players, `render.rs` drawing, `lesson.rs` the new opening lesson), window `src/bin/mould.rs`, harness `examples/mouldprobe.rs`. Write-up with every measurement and what was tried: `/home/user/Pixel_Physics/Reports/mould-probe-2026-11-07.md` (read its sections on what was built and the 2026-11-08 sections; skip the rest unless you need a number).

- **The model** is Jones (2010)'s agent slime mould: about 600-1,800 "walkers" on a 256x160 dish, one per cell. Each senses a trail field at three points 9 cells ahead, turns toward the strongest, steps, and lays trail. Trail spreads and fades. Food piles give off a separate scent. The result is real Physarum-like behaviour: tubes that thicken where used, shortest-path-ish networks between food, exploring fingers, loops.
- **An economy:** food only reaches the mould through tubes. A breadth-first search runs along tube from every pile the mould is eating, out to 70 cells. Beyond that the mould is unfed: it can't divide, slowly dies, and its tube is drawn grey. Cut a tube and the far side starves visibly over seconds. Each pile feeds at most 40 eaters, so the mould's size is set by how much food it is joined to.
- **Tools:** Lamp (held, unlimited): since 2026-11-08 it *pushes* trail and walkers outward, so a tube bends around the light rather than breaking. Oats x5, one back every 15 s: small food that pulls the mould toward it. Salt x1: a wall line the mould can't cross. Snip x3: cut a tube.
- **A dish:** 90 s. Five food piles (a sixth and the far one arrive at 0:30 and 0:55). At 1:30 a storm cuts one tube, the one whose loss costs the most food, with a 5 s purple warning line. Three stars: join all the food; network no more than 1.15x the shortest possible; lose nothing to the storm (read after 8 s of regrowth).
- **New today (on a branch, about to land):** an opening six-step lesson (lamp, oats, join all, snip, salt, storm), a briefing card, labels on the storm line and scar.
- **Scripted hands, 5 dishes, stars:** do nothing 0, random 3, simple plan 8, scripted fresh player 9. The lamp alone never joins all the food. Hand-to-answer: oats pull the front over in 0.5-0.7 s; the lamp clears the mould under it in under 0.1 s.
- **Cost:** a frame is about 2.6 ms (worst 17 ms). The dish is small and bounded.

Pictures: the first page https://claude.ai/artifact/R4Yjkt4bsozfZDV1CkpcDc and the before/after page https://claude.ai/artifact/52KtqyTGjyhKMb7L14Eqq6 (you may not be able to open them; the report has the numbers).

## The project around it

Pixel Physics is a falling-sand engine (Rust) with powders, liquids, gases, fire, heat, weather, plants and ants. The mould probe is its own binary on its own private fields and touches no shared engine file. It **could** be put into the engine's world (sand, water, gas, fire, plants, ants exist there), which is one direction worth weighing, but every engine cost has to be named. The engine's main game is "the lab", a sealed box of soil with plants and ants.

Seven other prototypes were built the same week (lightning, mine gas, gas-eating mould, lava dam, ice wall, kudzu vine, tree genome); `/mnt/project-files/game-design/new-game-brainstorm-2026-11-07/` has the original brainstorm (`brief.md`, `explorer-b-growth.md`, `review-3-synthesis.md`), the Mould prototype brief (`prototypes/2-mould.md`) and the testers' review of Mould (`playtest-2026-11-08/mould-review.md`). One of them, **Seep** (`--bin seep`), already has a graph mould that eats mine gas; the owner said today that he connects to a slime mould more than to a gas.

Standing tastes of this owner (from CLAUDE.md): **it has to feel satisfying**, and two laws: an outcome is a spread, not a yes/no (a graded death, not a disappearance); and there must be a player verb that delivers something visible. Fun over fidelity: real biology is inspiration, not the bar. He judges by playing, reads on a phone, and wants plain words.

## Rules for every brainstormer

- **Do not edit anything in `/home/user/Pixel_Physics`.** Read code and reports there as much as you like. No git, no builds needed (you may run `cargo run --release --example mouldprobe ...` if a number would settle something, but keep it short and say what you ran).
- Write your output to the one file named in your brief, in this folder. `/mnt/project-files` is a network mount: use paths with a trailing slash for directories, never search the whole of `/mnt/project-files`, and read your file back about ten seconds after writing it to confirm it saved.
- Plain words, short sentences, no jargon without a gloss. Lead with your best three ideas and a one-line reason for each, then the detail. Be concrete: name the dish, what's on screen, what the player does in the first 30 seconds, and why it's fun or hard. Say for each idea what in the current code it reuses and what it would need that doesn't exist.
- Ground ideas in what the mould actually does in this model (read `src/mould/mod.rs`), not in what real Physarum does, where the two differ.
- About 1,500-3,000 words. Better five sharp ideas than twenty thin ones.
