"""Part 2 picture: what s161 gen 336 reads, and what that predicts at 8 rows."""
import sys, ast, collections, statistics as st
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt

S, OUT = sys.argv[1], sys.argv[2]
INK, INK2, MUTED, GRID, SURF = "#1f1f1e", "#55544f", "#8a8984", "#e6e5e1", "#fcfcfb"
BLUE, ORANGE, GREY, LIGHT = "#2a78d6", "#eb6834", "#55544f", "#c3c2b7"
names = ["score", "cuts", "s1", "s2", "s3", "held", "dumped", "refill_pellet", "refill_fall", "away_share", "alive", "energy_left", "dig_j",
         "holding", "spent", "chamber_fill", "fall_putdown", "s4", "up_rows", "deep_cuts", "rim_cuts", "loads", "load_base", "load_share",
         "full_loads", "full_cuts", "full_share"]
rows = lambda f: [dict(zip(names, ast.literal_eval(l[l.index("["):]))) for l in open(f) if l.startswith("NOISE seed")]

fig = plt.figure(figsize=(14, 9.5)); fig.patch.set_facecolor(SURF)
gs = fig.add_gridspec(2, 2, height_ratios=[1.1, 1])
axA = fig.add_subplot(gs[:, 0]); axB = fig.add_subplot(gs[0, 1]); axC = fig.add_subplot(gs[1, 1])
for a in (axA, axB, axC):
    a.set_facecolor(SURF)
    for sp in ("top", "right"): a.spines[sp].set_visible(False)
    for sp in ("left", "bottom"): a.spines[sp].set_color(MUTED)
    a.tick_params(colors=INK2, labelsize=9)

# A: switch one sense off
arms = [("intact", "none"), ("crowd (always 0 here: the switch's own check)", "crowd"), ("time since nest", "since_nest"),
        ("hole / roof warnings", "hole_drop,roof_cut"), ("energy, age", "energy,d_energy,age"), ("scent", "a,b,d_a,d_b"),
        ("roof overhead", "roofed"), ("depth", "depth"), ("home distance", "home_dist,d_home"), ("carrying soil", "spoil"),
        ("at the nest", "at_nest"), ("never moves (control)", "CONTROL-rest")]
base = [r["full_loads"] for r in rows(f"{S}/inv/abl/none.txt")]
ys = list(range(len(arms)))[::-1]
for y, (lab, f) in zip(ys, arms):
    R = rows(f"{S}/inv/abl/{f}.txt"); v = [r["full_loads"] for r in R]
    d = [x - b for x, b in zip(v, base)]; se = st.stdev(d) / len(d) ** 0.5 if len(set(d)) > 1 else 0
    m = st.mean(v)
    col = BLUE if f in ("none", "crowd") else (GREY if "CONTROL" in f else (ORANGE if m < 7.21 - 2 * max(se, 1e-9) else LIGHT))
    axA.barh(y, m, color=col, height=0.62)
    axA.errorbar(m, y, xerr=se, color=INK, capsize=2, lw=1) if se else None
    axA.text(m + se + 0.2, y, f"{m:.2f}", va="center", fontsize=8.5, color=INK)
axA.set_yticks(ys); axA.set_yticklabels([a for a, _ in arms], fontsize=9)
axA.axvline(7.21, color=BLUE, lw=1, ls=":")
axA.set_xlabel("full loads per world, 24 sealed worlds (bar: ± 1 s.e. of the paired difference from intact)", color=INK, fontsize=9)
axA.set_xlim(0, 9.2); axA.grid(True, axis="x", color=GRID); axA.set_axisbelow(True)
axA.set_title("A. Switch one sense off (it reads 0 at every decision).\nOrange: a loss beyond 2 s.e. Digging rests on 'at the nest',\n'carrying soil' and 'home distance'; depth and roof help.",
              loc="left", fontsize=10, color=INK)

