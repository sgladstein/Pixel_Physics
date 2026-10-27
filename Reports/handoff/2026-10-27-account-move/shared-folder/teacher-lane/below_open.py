"""SCRATCH (teacher lane): a CONTEXT sense the job stage reads directly -- how far below the head the cavity it stands
in goes (deepest open row within reach R, minus the head's row; recovered from the Q row's 15th field) -- against the
option-level readings. Same probe as dig_bottom.py (logistic, 4 folds by world; AUC and wall false alarms at 0.9 recall).
usage: python3 -I below_open.py COPIER_DIR LOG"""
import sys
sys.path.insert(0, sys.argv[1])
sys.argv = sys.argv[:2] + sys.argv[2:]
import numpy as np
import copier
import importlib.util
spec = importlib.util.spec_from_file_location("db", __file__.replace("below_open.py", "dig_bottom.py"))
db = importlib.util.module_from_spec(spec)
_argv = sys.argv
sys.argv = [_argv[0], _argv[1]]  # no logs: dig_bottom's loop runs over nothing
spec.loader.exec_module(db)
REACH = db.REACH
for path in _argv[2:]:
    dec, _ = copier.parse_log(path)
    bot = db.bottoms(path)
    rows = []
    for r, b in zip(dec, bot):
        if r["dig"] not in ("M", "N"):
            continue
        k = next(k for k, o in enumerate(r["opts"]) if o[0] == db.DIG)
        ty = r["opts"][k][2]
        below = [(ty - b[k][i]) - r["y"] for i in range(len(REACH))]  # deepest open row minus head row
        cm = np.concatenate([r["ctx"], r["mem"]])
        cm[18] = 0.0  # depth cut
        rows.append((r["tag"], r["dig"] == "M", below, cm))
    tags = np.array([t for t, *_ in rows]); y = np.array([m for _, m, *_ in rows])
    B = np.array([b for *_, b, _ in rows], float); CM = np.array([c for *_, c in rows])
    print(f"== {path}: {len(rows)} decisions")
    for i, R in enumerate(REACH):
        print(f"   open below R{R}: alone AUC {db.auc(-B[:, i], y):.3f}; at the foot median {np.median(B[y, i]):.0f}, walls-only median {np.median(B[~y, i]):.0f}")
    for name, X in [("depth cut", CM)] + [(f"depth cut + open below R{R}", np.hstack([CM, B[:, [i]]])) for i, R in enumerate(REACH)] + [("depth cut + open below R3 + R6", np.hstack([CM, B[:, [1, 3]]]))]:
        s = db.heldout(X, y, tags)
        print(f"   probe {name:34s} AUC {db.auc(s, y):.3f}   wall false alarms at 0.9 recall {db.fa_at_recall(s, y):.4f}")
