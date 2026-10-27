"""SCRATCH (copy test, 2026-10-27): the mind's `decide` as a differentiable torch function, for fitting a whole genome
to a teacher's labels at once (`examples/homefit.rs` fits by central differences, one slot at a time, which is fine for
80 slots and hopeless for 2,000+).

Mirrors `src/sim/creature/mind.rs` at layout 8 (`LAYOUT`): `decide_planned`'s scorer (context and memory into each unit,
the verb row, the clamp gate times the feature sum, the output squash and the sharpness squash), the job head (context,
memory, the verb's best score, its bias, the hidden units, the pull towards the last verb), and the two draws
(`choose_weighted` at `PICK_FLOOR`, over the verbs' jobs, then over the verb's options' scores), as `homefit::probs`
reads them. Left out, as homefit leaves them: the dig head's re-rank (`PP_DIGHEAD`, off), the recall tilt, and the
gate's reach row (`REACH_GATE`: the last decision's reach is not in the copy log, and std_pool carries none). Memory is
read from the log as it was written (teacher-forced) unless `mem_model` is used.

Checked against the game: `check.py` compares every option score and job score with `examples/copyreplay.rs` on the
same rows and genome.
"""
import numpy as np
import torch

UNITS, MEMORY, EMITS, HIDDEN, CONTEXT, FEATURES, VERBS = 16, 12, 3, 8, 22, 45, 9
SWITCH = 0
SHARP = SWITCH + VERBS
PULL = SHARP + 1
RATE = PULL + 1
S_CTX = RATE + MEMORY
S_IN = CONTEXT + MEMORY + 1
S_FEAT = S_CTX + UNITS * S_IN
S_VERB = S_FEAT + UNITS * FEATURES
S_OUT = S_VERB + UNITS * VERBS
JOB = S_OUT + UNITS + 1
J_IN = CONTEXT + MEMORY + 2
MEM = JOB + VERBS * J_IN
M_IN = CONTEXT + 2 + VERBS + MEMORY + 1
EMIT = MEM + MEMORY * M_IN
E_IN = CONTEXT + MEMORY + 1
HID = EMIT + EMITS * E_IN
H_IN = CONTEXT + MEMORY + 1
HID_OUT = HID + HIDDEN * H_IN
GATE = HID_OUT + VERBS * HIDDEN
GATE_IN = CONTEXT + MEMORY + 1
REACH_MEM = GATE + UNITS * GATE_IN
REACH_GATE = REACH_MEM + MEMORY * VERBS
WRITE = REACH_GATE + UNITS * VERBS
W_IN = CONTEXT + MEMORY + VERBS + 1
GENOME_LEN = WRITE + MEMORY * W_IN
GENOME_LEN_V5, GENOME_LEN_V6, GENOME_LEN_V7 = GATE, REACH_MEM, WRITE
FIRST_LINK = S_CTX
PICK_FLOOR = 0.05
REACH_FLOOR = 0.25
VERB_NAMES = ['walk', 'turn', 'rest', 'dig', 'pick_up', 'drop', 'eat', 'feed', 'attack']
ATTACK = 8
assert GENOME_LEN == 4137, GENOME_LEN


def squash(x):
    return x / (1.0 + x.abs())


# ---- genome files ---------------------------------------------------------------------------------------------------

def read_genome_text(text):
    """A genome file's text -> (vector in TODAY's layout, riders kept after the core), as `mind::enter_gate` reads it."""
    lines = [l for l in text.split('\n') if l.strip()]
    head = lines[0].split()
    assert head[0] == 'genome'
    n = int(head[1])
    layout = next((int(t[7:]) for t in head[2:] if t.startswith('layout=')), None)
    g = np.zeros(n, dtype=np.float32)
    for l in lines[1:]:
        i, w = l.split()
        g[int(i)] = float(w)
    if layout is not None and layout >= 8:
        return g
    if layout in (6, 7):
        at = GENOME_LEN_V6 if layout == 6 else GENOME_LEN_V7
    elif GENOME_LEN_V5 <= n < GENOME_LEN_V6:
        at = GENOME_LEN_V5
    elif layout is None and GENOME_LEN_V6 <= n < GENOME_LEN_V7:
        at = GENOME_LEN_V6
    elif layout is None and GENOME_LEN_V7 <= n < GENOME_LEN:
        at = GENOME_LEN_V7
    else:
        return g
    return np.concatenate([g[:at], np.zeros(GENOME_LEN - at, np.float32), g[at:]])


