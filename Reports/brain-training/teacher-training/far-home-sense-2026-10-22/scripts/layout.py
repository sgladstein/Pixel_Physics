VERBS=9;MEMORY=12;UNITS=16;CONTEXT=22;FEATURES=45;EMITS=3;HIDDEN=8
SWITCH=0;SHARP=SWITCH+VERBS;PULL=SHARP+1;RATE=PULL+1;S_CTX=RATE+MEMORY
S_IN=CONTEXT+MEMORY+1;S_FEAT=S_CTX+UNITS*S_IN;S_VERB=S_FEAT+UNITS*FEATURES;S_OUT=S_VERB+UNITS*VERBS
JOB=S_OUT+UNITS+1;J_IN=CONTEXT+MEMORY+2;MEM=JOB+VERBS*J_IN;M_IN=CONTEXT+2+VERBS+MEMORY+1
EMIT=MEM+MEMORY*M_IN;E_IN=CONTEXT+MEMORY+1;HID=EMIT+EMITS*E_IN;H_IN=CONTEXT+MEMORY+1;HID_OUT=HID+HIDDEN*H_IN
GENOME_LEN=HID_OUT+VERBS*HIDDEN
def load(p):
    g=[0.0]*GENOME_LEN
    for l in open(p).read().split('\n')[1:]:
        if l.strip():
            i,v=l.split(); g[int(i)]=float(v)
    return g
def ctx_slots(c):
    out=[S_CTX+u*S_IN+c for u in range(UNITS)]+[JOB+v*J_IN+c for v in range(VERBS)]+[MEM+m*M_IN+c for m in range(MEMORY)]
    out+=[EMIT+e*E_IN+c for e in range(EMITS)]+[HID+h*H_IN+c for h in range(HIDDEN)]
    return out
def feat_slots(f): return [S_FEAT+u*FEATURES+f for u in range(UNITS)]
