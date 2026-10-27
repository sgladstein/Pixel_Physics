"""SCRATCH (copy test): is `brain.forward` the game's `decide`? Replays a copy log through `examples/copyreplay.rs` with a
genome and compares every option score and job score with the torch pass on the same rows.

usage: check.py LOG POOL_OR_GENOME [member] [rows]
"""
import os, subprocess, sys, tempfile
import numpy as np
import torch
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import brain as B

log, src = sys.argv[1], sys.argv[2]
member = int(sys.argv[3]) if len(sys.argv) > 3 else 0
nrows = int(sys.argv[4]) if len(sys.argv) > 4 else 3000
g = B.read_pool(src)[member] if src.endswith('pool.txt') else B.read_genome_text(open(src).read())
core = g[:B.GENOME_LEN]
tmp = tempfile.mkdtemp()
gp, out = f'{tmp}/core.txt', f'{tmp}/replay.txt'
B.write_genome(gp, core)
subprocess.run(['target/release/examples/copyreplay', log, gp, out, str(nrows)], check=True)
ref = [l.rstrip('\n').split('|') for l in open(out)]
R = B.parse_rows([log], labelled_only=False, max_rows=nrows)
assert R.n == len(ref), (R.n, len(ref))
T = B.to_torch(R)
with torch.no_grad():
    s, jobs, logp, on = B.forward(torch.from_numpy(core), T)
s, jobs = s.numpy(), jobs.numpy()
rs = np.concatenate([np.array(r[1].split(' '), np.float32) for r in ref])
rj = np.stack([np.array([np.nan if x == 'nan' else float(x) for x in r[2].split(' ')], np.float32) for r in ref])
on = on.numpy()
ds = np.abs(s[on] - rs[on])
both = ~np.isnan(rj) & ~np.isnan(jobs)
dj = np.abs(jobs[both] - rj[both])
print(f'rows {R.n} options {len(s)} (on {on.sum()}): score |diff| max {ds.max():.2e} mean {ds.mean():.2e}; '
      f'job |diff| max {dj.max():.2e}; job presence agrees {(np.isnan(rj) == np.isnan(jobs)).mean():.4f}')
dig = (R.verb == 3) & on
print(f'dig options {dig.sum()}: job diff on dig {np.abs(jobs[:, 3] - rj[:, 3])[~np.isnan(rj[:, 3])].max() if (~np.isnan(rj[:, 3])).any() else 0:.2e}')
