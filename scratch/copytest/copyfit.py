"""SCRATCH (copy test, 2026-10-27): fit a whole mind genome to a teacher's labels (the shadow log's 16th field), by
gradient on `brain.forward` -- the game's `decide` in torch, checked by `check.py`.

usage: copyfit.py OUT START [key=value ...]
  START       a genome file, or POOL.txt:K (member K of a pool)
  train=a,b   copy logs to fit on; test=c,d  held-out logs (other seeds), scored only
  rows=N      rows sampled from the train logs (default 150000); trows=N from the test logs (default 40000)
  steps=N     Adam steps (default 3000); lr=x (default 0.02); batch=N rows per step (default 8192)
  l1=x        L1 on the trained weights (default 0, sparsity; the brain pays per live link)
  slots=all|nogate|scorer   which blocks move (default all: scorer, job, hidden units, gates, sharpness, pull)
  sharp_max=x the ceiling `PP_SHARP_MAX` the run will use (default 4)
  seed=N
Writes OUT.raw.txt (today's layout, START's riders kept) and OUT.json (curves and the final scores).
P(label) is the brain's chance of picking a labelled option, averaged over decisions.
"""
import json, os, sys, time
import numpy as np
import torch
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import brain as B

torch.set_num_threads(int(os.environ.get('THREADS', '4')))
OUT, START = sys.argv[1], sys.argv[2]
A = dict(a.split('=', 1) for a in sys.argv[3:])
num = lambda k, d: type(d)(A.get(k, d))
rng = np.random.default_rng(num('seed', 1))
torch.manual_seed(num('seed', 1))


def load_start(s):
    if ':' in s and s.split(':')[0].endswith('pool.txt'):
        p, k = s.rsplit(':', 1)
        return B.read_pool(p)[int(k)]
    return B.read_genome_text(open(s).read())


def sample_rows(paths, n):
    R = B.parse_rows(paths, per_file=int(1.15 * n / len(paths)) + 1)
    if R.n > n:
        R = R.subset(np.sort(rng.choice(R.n, n, replace=False)))
    return R


def slot_mask(kind):
    m = np.zeros(B.GENOME_LEN, bool)
    m[B.S_CTX:B.JOB] = True                # scorer: context+memory+bias, features, verb rows, outputs and constant
    if kind == 'scorer':
        return m
    m[B.JOB:B.MEM] = True                  # job head
    m[B.HID:B.GATE] = True                 # hidden units and their outputs onto the jobs
    m[B.SHARP] = m[B.PULL] = True
    if kind == 'all':
        m[B.GATE:B.REACH_MEM] = True       # context gates (layout 6)
    return m


PLAIN_K = num('plain_k', 20.0)  # the logs' PP_SHADOW_SAMPLE


PATHS = {}


def tt(R, paths):
    # A DAgger log (`PP_OLDLAB_ALL`, under `/dag/`) keeps every labelled row, so its plain rows stand for one decision.
    k = np.array([1.0 if '/dag/' in paths[m[5]] else PLAIN_K for m in R.meta], np.float32)
    T = B.to_torch(R)
    T['w'] = torch.from_numpy(B.row_weights(R, k))
    return T


def chunks(R, size):
    idx = rng.permutation(R.n)
    return [tt(R.subset(np.sort(idx[i:i + size])), TRP) for i in range(0, R.n, size)]


def score(g, T, R=None):
    """mean P(label), mean -ln P(label) (both weighted back to every decision), and P(label) by the label's verb."""
    with torch.no_grad():
        _, _, logp, on = B.forward(g, T, sharp_max=SHARP_MAX)
        lp, p = B.label_logp(logp, T)
    wt = T['w']
    out = dict(p=float((p * wt).sum() / wt.sum()), nll=float(-(lp * wt).sum() / wt.sum()), n=T['n'])
    if R is not None:
        first = {}
        for o in np.nonzero(R.lab)[0]:
            first.setdefault(R.row_of[o], R.verb[o])
        lv = np.array([first.get(i, -1) for i in range(R.n)])
        pn = p.numpy()
        out['by_verb'] = {B.VERB_NAMES[v]: [float(pn[lv == v].mean()), int((lv == v).sum())] for v in range(B.VERBS) if (lv == v).any()}
    return out