def read_pool(path):
    """A `*pool.txt` (header `pool N genome L layout=K`, then `member k` blocks of `slot value`) -> list of genomes in
    today's layout (riders kept)."""
    lines = [l for l in open(path).read().split('\n') if l.strip() and not l.startswith('#')]
    head = lines[0].split()
    n = int(head[3])
    layout = next((t for t in head[4:] if t.startswith('layout=')), '')
    # Each member after the first is written as member 0 plus its own lines (`exam.py`'s reader does the same).
    out, cur = [], None
    for l in lines[1:]:
        if l.startswith('member '):
            cur = list(out[0]) if out else []
            out.append(cur)
            continue
        cur.append(l)
    return [read_genome_text(f'genome {n} {layout}\n' + '\n'.join(m)) for m in out]


def write_genome(path, g):
    """Save in today's layout with the header `train::save_genome` writes."""
    with open(path, 'w') as f:
        f.write(f'genome {len(g)} layout=8\n')
        for i, w in enumerate(g):
            if w != 0.0:
                f.write(f'{i} {repr(float(np.float32(w)))}\n')


def live_links(g):
    return int((np.abs(g[FIRST_LINK:GENOME_LEN]) >= 1e-6).sum())


# ---- copy logs ------------------------------------------------------------------------------------------------------

class Rows:
    """Decisions from copy-log Q rows (`scratch_log::copy_decision`), options flattened with their row index."""

    def __init__(self, ctx, mem, last, row_of, verb, feats, lab, meta):
        self.ctx, self.mem, self.last = ctx, mem, last
        self.row_of, self.verb, self.feats, self.lab = row_of, verb, feats, lab
        self.meta = meta  # per row: (tag, frame, id, x, y)
        self.n = len(ctx)

    def subset(self, idx):
        idx = np.asarray(idx)
        keep = np.zeros(self.n, bool)
        keep[idx] = True
        new = -np.ones(self.n, np.int64)
        new[idx] = np.arange(len(idx))
        om = keep[self.row_of]
        # keep the rows' order as in idx
        order = np.argsort(new[self.row_of[om]], kind='stable')
        sel = np.nonzero(om)[0][order]
        return Rows(self.ctx[idx], self.mem[idx], self.last[idx], new[self.row_of[sel]], self.verb[sel], self.feats[sel],
                    self.lab[sel], [self.meta[i] for i in idx])


def parse_rows(paths, labelled_only=True, keep=None, max_rows=None, settle2=False, per_file=None, seed=1):
    """Read Q rows. `keep(row_fields) -> bool` filters; settle=2 rows (memory context written before a sampled row) are
    skipped unless `settle2`. `per_file=N` keeps about N rows of each file, drawn at random before parsing (so a
    multi-gigabyte log set never sits in memory)."""
    ctx, mem, last, row_of, verb, feats, lab, meta = [], [], [], [], [], [], [], []
    n = 0
    rs = np.random.default_rng(seed)
    for p in paths:
        prob = 1.0
        if per_file:
            total = sum(1 for l in open(p) if ',Q,' in l[:24] and l.split(',', 7)[6] != '2')
            prob = min(1.0, per_file / max(total, 1))
        for line in open(p):
            if ',Q,' not in line[:24]:
                continue
            if prob < 1.0 and rs.random() > prob:
                continue
            f = line.rstrip('\n').split(',')
            if f[1] != 'Q':
                continue
            if f[6] == '2' and not settle2:
                continue
            lf = f[15] if len(f) > 15 else '-'
            labs = set(int(i) for i in lf[2:].split(';')) if lf.startswith('1|') else None
            if labelled_only and labs is None:
                continue
            if keep is not None and not keep(f):
                continue
            ctx.append(np.array(f[11].split(' '), np.float32))
            mem.append(np.array(f[12].split(' '), np.float32))
            last.append(int(f[8]))
            meta.append((f[0], int(f[2]), int(f[3]), int(f[4]), int(f[5])))
            for k, o in enumerate(x for x in f[13].split(';') if x):
                q = o.split(':', 4)
                v = np.zeros(FEATURES, np.float32)
                for kv in q[4].split(' '):
                    if kv:
                        i, val = kv.split('=')
                        i = int(i)
                        if i < FEATURES:
                            v[i] = float(val)
                feats.append(v)
                verb.append(int(q[0]))
                row_of.append(n)
                lab.append(labs is not None and k in labs)
            n += 1
            if max_rows and n >= max_rows:
                break
        if max_rows and n >= max_rows:
            break
    return Rows(np.stack(ctx), np.stack(mem), np.array(last, np.int64), np.array(row_of, np.int64),
                np.array(verb, np.int64), np.stack(feats), np.array(lab, bool), meta)


# ---- the forward pass -----------------------------------------------------------------------------------------------

