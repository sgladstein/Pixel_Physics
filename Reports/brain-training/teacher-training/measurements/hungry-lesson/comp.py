import sys, collections
for path in sys.argv[1:]:
    c = collections.Counter(); ok = collections.Counter()
    for line in open(path):
        p = line.rstrip("\n").split(",")
        if len(p) < 16 or p[1] != "Q" or p[6] != "0" or p[15] == "-": continue
        ctx = [float(v) for v in p[11].split()]
        opts = [o.split(":", 4) for o in p[13].split(";")]
        lab = [int(i) for i in p[15].split("|")[1].split(";")]
        lv = int(opts[lab[0]][0]); pick = int(p[7]); pv = int(opts[pick][0])
        cat = ("laden" if ctx[2] > 0 else "empty") + ("-below" if ctx[18] > 0.05 else "-surf") + "-" + {0: "walk", 4: "pickup", 5: "drop"}[lv]
        c[cat] += 1; ok[cat] += pick in lab; c[cat + " verb"] += 0; ok[cat + " verb"] += pv == lv
    print(path.split("/")[-1], " ".join(f"{k} {ok[k]}/{c[k] if c[k] else c[k.replace(' verb','')]}" for k in sorted(set(k.replace(' verb','') for k in c)) for k in (k, k + " verb")))
