"""The dig rung's whole training history, s91 -> s111 -> s134 -> s161/s162, on one generation axis.

usage: lineage_plot.py SCRATCH OUT.png
"""
import ast, csv, statistics as st, sys
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt

S, OUT = sys.argv[1], sys.argv[2]
INK, INK2, MUTED, GRID, SURF = "#1f1f1e", "#55544f", "#8a8984", "#e6e5e1", "#fcfcfb"
TRUNK, B161, B162 = "#55544f", "#2a78d6", "#eb6834"

def hold(path):
    rows = list(csv.DictReader(open(path)))
    ch = {int(r["gen"]): r for r in rows if r["who"] == "champion"}
    stt = next(r for r in rows if r["who"] == "start")
    return ch, stt

def noise(k):
    rows = [ast.literal_eval(l[l.index("["):]) for l in open(f"{S}/r4/base/{k}-noise24.txt") if l.startswith("NOISE seed")]
    f = [r[24] for r in rows]
    return st.mean(f), st.stdev(f) / len(f) ** 0.5

# Segments: (holdout.csv, x offset, generation the next run branched from (None = a tip), colour, label)
s111 = hold(f"{S}/curve/dig-sq-s111-long2/holdout.csv")
s134 = hold(f"{S}/curve/s134/holdout.csv")
s161 = hold(f"{S}/r5/s161/x/holdout.csv")
s162 = hold(f"{S}/r5/s162/x/holdout.csv")
X111, X134, X16 = 0, 226, 226 + 34
segs = [("s111 run (from s91)", s111, X111, 226, TRUNK), ("s134 run (from s111 gen 226)", s134, X134, 34, TRUNK),
        ("s161 (from s134)", s161, X16, None, B161), ("s162 (from s134)", s162, X16, None, B162)]

fig, ax = plt.subplots(4, 1, figsize=(12, 16), sharex=True, gridspec_kw={"height_ratios": [1.25, 1, 0.8, 0.8]})
fig.patch.set_facecolor(SURF)
for a in ax:
    a.set_facecolor(SURF); a.grid(True, color=GRID, lw=0.8); a.set_axisbelow(True)
    for sp in ("top", "right"): a.spines[sp].set_visible(False)
    for sp in ("left", "bottom"): a.spines[sp].set_color(MUTED)
    a.tick_params(colors=INK2, labelsize=9)
    for xb in (X134, X16):
        a.axvline(xb, color=MUTED, lw=1, ls=":", zorder=0)

# --- Panel 1: the one consistent yardstick -----------------------------------------------------------------
a = ax[0]
pts = [("s91", "fretrain-s91", 0, TRUNK), ("s111 g126", "s111-g126", 126, TRUNK), ("s111 g226", "s111-g226", 226, TRUNK),
       ("s134", "s134-4b-g34", X16, TRUNK),
       ("s161 g91", "s161-4b4-g91", X16 + 91, B161), ("s161 g199", "s161-g199", X16 + 199, B161), ("s161 g336", "s161-g336", X16 + 336, B161),
       ("s162 g97", "s162-g97", X16 + 97, B162), ("s162 g190", "s162-g190", X16 + 190, B162), ("s162 g338", "s162-g338", X16 + 338, B162)]
val = {k: noise(f) for k, f, _, _ in pts}
trunk = [p for p in pts if p[3] == TRUNK]
a.plot([p[2] for p in trunk], [val[p[0]][0] for p in trunk], color=TRUNK, lw=2)
for col, name in ((B161, "s161"), (B162, "s162")):
    br = [pts[3]] + [p for p in pts if p[3] == col]
    a.plot([p[2] for p in br], [val[p[0]][0] for p in br], color=col, lw=2, label=name)
for lab, _, x, col in pts:
    m, se = val[lab]
    a.errorbar(x, m, yerr=se, fmt="o", ms=8, color=col, mec=SURF, mew=2, ecolor=col, elinewidth=1.5, capsize=3, zorder=3)
    below = col == B162
    nudge = {"s162 g190": -34, "s161 g199": 14, "s161 g91": -10, "s162 g97": 10}.get(lab, 0)
    a.annotate(f"{lab}\n{m:.2f}", (x, m), xytext=(nudge, -34 if below else 16), textcoords="offset points", ha="center", fontsize=8, color=INK2)
a.axhline(5, color=INK, lw=1.2, ls="--")
a.text(2, 5.12, "bar: 5 full loads per world (on 11 of 12 sealed worlds)", fontsize=8.5, color=INK)
a.set_ylim(0, 8.6)
a.set_ylabel("full loads carried out\nper world", color=INK, fontsize=10)
a.set_title("1. Digging, one yardstick for every brain: full loads per world on the same 24 sealed worlds, today's pay rule (foot=1).\n"
            "A full load = soil cut at the shaft's foot (4 rows down) and carried all the way up and out. Dots: mean ± 1 standard error.",
            loc="left", fontsize=10, color=INK)
a.legend(loc="upper left", frameon=False, fontsize=9)

