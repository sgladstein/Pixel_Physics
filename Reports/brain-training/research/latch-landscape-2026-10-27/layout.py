VERBS=9; MEMORY=12; UNITS=16; CONTEXT=22; FEATURES=45; EMITS=3; HIDDEN=8
VN=['Walk','Turn','Rest','Dig','PickUp','Drop','Eat','Feed','Attack']
CN=['energy','crop','spoil','at_nest','scA','scB','alarm','dA','dB','dAlarm','dEnergy','home_seen','home_dist','d_home','since_nest','age','self','crowd','depth','roofed','roof_cut','hole_drop']
SWITCH=0; SHARP=9; PULL=10; RATE=11; S_CTX=23; S_IN=35; S_FEAT=S_CTX+UNITS*S_IN; S_VERB=S_FEAT+UNITS*FEATURES
S_OUT=S_VERB+UNITS*VERBS; JOB=S_OUT+UNITS+1; J_IN=CONTEXT+MEMORY+2; MEM=JOB+VERBS*J_IN; M_IN=CONTEXT+2+VERBS+MEMORY+1
EMIT=MEM+MEMORY*M_IN; E_IN=35; HID=EMIT+EMITS*E_IN; H_IN=35; HID_OUT=HID+HIDDEN*H_IN; GATE=HID_OUT+VERBS*HIDDEN; GATE_IN=35
REACH_MEM=GATE+UNITS*GATE_IN; REACH_GATE=REACH_MEM+MEMORY*VERBS; CORE=REACH_GATE+UNITS*VERBS
assert GATE==2797 and REACH_MEM==3357 and CORE==3609
def cm(i):  # context+memory+bias row input name
    return CN[i] if i<CONTEXT else (f'mem{i-CONTEXT}' if i<CONTEXT+MEMORY else 'bias')
def name(s):
    if s<SHARP: return f'switch.{VN[s]}'
    if s==SHARP: return 'sharp'
    if s==PULL: return 'pull'
    if s<S_CTX: return f'rate.mem{s-RATE}'
    if s<S_FEAT: u,i=divmod(s-S_CTX,S_IN); return f'unit{u}.ctx.{cm(i)}'
    if s<S_VERB: u,i=divmod(s-S_FEAT,FEATURES); return f'unit{u}.feat{i}'
    if s<S_OUT: u,i=divmod(s-S_VERB,VERBS); return f'unit{u}.verb.{VN[i]}'
    if s<JOB: return f'out.unit{s-S_OUT}' if s-S_OUT<UNITS else 'out.bias'
    if s<MEM:
        v,i=divmod(s-JOB,J_IN); n=cm(i) if i<CONTEXT+MEMORY else ('best' if i==CONTEXT+MEMORY else 'const'); return f'job.{VN[v]}.{n}'
    if s<EMIT:
        m,i=divmod(s-MEM,M_IN)
        if i<CONTEXT: n=CN[i]
        elif i==CONTEXT: n='all_best'
        elif i==CONTEXT+1: n='all_mean'
        elif i<CONTEXT+2+VERBS: n='chose.'+VN[i-CONTEXT-2]
        elif i<CONTEXT+2+VERBS+MEMORY: n=f'mem{i-CONTEXT-2-VERBS}'
        else: n='bias'
        return f'mem{m}.{n}'
    if s<HID: e,i=divmod(s-EMIT,E_IN); return f'emit{e}.{cm(i)}'
    if s<HID_OUT: h,i=divmod(s-HID,H_IN); return f'hid{h}.{cm(i)}'
    if s<GATE: v,h=divmod(s-HID_OUT,HIDDEN); return f'job.{VN[v]}.hid{h}'
    if s<REACH_MEM: u,i=divmod(s-GATE,GATE_IN); return f'gate{u}.{cm(i)}'
    if s<REACH_GATE: m,v=divmod(s-REACH_MEM,VERBS); return f'mem{m}.reach.{VN[v]}'
    if s<CORE: u,v=divmod(s-REACH_GATE,VERBS); return f'gate{u}.reach.{VN[v]}'
    return f'rider{s-CORE}'
def read_pool(path):
    hdr=None; mem=[]; 
    for l in open(path):
        if l.startswith('#'): continue
        if hdr is None: hdr=l.split(); continue
        if l.startswith('member'): mem.append({}); continue
        i,w=l.split(); mem[-1][int(i)]=float(w)
    n=int(hdr[3]); lay=[t for t in hdr if t.startswith('layout=')]
    return hdr, n, (int(lay[0][7:]) if lay else None), mem
def to7(g, n, layout):
    # shift sparse slots of an older genome into layout 7
    out={}
    for s,w in g.items():
        if layout is not None and layout<=5 and s>=2797: s+=CORE-2797
        elif layout==6 and s>=3357: s+=CORE-3357
        out[s]=w
    return out
