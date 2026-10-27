import glob, os, re, csv, collections
from lib import D
# colony-rounds per arm (training) + garden runs
cr = {}; gr = {}; secs = {}
for p in glob.glob(f"{D}/*/*/colsel.csv"):
    b, a = p.split("/")[-3:-1]
    cr[(b, a)] = sum(1 for _ in open(p)) - 1
for p in glob.glob(f"{D}/*/*.timing.csv"):
    b = p.split("/")[-2]; a = os.path.basename(p)[:-11]
    s = collections.Counter(); n = collections.Counter()
    for r in csv.DictReader(open(p)): s[r["kind"]] += float(r["secs"]); n[r["kind"]] += 1
    secs[(b, a)] = s; gr[(b, a)] = n["cg"]
for p in glob.glob(f"{D}/*/*.inrun.csv"):
    b = p.split("/")[-2]; job = os.path.basename(p)[:-10]
    m = re.match(r"(.*)_(\d+)$", job); a = m.group(1)
    rounds = {l.split(",")[0] for l in open(p).read().splitlines()[1:] if l.split(",")[1] == "nest"}
    cr[(b, a)] = cr.get((b, a), 0) + len(rounds)
tot = sum(cr.values()); totg = sum(gr.values())
print("training colony-rounds", tot, "garden runs", totg)
inr = sum(v for (b, a), v in cr.items() if (b, a) not in secs); cs = tot - inr
print("in-run era colony-rounds", inr, "colony era", cs)
# duplicates: (batch, arm, rounds duplicated or None=all)
DUP = [("broodplant1c-1024","bp1pile",None),("colsel10-1022","cs10std",32),("colsel23-1023","cs23ctl",None),("colsel4-1022","cs4inh",16),
       ("colsel4-1022","cs4inhsp",None),("colsel9b-1022","cs9hcs",None),
       ("oldref-1022","kid3",None),("strict-1022","st0",None),("gate2-1022","ctl",None),("ha1next-1022","hA1c",None),("body-1022","hA1c",None),
       ("shortr-1022","hA1r200",None),("needdrop-1022","hA1off",None),("nob-1022","hA1off",None),("ha1next-1022","hA1hg3",None)]
PREFIX = [("colsel20-1023","cs20wean",9)]
dcr = 0; dsec = 0
for b, a, r in DUP:
    n = cr[(b, a)]; part = (r * 64 / n) if r else 1.0
    dcr += n * part
    if (b, a) in secs: dsec += (secs[(b, a)]["train"] + secs[(b, a)]["cg"]) * part
    print(f"  dup {b}/{a}: {n*part:.0f} colony-rounds")
allsec = sum(s["train"] + s["cg"] for s in secs.values())
print(f"duplicate colony-rounds {dcr:.0f} of {tot} = {dcr/tot:.1%}; colony-era duplicate cpu {dsec/3600:.0f} h of {allsec/3600:.0f} h = {dsec/allsec:.1%}")
# controls: same-batch control arms (fresh, not dup)
CTL = [("colsel-1022","csplain"),("colselinh-1022","csiplain"),("colsel3-1022","cs3lex"),("colsel8-1022","cs8hA1"),("colsel14-1023","cs14ctl"),("colsel15-1023","cs15ctl"),
       ("colsel20-1023","cs20wean"),("brood1b-1023","brood1ctl"),("stdcheck-1024b","stdwean"),("farfood1-1024","farctl")]
csec = sum(secs[k]["train"] + secs[k]["cg"] for k in CTL if k in secs)
print(f"fresh control arms (colony era): {csec/3600:.0f} cpu-h = {csec/allsec:.1%}")
