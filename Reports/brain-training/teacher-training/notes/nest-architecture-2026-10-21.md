# Real ant nests in vertical section: notes for an "ideal nest" blueprint

Written 2026-10-21 by a research agent for the teacher design ([`../teacher-design-2026-10-21.md`](../teacher-design-2026-10-21.md)) and the dig ladder (`Reports/brain-training/ladder/dig-rung-design.md` sections 3-4). 

## Verification tags

| Tag | Meaning |
|---|---|
| **[FT]** | Read in the paper's full text, retrieved through the PubMed Central connector |
| **[ABS]** | Read in the abstract (PubMed or SciSpace record) |
| **[WS]** | From a web-search summary only. The page was not opened, because the session's proxy refused direct fetches of pnas.org, ncbi and others. Treat as secondary |
| **[DER]** | My own arithmetic or unit conversion from tagged numbers |
| **[GK]** | General knowledge, not sourced in this session |

**Scale used throughout.** BL is worker body length. The game ant is `body: Chain(2)` (`src/sim/creature.rs:3761`, `:3786`), so **1 BL = 2 cells**. The training box has about 60 soil rows, which is about **30 BL of soil**, and 256 columns, which is **128 BL**. The owner's legibility ruling is *"passage ~4"* cells, which is 2 BL (quoted at `creature.rs:3763`).

---

## 0. The six things that matter for the blueprint

1. **Real chambers are flat lenses, not rooms.** Height is about 1 to 2 BL and does not grow as the chamber grows. Width runs from 4 BL to 25+ BL. The floor is horizontal and the roof is flat or slightly domed. P. badius chambers are "about 1 cm high, floor to ceiling, no matter what the floor area" [FT]. Camponotus socius chambers are 1.0 to 1.5 cm, "thinner at the outer edges than at the connection to the shaft" [FT]. *The ladder's planned 8-16-cell-tall chamber is 4-8 BL tall, three to five times taller than any real chamber.* A real-looking chamber in this game is about 2-4 cells tall at true scale. Drawn twice as tall for legibility, it is about 4-8 cells tall. In both cases it should be **at least 3x as wide as it is tall**, which also meets the "at least as wide as tall" spec.
2. **Tunnels are about one body length wide.** Fire-ant incipient tunnels measure D = 1.06 ± 0.23 L. Below 1.31 L, a falling ant can always jam itself and stop [ABS]. P. badius shafts are "a little less than 1 cm", about 1.1-1.5 BL, flattening to "up to about 2 cm" (about 2.5 BL) near the surface [FT]. So the owner's 4-cell (2 BL) passage is inside the real range for upper-nest shafts.
3. **The basic unit is the "shish-kebab": one descending shaft with horizontal chambers hung on it.** Tschinkel's generalisation from about 20 cast species: nests are made of chambers and shafts. Variation in their size, spacing and joining produces the species differences. Big nests add more shaft-chamber units. Overall shape does not change as a nest grows. Ants are denser toward the bottom [FT].
4. **Most nests are top-heavy.** P. badius has about half its chamber area in the top quarter [FT]. F. pallidefulva volume declines exponentially with depth [ABS]. Chamber spacing grows with depth. *Exception:* Aphaenogaster floridana is bottom-heavy, with its largest chambers in the 6th-7th deciles [FT].
5. **Nest size tracks worker count, slightly sub-linearly, and shape stays the same as it grows.** For P. badius, area ∝ workers^0.87 and depth ∝ workers^0.37 [FT]. In 2D lab arenas the relation is close to linear [ABS].
6. **Spoil is placed so it cannot roll back into the hole.** Dorymyrmex workers dump "just beyond the ridge of the pile". 18-27% of loads roll down the **outer** slope, and "almost never" does a load roll back to the entrance [ABS]. Pheidole oxyops read the pile's slope and drop where it turns from up to down [ABS]. Several species dump in a one-sided crescent at a fixed distance [ABS].

---

## 1. Tschinkel's casts: field nests in vertical section

### 1.1 Pogonomyrmex badius, the Florida harvester ant (Tschinkel 2004, *J Insect Sci* 4:21, [doi:10.1093/jis/4.1.21](https://doi.org/10.1093/jis/4.1.21), PMC528881) [FT]

**Body length.** Workers are polymorphic, about 6.35 mm (minor) to 9.5 mm (major) [WS: Wikipedia/Edisto, consistent]. **1 BL ≈ 0.8 cm** is used below.

