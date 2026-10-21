import json, math, sys
OUT = sys.argv[1]
CELLS_PER_BL = 2
LEGIBLE_CHAMBER_H = 2.5   # BL, i.e. 5 cells
LEGIBLE_PASSAGE = 2.0     # BL, i.e. 4 cells (owner: "passage ~4")

T = {"FT":"read in full text (PMC)","ABS":"read in abstract","WS":"web-search summary only, page not opened",
     "DER":"my arithmetic/conversion","ASSUMED":"design choice where the literature gave no number","GK":"general knowledge, unsourced here"}

def ch(id, floor, width, height, shape, side, x_inner, connection, contents=None, tag="DER", **kw):
    d = dict(id=id, floor_depth_bl=floor, width_bl=width, height_true_bl=height,
             height_legible_bl=max(height, LEGIBLE_CHAMBER_H), shape=shape, side=side,
             x_inner_edge_bl=x_inner, connection=connection, tag=tag)
    if contents: d["contents"] = contents
    d.update(kw)
    return d

def tun(frm, to, width, path, tag="DER", **kw):
    d = dict(**{"from": frm, "to": to}, width_true_bl=width, width_legible_bl=max(width, LEGIBLE_PASSAGE), path=path, tag=tag)
    d.update(kw); return d

def area(bp):
    """2D section area estimate: chambers as w*h*0.8 (lens/dome fill factor), tunnels as length*width."""
    out = {}
    for mode in ("true","legible"):
        a = 0.0
        for c in bp["chambers"]:
            h = c["height_true_bl"] if mode=="true" else c["height_legible_bl"]
            a += 0.8 * c["width_bl"] * h
        for t in bp["tunnels"]:
            w = t["width_true_bl"] if mode=="true" else t["width_legible_bl"]
            a += t["approx_length_bl"] * w
        out[mode] = a
    return out

bps = []

