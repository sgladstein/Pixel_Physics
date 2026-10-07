#!/usr/bin/env python3
"""Is there a real gradient? Solve candidate gas-like fields on the shared
baseline's nest maps (main 3ba1e7bd5) and measure what an ant could read.

Fields (all steady state, 4-neighbour diffusion, so nothing leaks through a
corner):
  air_sealed   nest air: ants emit 1, brood 0.25; soil is a wall; open sky
               (an open cell above the ground with nothing over it) is 0.
  air_p02      the same, soil passes 2% of what air does (leaky walls).
  air_plane    the same, soil passes everything: what pheromone.rs does,
               since its planes ignore terrain.
  larva_L3     larva scent: brood emit 1, walls sealed, decays with a
  larva_L10    decay length of 3 / 10 cells.

Sources are averaged over the 11 maps at f-5000..f+5000 (ant positions
over ~10k frames), on the target map's geometry.

Measured over the dug nest (open cells below the ground line):
  flat      share of cells where no heading's 6-cell sensor reads a contrast
            (front-here)/(front+here) above 1% / 0.1%.
  out       share whose steepest walkable descent reaches open sky.
  agree     share whose steepest descent step also shortens the walk out
            (BFS geodesic, 8-neighbour, from open sky).
  wallward  share whose steepest descent over ALL neighbours is a soil cell.
  ascent    where steepest ascent ends: number of tops and what sits there.
  range     max/min over the nest, and 1-cell differences under 1/65536 and
            1/256 of the nest max.
  settle    frames for the current plane's diffusion rate (D = 0.25/36
            cells^2 per frame, 4-neighbour flux form) to bring the nest within
            10% of steady state, from empty.
"""
import sys, os, json, math
import numpy as np
import scipy.sparse as sp
import scipy.sparse.linalg as spla
from scipy.sparse.csgraph import connected_components
from collections import deque

BASE = '/mnt/project-files/deep-trace/baseline/3ba1e7bd5'
OPEN = list('.abc')  # open air; brood and crumbs are pushed past (PushPast)
N4 = ((0, 1), (1, 0), (0, -1), (-1, 0))
N8 = N4 + ((1, 1), (1, -1), (-1, 1), (-1, -1))
GROUND_ROW = 60  # ground_y - y0 (deeptrace write_map: y0 = ground_y - 60)


def load(path):
    with open(path) as f:
        x0, y0, w, h = map(int, f.readline().split())
        rows = [f.readline().rstrip('\n') for _ in range(h)]
    return np.array([list(r.ljust(w, '#')[:w]) for r in rows])


def masks(g):
    h, w = g.shape
    openm = np.isin(g, OPEN)
    wall = ~openm
    first_wall = np.where(wall.any(axis=0), wall.argmax(axis=0), h)
    rows = np.arange(h)[:, None]
    sky = openm & (rows < first_wall[None, :]) & (rows < GROUND_ROW)
    nest = openm & (rows > GROUND_ROW)
    mound = openm & (rows < GROUND_ROW) & ~sky
    return openm, sky, nest, mound


def pairs(h, w, dy, dx):
    ys, xs = np.mgrid[0:h, 0:w]
    ny, nx = ys + dy, xs + dx
    ok = (ny >= 0) & (ny < h) & (nx >= 0) & (nx < w)
    return ys[ok], xs[ok], ny[ok], nx[ok]


