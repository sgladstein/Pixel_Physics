import sys
S = sys.argv[1]
F = open(f'{S}/founders_std.txt').read().strip()
base = f"dist=25 spread=10 window=4000 sel=off food=mix patches=20000:40:120"
DOOR_T = "setenv=PP_BED_W=448,PP_MIND_DOOR_OPEN=drop"
DOOR_L = "setenv=PP_BED_W=448"
ALL = "oldage=40000 lay_cap=off grants=1 tick=lab labrain=1"
arms = {
 'T':   f"grants=3 {DOOR_T}",
 'A':   f"grants=3 {DOOR_T} oldage=40000",
 'B':   f"grants=3 {DOOR_T} lay_cap=off",
 'C':   f"grants=1 {DOOR_T}",
 'D':   f"grants=3 {DOOR_T} tick=lab",
 'E':   f"grants=3 {DOOR_T} tick=lab labrain=1",
 'F':   f"grants=3 {DOOR_L}",
 'ALL': f"{DOOR_L} {ALL}",
}
long = {'LT': arms['T'], 'LALL': arms['ALL']}
old = {'OT': arms['T'] + " nomind=1", 'OALL': arms['ALL'] + " nomind=1"}
jobs = []
seeds = [c * 1000 + 99 for c in range(1, 17)]
def add(name, extra, frames):
    for s in seeds:
        jobs.append(f"{name} {s} founders={F} {base} {extra} frames={frames} seed={s} out={S}/runs/{name}/{s}")
for k, v in arms.items(): add(k, v, 200000)
for k, v in old.items(): add(k, v, 200000)
for k, v in long.items(): add(k, v, 600000)
open(f'{S}/jobs.txt', 'w').write('\n'.join(jobs) + '\n')
print(len(jobs))