# ---------------- A. Harvester stack ----------------
A = dict(
 id="harvester_stack",
 name="Harvester stack (top-heavy helix)",
 inspiration="Pogonomyrmex badius, Florida harvester ant: Tschinkel 2004 J Insect Sci 4:21 [FT]; Tschinkel 1999 Ecol Entomol [ABS]; Tschinkel 2015 PLoS ONE biomantling [FT]",
 body_length_mm="6.35-9.5 (polymorphic) [WS]; 1 BL ~ 0.8 cm used",
 true_scale_note="Real nests: incipient 29-37 cm (~40 BL), mature 2.5-3.0 m (~300-380 BL). This blueprint is the whole nest compressed to the box at RELATIVE depth (rel = floor/D). The size-free shape is the same at every colony size [FT], so compressing it keeps it recognisable.",
 visual_signature="One zig-zag shaft with wide flat chambers hung off alternating sides. Big and close together near the top, small and far apart below. Reads as a Christmas tree upside down, drawn in thin flat leaves.",
 nest_depth_D_bl=26,
 shaft=dict(width_true_bl=1.2, width_upper_bl=2.0, upper_zone_rel=[0,0.15],
            angle_from_horizontal_deg="20-30 at the top, 45-60 by rel~0.2, up to ~70 deep [FT]",
            plan="Helix 5-8 BL across [FT/DER]. In section it is a zig-zag with +/-2-3 BL swing whose pitch grows with depth (8-10 cm then ~20 cm per turn [FT]).",
            waypoints_x_depth_bl=[[0,0],[3,1.5],[1,3.5],[-1.5,5.5],[1.5,8],[-1.5,11.5],[1.5,16],[-1.5,21.5],[1.5,25]],
            tag="FT+DER"),
 chambers=[
  ch("A0_superficial",1.5,14,1.2,"looping lobed horizontal tunnel-chamber, lobes on BOTH sides of the shaft","both",-7,
     "Is a widened near-horizontal shaft; lies inside the soil under the disc [FT]","foragers, guards (top 15 cm) [ABS]","FT+DER", rel_depth=0.06),
  ch("A1",3.5,12,1.2,"flat floor, slightly domed roof, lobed outline","right",1.5,
     "Inner edge meets the shaft at 25-70 deg. Starts as a niche on the OUTSIDE of the helix [FT]","sparse older workers",rel_depth=0.13),
  ch("A2",5.5,11,1.2,"flat floor, slight dome, lobed","left",-1.5,"same as A1","seeds (rel 0.15-0.4) [ABS/DER]",rel_depth=0.21),
  ch("A3",8,9,1.2,"flat floor, slight dome","right",1.5,"same","seeds",rel_depth=0.31),
  ch("A4",11.5,7,1.2,"flat floor, slight dome","left",-1.5,"same","seeds / mixed",rel_depth=0.44),
  ch("A5",16,5.5,1.2,"flat floor, near-round niche","right",1.5,"same","young workers, brood",rel_depth=0.62),
  ch("A6",21.5,4,1.2,"flat floor, round niche","left",-1.5,"same; widest gap above it (rel 0.7-0.8) [FT]","brood + callows, densest [ABS]",rel_depth=0.83),
  ch("A7",25,3.5,1.2,"flat floor, round niche (bottom)","right",1.5,"same","brood, densest",rel_depth=0.96),
 ],
 tunnels=[tun("entrance","A7",1.2,"the zig-zag shaft through all chambers' inner edges",approx_length_bl=30)],
 chamber_rules=dict(
   height="~1.2 BL true at ALL chamber sizes [FT]. Legible: 2.5 BL. Keep width >= 3x height.",
   spacing="Gap grows with depth, smallest near the top (2-4 cm ~ 3-5 BL real), largest at rel 0.7-0.8 [FT]. Box: 2, 2, 2.5, 3.5, 4.5, 5.5, 3.5 BL.",
   top_heaviness="About half the section width/area in the top quarter. Each decile ~25-50% less than the one above [FT]. This layout: 56% of chamber width in the top quarter [DER].",
   attachment="Chambers hang off the OUTSIDE of the shaft's turns, so in section they alternate sides [FT/DER]."),
 surface_mound=dict(
   shape="flat disc (blanket), not a cone; real ones are charcoal-decorated [FT]",
   real_dimensions="diameter 10-85 cm (mean ~44 cm ~ 55 BL); mean thickness ~2 cm ~ 2.5 BL (volume/area) [FT+DER]",
   box_fit=dict(inner_radius_bl=2.5, crest_radius_bl=4, outer_radius_bl=22, height_bl=1.5, inner_slope="at the sim's repose angle", outer_slope_deg="feathered, below repose (weather spreads it)", tag="DER+ASSUMED"),
   note="The real disc runs right up to the hole. In a loose-grain sim, keep a 2.5 BL collar clear and a low rim (crest 4 BL) so the inner toe stays outside the shaft lip (section 3 of notes)."),
 scaling_with_workers=dict(
   area="log A(cm2) = 0.55 + 0.87 log W  ->  A proportional to W^0.87 [FT]",
   depth="log D(cm) = 0.95 + 0.37 log W  ->  D proportional to W^0.37 [FT]. Extrapolated to 12 / 24 workers: ~22 / ~28 cm = 28 / 36 BL [DER]",
   chambers="added at every decile, so spacing vs relative depth stays the same; enlargement of existing chambers adds more area than new chambers [FT]",
   series="2nd shaft-chamber series from ~3,200 workers (1 series ~1,700, 3 ~4,600, 4 ~5,000), branching <40 cm deep, at the base of an upper chamber [FT]",
   game="12 ants: A0-A4 + shaft (~5 chambers). 24 ants: all 8. More ants after that: a 2nd series from A1's base, smaller chambers [DER]"),
)
bps.append(A)