def solve(openm, sky, src, eps, k):
    """Steady state of div(g grad c) - k c + src = 0, c = 0 on sky."""
    h, w = openm.shape
    unk = (openm & ~sky) if eps == 0 else ~sky
    idx = -np.ones((h, w), np.int64)
    n = int(unk.sum())
    idx[unk] = np.arange(n)
    diag = np.full(n, float(k))
    R, C, V = [], [], []
    for dy, dx in N4:
        y, x, ny, nx = pairs(h, w, dy, dx)
        a = unk[y, x]
        y, x, ny, nx = y[a], x[a], ny[a], nx[a]
        g = np.where(openm[y, x] & openm[ny, nx], 1.0, eps)
        i = idx[y, x]
        np.add.at(diag, i, g)
        nb = unk[ny, nx] & (g > 0)
        R.append(i[nb]); C.append(idx[ny, nx][nb]); V.append(-g[nb])
    R = np.concatenate(R); C = np.concatenate(C); V = np.concatenate(V)
    A = sp.csr_matrix((V, (R, C)), shape=(n, n))
    rowsum = diag + np.asarray(A.sum(axis=1)).ravel()  # sink strength per row
    ncomp, lab = connected_components(A, directed=False)
    sealed_comp = np.ones(ncomp, bool)
    np.logical_and.at(sealed_comp, lab, ~(rowsum > 1e-12))
    sealed = sealed_comp[lab]
    keep = np.nonzero(~sealed)[0]
    A = (A + sp.diags(diag)).tocsr()[keep][:, keep]
    b = src[unk].astype(float)[keep]
    sol = np.zeros(n)
    sol[keep] = spla.spsolve(A.tocsc(), b)
    c = np.full((h, w), np.nan)
    c[sky] = 0.0
    c[unk] = sol
    sealed_mask = np.zeros((h, w), bool); sealed_mask[unk] = sealed
    c[sealed_mask] = np.nan
    if eps == 0:
        c[~openm] = np.nan
    return c, sealed_mask


def geodesic(openm, sky):
    h, w = openm.shape
    d = np.full((h, w), -1, np.int64)
    q = deque()
    for y, x in zip(*np.nonzero(sky)):
        d[y, x] = 0; q.append((y, x))
    while q:
        y, x = q.popleft()
        for dy, dx in N8:
            ny, nx = y + dy, x + dx
            if 0 <= ny < h and 0 <= nx < w and openm[ny, nx] and d[ny, nx] < 0:
                d[ny, nx] = d[y, x] + 1; q.append((ny, nx))
    return d


def neighbour_stack(c, valid):
    h, w = c.shape
    P = np.pad(np.where(valid, c, np.nan), 1, constant_values=np.nan)
    return np.stack([P[1 + dy:1 + dy + h, 1 + dx:1 + dx + w] for dy, dx in N8])


def flow(c, walk, sign):
    """Steepest walkable descent (sign=-1) or ascent (+1): next cell per cell."""
    h, w = c.shape
    vals = neighbour_stack(sign * c, walk & np.isfinite(c))
    vals = np.where(np.isnan(vals), -np.inf, vals)
    best = vals.argmax(axis=0)
    bestv = vals.max(axis=0)
    here = np.where(np.isfinite(c), sign * c, -np.inf)
    move = bestv > here + 1e-15
    dys = np.array([d[0] for d in N8]); dxs = np.array([d[1] for d in N8])
    ys, xs = np.mgrid[0:h, 0:w]
    ny = np.where(move, ys + dys[best], ys)
    nx = np.where(move, xs + dxs[best], xs)
    return ny, nx


def terminals(ny, nx, cells):
    out = {}
    for (y, x) in cells:
        path = []
        cy, cx = y, x
        steps = 0
        while (ny[cy, cx], nx[cy, cx]) != (cy, cx) and steps < 5000:
            if (cy, cx) in out:
                break
            path.append((cy, cx))
            cy, cx = ny[cy, cx], nx[cy, cx]
            steps += 1
        t = out.get((cy, cx), (cy, cx))
        for p in path:
            out[p] = t
        out[(y, x)] = t
    return out


def sensor_contrast(c, walk, cells):
    """Best |front-here|/(front+here) over the 8 headings, front 6 cells
    ahead (4,4 on a diagonal), front must be walkable with a value."""
    h, w = c.shape
    best = {}
    for (y, x) in cells:
        cp = c[y, x]
        m = -1.0
        for dy, dx in N8:
            s = 6 if (dy == 0 or dx == 0) else 4
            fy, fx = y + s * dy, x + s * dx
            if 0 <= fy < h and 0 <= fx < w and walk[fy, fx] and np.isfinite(c[fy, fx]):
                cf = c[fy, fx]
                den = cf + cp
                r = abs(cf - cp) / den if den > 0 else 0.0
                m = max(m, r)
        best[(y, x)] = m
    return best


