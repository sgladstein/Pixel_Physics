VERBS=9;MEMORY=12;UNITS=16;CONTEXT=22;FEATURES=45;HIDDEN=8;EMITS=3
SWITCH=0;SHARP=VERBS;PULL=SHARP+1;RATE=PULL+1;S_CTX=RATE+MEMORY
S_IN=CONTEXT+MEMORY+1;S_FEAT=S_CTX+UNITS*S_IN;S_VERB=S_FEAT+UNITS*FEATURES;S_OUT=S_VERB+UNITS*VERBS
JOB=S_OUT+UNITS+1;J_IN=CONTEXT+MEMORY+2;MEM=JOB+VERBS*J_IN;M_IN=CONTEXT+2+VERBS+MEMORY+1
EMIT=MEM+MEMORY*M_IN;E_IN=CONTEXT+MEMORY+1;HID=EMIT+EMITS*E_IN;H_IN=CONTEXT+MEMORY+1;HID_OUT=HID+HIDDEN*H_IN;GATE=HID_OUT+VERBS*HIDDEN
print('EMIT',EMIT,'alarm row',EMIT+2*E_IN,EMIT+3*E_IN-1,'GATE(len v5)',GATE)
def alarm_read(ch=2):
    ctx=[4+ch]; s=set()
    for u in range(UNITS):
        for k in ctx: s.add(S_CTX+u*S_IN+k)
        for k in range(3): s.add(S_FEAT+u*FEATURES+9+3*ch+k)
    for v in range(VERBS):
        for k in ctx: s.add(JOB+v*J_IN+k)
    for m in range(MEMORY):
        for k in ctx: s.add(MEM+m*M_IN+k)
    for e in range(EMITS):
        for k in ctx: s.add(EMIT+e*E_IN+k)
    for h in range(HIDDEN):
        for k in ctx: s.add(HID+h*H_IN+k)
    return s
R=alarm_read(); RB=alarm_read(1)
emit=set(range(EMIT+2*E_IN,EMIT+3*E_IN))
import sys
for f in sys.argv[1:]:
    g={int(l.split()[0]):float(l.split()[1]) for l in open(f).read().split('\n')[1:] if l.strip()}
    nz=[i for i in g if g[i]!=0 and i>=S_CTX]
    print(f.split('newchan/')[1], 'live links',len(nz),'alarm-read live',sum(1 for i in nz if i in R),'of',len(R),' B-read live',sum(1 for i in nz if i in RB),' alarm emit row live',sum(1 for i in nz if i in emit), 'sum|w| alarm-read %.1f'%sum(abs(g[i]) for i in nz if i in R), 'B-read %.1f'%sum(abs(g[i]) for i in nz if i in RB))
