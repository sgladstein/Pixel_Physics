# plantwean3: colonies learn to live on plants (2026-10-24)

Run by the Teacher lane on the VM (06:46-08:10Z, main ~06:40Z): 64 colonies x 32 rounds from hcs, `pilecap=45:5:10:10 brood=off std=off`, the body-rule scaffold weaned over rounds 8-32. Files: `plantwean3/` (colsel.csv, cg.csv, jobs.txt). Picture: `plantwean3-curve.png`. Earlier history: `findings-2026-10-23.md`.

**Result: by round 19 the common garden holds on plants plus a 10-cell pile, and keeps improving. The starting ant dies there.**

Founder control (run here on main 03a95e9d, `scratchpad/g/cgctl3.py`): hcs x30, sel=off, the same CG args and the same 64 worlds (seed c*1000+99), cap 10. Median alive 0, starved 24, nest 33, crater 58.

| common garden | cap | selected alive (median) | pairs better / worse than founder at cap 10 | nest selected vs founder | crater selected vs founder |
|---|---|---|---|---|---|
| r7 | 20 | 22 | (founder at cap 20 not run at this scale; pw3mini: 4.5) | 66 | 17 |
| r9 | 10 | 1 | 35 / 5 | 24 vs 32 | 53 vs 60 |
| r17 | 10 | 5 | 47 / 3 | 54 vs 33 | 23 vs 57 |
| r19 | 10 | 19 | 47 / 3 | 100.5 vs 33 | 30.5 vs 58 |
| r25 | 10 | 21 | 56 / 1 | 160 vs 33 | 11 vs 58 |
| r31 | 10 | 21 | 53 / 2 | 184.5 vs 33 | 11 vs 58 |

- The cap went 45 → 15 by round 8, then mostly 10 (the floor), with a few rounds back at 15. Training-round median starved: 65, 46, then 1-5 through round 8, 26-89 while the cap first hit 10 (r9-14), and 6.5-39 from round 17 on.
- **The crash and recovery at r9-r17 is the colonies adapting to the 10-cell pile.** The CG went 22 → 1 alive when the cap halved from 20 to 10, and took about 8 rounds to come back.
- **Digging came back with it.** The CG nest grew 24 → 184.5 (founder 33) and the crater fell 53 → 11, with the scaffold weaned out. That beats the in-round rules-on crater of 8-16.
- Still noisy: training-round starved never settled below the step threshold for long, so the cap stayed at 10-15.

## Next step (proposed)

plantwean4: today's standard pool (`assets/minds/std_pool.txt`) with the standard line (breeding on, grazing default-on), plus `plants=grass,herb,shrub,scrambler weed=30 fjoule=1 pilecap=45:5:10:0`. Floor 0 lets the pile go all the way, so the run answers whether a colony can live on plants alone. Before that, a cheap probe on plantwean3's final carry: CG at caps 5 and 0. If cap 0 already holds some colonies, floor 0 is safe; if not, use floor 5.

## Probe: plantwean3's final colonies with less pile (2026-10-24 ~09:30Z, local)

The round-31 carry of all 64 colonies, in the same common garden (rules off, sel=off, seeds c*1000+99), on main 03a95e9d. **Positive control:** at cap 10 the local runs reproduce the VM's r31 CG exactly on colonies 1-8 (alive and nest identical). Script: `scratchpad/g/cgprobe.py`.

| pile cap | alive median (mean) | colonies ≥ 12 alive | colonies ≥ 20 alive | colonies dead | nest | crater | plant share of field energy* | loads home |
|---|---|---|---|---|---|---|---|---|
| 10 (VM r31) | 21 | 45 | 36 | 10 | 184.5 | 11 | 0.57 (c1-8) | 3,710 (c1-8) |
| 5 | 4 (7.6) | 20 | 10 | 19 | 105 | 11 | 0.67 | 1,575 |
| 0 | 0 (0.1) | 0 | 0 | 60 | 32 | 4 | 0.96 | 196 |
| founder, cap 10 | 0 | | | | 33 | 58 | 0.39 | 109 |

*Bites not worth 960 J (a pile cell) over all field bites by energy, so crumbs count as plant. Rough.

- **No colony lives on plants alone yet.** At cap 0 only 4 of 64 colonies keep anyone alive (1-2 ants). The ants keep biting plants in the field (2,052 bites) but carry almost nothing home (196 loads, against 1,575 at cap 5). The larder is still filled from the pile.
- **Cap 5 splits the colonies.** A third keep at least half their ants and a sixth hold 20 or more, so selection has something to act on there.
- **Floor for plantwean4: 5.** The colsel rule (lower FLOOR to 0 only once a garden at cap 0 holds) says not yet.
- What blocks plants-only is **carrying plant food home**, not eating it. A leaf pays 1/24 of a pile cell, so a forager would need ~24 loads to bring home what one pile cell gives. Candidates: a CG case for plant loads home, or a bed where the plants grow nearer the door.

# plantwean4: the standard pool with breeding (2026-10-24, VM 10:30-12:40Z; judged ~13:50Z)

std_pool, standard defaults (breeding on), lesson c1:0.1, `plants=grass,herb,shrub,scrambler weed=30 fjoule=1 pilecap=45:5:10:5`, 64 colonies x 32 rounds. Files: `plantwean4/`.

- **It never weaned.** The cap went 45 → 25 by round 8, then bounced 25 ↔ 20 for 23 rounds. At 25, median starved was 6-17 (step down). At 20 it was 43-65, more than 30, so it stepped straight back up. With breeding, starvation tracks the pile cut, not foraging. Colonies dying out per round: 2-9% at 25, 11-25% at 20.
- **The CG excludes dead colonies** (no carry), so its medians are survivors only. Counting them as 0 alive, the CG at cap 20 is 62 (r9) → 65.5 (r31): no progress.
- **Paired control: plantwean4's selection made colonies worse than the pool.** Local, main 7dd6b0d4, CG worlds 1-16. The local rerun matches the VM's r31 CG exactly on c1, c2 and c4. std_pool itself at cap 20 keeps a median 80.5 alive (1 of 16 dead), nest 309.5. r31's colonies keep 71.5 alive (4 dead), nest 116; worse in 10 of 16 pairs.
- **Where the pool fails** (std_pool, CG worlds 1-16):

| cap | median alive | dead of 16 | starved | nest | plant share of field energy |
|---|---|---|---|---|---|
| 20 | 80.5 | 1 | 22.5 | 309.5 | 0.50 |
| 10 | 0 | 10 | 191.5 | 66.5 | 0.61 |
| 5 | 0 | 14 | 115.5 | 21 | 0.58 |
| 0 | 0 | 16 | 24.5 | 34.5 | 0.97 |

So the cliff is between 20 and 10, and plantwean4 never trained there. Compare plantwean3's line (hcs, no breeding), which holds at cap 10 (21 of 24 alive).

**Fix (PR #855):** colsel `pilegate=extinct:LOW:HIGH` steps the cap on the share of colonies that died out. With 10:25 it would have held plantwean4 at 20 from round 9. Job `plantwean5.txt`: plantwean4's line with `pilecap=30:5:10:5 pilegate=extinct:10:25`. **Judge it against std_pool in the same worlds, and on nest as well as alive:** plantwean4 lost digging (nest 310 → 116) while selecting at the cliff.