def settle_frames(openm, sky, src, eps, target, nest, D=0.25 / 36, pass_frames=12,
                  max_frames=1_500_000, tol=0.10):
    """Explicit 4-neighbour flux diffusion at the current plane's rate (beta =
    D * pass_frames per pass), sources added per pass, sky held at 0. Frames
    until the nest's relative L1 error to steady state is under tol."""
    beta = D * pass_frames
    h, w = openm.shape
    g = {}
    for dy, dx in N4:
        nb = np.zeros((h, w), bool)
        ysl = slice(max(0, -dy), h - max(0, dy)); xsl = slice(max(0, -dx), w - max(0, dx))
        nysl = slice(max(0, dy), h - max(0, -dy) if max(0, -dy) else h); nxsl = slice(max(0, dx), w - max(0, -dx) if max(0, -dx) else w)
        cond = np.zeros((h, w))
        both = openm[ysl, xsl] & openm[nysl, nxsl]
        cond[ysl, xsl] = np.where(both, 1.0, eps)
        g[(dy, dx)] = (cond, ysl, xsl, nysl, nxsl)
    c = np.zeros((h, w))
    alive = openm if eps == 0 else np.ones((h, w), bool)
    alive = alive & ~sky
    # the solve is in source units at unit conductance; the stepped field's
    # steady state is that divided by D (sources are per frame)
    tgt = target[nest & np.isfinite(target)] / D
    norm = np.abs(tgt).sum()
    s = src * pass_frames
    frames = 0
    while frames < max_frames:
        flux = np.zeros((h, w))
        for (dy, dx), (cond, ysl, xsl, nysl, nxsl) in g.items():
            flux[ysl, xsl] += cond[ysl, xsl] * (c[nysl, nxsl] - c[ysl, xsl])
        c = np.where(alive, c + beta * flux + s, 0.0)
        frames += pass_frames
        if frames % 6000 == 0:
            err = np.abs(c[nest & np.isfinite(target)] - tgt).sum() / norm
            if err < tol:
                return frames
    return None


def avg_sources(seed, f, openm, kind):
    acc = np.zeros(openm.shape)
    n = 0
    for df in range(-5000, 5001, 1000):
        p = f'{BASE}/s{seed}/map_f{f + df:06d}.txt'
        if not os.path.exists(p):
            continue
        g = load(p)
        if g.shape != openm.shape:
            continue
        if kind == 'air':
            acc += (g == 'a') * 1.0 + (g == 'b') * 0.25
        else:
            acc += (g == 'b') * 1.0
        n += 1
    return np.where(openm, acc / max(n, 1), 0.0)


def describe_top(g, y, x):
    win = g[max(0, y - 3):y + 4, max(0, x - 3):x + 4]
    return {'brood': int((win == 'b').sum()), 'ants': int((win == 'a').sum()),
            'row_below_ground': int(y - GROUND_ROW)}