def to_torch(r):
    return dict(ctx=torch.from_numpy(r.ctx), mem=torch.from_numpy(r.mem), last=torch.from_numpy(r.last),
                row_of=torch.from_numpy(r.row_of), verb=torch.from_numpy(r.verb), feats=torch.from_numpy(r.feats),
                lab=torch.from_numpy(r.lab), n=r.n)


def forward(g, T, sharp_max=4.0, attack_on=False):
    """g: torch vector (today's core first). Returns (scores[M], jobs[N,V] with nan where absent, logp[M] the log
    probability of each option, on[M])."""
    N, ctx, mem, row_of, verb, feats = T['n'], T['ctx'], T['mem'], T['row_of'], T['verb'], T['feats']
    cm = torch.cat([ctx, mem], 1)                                     # [N, 34]
    Wc = g[S_CTX:S_FEAT].view(UNITS, S_IN)
    pre = cm @ Wc[:, :CONTEXT + MEMORY].T + Wc[:, CONTEXT + MEMORY]    # [N, U]
    Gr = g[GATE:REACH_MEM].view(UNITS, GATE_IN)
    gate = torch.clamp(1.0 + cm @ Gr[:, :CONTEXT + MEMORY].T + Gr[:, CONTEXT + MEMORY], 0.0, 2.0)
    F = g[S_FEAT:S_VERB].view(UNITS, FEATURES)
    V = g[S_VERB:S_OUT].view(UNITS, VERBS)
    out = g[S_OUT:S_OUT + UNITS]
    c0 = g[S_OUT + UNITS]
    sharp = torch.clamp(1.0 + g[SHARP], 0.25, sharp_max)
    z = pre[row_of] + V.T[verb] + gate[row_of] * (feats @ F.T)       # [M, U]
    raw = c0 + squash(z) @ out
    s = squash(sharp * raw)
    acts = torch.ones(VERBS, dtype=torch.bool)
    acts[ATTACK] = attack_on
    on = acts[verb] & (g[SWITCH + verb] > 0)
    # each verb's best score
    key = row_of * VERBS + verb
    neg = torch.full((N * VERBS,), float('-inf'))
    best = neg.scatter_reduce(0, key[on], s[on], reduce='amax', include_self=True).view(N, VERBS)
    present = torch.isfinite(best)
    bestz = torch.where(present, best, torch.zeros_like(best))
    # hidden units
    Hr = g[HID:HID_OUT].view(HIDDEN, H_IN)
    hid = squash(cm @ Hr[:, :CONTEXT + MEMORY].T + Hr[:, CONTEXT + MEMORY])
    J = g[JOB:MEM].view(VERBS, J_IN)
    HO = g[HID_OUT:GATE].view(VERBS, HIDDEN)
    pull = torch.clamp(g[PULL], 0.0, 1.0)
    lastv = T['last']
    lasthot = torch.zeros(N, VERBS)
    has = lastv >= 0
    lasthot[has, lastv[has]] = 1.0
    jsum = cm @ J[:, :CONTEXT + MEMORY].T + J[:, CONTEXT + MEMORY] * bestz + J[:, CONTEXT + MEMORY + 1] + hid @ HO.T \
        + pull * lasthot
    jobs = squash(sharp * jsum)
    wv = (PICK_FLOOR + (jobs + 1.0) / 2.0) ** 2
    wv = torch.where(present, wv, torch.zeros_like(wv))
    logpv = torch.log(wv.clamp_min(1e-30)) - torch.log(wv.sum(1, keepdim=True))   # [N, V]
    wo = (PICK_FLOOR + (s + 1.0) / 2.0) ** 2
    wo = torch.where(on, wo, torch.zeros_like(wo))
    tot = torch.zeros(N * VERBS).index_add(0, key, wo)
    logpo = torch.log(wo.clamp_min(1e-30)) - torch.log(tot[key].clamp_min(1e-30))
    logp = torch.where(on, logpv.view(-1)[key] + logpo, torch.full_like(s, float('-inf')))
    jobs_out = torch.where(present, jobs, torch.full_like(jobs, float('nan')))
    return s, jobs_out, logp, on


def row_weights(R, plain_k):
    """Undo `PP_SHADOW_SAMPLE=k`: a row whose labels are all plain (Walk, Rest, Turn around) was kept one in k, every
    other labelled row always, so a plain row stands for k decisions."""
    plain = np.ones(R.n, bool)
    lv = R.verb[R.lab]
    np.logical_and.at(plain, R.row_of[R.lab], np.isin(lv, [0, 1, 2]))
    return np.where(plain, float(plain_k), 1.0).astype(np.float32)


def label_logp(logp, T):
    """log P(the brain picks a labelled option), per row."""
    N = T['n']
    p = torch.zeros(N).index_add(0, T['row_of'][T['lab']], torch.exp(logp[T['lab']]))
    return torch.log(p.clamp_min(1e-6)), p