# ---------------- B. Garden-ant farm ----------------
B = dict(
 id="garden_ant_farm",
 name="Garden-ant farm (meandering branching galleries)",
 inspiration="Lasius niger in a thin formicarium. Dimensions from the Lasius neoniger field study [WS], L. flavus micro-CT (Minter et al. 2012) [ABS/FT via Gautrais 2014], L. niger 2D digging (Rasse & Deneubourg 2001) [ABS]",
 body_length_mm="3.5-5.0 (L. niger, AntWiki) [WS]; 1 BL ~ 0.4 cm used",
 true_scale_note="L. neoniger nests sit mostly in the top 0.3 m (~75 BL) with a few galleries to ~0.7 m [WS]. The whole 30-BL box is the 'upper zone' at true scale.",
 visual_signature="A tangle: 2-4 galleries slant down from the entrance, wander, split, and sometimes rejoin. Chambers are elongated swellings along them. Wider than deep. No single trunk.",
 nest_depth_D_bl=26,
 shaft=dict(width_true_bl=1.0, plan="short starter shaft 3 BL straight down, then 2-4 galleries slanting downward at 20-45 deg [ABS/FT via Gautrais 2014 on Minter 2012]", tag="ABS+DER"),
 chambers=[
  ch("B1",3,9,1.5,"elongated horizontal swelling, flat floor, low dome","left",-4,"swelling along gallery G1 [WS/DER]","workers",tag="WS+ASSUMED"),
  ch("B2",5,8,1.5,"elongated swelling","right",5,"at the G2/G3 fork","brood",tag="WS+ASSUMED"),
  ch("B3",9,10,1.8,"elongated swelling, two lobes","left",-15,"on G1, where a cross-link G5 arrives","brood",tag="WS+ASSUMED"),
  ch("B4",11,7,1.5,"elongated swelling","right",14,"end of G2","workers",tag="WS+ASSUMED"),
  ch("B5",16,6,1.5,"elongated swelling","right",6,"on G3","brood",tag="WS+ASSUMED"),
  ch("B6",24,4,1.3,"small swelling at a dead end","left",-3,"end of the one deep gallery G4","overwintering / cool retreat [ASSUMED]",tag="ASSUMED"),
 ],
 tunnels=[
  tun("entrance","B1 (G1)",1.0,"down-left at ~30 deg, wandering +/-20 deg every 3-5 BL",approx_length_bl=8),
  tun("B1","B3 (G1)",1.0,"continues down-left",approx_length_bl=12),
  tun("starter","B2 (G2/G3 fork)",1.0,"down-right ~35 deg",approx_length_bl=7),
  tun("B2","B4 (G2)",1.0,"right, shallow",approx_length_bl=11),
  tun("B2","B5 (G3)",1.0,"steeper down-right",approx_length_bl=12),
  tun("B1","B6 (G4)",1.0,"the one steep gallery, ~60 deg",approx_length_bl=22),
  tun("B5","B3 (G5 cross-link)",1.0,"a loop: meshedness > 0 appears around 100+ ants in 2D Messor data [FT]",approx_length_bl=22),
 ],
 chamber_rules=dict(height="1.3-1.8 BL true [ASSUMED: ~ant height rule from Khuong 2016 roof template ABS and other species FT]; legible 2.5 BL",
                    length="7-12 BL (L. neoniger chambers 30-50 mm long at ~4 mm BL) [WS+DER]",
                    gallery_width="0.4-1.2 BL real (1.5-5 mm) [WS+DER]; use 1 BL true, 2 BL legible"),
 surface_mound=dict(
   shape="two-sided ring/crater of loose crumbs around the entrance (Dorymyrmex dumping rule [ABS]). Optional variant: L. niger's built earthen mound with internal galleries (pillars roofed at body height, Khuong 2016 [ABS]). The variant is risky in loose grains and is exactly the 'ants dig into mound' failure",
   box_fit=dict(inner_radius_bl=2.5, crest_radius_bl=5, outer_radius_bl=10, height_bl=2, inner_slope="repose; nothing new is put on it", outer_slope="repose; all loads go here, beyond the crest", tag="ABS+DER"),
   second_entrance="optional 2nd opening 15-25 BL away (a G2 gallery surfacing) with its own small ring [ASSUMED]"),
 scaling_with_workers=dict(
   area="excavated volume ~proportional to N (L. niger 2D, Rasse & Deneubourg 2001) [ABS]",
   topology="more ants -> more branches and more loops; meshedness 0 / 0.09 / 0.19 at 50/100/200 Messor ants [FT]",
   game="12 ants: G1, G2 and B1, B2, B4 (tree, no loop). 24 ants: add G3, B3, B5; G5 loop optional. 50+ ants: loops [DER]"),
)
bps.append(B)