def analyse(seed, f, do_settle):
    g = load(f'{BASE}/s{seed}/map_f{f:06d}.txt')
    openm, sky, nest, mound = masks(g)
    geo = geodesic(openm, sky)
    nest_cells = list(zip(*np.nonzero(nest & (geo >= 0))))
    unreach = int((nest & (geo < 0)).sum())
    res = {'seed': seed, 'frame': f, 'nest_cells': len(nest_cells), 'nest_unreachable': unreach,
           'ants_in_nest': int(((g == 'a') & nest).sum()), 'ants_in_mound': int(((g == 'a') & mound).sum()),
           'brood': int((g == 'b').sum()), 'fields': {}}
    if not nest_cells:
        res['note'] = 'no dug-nest cell reachable from open sky (door shut)'
        return res, g, {}, geo, (openm, sky, nest, mound)
    air = avg_sources(seed, f, openm, 'air')
    larva = avg_sources(seed, f, openm, 'larva')
    specs = [('air_sealed', air, 0.0, 0.0), ('air_p02', air, 0.02, 0.0), ('air_plane', air, 1.0, 0.0),
             ('larva_L3', larva, 0.0, 1 / 9), ('larva_L10', larva, 0.0, 1 / 100)]
    fields = {}
    for name, src, eps, k in specs:
        c, sealed = solve(openm, sky, src, eps, k)
        fields[name] = c
        walk = openm
        cn = np.array([c[p] for p in nest_cells])
        fin = np.isfinite(cn)
        d_ny, d_nx = flow(c, walk, -1)
        a_ny, a_nx = flow(c, walk, +1)
        dterm = terminals(d_ny, d_nx, nest_cells)
        aterm = terminals(a_ny, a_nx, nest_cells)
        out = sum(1 for p in nest_cells if sky[dterm[p]])
        agree = 0; agree_n = 0
        for p in nest_cells:
            q = (d_ny[p], d_nx[p])
            if q != p:
                agree_n += 1
                agree += geo[q] < geo[p]
        # wallward: steepest descent over ALL 8 neighbours (only meaningful when walls hold values)
        wallward = None
        if eps > 0:
            allv = neighbour_stack(-c, np.isfinite(c))
            allv = np.where(np.isnan(allv), -np.inf, allv)
            bi = allv.argmax(axis=0)
            dys = np.array([d[0] for d in N8]); dxs = np.array([d[1] for d in N8])
            ww = 0
            for (y, x) in nest_cells:
                k8 = bi[y, x]
                ty, tx = y + dys[k8], x + dxs[k8]
                if 0 <= ty < g.shape[0] and 0 <= tx < g.shape[1] and not openm[ty, tx]:
                    ww += 1
            wallward = ww / len(nest_cells)
        sc = sensor_contrast(c, walk, nest_cells)
        scv = np.array([sc[p] for p in nest_cells])
        tops = {}
        for p in nest_cells:
            t = aterm[p]
            tops[t] = tops.get(t, 0) + 1
        top_list = sorted(tops.items(), key=lambda kv: -kv[1])
        nmax = np.nanmax(cn) if fin.any() else float('nan')
        pos = cn[fin & (cn > 0)]
        # 1-cell walkable differences, best neighbour
        diffs = []
        for p in nest_cells:
            y, x = p
            best = 0.0
            for dy, dx in N8:
                ny_, nx_ = y + dy, x + dx
                if 0 <= ny_ < g.shape[0] and 0 <= nx_ < g.shape[1] and openm[ny_, nx_] and np.isfinite(c[ny_, nx_]):
                    best = max(best, abs(c[ny_, nx_] - c[p]))
            diffs.append(best)
        diffs = np.array(diffs)
        from scipy import ndimage
        wide = ndimage.uniform_filter(openm.astype(float), size=7) > 0.7
        room = np.array([wide[p] for p in nest_cells])
        fr = {
            'flat_1pct_room': float((scv[room] < 0.01).mean()) if room.any() else None,
            'flat_1pct_tunnel': float((scv[~room] < 0.01).mean()) if (~room).any() else None,
            'room_share': float(room.mean()),
            'flat_1pct': float((scv < 0.01).mean()), 'flat_0p1pct': float((scv < 0.001).mean()),
            'no_readable_heading': float((scv < 0).mean()),
            'contrast_p50': float(np.median(scv[scv >= 0])) if (scv >= 0).any() else None,
            'out': out / len(nest_cells), 'agree': agree / max(agree_n, 1), 'wallward': wallward,
            'sealed_cells': int((sealed & nest).sum()),
            'tops': len(top_list),
            'top3': [{'row': int(t[0][0]), 'col': int(t[0][1]), 'share': t[1] / len(nest_cells),
                      **describe_top(g, *t[0]), 'geo': int(geo[t[0]])} for t in top_list[:3]],
            'range_max_over_min': float(nmax / pos.min()) if pos.size else None,
            'step_under_1_65536': float((diffs < nmax / 65536).mean()) if np.isfinite(nmax) else None,
            'step_under_1_256': float((diffs < nmax / 256).mean()) if np.isfinite(nmax) else None,
        }
        if do_settle and name in ('air_sealed', 'air_plane'):
            fr['settle_frames_10pct'] = settle_frames(openm, sky, (air if name.startswith('air') else larva),
                                                      0.0 if name == 'air_sealed' else 1.0, c, nest)
        res['fields'][name] = fr
    return res, g, fields, geo, (openm, sky, nest, mound)