# --- Panel 2: each run's own selection score ----------------------------------------------------------------
a = ax[1]
for lab, (ch, stt), x0, branch, col in segs:
    g = sorted(ch)
    keep = [k for k in g if branch is None or k <= branch]
    after = [k for k in g if branch is not None and k >= branch]
    a.plot([x0 + k for k in keep], [float(ch[k]["soil_fit"]) for k in keep], color=col, lw=1.4)
    if after:
        a.plot([x0 + k for k in after], [float(ch[k]["soil_fit"]) for k in after], color=col, lw=1, alpha=0.25)
    last = (keep[-1] if branch is None else branch)
    a.hlines(float(stt["soil_fit"]), x0 + g[0], x0 + (g[-1] if branch is None else branch), color=col, lw=1, ls="--", alpha=0.8)
a.text(X111 + 4, 66, "s111 run: wall cuts paid (rev 2.8),\n12 hold-out worlds", fontsize=8, color=INK2, va="top")
a.text(X134 + 2, 66, "s134 run: foot\nonly, 12 worlds", fontsize=8, color=INK2, va="top")
a.text(X16 + 4, 128, "round 4 on: foot only (4b), 48 test worlds a generation, 24 hold-out worlds", fontsize=8, color=INK2, va="top")
a.set_ylabel("dig score of the\ngeneration's champion", color=INK, fontsize=10)
a.set_title("2. Each run's own hold-out dig score of its champion, every generation (solid), against that run's start brain (dashed).\n"
            "The pay rule and the hold-out worlds change at each dotted line, so heights are comparable within a run, not across one. Faded: after the next run branched off.",
            loc="left", fontsize=10, color=INK)

# --- Panels 3-4: kept skills -----------------------------------------------------------------------------------
def kept(arena, k):
    rows = [l.split() for l in open(f"{S}/kept24/{arena}-{k}.txt") if l.startswith("NOISE seed")]
    f = [float(r[3]) for r in rows]
    assert len(f) == 24, (arena, k, len(f))
    return st.mean(f), st.stdev(f) / len(f) ** 0.5
ck = [("s91", 0, TRUNK), ("s111-g126", 126, TRUNK), ("s111-g226", 226, TRUNK), ("s134", X16, TRUNK),
      ("s161-g91", X16 + 91, B161), ("s161-g199", X16 + 199, B161), ("s161-g336", X16 + 336, B161),
      ("s162-g97", X16 + 97, B162), ("s162-g190", X16 + 190, B162), ("s162-g338", X16 + 338, B162)]
for a, arena, col_name, what in ((ax[2], "fetch", "fetch_fit", "fetch: find food 10-30 columns out and carry it home"),
                                 (ax[3], "trail", "trail_fit", "trail: walk the length of a pheromone trail")):
    for lab, (ch, stt), x0, branch, col in segs:
        g = sorted(ch); keep = [k for k in g if branch is None or k <= branch]
        s0 = float(stt[col_name])
        a.plot([x0 + k for k in keep], [float(ch[k][col_name]) / s0 for k in keep], color=col, lw=0.8, alpha=0.22)
    base = kept(arena, "s91")[0]
    v = {k: kept(arena, k) for k, _, _ in ck}
    tr = [c for c in ck if c[2] == TRUNK]
    a.plot([c[1] for c in tr], [v[c[0]][0] / base for c in tr], color=TRUNK, lw=2)
    for colr, nm in ((B161, "s161"), (B162, "s162")):
        br = [ck[3]] + [c for c in ck if c[2] == colr]
        a.plot([c[1] for c in br], [v[c[0]][0] / base for c in br], color=colr, lw=2)
    for k, x, colr in ck:
        m, se = v[k]
        a.errorbar(x, m / base, yerr=se / base, fmt="o", ms=8, color=colr, mec=SURF, mew=2, ecolor=colr, elinewidth=1.5, capsize=3, zorder=3)
        end = x > X16 + 300
        a.annotate(f"{m / base:.2f}", (x, m / base), xytext=((-12 if colr == B161 else 12, -3) if end else (0, -26 if colr == B162 else 12)),
                   textcoords="offset points", ha=("right" if colr == B161 else "left") if end else "center", fontsize=8, color=INK2)
    a.axhline(1.0, color=MUTED, lw=1)
    a.axhline(0.8, color=INK, lw=1.2, ls="--")
    a.text(2, 0.815, "0.8 of s91: the fixed floor (keepref=s91) from the next run on", fontsize=8.5, color=INK)
    a.set_ylim(0.5, 1.45)
    a.set_ylabel(f"{arena}, as a share\nof s91's score", color=INK, fontsize=10)
    a.set_title(f"{3 if arena == 'fetch' else 4}. Kept skill, {what}.\nDots: each brain's score on the same 24 sealed worlds over s91's (mean ± 1 s.e.). "
                f"\nFaint lines: every generation's champion over its own run's start, on that run's hold-out worlds (what the old 0.8 floor checked).",
                loc="left", fontsize=10, color=INK)
ax[3].set_xlabel("generations of dig training since s91, along the line of brains that led to s161 and s162 (s111 gens 0-226, then the s134 run's gens 0-34, then round 4 on)",
                 color=INK, fontsize=9.5)
fig.suptitle("The ant brain's dig training, every run from s91 to today", x=0.01, ha="left", fontsize=14, color=INK, fontweight="bold")
fig.tight_layout(rect=(0, 0, 1, 0.975))
fig.savefig(OUT, dpi=110, facecolor=SURF)
print("wrote", OUT)