# ---------------- C. Round room that buds ----------------
C = dict(
 id="round_room_budding",
 name="Round room that buds (density-driven shape transition)",
 inspiration="Messor sancta 2D digging: Toffin et al. 2009 PNAS [ABS], Toffin, Kindekens & Deneubourg 2010 Proc R Soc B [ABS], Buhl et al. 2004/2005 [ABS], Gautrais et al. 2014 [FT]",
 body_length_mm="not found for M. sancta; Messor generally 4-10 mm, polymorphic [WS]. Expressed in BL by analogy",
 true_scale_note="Toffin's arenas were HORIZONTAL sand layers under glass, so 'round' is round in plan. In a vertical slice, gravity flattens the floor. The analogue here is a dome-roofed oval room 2-3x wider than tall [DER].",
 visual_signature="One fat room at the foot of a short straight shaft. As the colony grows, short buds appear around its edge, and 2-4 of them stretch into thin radiating galleries with small knots at their ends. A blob that sprouts arms.",
 nest_depth_D_bl=22,
 growth_stages=[
  dict(stage=1, name="round cavity", rule="Dig uniformly around the whole wall while worker density at the wall is high. The room grows isotropically [ABS/FT]", trigger_to_next="dug area passes a threshold. Lower in cohesive substrate. Values not retrieved [ABS]"),
  dict(stage=2, name="budding", rule="3-6 short buds (1 BL wide, 1-2 BL long) at roughly even spacing around the sides and lower perimeter [ABS+ASSUMED count]", trigger_to_next="second area threshold [ABS]"),
  dict(stage=3, name="galleries", rule="2-4 buds lengthen into galleries ~1 BL wide. New branches start from gallery walls at right angles, at a constant rate per wall length [FT, Gautrais 2014]. Small node-chambers where galleries end or cross", trigger_to_next="digging stops at an area threshold independent of shape and material [ABS]"),
 ],
 shaft=dict(width_true_bl=1.2, depth_bl=4, plan="straight vertical, entrance -> room roof. This is the current 8-row (4 BL) shaft", tag="DER"),
 chambers=[
  ch("C0_room",10,9,4,"dome-roofed oval, flat floor (stage 1: width:height ~2.2:1)","centre",-4.5,
     "shaft enters the roof centre",
     "everyone, brood pile on the floor centre",tag="ABS+DER", height_legible_bl=4),
  ch("C1_node",14,3,2,"small round node at a gallery end","left",-14,"end of gallery g1 (stage 3)","overflow / brood",tag="ASSUMED"),
  ch("C2_node",17,3,2,"small round node","right",12,"end of gallery g2","overflow",tag="ASSUMED"),
  ch("C3_node",21,2.5,2,"small round node","centre",-1,"end of gallery g3 (downward)","seeds",tag="ASSUMED"),
 ],
 tunnels=[
  tun("entrance","C0_room",1.2,"straight down 4 BL",approx_length_bl=4),
  tun("C0_room left wall","C1_node (g1)",1.0,"out and slightly down",approx_length_bl=9),
  tun("C0_room right wall","C2_node (g2)",1.0,"out and down ~30 deg",approx_length_bl=9),
  tun("C0_room floor","C3_node (g3)",1.0,"down",approx_length_bl=9),
 ],
 buds=dict(count="3-6", width_bl=1.0, length_bl="1-2", positions="sides and lower perimeter of C0; none in the roof (keeps the roof whole) [ASSUMED]", tag="ABS+ASSUMED"),
 surface_mound=dict(
   shape="ring crater, Dorymyrmex rule: put down just beyond the crest so loads roll outward [ABS]",
   box_fit=dict(inner_radius_bl=2.5, crest_radius_bl=4.5, outer_radius_bl=9, height_bl=1.5, inner_slope="repose, receives nothing", outer_slope="repose, receives every load", tag="ABS+DER")),
 scaling_with_workers=dict(
   area="total dug area ~proportional to N (Buhl 2005: 'almost proportional') [ABS]; logistic in time: exponential start, then a stop [ABS]",
   shape="the round->bud->gallery transition happens at all group sizes but more often with more workers [ABS]; 96% of cohesive vs 50% of granular runs went all the way [ABS]",
   meshedness="0 at 50 ants, 0.09 at 100, 0.19 at 200 (2D arena) [FT]",
   game="12 ants: stage 1 only (a round room). 24 ants: stage 2 (room + buds), maybe one gallery. Graded rather than binary [DER]"),
)
bps.append(C)

