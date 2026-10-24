import sys; sys.path.insert(0, "/home/user/Pixel_Physics/Reports/brain-training/teacher-training/measurements/blueprints")
import numpy as np, nestscore as ns
G, X0 = 64, 128
def base(depth=16):
    m = np.zeros((128, 256), bool); m[:G] = True
    m[G:G+depth, X0-2:X0+2] = True   # 4-wide shaft
    return m
def room(m, h, w, depth=16):
    # a flat-floored side room off the shaft bottom, floor at the shaft's floor
    fl = G + depth - 1
    m[fl-h+1:fl+1, X0+2:X0+2+w] = True
    return m
print("shaft only", ns.score(base(), G, X0)[0]["score"])
print("h \\ w " + " ".join(f"{w:>6d}" for w in (4, 7, 10, 14, 21, 28, 40)))
for h in (2, 3, 4, 5, 6, 7, 8, 10):
    row = []
    for w in (4, 7, 10, 14, 21, 28, 40):
        s = ns.score(room(base(), h, w), G, X0)[0]
        row.append(s["score"])
    print(f"h={h:2d}  " + " ".join(f"{v:6.0f}" for v in row))
# cell-by-cell growth of a 7-tall chamber, dug like an ant: floor row first outward, then rows up (a lens rising)
m = base(); prev = ns.score(m, G, X0)[0]["score"]; fl = G + 15
traj = []
for r in range(7):
    for x in range(X0+2, X0+2+28):
        m[fl - r, x] = True
    s = ns.score(m, G, X0)[0]["score"]; traj.append((r+1, s, s - prev)); prev = s
print("row by row, 28 wide:", [(r, round(s), round(d)) for r, s, d in traj])
