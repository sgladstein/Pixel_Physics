"""Hungry ants underground (tank < 0.5): how often food is beside them with jaws free, how often they take it, and how
each crop load taken at home ends -- put down again (Drop with the crop held) or kept until it emptied by digestion.
usage: eatrate.py ARM (reads deep/ARM.hungry.log and deep/ARM_w68/r3)"""
import csv, sys, collections
arm = sys.argv[1]
d = f'deep/{arm}_w68/r3'
surf = [int(l) for l in open(f'{d}/surface.txt') if l.strip()]
n = collections.Counter(); spell = {}
for l in open(f'deep/{arm}.hungry.log'):
    p = l.split(',', 14); i, x, y, v = int(p[3]), int(p[4]), int(p[5]), int(p[6])
    ctx = [float(z) for z in p[10].split()]; best = [float(z) for z in p[13].split()]
    if y <= surf[x]:
        spell.pop(i, None); continue
    n['hungry underground decisions'] += 1
    n['  holding a pellet'] += ctx[2] > 0
    free = ctx[2] == 0 and ctx[1] == 0
    n['  jaws+crop empty'] += free
    if free and best[4] == best[4]:
        n['  ...food beside (pick-up offered)'] += 1
        n['  ...took it'] += v == 4
    if ctx[1] > 0:
        spell.setdefault(i, 0)
        if v == 5:
            n['crop loads ended by a Drop'] += 1; spell.pop(i)
    elif i in spell:
        n['crop loads emptied by digestion'] += 1; spell.pop(i)
print(arm, dict(n))