# ---------------- D. Carpenter shallow ----------------
D = dict(
 id="carpenter_shallow",
 name="Carpenter shallow (upturned fingers over compact lenses)",
 inspiration="Camponotus socius, a ground-nesting carpenter ant: Tschinkel 2005 J Insect Sci 5:9 [FT]",
 body_length_mm="minors 7.5-10.5, majors 11-16 [WS]; 1 BL ~ 0.9 cm (minor) used",
 true_scale_note="Real depth 15-66 cm (~17-73 BL). A small nest fits the box at near true scale.",
 visual_signature="Shallow and lateral. Near the top, long thin chambers run sideways from the shaft and curl UP to stop just under the surface, never breaking through. Lower down there are a few compact wedge-shaped lenses, thick at the shaft and thin at the tip. The shaft wanders.",
 nest_depth_D_bl=22,
 shaft=dict(width_true_bl=1.2, angle_from_horizontal_deg="45 near the surface to ~90 deep [FT]",
            plan="wandering zig-zag / loose helix [FT]; larger nests may have a 2nd shaft branching in the upper part (5 of 24 nests) [FT]",
            waypoints_x_depth_bl=[[0,0],[2,2],[1,4],[3,7],[1,10],[2.5,13],[0.5,16],[2,19],[1,22]], tag="FT+DER"),
 chambers=[
  ch("D1_finger",2.5,11,1.3,"elongate tunnel-like chamber that curls upward at its far end to ~1 BL below the surface; never opens [FT]","right",2,
     "leaves the shaft horizontally, tip rises ~1.5 BL","workers",tag="FT+DER"),
  ch("D2_finger",4.5,12,1.3,"elongate, curls upward at the tip [FT]","left",1,"same, other side","workers",tag="FT+DER"),
  ch("D3",9,7,1.5,"compact lens; ~1.6 BL thick at the shaft, tapering to ~0.8 at the tip [FT: thinner at outer edges]","right",3,"inner end on the shaft","brood (if season) [ASSUMED]",tag="FT+DER"),
  ch("D4",13.5,6,1.4,"compact, 2-lobed (lobes appear as chambers enlarge) [FT]","left",1,"inner end on the shaft","brood",tag="FT+DER"),
  ch("D5",18.5,5,1.3,"compact lens","right",2,"inner end on the shaft","workers",tag="FT+DER"),
  ch("D6",22,4,1.2,"compact lens (bottom)","left",1,"inner end on the shaft","workers",tag="FT+DER"),
 ],
 tunnels=[tun("entrance","D6",1.2,"the wandering shaft",approx_length_bl=25)],
 chamber_rules=dict(height="1.0-1.5 cm = 1.1-1.7 BL, invariant with area [FT]",
                    spacing="2-14 cm, mean ~4 cm rising to ~6.5 cm with depth (~4.5-7 BL) [FT+DER]. Here: 2, 4.5, 4.5, 5, 3.5",
                    shape_by_depth="upper chambers narrow and linear (shaft-like); deeper more compact; large ones lobed [FT]"),
 polydomy=dict(fact="the average colony occupies 2-3 nests (queen in 6 of 14 casts -> ~2.3 nests per colony) [FT]",
               game="optional satellite nest 25-40 BL along the surface: 1 short shaft + 2 lenses, linked by a surface trail [ASSUMED distance]"),
 surface_mound=dict(
   shape="no species data retrieved. Use a small ONE-SIDED crescent dumped downslope / to one side (Trachymyrmex rule: one side, arc ~75 deg, fixed distance, rebuilt at the same place [ABS])",
   box_fit=dict(inner_radius_bl=3, crest_radius_bl=5, outer_radius_bl=10, height_bl=1.5, side="one side only (e.g. the side away from D1's finger)", tag="ABS+ASSUMED")),
 scaling_with_workers=dict(
   area="A(cm2) = 0.70 W + 67.9 (W = 2-590, R2 84%) [FT]. In BL2 at 0.9 cm: A ~ 0.86 W + 84 [DER]",
   chambers="2 -> 10 as the nest grows; mean chamber area 13 -> 47 cm2; enlargement contributes most [FT]",
   depth="15 -> 66 cm [FT]",
   game="12 ants: D1, D2, D3 (+ shaft to ~10 BL). 24 ants: D1-D5. Extra ants: lobing of D3/D4, then a satellite nest [DER]"),
)
bps.append(D)

