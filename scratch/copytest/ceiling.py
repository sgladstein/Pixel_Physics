"""SCRATCH (copy test, 2026-10-27): **how much of the teacher's choice the brain's inputs carry at all** -- the ceiling
for any brain that reads only this decision's context, memory and option features. An unconstrained scorer (an MLP over
[context, memory, the option's features, its verb, the last verb], two hidden layers) picks among the decision's options
by a softmax, fitted to the same labels as `copyfit.py`. If it cannot reach the labels either, the senses are missing,
not the brain's shape; if it can and the mind cannot, the mind's shape is the limit.

usage: ceiling.py OUT train=a,b test=c,d [rows=N] [trows=N] [steps=N] [hidden=H] [mem=0|1] [hist=0|1] [state=0|1] [old=0|1|2]
"""
import json, os, sys, time
import numpy as np
import torch
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import brain as B

torch.set_num_threads(int(os.environ.get('THREADS', '4')))
OUT = sys.argv[1]
A = dict(a.split('=', 1) for a in sys.argv[2:])
num = lambda k, d: type(d)(A.get(k, d))
rng = np.random.default_rng(num('seed', 1))
torch.manual_seed(num('seed', 1))


def rows(paths, n):
    R = B.parse_rows(paths, per_file=int(1.15 * n / len(paths)) + 1, hist=HIST)
    return R.subset(np.sort(rng.choice(R.n, n, replace=False))) if R.n > n else R


HIST = A.get('hist', '0') == '1'  # add what the teacher did one decision ago (`brain.parse_rows(hist=True)`)
Rtr = rows(A['train'].split(','), num('rows', 150000))
Rte = rows(A['test'].split(','), num('trows', 40000))
USE_MEM = num('mem', 1)
STATE = A.get('state', '0') == '1'
OLD = num('old', 0)


def inputs(R):
    T = B.to_torch(R)
    lasthot = torch.zeros(R.n, B.VERBS + 1)
    lasthot[torch.arange(R.n), torch.from_numpy(R.last) + 1] = 1.0
    per_row = torch.cat([T['ctx'], T['mem'] * USE_MEM, lasthot], 1)
    vhot = torch.nn.functional.one_hot(T['verb'], B.VERBS).float()
    x = torch.cat([per_row[T['row_of']], T['feats'], vhot], 1)
    if HIST:
        pv = torch.zeros(R.n, B.VERBS + 1)
        pv[torch.arange(R.n), torch.from_numpy(R.prev_verb) + 1] = 1.0
        x = torch.cat([x, pv[T['row_of']], torch.from_numpy(R.opt_hist)], 1)
    if STATE:
        # `state=1`: the old ant's own walk state (`PP_SHADOW_STATE=1` logs), which no mind input carries.
        x = torch.cat([x, torch.from_numpy(R.state)[T['row_of']], torch.from_numpy(R.off)], 1)
    if OLD:
        # `old=1`: the old walk's exploration k and persistence (its own brain's outputs); `old=2` also the score it
        # gave each option's heading -- the hand-built ant's draw itself, the positive control for the ceiling.
        x = torch.cat([x, torch.from_numpy(R.old)[T['row_of']]], 1)
        if OLD >= 2:
            x = torch.cat([x, torch.from_numpy(R.oscore)], 1)
    T['w'] = torch.from_numpy(B.row_weights(R, float(A.get('plain_k', 20))))
    return x, T


Xtr, Ttr = inputs(Rtr)
Xte, Tte = inputs(Rte)
H = num('hidden', 128)
net = torch.nn.Sequential(torch.nn.Linear(Xtr.shape[1], H), torch.nn.ReLU(), torch.nn.Linear(H, H), torch.nn.ReLU(),
                          torch.nn.Linear(H, 1))
opt = torch.optim.Adam(net.parameters(), lr=num('lr', 0.002))


def logp_of(x, T, idx_rows=None):
    s = net(x).squeeze(1)
    N = T['n']
    m = torch.full((N,), -1e30).scatter_reduce(0, T['row_of'], s, reduce='amax')
    e = torch.exp(s - m[T['row_of']])
    tot = torch.zeros(N).index_add(0, T['row_of'], e)
    p = e / tot[T['row_of']]
    pl = torch.zeros(N).index_add(0, T['row_of'][T['lab']], p[T['lab']])
    return torch.log(pl.clamp_min(1e-6)), pl


# minibatches by row
def batch(X, T, R, rows_idx):
    keep = np.zeros(R.n, bool)
    keep[rows_idx] = True
    om = torch.from_numpy(keep[R.row_of])
    new = -torch.ones(R.n, dtype=torch.long)
    new[torch.from_numpy(rows_idx)] = torch.arange(len(rows_idx))
    Tb = dict(row_of=new[T['row_of'][om]], lab=T['lab'][om], n=len(rows_idx), w=T['w'][torch.from_numpy(rows_idx)])
    return X[om], Tb


t0 = time.time()
steps, bs = num('steps', 3000), num('batch', 8192)
curve = []
for step in range(1, steps + 1):
    idx = np.sort(rng.choice(Rtr.n, min(bs, Rtr.n), replace=False))
    xb, Tb = batch(Xtr, Ttr, Rtr, idx)
    opt.zero_grad()
    lp, _ = logp_of(xb, Tb)
    (-(lp * Tb['w']).sum() / Tb['w'].sum()).backward()
    opt.step()
    if step % 500 == 0 or step == steps:
        with torch.no_grad():
            _, ptr = logp_of(Xtr, Ttr)
            _, pte = logp_of(Xte, Tte)
        wm = lambda p, T: float((p * T['w']).sum() / T['w'].sum())
        curve.append(dict(step=step, train=wm(ptr, Ttr), test=wm(pte, Tte)))
        print(f'step {step}: train P {wm(ptr, Ttr):.4f} test P {wm(pte, Tte):.4f} | {time.time() - t0:.0f}s', flush=True)


def by_verb(R, p):
    first = {}
    for o in np.nonzero(R.lab)[0]:
        first.setdefault(R.row_of[o], R.verb[o])
    lv = np.array([first.get(i, -1) for i in range(R.n)])
    return {B.VERB_NAMES[v]: [float(p[lv == v].mean()), int((lv == v).sum())] for v in range(B.VERBS) if (lv == v).any()}


if A.get('dump'):
    # `dump=PATH`: one line per held-out decision -- file index, frame, ant, the label's verb, P(label) -- to join with
    # other per-tick records (the walk-reason `W` rows) by (file, frame, ant).
    first = {}
    for o in np.nonzero(Rte.lab)[0]:
        first.setdefault(Rte.row_of[o], Rte.verb[o])
    with open(A['dump'], 'w') as f:
        for i, m in enumerate(Rte.meta):
            f.write(f"{m[5]},{m[1]},{m[2]},{first.get(i, -1)},{float(pte[i]):.5f}\n")
json.dump(dict(args=A, curve=curve, train_by_verb=by_verb(Rtr, ptr.numpy()), test_by_verb=by_verb(Rte, pte.numpy())),
          open(OUT + '.json', 'w'), indent=1)
print('test by verb', json.dumps(by_verb(Rte, pte.numpy())))