SHARP_MAX = num('sharp_max', 4.0)
g0 = load_start(START)
riders = g0[B.GENOME_LEN:]
# `switches=v,v` (verb indices): switch these verbs on (gene 1.0) before fitting -- a verb's switch is a threshold
# gene a gradient cannot move, and std_pool has Turn around off in every member while the old ant turns.
for v in [int(x) for x in A.get('switches', '').split(',') if x]:
    g0[B.SWITCH + v] = max(g0[B.SWITCH + v], 1.0)
core0 = torch.from_numpy(g0[:B.GENOME_LEN].copy())
t0 = time.time()
TRP, TEP = A['train'].split(','), A.get('test', '').split(',')
Rtr = sample_rows(TRP, num('rows', 150000))
Rte = sample_rows(TEP, num('trows', 40000)) if A.get('test') else None
print(f'rows: train {Rtr.n} ({len(Rtr.verb)} options), test {Rte.n if Rte else 0}; {time.time() - t0:.0f}s', flush=True)
Ttr_all, Tte = tt(Rtr, TRP), (tt(Rte, TEP) if Rte else None)
batches = chunks(Rtr, num('batch', 8192))
mask = torch.from_numpy(slot_mask(A.get('slots', 'all')))
w = core0.clone().requires_grad_(True)
opt = torch.optim.Adam([w], lr=num('lr', 0.02))
l1 = num('l1', 0.0)
PROX = num('prox', 0.0)
steps = num('steps', 3000)
log = dict(args=A, start=START, curve=[])
start_tr, start_te = score(core0, Ttr_all, Rtr), (score(core0, Tte, Rte) if Tte else None)
print(f'start: train P {start_tr["p"]:.4f} nll {start_tr["nll"]:.4f}' + (f' | test P {start_te["p"]:.4f}' if start_te else ''), flush=True)
log['start'] = dict(train=start_tr, test=start_te, live=B.live_links(g0))
for step in range(1, steps + 1):
    T = batches[step % len(batches)]
    opt.zero_grad()
    _, _, logp, _ = B.forward(w, T, sharp_max=SHARP_MAX)
    lp, _ = B.label_logp(logp, T)
    loss = -(lp * T['w']).sum() / T['w'].sum() + l1 * (w * mask).abs().sum()
    loss.backward()
    w.grad[~mask] = 0.0
    opt.step()
    with torch.no_grad():
        # `prox=x`: soft-threshold the trained weights by lr*x after each step (an L1 that reaches exact zeros), so the
        # copy can be made as sparse as the brain's per-link bill wants; settings (sharpness, pull) are left alone.
        if PROX > 0:
            links = mask.clone()
            links[:B.FIRST_LINK] = False
            shrunk = torch.sign(w) * torch.clamp(w.abs() - num('lr', 0.02) * PROX, min=0.0)
            w[links] = shrunk[links]
        w.clamp_(-40.0, 40.0)
        w[~mask] = core0[~mask]
    if step % 250 == 0 or step == steps:
        tr = score(w.detach(), Ttr_all)
        te = score(w.detach(), Tte) if Tte else None
        live = B.live_links(np.concatenate([w.detach().numpy(), riders]))
        log['curve'].append(dict(step=step, train=tr['p'], test=te['p'] if te else None, nll=tr['nll'], live=live))
        print(f'step {step}: train P {tr["p"]:.4f} nll {tr["nll"]:.4f}' + (f' | test P {te["p"]:.4f} nll {te["nll"]:.4f}' if te else '')
              + f' | live {live} | {time.time() - t0:.0f}s', flush=True)
g = np.concatenate([w.detach().numpy(), riders]).astype(np.float32)
g[np.abs(g) < 1e-6] = 0.0
B.write_genome(OUT + '.raw.txt', g)
log['final'] = dict(train=score(torch.from_numpy(g[:B.GENOME_LEN]), Ttr_all, Rtr),
                    test=score(torch.from_numpy(g[:B.GENOME_LEN]), Tte, Rte) if Tte else None, live=B.live_links(g))
json.dump(log, open(OUT + '.json', 'w'), indent=1)
print('final', json.dumps(log['final']), flush=True)