# ---------------- E. Staircase shish-kebab ----------------
E = dict(
 id="staircase_kebab",
 name="Staircase shish-kebab (edge-out, centre-in)",
 inspiration="Aphaenogaster floridana / treatae / ashmeadi: Tschinkel 2011 J Insect Sci 11:105 [FT]",
 body_length_mm="5.2-6.6 across the three species [FT]; 1 BL ~ 0.6 cm used",
 true_scale_note="A. treatae / ashmeadi nests are 7-25 cm (~12-42 BL) deep: the only real family that fits the 30-BL box at TRUE scale. Depth vs levels: D = -10.2 + 8.68 x levels cm (~14 BL per level) [FT+DER].",
 visual_signature="A neat vertical stack of round-ish flat chambers directly under the entrance, starting with a small one just under the surface. Each shaft leaves a chamber at its edge and lands in the middle of the next one, so the stack stair-steps left-right-left. The top pair is joined by 2-3 parallel shafts.",
 nest_depth_D_bl=26,
 shaft=dict(width_true_bl=1.0, plan="vertical segments, one per level; each starts at the EDGE of the chamber above and ends at the CENTRE of the chamber below [FT]", tag="FT"),
 chambers=[
  ch("E0_vestibule",1.5,4,1.2,"small flat chamber immediately below the surface ('all have a small chamber immediately below the surface') [FT]","centre",-2,"entrance drops into its centre","guards / foragers",tag="FT+ASSUMED"),
  ch("E1",8,6.5,1.5,"near-circular in plan, so a flat lens in section, flat floor [FT]","centre-right",-1.25,"centred under E0's RIGHT edge (x=+2); its own descending shaft leaves its LEFT edge","workers",tag="FT+DER", x_centre_bl=2),
  ch("E2",16,7,1.5,"flat lens, more circular deeper [FT]","centre-left",-4.75,"centred under E1's LEFT edge (x=-1.25); leaves from its RIGHT edge","brood",tag="FT+DER", x_centre_bl=-1.25),
  ch("E3",25,7.5,1.5,"flat lens, largest (A. floridana is bottom-heavy: most area in deciles 7-8) [FT]","centre-right",-1.5,"centred under E2's RIGHT edge (x=+2.25)","brood, densest (ants denser toward the bottom) [FT]",tag="FT+DER", x_centre_bl=2.25),
 ],
 tunnels=[
  tun("entrance","E0 centre",1.0,"straight down 1 BL",approx_length_bl=1),
  tun("E0 right edge","E1 centre",1.0,"vertical, x=+2; plus 1-2 PARALLEL shafts from E0's other edge (A. floridana upper-quarter multiple shafts) [FT]",approx_length_bl=12),
  tun("E1 left edge","E2 centre",1.0,"vertical, x=-1.25",approx_length_bl=6.5),
  tun("E2 right edge","E3 centre",1.0,"vertical, x=+2.25",approx_length_bl=7.5),
 ],
 chamber_rules=dict(mean_area="9.5-13 cm2 in all three species -> ~3.5-4 cm across ~ 6-7 BL [FT+DER]",
                    height="not reported; 1.5 BL assumed (ant-height rule) [ASSUMED]",
                    spacing="increases with depth in A. floridana; here 6.5, 8, 9 BL [FT+DER]",
                    spread="'all chambers lie directly below the nest entrance', little lateral spread [FT]"),
 surface_mound=dict(
   shape="no species data retrieved. Small ring crater, Dorymyrmex rule [ABS]",
   box_fit=dict(inner_radius_bl=2.5, crest_radius_bl=4, outer_radius_bl=8, height_bl=1.2, tag="ABS+ASSUMED")),
 scaling_with_workers=dict(
   chambers="+1 chamber per ~100 workers (A. floridana) [FT]",
   depth="+0.09 cm (~0.15 BL) per worker; each added chamber ~5.2 cm (~9 BL) deeper (floridana), ~3.7 cm in the smallest species [FT+DER]",
   area="+0.057 cm2 floor per worker; area per worker falls from ~0.8 to ~0.2 cm2 as colonies grow, so crowding rises 5-6x [FT]",
   game="12 ants: E0 + E1 (one level). 24 ants: E0-E2. E3 and the parallel upper shafts from ~50+ ants. A real nest of 12-24 workers would have 1-2 chambers [DER]"),
)
bps.append(E)

for bp in bps:
    a = area(bp)
    bp["excavated_section_area"] = dict(
        true_bl2=round(a["true"],1), true_cells=round(a["true"]*CELLS_PER_BL**2),
        legible_bl2=round(a["legible"],1), legible_cells=round(a["legible"]*CELLS_PER_BL**2),
        ants_supported_at_8pct_crowding_true=round(a["true"]*CELLS_PER_BL**2/25),
        ants_supported_at_8pct_crowding_legible=round(a["legible"]*CELLS_PER_BL**2/25),
        spoil_length_bl_if_mound_2bl_tall=dict(true=round(a["true"]/2,1), legible=round(a["legible"]/2,1)),
        method="chambers 0.8*w*h (lens fill), tunnels length*width; 25 open cells per 2-cell ant = 8% crowding [DER]")