**Shaft**
- There is usually one entrance, sometimes 2-3. The shaft leaves the entrance at 20-30° from horizontal and steepens to 45-60° by about 50 cm. The abstract says it reaches about 70° below 50 cm [FT].
- It spirals, usually to the left, as a **helix 4-6 cm across (about 5-8 BL)**. Pitch is 8-10 cm per turn near the surface and about 20 cm deeper [FT]. In a vertical slice the helix reads as a zig-zag of 5-8 BL amplitude [DER].
- Diameter is "a little less than 1 cm" (about 1.2 BL). Upper shafts are flattened ovals, "racetrack" or "twisted ribbon" in shape, up to about 2 cm (about 2.5 BL) [FT].
- Branches always start less than 40 cm deep. The first branch is at 12-17 cm and the second at 26-33 cm. There are 1 to 4 shaft-chamber series, rarely 5 [FT].

**Chambers**
- Superficial chambers (< 10-15 cm deep) are near-horizontal shafts widened into "complex, lobed, interconnecting, looping chambers", with lobes on **both** sides. They "often lie within the soil excavated from deeper and dumped around the nest entrance" [FT].
- Deeper chambers (below 15-20 cm) **always begin as lateral, horizontal-floored extensions from the outside of the descending helix**. The shaft meets the chamber's inner edge at 25-70°. A chamber starts as a circular niche and becomes lobed as it is enlarged. Its perimeter is 2-7x that of a circle of equal area in the upper third [FT].
- In section a chamber is "flattened or slightly domed, with the horizontal dimension much greater than the vertical". Height is about 1 cm (about 1.2 BL) at every area [FT].
- **Spacing**: 2-4 cm vertical separation near the surface (about 2.5-5 BL), rising to 20-30 cm deeper (about 25-40 BL). The widest gap is at 70-80% of maximum depth. Spacing is the same at every nest size [FT].
- **Size with depth**: chambers near the surface average 5-6x the area of those near the bottom. Each depth decile holds 25-40% less area than the one above (abstract). The body text gives "overall each decile averaged half the area of the one above it". Decile area falls as depth^-0.59 [FT].
- Superficial chamber mean area runs from 26 cm² in the smallest colonies to about 330 cm² in the largest [FT]. In a vertical slice that is about 6-20 cm wide (7-25 BL) against a 1 cm (1.2 BL) height, so **width:height is about 6:1 to 20:1** [DER].

