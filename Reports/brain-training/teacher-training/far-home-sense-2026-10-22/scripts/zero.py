# usage: zero.py IN OUT -- drop every link on context slot 9 (d_alarm) and feature slot 40 (last_food), PP_HOMEVEC's slots
import sys
from layout import *
kill=set(ctx_slots(9))|set(feat_slots(40))
L=open(sys.argv[1]).read().split('\n'); out=[L[0]]; n=0
for l in L[1:]:
    if not l.strip(): continue
    if int(l.split()[0]) in kill: n+=1; continue
    out.append(l)
open(sys.argv[2],'w').write('\n'.join(out)+'\n'); print('zeroed',n)
