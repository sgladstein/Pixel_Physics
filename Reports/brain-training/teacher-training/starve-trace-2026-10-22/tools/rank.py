# usage: rank.py ARM... -- every starved ant into one cause (first match wins), from dead.json
import json,glob,collections,sys
def cause(o):
    dx=abs(o['dx']); sp=o['spoil_frac']>0.5
    if o['death']<6000: return "0 starved at the round's start (founders begin at 3 tanks)"
    if dx>=60: return "1 roamed past the food and never came back" + (" (at a side wall)" if dx>=100 else " (open field)")
    if dx<=15 and sp and o['burst']>=5: return "2 colony famine at home, holding a soil pellet (5+ deaths within 3k frames)"
    if dx<=15 and sp: return "3 alone at home with a pellet it could not put down"
    if sp: return "4 holding a pellet out in the food band"
    return "5 other: empty, near home or in the food band"
for arm in sys.argv[1:]:
    c=collections.Counter(); n=0
    for f in glob.glob(f"l/{arm}[0-9]*/dead.json"):
        for o in json.load(open(f)): c[cause(o)]+=1; n+=1
    print(f"{arm}: {n} starved")
    for k in sorted(c): print(f"   {c[k]:3d} ({c[k]/n:.0%})  {k[2:]}")
