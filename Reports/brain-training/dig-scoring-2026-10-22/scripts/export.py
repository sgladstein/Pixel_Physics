"""Write the nest score's controls as a fixture for the Rust port: each case's mask (rows below the ground, as runs)
and the Python score's numbers, so the port is checked against the prototype cell for cell."""
import sys, json; sys.path.insert(0, "/home/user/Pixel_Physics/Reports/brain-training/teacher-training/measurements/blueprints")
import numpy as np, nestscore as ns, render_blueprints as rb
exec(open("climb2.py").read().split("spec = json.load")[0])
niche_score = score
B = "/home/user/Pixel_Physics/Reports/brain-training/teacher-training/measurements/blueprints/"
spec = json.load(open(B + "nest-blueprint-specs.json"))
cases = [(f"blueprint {b['id']} (7-cell chambers)", ns.grid_of(rb.build(rb.owner_scale(b)))) for b in spec["blueprints"]]
cases.append(("blueprint harvester_stack (5-cell chambers, as first drawn)", ns.grid_of(rb.build(spec["blueprints"][0]))))
cases.append(("ladder plan (8-16 tall rooms)", ns.grid_of(rb.build(None, ladder=True))))
cases += [("bad: " + k, ns.grid_of(v)) for k, v in ns.bad_shapes(rb).items()]
# the climb: a 16-row shaft with a 28-wide side chamber dug floor first, one row at a time
G, X0 = 64, 128
m = np.zeros((128, 256), bool); m[:G] = True; m[G:G + 16, X0 - 2:X0 + 2] = True
cases.append(("climb: shaft only", m.copy()))
for r in range(7):
    m[G + 15 - r, X0 + 2:X0 + 30] = True
    cases.append((f"climb: {r + 1} rows of a 28-wide chamber", m.copy()))
# the niche's own controls: a niche dug mid-shaft (two openings), and combs of shelves off a shaft (the gaming shape
# that made the niche credit capped)
m = np.zeros((128, 256), bool); m[:G] = True; m[G:G + 30, X0 - 2:X0 + 2] = True
for r in range(4):
    m[G + 15 - r, X0 + 2:X0 + 30] = True
cases.append(("niche: 4 rows of a 28-wide chamber mid-shaft", m.copy()))
def comb(tall, gap, length, depth=56):
    m = np.zeros((128, 256), bool); m[:G] = True; m[G:G + depth, X0 - 2:X0 + 2] = True; y = G + 4
    while y + tall <= G + depth:
        m[y:y + tall, X0 + 2:X0 + 2 + length] = True; m[y:y + tall, X0 - 2 - length:X0 - 2] = True
        y += tall + gap
    return m
for tall, gap in ((1, 1), (2, 1), (4, 1), (4, 3)):
    cases.append((f"comb: shelves {tall} tall, walls {gap}, 30 long", comb(tall, gap, 30)))
out = ["# nest score controls, exported from nestscore.py (PR #753) and the niche prototype (climb2.py, project files nest-score/) for src/lab/nestscore.rs's tests",
       "# grid 256x128, ground row 64 (every row above is open sky), door column 128, profile harvester",
       "# '= name|score|dug|chambers|good_chambers|chamber_credit|layout|tunnel_cells|tunnel_credit|cracks|pit|stray|with_niches'",
       "# then one line per row below the ground holding open cells: 'y x0-x1 x0-x1 ...', and '.' to end the case"]
for name, g in cases:
    s, _ = ns.score(g, G, X0)
    assert g[:G].all()
    out.append("= " + "|".join(str(v) for v in (name, s["score"], s["dug"], s["chambers"], s["good_chambers"], s["chamber_credit"],
                                                s["layout"], s["tunnel_cells"], s["tunnel_credit"], s["cracks"], s["pit"], s["stray"], niche_score(g, G, X0))))
    for y in range(G, 128):
        row = g[y]; xs = np.nonzero(row)[0]
        if not len(xs): continue
        runs, start, prev = [], xs[0], xs[0]
        for x in xs[1:]:
            if x != prev + 1: runs.append((start, prev)); start = x
            prev = x
        runs.append((start, prev))
        out.append(f"{y} " + " ".join(f"{a}-{b}" for a, b in runs))
    out.append(".")
    print(f"{name:60s} {s['score']:8.1f} {niche_score(g, G, X0):8.1f}")
open("nestscore-controls.txt", "w").write("\n".join(out) + "\n")