def render(g, fields, geo, ms, path, crop=(28, 131, 15, 136), cell=4):
    from PIL import Image, ImageDraw
    openm, sky, nest, mound = ms
    r0, r1, c0, c1 = crop
    H, W = (r1 - r0) * cell, (c1 - c0) * cell
    names = ['map', 'air_sealed', 'air_plane', 'geodesic', 'larva_L10']
    img = Image.new('RGB', (W * len(names) + 8 * (len(names) - 1), H + 16), (255, 255, 255))
    dr = ImageDraw.Draw(img)
    col = {'s': (140, 105, 70), '#': (90, 70, 50), '.': (20, 20, 28), 'a': (230, 60, 50), 'b': (245, 245, 235),
           'f': (60, 170, 60), 'c': (200, 190, 80), 'x': (120, 40, 120), '~': (40, 90, 200)}

    def ramp(t):  # fixed dark->bright ramp (CLAUDE.md: never blend into the cell's colour)
        t = min(max(t, 0.0), 1.0)
        return (int(255 * min(1, 1.6 * t)), int(255 * max(0, min(1, 1.6 * t - 0.4))), int(255 * max(0, 2.5 * t - 1.5)))

    for k, name in enumerate(names):
        ox = k * (W + 8)
        dr.text((ox + 2, H + 2), name, fill=(0, 0, 0))
        if name == 'map':
            for y in range(r0, r1):
                for x in range(c0, c1):
                    dr.rectangle([ox + (x - c0) * cell, (y - r0) * cell, ox + (x - c0 + 1) * cell - 1, (y - r0 + 1) * cell - 1],
                                 fill=col.get(g[y, x], (255, 0, 255)))
            continue
        if name == 'geodesic':
            v = np.where(geo >= 0, geo.astype(float), np.nan)
            vmax = np.nanmax(v[r0:r1, c0:c1])
            t = v / vmax
        else:
            v = fields[name]
            sub = v[r0:r1, c0:c1]
            pos = sub[np.isfinite(sub) & (sub > 0)]
            lo, hi = (np.log10(pos.min()), np.log10(pos.max())) if pos.size else (0, 1)
            t = (np.log10(np.where(np.isfinite(v) & (v > 0), v, np.nan)) - lo) / max(hi - lo, 1e-9)
        for y in range(r0, r1):
            for x in range(c0, c1):
                if name in ('air_plane',) and np.isfinite(t[y, x]):
                    fill = ramp(t[y, x]) if openm[y, x] else tuple(int(0.45 * q) for q in ramp(t[y, x]))
                elif not openm[y, x]:
                    fill = (70, 55, 40)
                elif not np.isfinite(t[y, x]):
                    fill = (0, 0, 90) if (name != 'geodesic' and fields.get(name) is not None and fields[name][y, x] == 0) else (0, 0, 0)
                else:
                    fill = ramp(t[y, x])
                dr.rectangle([ox + (x - c0) * cell, (y - r0) * cell, ox + (x - c0 + 1) * cell - 1, (y - r0 + 1) * cell - 1], fill=fill)
        # ground line
        gy = (GROUND_ROW - r0) * cell
        dr.line([ox, gy, ox + W, gy], fill=(80, 160, 255))
    img.save(path)


if __name__ == '__main__':
    out = sys.argv[1]
    os.makedirs(out, exist_ok=True)
    seeds = [int(s) for s in sys.argv[2].split(',')]
    frames = [int(f) for f in sys.argv[3].split(',')]
    settle = len(sys.argv) > 4 and sys.argv[4] == 'settle'
    allres = []
    for s in seeds:
        for f in frames:
            res, g, fields, geo, ms = analyse(s, f, settle)
            allres.append(res)
            print(json.dumps(res), flush=True)
            if fields:
                render(g, fields, geo, ms, f'{out}/fields_s{s}_f{f:06d}.png')
    with open(f'{out}/results.json', 'w') as fh:
        json.dump(allres, fh, indent=1)
