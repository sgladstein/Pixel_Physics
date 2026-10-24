import sys; sys.argv = ["x"]
exec(open("climb2.py").read().split("spec = json.load")[0])
import matplotlib; matplotlib.use("Agg"); import matplotlib.pyplot as plt
from matplotlib.gridspec import GridSpec
G, X0 = 64, 128
ink, ink2, gridc, s1, s2 = "#0b0b0b", "#52514e", "#e4e3df", "#2a78d6", "#eb6834"
COL = dict(soil=(0.42, 0.31, 0.22), sky=(0.82, 0.88, 0.94), chamber=(0.24, 0.59, 0.27), niche=(0.93, 0.63, 0.0),
           tunnel=(0.27, 0.43, 0.63), crack=(0.86, 0.24, 0.20))
def picture(m, ax, title, x0=60, x1=200, y1=124):
    tot, parts, nm, cr = score(m, G, X0, verbose='parts')
    img = np.zeros(m.shape + (3,)); img[:] = COL["soil"]; img[:G] = COL["sky"]
    img[parts["nest"]] = COL["tunnel"]; img[parts["chambers"]] = COL["chamber"]; img[nm] = COL["niche"]; img[cr] = COL["crack"]
    old = ns.score(m, G, X0)[0]["score"]
    ax.imshow(img[G - 6:y1, x0:x1], interpolation="nearest"); ax.set_xticks([]); ax.set_yticks([])
    for sp in ax.spines.values(): sp.set_visible(False)
    ax.set_title(f"{title}\nas built {old:.0f}  ->  with niches {tot:.0f}", fontsize=10, color=ink, loc="left")
fig = plt.figure(figsize=(13, 8.2), facecolor="#fcfcfb")
gs = GridSpec(3, 3, figure=fig, height_ratios=[1.25, 1, 1])
a = fig.add_subplot(gs[0, :])
a.set_facecolor("#fcfcfb"); a.spines[["top", "right"]].set_visible(False); a.grid(axis="y", color=gridc); a.set_axisbelow(True)
m = np.zeros((128, 256), bool); m[:G] = True; m[G:G + 16, X0 - 2:X0 + 2] = True
old, new, steps = [ns.score(m, G, X0)[0]["score"]], [score(m, G, X0)], [m.copy()]
for r in range(7):
    m[G + 15 - r, X0 + 2:X0 + 30] = True; old.append(ns.score(m, G, X0)[0]["score"]); new.append(score(m, G, X0)); steps.append(m.copy())
a.plot(range(8), old, color=s2, lw=2, marker="o", ms=7, mec="#fcfcfb", mew=2, label="as built")
a.plot(range(8), new, color=s1, lw=2, marker="o", ms=7, mec="#fcfcfb", mew=2, label="with niches")
a.axhline(0, color=ink2, lw=0.8)
a.set_xlabel("rows of a 28-wide side chamber dug so far (floor first)", color=ink2); a.set_ylabel("nest score", color=ink2)
a.set_title("Every row of a chamber now pays, from the first", loc="left", fontsize=12, color=ink)
a.legend(frameon=False, loc="upper left")
picture(steps[2], fig.add_subplot(gs[1, 0]), "2 rows dug: a niche, not a crack")
picture(steps[4], fig.add_subplot(gs[1, 1]), "4 rows dug: a deeper niche")
mm = np.zeros((128, 256), bool); mm[:G] = True; mm[G:G + 30, X0 - 2:X0 + 2] = True; mm[G + 12:G + 16, X0 + 2:X0 + 30] = True
picture(mm, fig.add_subplot(gs[1, 2]), "a niche mid-shaft (two ways in)")
maze = ns.grid_of(ns.bad_shapes(rb)["corridor maze, no rooms"]); picture(maze, fig.add_subplot(gs[2, 0]), "corridor maze: no niches (too many ways in)", x0=58, x1=198)
def comb(tall, gap, length, depth=56):
    c = np.zeros((128, 256), bool); c[:G] = True; c[G:G + depth, X0 - 2:X0 + 2] = True; y = G + 4
    while y + tall <= G + depth:
        c[y:y + tall, X0 + 2:X0 + 2 + length] = True; c[y:y + tall, X0 - 2 - length:X0 - 2] = True; y += tall + gap
    return c
picture(comb(2, 1, 30), fig.add_subplot(gs[2, 1]), "shelf comb: niches, but capped", x0=88, x1=168)
import json
spec = json.load(open("/home/user/Pixel_Physics/Reports/brain-training/teacher-training/measurements/blueprints/nest-blueprint-specs.json"))
picture(ns.grid_of(rb.build(rb.owner_scale(spec["blueprints"][0]))), fig.add_subplot(gs[2, 2]), "harvester blueprint: unchanged", x0=40, x1=216)
fig.text(0.01, 0.005, "green chamber   amber niche   blue tunnel   red crack", color=ink2, fontsize=10)
fig.tight_layout(rect=(0, 0.02, 1, 1)); fig.savefig("/mnt/project-files/nest-score/climb-fixed.png", dpi=120)