# B: the two decisions, opened
terms_dig = [("'at the nest'", 6.50), ("memory", -4.04), ("constant", -6.30), ("pull to repeat", -0.90), ("hidden units", 0.52), ("depth", 0.39)]
terms_drop = [("hidden units", 20.67), ("constant", -18.66), ("memory", -2.28), ("pull to repeat", -0.97), ("best target", 0.43)]
for i, (title, terms) in enumerate((("dig instead of walk\n(empty, below ground, n=262)", terms_dig), ("drop instead of walk\n(carrying, on the surface, n=314)", terms_drop))):
    x0 = i * 7.5
    for j, (lab, v) in enumerate(terms):
        axB.bar(x0 + j, v, color=ORANGE if v > 0 else LIGHT, width=0.75)
        axB.text(x0 + j, v + (0.6 if v >= 0 else -0.6), f"{v:+.1f}", ha="center", va="bottom" if v >= 0 else "top", fontsize=8, color=INK)
    axB.set_xticks([x0 + j for j in range(len(terms))], minor=False) if i == 0 else None
    axB.text(x0 + (len(terms) - 1) / 2, 23.5, title, ha="center", fontsize=9, color=INK)
xt = [j for j in range(len(terms_dig))] + [7.5 + j for j in range(len(terms_drop))]
axB.set_xticks(xt); axB.set_xticklabels([l for l, _ in terms_dig] + [l for l, _ in terms_drop], rotation=35, ha="right", fontsize=8.5)
axB.axhline(0, color=MUTED, lw=1); axB.set_ylim(-22, 27); axB.grid(True, axis="y", color=GRID); axB.set_axisbelow(True)
axB.set_ylabel("push towards the chosen verb\n(exact term of its score, pre-squash)", color=INK, fontsize=9)
axB.set_title("B. The two decisions, opened (exact, reproduced from the genome). It digs when it\nfeels 'at the nest'; it drops when a hidden unit sees 'far enough from home'.",
              loc="left", fontsize=10, color=INK)

# C: 4 rows against 8 rows
src = open(f"{S}/inv/pellets.py").read().split("allrec = []")[0].replace('print(f"worlds parsed', "#")
res = {}
for lab, log, shaft in (("4-row shaft\n(trained)", f"{S}/inv/trace/s161.log", 4), ("8-row shaft\n(never trained)", f"{S}/inv/trace/s161-shaft8-f6000.log", 8)):
    env = {}; sys.argv = ["x", log, str(shaft)]; exec(src, env)
    recs = [x for w in env["worlds"] for x in env["fates"](w)]
    res[lab] = [sum(1 for x in recs if x["cls"] == c) / 24 for c in ("foot", "wall", "rim", "recut")]
labs = list(res)
cols = [BLUE, ORANGE, LIGHT, GREY]; names_c = ["at the foot (pays)", "shaft wall (no pay)", "top row (no pay)", "re-cut (no pay)"]
for i, lab in enumerate(labs):
    bottom = 0
    for k, v in enumerate(res[lab]):
        axC.bar(i, v, bottom=bottom, color=cols[k], width=0.55, label=names_c[k] if i == 0 else None)
        if v > 0.5: axC.text(i, bottom + v / 2, f"{v:.1f}", ha="center", va="center", fontsize=8.5, color="white" if k in (0, 3) else INK)
        bottom += v
axC.set_xticks(range(len(labs))); axC.set_xticklabels(labs, fontsize=9)
axC.set_ylabel("cuts per world (6,000 frames)", color=INK, fontsize=9)
axC.legend(frameon=False, fontsize=8.5, loc="upper left", bbox_to_anchor=(1.0, 1.0))
axC.grid(True, axis="y", color=GRID); axC.set_axisbelow(True)
axC.set_title("C. 'At the nest' is on along the whole shaft, and the target scorer has no link to a\ncell's depth. On an 8-row shaft s161 cuts the wall more often than the foot.",
              loc="left", fontsize=10, color=INK)

fig.suptitle("Part 2: what s161 gen 336 reads when it digs (24 sealed worlds)", x=0.01, ha="left", fontsize=12.5, color=INK, fontweight="bold")
fig.tight_layout(rect=(0, 0, 1, 0.96))
fig.savefig(OUT, dpi=110, facecolor=SURF)
print("wrote", OUT)