doc = dict(
 title="Ideal-nest blueprint candidates (vertical section, body-length units)",
 written="2026-10-21",
 companion_notes="../../notes/nest-architecture-2026-10-21.md: citations, numbers, tags",
 tags=T,
 units=dict(
   BL="worker body length",
   cells_per_bl=CELLS_PER_BL,
   cells_per_bl_source="ant.ron body: Chain(2), src/sim/creature.rs:3761/3786",
   x="columns from the entrance centre in BL, + = right",
   depth="rows below the original surface in BL, to the chamber FLOOR",
   box="~60 soil rows = 30 BL deep, 256 columns = 128 BL wide; blueprints use D <= 26 BL"),
 legibility=dict(
   passage_min_cells=4, passage_note="owner ruling 'passage ~4' (creature.rs:3763) = 2 BL. Real tunnels are ~1.0-1.3 BL (Gravish 2013 [ABS]); P. badius upper shafts reach ~2.5 BL [FT], so 2 BL is inside the real range near the top",
   chamber_height_true_bl="1.1-1.7 (P. badius ~1.2, C. socius 1.1-1.7, P. occidentalis <=2) [FT/ABS]",
   chamber_height_legible_bl=LEGIBLE_CHAMBER_H,
   chamber_aspect="keep width >= 3x height. Real width:height runs ~6:1 to 20:1 [DER from FT]",
   warning="The ladder's planned 8-16-cell-tall chambers are 4-8 BL tall, 3-5x taller than any real chamber. 5-6 cells tall and 3x+ as wide reads as a real chamber at this ant size [DER]"),
 common_rules=[
   "A nest is shafts plus flat horizontal chambers hung on them (shish-kebab). Big nests add more shaft-chamber units [FT]",
   "Chamber height is invariant (~1-2 BL). Chambers grow by widening and lobing, never by getting taller [FT]",
   "Shape is size-free: the same relative layout at every colony size, so a blueprint can be scaled by relative depth [FT]",
   "Most species are top-heavy: big, close chambers near the top; small, sparse ones deep. Aphaenogaster floridana is the exception [FT]",
   "Ants and brood are densest in the deepest chambers; foragers stay near the top [FT/ABS]",
   "A small vestibule or entrance chamber just under the surface is common (Aphaenogaster [FT]; P. barbatus entrance hub with 1-6 side chambers [ABS])",
   "Dug area grows ~linearly (2D lab) or as N^0.87 (P. badius field) with workers; depth grows slower (N^0.37) [ABS/FT]",
   "2D conservation [DER]: in a vertical slice the spoil section equals the dug section, with no 3D dilution. A legible nest (taller chambers, 4-cell passages) roughly doubles the spoil. Budget mound length or backfill for it (P. occidentalis lab nests backfilled ~51% [ABS]; P. badius field ~2.5% [ABS])",
 ],
 mound_rules=[
   "Put down beyond the crest, on the OUTER flank, so loads roll away from the hole. Dorymyrmex: 73-82% stay, 18-27% roll outward, almost none roll back [ABS]",
   "Read slope: drop where the surface turns from rising to falling. Tilting a pile shifts the drop point (Pheidole oxyops, 0.46/0.53/0.63 of the pile width) [ABS]",
   "Keep a fixed distance and side: Pheidole crescent ~13 cm from the hole; Trachymyrmex one-sided crescent at 8.0 cm, arc ~75 deg, rebuilt at the same orientation, turned downslope over days [ABS]",
   "Each digger may keep its own dump direction (Cataglyphis, visual landmarks) [ABS]",
   "Grain-sim geometry [DER/GK]: inner toe radius = crest radius - h/tan(repose) must stay >= shaft half-width + 1 BL. At a 45 deg cell repose: crest_radius >= shaft_half_width + 1 + h (BL)",
   "Weather spreads real discs: a 20 cm disc's sand was found >50 cm out within a year (P. badius) [FT]",
   "The ladder's measured falls came from RIM CUTS collapsing the shaft shoulder, not from put-downs (dig-rung-design step 3/4 notes). The literature above is about placing loads and does not address rim collapse",
 ],
 blueprints=bps,
)
with open(OUT,"w") as f:
    json.dump(doc,f,indent=1,ensure_ascii=False)
for bp in bps:
    print(bp["id"], bp["excavated_section_area"])
