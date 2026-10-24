import sys, json; sys.path.insert(0, "/home/user/Pixel_Physics/Reports/brain-training/teacher-training/measurements/blueprints")
import numpy as np, nestscore as ns, render_blueprints as rb
ns.PROFILES["low"] = dict(ns.PROFILES["harvester"], core=(3, 1), h_zero=(2, 13))
spec = json.load(open("/home/user/Pixel_Physics/Reports/brain-training/teacher-training/measurements/blueprints/nest-blueprint-specs.json"))
cases = [(b["id"], rb.build(rb.owner_scale(b))) for b in spec["blueprints"]] + list(ns.bad_shapes(rb).items())
for name, cells in cases:
    g = ns.grid_of(cells)
    a = ns.score(g, 64, 128)[0]["score"]; b = ns.score(g, 64, 128, profile="low")[0]["score"]
    print(f"{name:40s} {a:7.0f} {b:7.0f}")
G, X0 = 64, 128
m = np.zeros((128, 256), bool); m[:G] = True; m[G:G+16, X0-2:X0+2] = True
fl = G + 15; out = []
for r in range(7):
    m[fl - r, X0+2:X0+30] = True
    out.append((r+1, round(ns.score(m, G, X0)[0]["score"]), round(ns.score(m, G, X0, profile="low")[0]["score"])))
print("row by row (rows, as is, 7x3 core):", out)