**Contents by depth** (Tschinkel 1999 *Ecol Entomol*, [doi:10.1046/j.1365-2311.1999.00184.x](https://doi.org/10.1046/j.1365-2311.1999.00184.x) [ABS]; 2004 [FT]; Tschinkel & Hanley 2017 PLoS ONE [ABS])
- **Foragers** are in the top 15 cm only. **Seeds** are stored at an absolute depth of 40-100 cm. Colonies shallower than 100 cm store seeds in their deepest chambers. **Brood and callow workers** are mostly in the bottom third. Older workers move upward as they age.
- **Density is highest at the bottom**: workers 2-8 per cm² and brood 2-25 per cm². Brood-holding chambers hold 20-40 brood in the top half and 60-110 in the bottom half, with 90-130 workers in each bottom-half chamber [FT].
- In game terms: the deep, small chambers are packed nurseries. The big shallow ones are sparse lobbies and forager staging.

**Scaling with colony size** [FT]
- log TotalArea(cm²) = 0.55 + 0.87 log(dark workers).
- log MaxDepth(cm) = 0.95 + 0.37 log(dark workers).
- A 10x rise in workers gives about 7.5x area and about 2.4x depth.
- Incipient nests are 29-37 cm deep (abstract: 40-50 cm). Mature nests are 2.5-3.0 m deep (deepest 3.06 m) with 5-150 chambers.
- Mean workers by shaft series: 1 series 1700, 2 series 3200, 3 series 4600, 4 series 5000.
- Each added chamber adds a mean of 98 cm² in the first series and 43 cm² in the fourth.
- **[DER] at 12 workers**: area ≈ 31 cm², depth ≈ 22 cm ≈ **28 BL**, which is about the box's depth. At 24 workers: area ≈ 56 cm², depth ≈ 28 cm ≈ 36 BL. Both are extrapolations below the sampled range.
- Excavation rate: old workers dig about 0.45 cm² of chamber plus 0.13 cm of shaft per worker-day. A whole nest takes 3-6 days. Sand removed ranges from about 40 g (incipient) to about 40 kg [FT].

**Surface: a flat disc, not a cone** (Tschinkel 2015 *PLoS ONE* biomantling, [doi:10.1371/journal.pone.0120407](https://doi.org/10.1371/journal.pone.0120407), PMC4368782) [FT]
- Soil goes into a disc around the entrance, decorated with charcoal bits. Disc area ranges 80-5,700 cm² with a mean of 1,540 cm², which is a diameter of **10-85 cm, mean about 44 cm ≈ 55 BL** [DER].
- Nest volume brought up is 0.1-12 L. log Vol = 0.098 + 1.12 log DiscArea; MaxDepth = 71.3 + 0.049 × DiscArea.
- **[DER]** Mean volume over mean disc area gives a mean disc thickness of about 2 cm ≈ 2.5 BL. The disc is a thin blanket.
- A disc spreads. Sand from a 20 cm test disc had moved more than 50 cm from its edge after a year.
- About 2.5% of excavated sand stays below ground, mostly in the top 30-40 cm, as backfill (Tschinkel, Rink & Kwapich 2015, [doi:10.1371/journal.pone.0139922](https://doi.org/10.1371/journal.pone.0139922)) [ABS].
- Sand travels up the nest in relays, so it is put down and picked up again on the way [ABS].

**Rebuild fidelity.** Colonies move about once a year and rebuild essentially the same nest in under a week. The CO2 template hypothesis failed (Tschinkel 2013 PLoS ONE, [doi:10.1371/journal.pone.0059911](https://doi.org/10.1371/journal.pone.0059911)) [ABS]. Tschinkel 2017 *Ecology* asks whether colonies have a "plan" ([doi:10.1002/ecy.1739](https://doi.org/10.1002/ecy.1739)) [ABS, title only].

### 1.2 Aphaenogaster floridana, A. treatae, A. ashmeadi (Tschinkel 2011, *J Insect Sci* 11:105, [doi:10.1673/031.011.10501](https://doi.org/10.1673/031.011.10501), PMC3281374) [FT]

**Body length.** Workers are 6.6, 6.2 and 5.2 mm across the three species. The extraction lost which value belongs to which species. Use **1 BL ≈ 0.6 cm**.

- Nests are simple, small and vertical, "all chambers lie directly below the nest entrance". Chambers are horizontal and the connecting shafts vertical.
- There is **always a small chamber immediately below the surface**.
- **Descending shafts leave from a chamber's EDGE. The shaft arriving from above enters at the chamber's CENTER.** The proposed rule: dig down from the edge, then dig radially outward equally in all directions at the bottom, so the new chamber centres on the arriving shaft. In a vertical slice this gives a **stair-step stack** [DER].
- In A. floridana, chambers in the top quarter are joined by **2-3 parallel shafts**. Lower down, single shafts are the rule.
- Size ranges:

  | Species | Chambers | Total area | Depth | Mean depth |
  |---|---|---|---|---|
  | A. floridana | 2-11 | 16-228 cm² | 13-92 cm | 41 cm |
  | A. treatae (per the extraction) | 2-7 (9) | 11-80 cm² | 8-22 cm | |
  | A. ashmeadi | 1-4 | 10-69 cm² | 7-25 cm | |

- Mean chamber area is 9.5-13 cm² in all three species. As a circle that is about 3.5-4 cm across, about 6-7 BL [DER].
- **A. floridana is bottom-heavy**, with the most area in deciles 7-8. Spacing grows with depth. Lower chambers are more circular. The other two species are vertically uniform.
- Depth vs levels: max depth = −10.2 + 8.68 × levels (cm), about 14 BL per level [DER].
- **Scaling (A. floridana)**: +0.057 cm² floor per worker, +0.09 cm depth per worker, +1 chamber per 100 workers. Area per worker falls from about 0.8 cm² (small) to about 0.2 cm² (large).
- **Crowding**: worker silhouettes cover 3-5% of floor at 10 workers and 23-28% at 400. With legs that is 12-15% and 70-80%. Formica pallidefulva, Pogonomyrmex badius and Camponotus socius hold steady at about 6-8%.
- **Fit to the box at true scale [DER]**: A. ashmeadi and A. treatae nests are about 12-45 BL deep. This is the only real nest family that fits the 30-BL box without compression.

### 1.3 Camponotus socius (Tschinkel 2005, *J Insect Sci* 5:9, [doi:10.1093/jis/5.1.9](https://doi.org/10.1093/jis/5.1.9), PMC1283890) [FT]

**Body length.** Minors 7.5-10.5 mm and majors 11-16 mm [WS: AntWiki/Wikipedia]. Minor head width is 1.45-1.65 mm [FT]. Use **1 BL ≈ 0.9 cm** (minor).

- 2-10 chambers along a shaft at 45° to nearly 90°. It is shallower near the surface and "wanders in a vaguely zig-zag or helical manner". 5 of 24 nests had 2 shafts, always branching in the upper part of larger nests.
- Chamber height is 1.0-1.5 cm (about 1.1-1.7 BL). Chambers are **thinner at the outer edge than at the shaft**, so in section they read as a wedge or lens tapering away from the shaft.
- **Upper chambers are elongate and tunnel-like, and "often angled upward to just beneath the soil surface, but never broke through".** Deeper ones are compact. Large chambers become lobed.
- Spacing is 2-14 cm, mean about 4 cm rising to 6.5 cm with depth (about 4.5-7 BL) [DER]. Depth runs 15-66 cm (about 17-73 BL).
- **Scaling**: Area = 0.70 × workers + 67.9 cm² for 2-590 workers (R² 84%). Area ranges 26-467 cm² and volume 70-765 cm³. Chamber count goes 2→10 and mean chamber area 13→47 cm².
- **Polydomous**: on average 2.3 nests per colony.

### 1.4 Other casts (supporting)
- **Odontomachus brunneus** (Cerquera & Tschinkel 2010, [doi:10.1673/031.010.6401](https://doi.org/10.1673/031.010.6401)) [ABS]: a single vertical shaft through simple horizontal chambers. 11-177 workers, 2-17 chambers, 28-340 cm², depth 18-184 cm. **About 2 cm² of floor per worker at all sizes.**
- **Formica pallidefulva** (Mikheyev & Tschinkel 2004, *Insectes Soc* 51:30, [doi:10.1007/s00040-003-0703-3](https://doi.org/10.1007/s00040-003-0703-3)) [ABS]: shallow, 30-45 cm. Near-vertical shafts bear chambers, and shafts are the modular growth unit. Top-heavy, with volume falling exponentially with depth. Volume vs workers R² = 0.87. Rule: "chambers are formed in the direction of the tunnels leading up to them".
- **Solenopsis invicta**, the fire ant (Cassill, Tschinkel & Vinson 2002; Tschinkel 2018 *Myrmecol News*) [WS]: the mound is "a dense matrix of narrow tunnels". Shafts cross the root zone (10-20 cm). Chambers average about 5 cm² at 10-80 cm depth, about 200 workers per chamber, hundreds of chambers in a large nest. Fire ant BL is 3.5 ± 0.5 mm [ABS, Gravish 2013].
- **Prenolepis imparis** (Tschinkel 1987, *Insectes Soc* 34:143) [WS]: no chambers shallower than 60 cm, deepest about 3.6 m. A single descending tunnel with meshing near the surface.
- **Pogonomyrmex barbatus** (Gordon 2025, *Ecol Evol*, [doi:10.1002/ece3.72122](https://doi.org/10.1002/ece3.72122)) [ABS]: an **entrance chamber is the hub**. It is surrounded by 1-6 adjacent chambers not linked to each other, with 2-3 tunnels leading to "strings of widely spaced chambers descending 1-2 m". The topology is a minimum spanning tree.
- **Pogonomyrmex occidentalis in a glass-sided enclosure**, the closest real analogue of an ant farm (Halfen & Hasiotis 2010, *PALAIOS* 25:703, [doi:10.2110/palo.2010.p10-005r](https://doi.org/10.2110/palo.2010.p10-005r)) [ABS]: 50 ants over 6 weeks in silty clay. "Numerous, intricately interconnected galleries and chambers". **Galleries about 0.9 cm in diameter, chambers about 3-10 cm long and up to 1.5 cm high.** Worker BL is about 6.3-8.0 mm [WS], giving galleries about 1.2 BL and chambers 4-14 BL long by ≤2 BL high [DER]. In the 1,000-ant enclosure, about 51% of excavated sediment went back into the nest as backfill or lining (Halfen & Hasiotis, IUSS 2010) [ABS].

### 1.5 *Ant Architecture* (Tschinkel 2021, Princeton UP) [WS]
This is the book form of the casting work: plaster, molten metal or wax casts, 248 pp, PROSE award 2022. I did not read its contents. The quantitative statements above come from the primary papers it is built on.

---

## 2. Two-dimensional laboratory nests (the closest match to the game)

| Study | Species / setup | Finding [tag] |
|---|---|---|
| **Toffin et al. 2009** *PNAS* 106:18616, [doi:10.1073/pnas.0902685106](https://doi.org/10.1073/pnas.0902685106) | Messor sancta in a 2D sand layer, digging out from a central hole | "the primary circular cavity evolves into a ramified structure through a branching process". The shift happens at every group size but **more often with more workers**. A stochastic model makes **density** the driver [ABS]. Gautrais et al. 2014 [FT] summarise the mechanism: high worker density along the small initial wall keeps digging uniform, so the cavity stays round. Once the cavity passes a critical size, density at the wall falls, local digging amplifies, and buds appear. A 2025 manuscript describes the substrate as 2 mm thick [WS, unconfirmed] |
| **Toffin, Kindekens & Deneubourg 2010** *Proc R Soc B* 277:2617, [doi:10.1098/rspb.2010.0176](https://doi.org/10.1098/rspb.2010.0176) | same, with cohesive vs granular substrate | **Two successive transitions: (1) budding along the perimeter of the circular cavity, (2) tunnelling of galleries.** Both occurred in 96% of cohesive runs and 50% of granular runs. Transitions and the stop of digging follow **dug-area thresholds**. Shape thresholds are lower in cohesive substrate, and the stop threshold is the same for both materials [ABS]. The threshold values in cm² were not retrieved |
| **Buhl et al. 2004** *Naturwissenschaften*, [doi:10.1007/s00114-004-0577-x](https://doi.org/10.1007/s00114-004-0577-x) | M. sancta, 50-200 workers, 3 days | Excavation is non-linear: exponential growth, then saturation. Final volume rises with workers. Networks are planar graphs with power-law degree. More ants give more edges and vertices, plus "enlarged and highly connected vertices" [ABS] |
| **Buhl et al. 2005** *Behav Ecol Sociobiol*, [doi:10.1007/s00265-004-0906-2](https://doi.org/10.1007/s00265-004-0906-2) | M. sancta, thin sand disc | Logistic excavation curve. "The total volume of excavated sand was almost proportional to the number of workers" [ABS] |
| **Gautrais et al. 2014** *PLoS ONE* 9:e109436, [doi:10.1371/journal.pone.0109436](https://doi.org/10.1371/journal.pone.0109436) | M. sancta, sand disc under glass, 50/100/200 ants (n = 5/5/18), dug inward from the rim | Meshedness of final networks was **0.0, 0.092, 0.194** at 50, 100 and 200 ants. In the boundary-free model, meshedness rises with N and plateaus above about 2,000 ants. New tunnels start along existing walls at a constant per-length rate, at right angles to the parent tunnel. Its Table 1 compiles field nests running from a single vertical tunnel (9-300 workers), through stacked chambers on one tunnel (368-1,000), to highly meshed nests (2,500 to 500,000) [FT; the numeric parameters were stripped in extraction] |
| **Rasse & Deneubourg 2001** *J Insect Behav*, [doi:10.1023/A:1011163804217](https://doi.org/10.1023/A:1011163804217) | Lasius niger | Final volume and digging rate are "almost proportional" to the number of ants. Adding ants restarts digging until the volume is adjusted. Digging acts as negative feedback [ABS] |
| **Halley, Burd & Wells 2005** *Insectes Soc*, [doi:10.1007/s00040-005-0818-9](https://doi.org/10.1007/s00040-005-0818-9) | Linepithema humile, 10/100/1,000 workers, X-ray CT | Density-dependent stimulation of digging that subsides as the nest grows. The 1,000-worker nests were **not** 10x the 100-worker ones. Surface area grows allometrically with volume [ABS] |
| **Minter, Franks & Robson Brown 2012** *J R Soc Interface* 9:586, [doi:10.1098/rsif.2011.0377](https://doi.org/10.1098/rsif.2011.0377) | Lasius flavus, micro-CT, sub-colonies of over 1,000 workers | Ants dig **horizontal tunnels along planes between sediment layers**. Tunnel dimensions are colony-specific. Dynamics fit negative feedback, and tunnels compete for miners. Per Gautrais 2014 [FT]: from a vertical starter shaft in homogeneous substrate, ants branched **2-4 tunnels slanting downwards** [ABS + FT] |
| **Khuong et al. 2016** *PNAS* 113:1303, [doi:10.1073/pnas.1509829113](https://doi.org/10.1073/pnas.1509829113) | Lasius niger *building*, not digging | A pheromone in the building material amplifies deposition, producing regularly spaced pillars. Ants **use their body size as a template for the height at which they start to roof over pillars**. Caps merge into a roof [ABS]. The pillar height in mm was not retrieved |
| **Gravish et al. 2013** *PNAS* 110:9746, [doi:10.1073/pnas.1302428110](https://doi.org/10.1073/pnas.1302428110) | S. invicta, BL = 3.5 ± 0.5 mm | Tunnel diameter is about body length, incipient D = 1.06 ± 0.23 L. Below D_s = 1.31 L, falls are always arrested by jamming. Ants move > 9 BL/s in tunnels [ABS] |
| **Gravish et al. 2012** *J R Soc Interface* 9:3312, [doi:10.1098/rsif.2012.0423](https://doi.org/10.1098/rsif.2012.0423) | S. invicta, quasi-2D arenas of wetted glass beads | Large workers dig wider but shorter tunnels. Network topology (edge-vertex scaling, degree distribution) is the same for all sizes. Excavated amount tracks active workers [ABS] |
| **Monaenkova et al. 2015** *J Exp Biol* 218:1295, [doi:10.1242/jeb.113795](https://doi.org/10.1242/jeb.113795) | S. invicta, X-ray CT, particle size × moisture | Ants nest in anything but fully dry or saturated substrate. Volume is insensitive to moisture. **Tunnels are deepest at intermediate moisture, tracking cohesive strength.** Pellet size is the same across substrates, set by load and by passing in crowded tunnels [ABS] |
| **Römer & Roces 2015** *Insectes Soc*, [doi:10.1007/s00040-015-0419-1](https://doi.org/10.1007/s00040-015-0419-1); **Fröhle 2009** thesis | Acromyrmex lundi | High worker density stimulates digging. Workers first **over-excavate, then refill with pellets**. Fröhle: chambers and tunnels are trimmed back with pellets "until a distance was reached at which perhaps two ants could pass one another" [ABS] |

**Ant farms (formicaria).** There is little peer-reviewed material on thin-slab nests specifically. The best analogue is Halfen & Hasiotis's glass-sided P. occidentalis enclosure above.
- An EGU 2015 abstract on L. niger in artificial formicaria [WS] reports that tunnel width tracked antenna spread, chambers took more excavated volume than tunnels, and excavation fell with depth.
- A field study of *Lasius neoniger* [WS] found galleries 1.5-5 mm across and chambers 10-20 mm by 30-50 mm, mostly in the top 0.3 m with a few vertical galleries to about 0.7 m. Nest volume was 20-250 cm³.
- L. niger BL is 3.5-5.0 mm [WS, AntWiki]. That converts to galleries about 0.4-1.2 BL and chambers about 2.5-5 by 7-12 BL, mostly in the top 70 BL [DER].
- An older *Insectes Sociaux* paper [WS, author unconfirmed] says L. niger tunnels are "longer, more branched, and less straight" than those of other species.
- Hobby reports of Uncle Milton farms with P. occidentalis [WS] are anecdotal: ants dig one tunnel system, abandon it and dig another.

**Synthesis for the "round room vs galleries" question.** One shape does not replace the other. With crowding, a nest **starts** as one round cavity and **turns** into a branched one as it grows. Density along the wall decides when. More workers make the switch more likely. Cohesive substrate makes it happen sooner, at a smaller dug area. A **graded** outcome (round room → buds → galleries → meshed galleries) is the realistic outcome, not a binary one.

---

## 3. Surface spoil: shape, distance, and what keeps it out of the hole

| Species | Shape | Distance | Rule [tag] |
|---|---|---|---|
| **Dorymyrmex** sp. (Tofilski & Ratnieks 2005, *J Insect Behav*, [doi:10.1007/s10905-005-5608-8](https://doi.org/10.1007/s10905-005-5608-8)) | Circular ring (crater) around the entrance | Not retrieved | Workers dump **"just beyond the ridge"** of the pile. 81.9% / 73.0% of loads stay put and 17.9% / 27.0% roll down the **outer** slope. Ants "almost never dumped in a way that resulted in the load rolling back to the entrance". If one side is removed, ants dump on the flattened side and restore the ring [ABS] |
| **Pheidole oxyops** (Tofilski & Ratnieks 2008, *Insectes Soc*, [doi:10.1007/s00040-007-0967-0](https://doi.org/10.1007/s00040-007-0967-0)) | **Crescent** pile, perpendicular to and about **13 cm from** the entrance | 13 cm | The hypothesis is that ants drop "near the top of a pile where the slope changes from upward to downward, to prevent material rolling back towards the entrance". On a pile scaled 0 (inner edge) to 1 (outer edge), the mean drop point was 0.458 tilted down, 0.53 level and 0.626 tilted up. **They read the slope** [ABS] |
| **Trachymyrmex septentrionalis** (Tschinkel & Bhatkar 1974, *Environ Entomol* 3:667, [doi:10.1093/ee/3.4.667](https://doi.org/10.1093/ee/3.4.667)) | **One-sided crescent**, arc about 75°, widening with time | Mean **8.0 cm** from the hole | A removed mound is rebuilt at the same orientation and distance. The cue is visual. If local slope is altered, the mound is reoriented **downslope** over days [ABS] |
| **Cataglyphis bicolor** (Wehner 1970, *Insectes Soc*, [doi:10.1007/BF02223069](https://doi.org/10.1007/BF02223069)) | Irregular crater | Closer to the hole at night | **Each digger has a constant preferred direction** for carrying sand out to the crater's edge, set by visual landmarks. *Camponotus compressus* builds very regular craters and platforms "that ramp up from inside to outside" [ABS, French] |
| **Pogonomyrmex badius** (see 1.1) | Flat disc, about 2 cm thick on average [DER], charcoal-decorated | 10-85 cm diameter | The disc is wide and thin, and weather spreads it [FT] |
| **P. occidentalis** (Cole; Wikipedia/AntWiki) | Large gravel cone with a cleared disc | No dimensions retrieved | "irregularly conical", and base diameter indicates colony size [WS] |

**Inside the nest.** Not all spoil leaves. The fractions:
- P. badius field nests: about 2.5% stays below ground [ABS].
- P. occidentalis in a lab enclosure: about 51% backfill or lining [ABS].
- Acromyrmex: over-excavated space is refilled with pellets [ABS].

**Translation to the game's failure** ("ants dug into the mound and soil spilled back into the nest"):
- The real solution is geometric. The **crest** stands between the hole and the deposit, and loads go on the **far** side of the crest. The inner slope between hole and crest is short and receives nothing new.
- Several species make the pile one-sided (a crescent) and push it downslope.
- The ladder's own trace (dig-rung-design, step 3/4 notes) found a **different** source: falling grains came from **rim cuts**. Cutting a rim cell two columns from the shaft dropped the shaft's shoulder into the chamber, not from put-downs. The literature above covers rolling of dumped loads, not rim collapse. I found nothing on how real ants keep the entrance rim from caving in loose sand, beyond the role of soil cohesion: Monaenkova 2015 found tunnels deepest at the moisture of maximum cohesion.
- **[DER/GK] Rule of thumb for a 2D grain sim**:
  - Dry sand rests at about 30-35° [GK]. A cell automaton with a 1:1 diagonal slide rests at about 45°.
  - A mound of height h with crest at radius r_c has an inner toe at about r_c − h/tan(θ).
  - Keep that toe at least 1-2 BL outside the shaft's lip: **r_inner_toe ≥ shaft_half_width + 1 BL**.
  - For 45° that means r_c ≥ shaft_half_width + 1 BL + h.
  - Put down only at r ≥ r_c, on the outer flank.

---

## 4. Scaling summary (for 12-24 ants and beyond)

| Quantity | Law | Source |
|---|---|---|
| P. badius chamber area | ∝ workers^0.87 (log A = 0.55 + 0.87 log W) | [FT] |
| P. badius max depth | ∝ workers^0.37 (log D = 0.95 + 0.37 log W) | [FT] |
| P. badius shaft series | 1 at ~1,700 workers, 2 at ~3,200, 3 at ~4,600, 4 at ~5,000 | [FT] |
| P. badius nest *shape* | the same at all sizes; spacing vs depth the same | [FT] |
| A. floridana | +1 chamber per 100 workers; +0.09 cm depth per worker; +0.057 cm² per worker | [FT] |
| C. socius | A = 0.70 W + 67.9 cm² | [FT] |
| O. brunneus | ~2 cm² floor per worker, constant | [ABS] |
| 2D lab (M. sancta, L. niger) | excavated volume ≈ proportional to N | [ABS] |
| Argentine ant (3D CT) | sub-linear (1,000 ants < 10x of 100) | [ABS] |
| Meshedness (2D, M. sancta) | 0 / 0.09 / 0.19 at 50 / 100 / 200 ants; plateau above ~2,000 (model) | [FT] |
| Crowding | ant bodies cover 6-8% of floor in most species; Aphaenogaster 3-5% → 23-28% as colonies grow | [FT] |

**[DER] Open area per ant in the game.** A game ant covers 2 cells. Real crowding has bodies covering 6-25% of floor. That gives about **8-33 open cells per ant**:
- 12 ants: about 100-400 cells, or 25-100 BL².
- 24 ants: about 200-800 cells.
- The most typical value, about 8%, is about 25 cells per ant: **about 300 cells for 12 ants and 600 for 24**.
- For scale, one 4-cell-tall by 24-cell-wide chamber is 96 cells.

---

## 5. Image sources the owner can look at (described, not downloaded)

None of these pages was opened from this session, because the proxy blocks direct fetches. URLs come from the search indexes and DOI records above.

1. **Tschinkel 2004, P. badius nest casts.** Open-access paper with the full-height plaster cast of a mature nest about 2.5-3 m tall. It shows the helical shaft, wide lobed upper chambers and sparse small deep ones, plus small, medium and large casts at one scale. https://academic.oup.com/jinsectscience/article/4/1/21/886534 (PMC528881).
2. **Tschinkel 2011, Aphaenogaster casts.** All casts at one scale, grouped by number of levels, with stereo pairs in the appendices. It shows the stair-stepped edge-to-centre shaft pattern and the small chamber just under the surface. https://www.ncbi.nlm.nih.gov/pmc/articles/PMC3281374/
3. **Tschinkel 2005, Camponotus socius casts.** 24 casts in stereo pairs. Note the upper chambers that curl upward toward the surface, and the lobed deep chambers. https://www.ncbi.nlm.nih.gov/pmc/articles/PMC1283890/
4. **Toffin et al. 2009, Figure 2.** The 2D Messor nests: a round cavity becoming a budded, branching gallery system. https://pmc.ncbi.nlm.nih.gov/articles/PMC2773997/figure/F2
5. **Gautrais et al. 2014, Figure 4.** Final 2D tunnel networks dug by 50, 100 and 200 Messor workers, from tree-like to meshed. https://journals.plos.org/plosone/article?id=10.1371/journal.pone.0109436
6. **Core77 on Tschinkel's aluminium casts.** Photographs of molten-aluminium casts, which read like silver trees with flat leaves and are the most legible "shaft with chambers" image. https://core77.com/posts/23607/walter-tschinkels-aluminum-casts-of-ant-colonies-reveals-insect-architecture-23607

Extra:
- Fire-ant aluminium cast (SAIC Earthly Observatory): https://sites.saic.edu/earthlyobservatory/artists/walter-tschinkel/
- Prenolepis imparis plaster cast (Harvard Forest): https://harvardforest.fas.harvard.edu/node/802
- CNRS photo report of Lasius niger pillar-and-roof building (Khuong): https://images.cnrs.fr/en/photo/20170018_0009

---

## 6. Gaps and cautions

- **No full text of Toffin 2009/2010, Khuong 2016 or Mikheyev 2004 was retrievable** (PMC returned empty full text, and the proxy refused the publishers). So the area thresholds for the round→bud→gallery transitions, the Khuong pillar height, and F. pallidefulva chamber dimensions are **not** given numerically here.
- **Minter et al. 2012 is Lasius *flavus*, not L. niger.** I found no "Minter 2012 Lasius niger 2D" paper. The 2D L. niger work is Rasse & Deneubourg 2001, and the building work is Khuong 2016.
- **Messor sancta body length was not found.** Messor workers are generally 4-10 mm and polymorphic [WS]. The Messor blueprint is therefore expressed in BL by analogy.
- **Body lengths are [WS]** except fire ant (Gravish [ABS]) and Aphaenogaster ([FT], values not assignable to species).
- **Field nests are 10-100x deeper in BL than the 30-BL box.** Every blueprint below is given at *relative* depth (fraction of the nest's maximum depth D) so it can be fitted to the box. Absolute values are given where the real nest is shallow enough to fit (Aphaenogaster, small Camponotus).
- **[DER] is my arithmetic**: width from a circular area, BL from mm, disc thickness from volume over area. Check before building a test on it.

---

## 7. What each blueprint costs in the box [DER]

Computed by `build/make_specs.py` (same folder). Chambers are counted as 0.8 × width × height (lens fill) and tunnels as length × width. 1 BL = 2 cells. "Ants supported" is the open area divided by 25 cells per ant, which is 8% crowding. "Spoil length" is how long a 2-BL-tall mound must be to hold the dug soil, since a 2D slice gets no 3D dilution.

| Blueprint | Dug, true scale | Dug, legible (chambers 2.5 BL tall, passages 2 BL) | Ants at 8% (true / legible) | Spoil length for a 2-BL mound (true / legible) |
|---|---|---|---|---|
| harvester_stack | 99 BL² = 397 cells | 192 BL² = 768 cells | 16 / 31 | 50 / 96 BL |
| garden_ant_farm | 149 BL² = 594 cells | 276 BL² = 1,104 cells | 24 / 44 | 74 / **138 BL (wider than the 128-BL box)** |
| round_room_budding | 74 BL² = 297 cells | 108 BL² = 431 cells | 12 / 17 | 37 / 54 BL |
| carpenter_shallow | 78 BL² = 312 cells | 140 BL² = 560 cells | 12 / 22 | 39 / 70 BL |
| staircase_kebab | 56 BL² = 224 cells | 104 BL² = 416 cells | 9 / 17 | 28 / 52 BL |

The consequence is that the spoil needs a budget. In a vertical slice the mound must hold as much section area as the nest. A legible nest for 24 ants needs a 2-BL-tall mound 50-140 BL long, against a box only 128 BL wide. There are three outs, and real ants use all of them:
- a taller mound with its crest well back from the hole;
- a mound that weather spreads into a wide, thin disc (P. badius);
- backfill into dead ends below ground (P. occidentalis in the lab, ~51%).
