"""SCRATCH (foraging-loop thread, 2026-10-22): the hungry lane's copier_h.py with the 16th field read as the carry-home
teacher's label (group "carry"); otherwise unchanged. Originally: SCRATCH (teacher-training step 1, not for main): the copier. Teaches a mind genome a teacher's calls by gradient
descent on the brain's own two-stage choice probabilities. Design: Reports/brain-training/teacher-training/
step1-copy-design-2026-10-21.md (v2, after the adversarial review).

usage:
  python3 -I copier.py replaycheck LOG GENOME REPLAY_OUT      forward pass against Rust decide (copyreplay output)
  python3 -I copier.py roundtrip GENOME OUT                   read and write a genome unchanged
  python3 -I copier.py perturb GENOME OUT SEED SIGMA          a perturbed genome for the replay check
  python3 -I copier.py memcheck LOG GENOME                    the one-step memory path reproduces the logged memory
  python3 -I copier.py train CONFIG.json                      copy (see train())

The forward pass mirrors `mind::decide` exactly (layout from src/sim/creature/mind.rs, GENOME_LEN 2797): the scorer's
16 units over each option's 45 features plus context, memory and verb; each verb's job from context, memory, its best
target's score, the hidden units and the pull of the last verb; both stages chosen by `choose_weighted`,
p_i = (k + max(w_i, 0))^2 / sum_j (k + max(w_j, 0))^2 with w = (score + 1) / 2 and k = PICK_FLOOR = 0.05.
"""
import json, sys, time
import numpy as onp
import autograd.numpy as np
from autograd import grad

VERBS, MEMORY, CONTEXT, UNITS, FEATURES, EMITS, HIDDEN = 9, 12, 22, 16, 45, 3, 8
SWITCH = 0; SHARP = SWITCH + VERBS; PULL = SHARP + 1; RATE = PULL + 1; S_CTX = RATE + MEMORY
S_IN = CONTEXT + MEMORY + 1; S_FEAT = S_CTX + UNITS * S_IN; S_VERB = S_FEAT + UNITS * FEATURES; S_OUT = S_VERB + UNITS * VERBS
JOB = S_OUT + UNITS + 1; J_IN = CONTEXT + MEMORY + 2; MEM = JOB + VERBS * J_IN; M_IN = CONTEXT + 2 + VERBS + MEMORY + 1
EMIT = MEM + MEMORY * M_IN; E_IN = CONTEXT + MEMORY + 1; HID = EMIT + EMITS * E_IN; H_IN = CONTEXT + MEMORY + 1
HID_OUT = HID + HIDDEN * H_IN; GENOME_LEN = HID_OUT + VERBS * HIDDEN
assert GENOME_LEN == 2797
K = 0.05
W_EPS = 1e-6
ZV = onp.zeros((1, 9, 1))  # explicit broadcast over verbs before a `where` (autograd's where vjp does not unbroadcast)
ACTS = onp.array([1, 1, 1, 1, 1, 1, 1, 1, 0], bool)  # Verb::acts: every verb but Attack
WALK, DIG = 0, 3
# each arena's verbs (`Challenge::verbs`): `apply_bans` switches every other verb off when the arena runs the genome,
# so a log's decisions must be recomputed with those verbs off (the kept-skill group read 0.054 KL at step 0 without)
ARENA_VERBS = {"soil": [0, 1, 2, 4, 6, 5, 7, 3], "fetch": [0, 1, 2, 4, 6, 5, 7], "trail": [0, 1, 2]}
ROOF_DROP_F = 44  # options::f::ROOF_DROP
DEPTH_F = 26
# dig-scoring thread (Teacher lane's pick, 23:46Z): Dig's own feature head (mind.rs scratch_log::DigHead). The
# trainable vector is the genome with the head appended: W (NH x (FEATURES + ND)), b (NH), a (NH), c; the head's output
# is added to every Dig option's raw score. The copy log carries the head's ND features as indices 100+k.
ND = 12
HEAD_H = [8]
TARGET_ONLY = [False]
NO_HANGING = [False]
WHEN = [False]
GATE = [False]  # PP_DIGHEAD_GATE: "when" only in empty-jaw dig states (no spoil, crop empty, below the datum, |x-128| <= 40)  # the head's "when" read-out into Dig's job (mind.rs DigHead::e)  # drop the old ant's cuts of a hanging bite (f::ROOF_DROP): in a mind's body they bury it


# ------------------------------------------------------------------ genome I/O (train::save_genome's sparse format)
def read_genome(path):
    lines = open(path).read().splitlines()
    assert lines[0].startswith(f"genome {GENOME_LEN}"), lines[0]
    g = onp.zeros(GENOME_LEN)
    for l in lines[1:]:
        if l.strip():
            i, v = l.split()
            g[int(i)] = float(v)
    return g, lines[0]


def write_genome(path, g, head):
    out = [head]
    for i, v in enumerate(g):
        v32 = float(onp.float32(v))
        if v32 != 0.0:
            out.append(f"{i} {v32!r}")
    open(path, "w").write("\n".join(out) + "\n")


def live_links(g):
    return int((onp.abs(onp.asarray(g, onp.float32)[S_CTX:]) >= W_EPS).sum())


# ------------------------------------------------------------------ the copy log
def parse_log(path, keep=None, arena="soil"):
    """Q rows (not settle) as dicts, in log order, each with `prev` = index of the same ant's previous decision in the
    same world (or -1). G/C/D/E rows per world tag in `worlds`. `keep(row)` filters which rows get their option
    arrays built (all rows still get linkage)."""
    dec, worlds, last = [], {}, {}
    for line in open(path):
        p = line.rstrip("\n").split(",")
        tag, kind = int(p[0]), p[1]
        if kind not in ("Q", "G", "C", "D", "E"):
            continue
        w = worlds.setdefault(tag, {"C": [], "D": []})
        if kind == "Q":
            if p[6] == "1":
                continue
            ctx_only = p[6] == "2"  # hungry lane: the sampled log's memory-context row (never a training row)
            dig, digset, down, home = p[10].split("|")
            ix = lambda s: [] if s == "-" else [int(i) for i in s.split(";")]
            opts = []
            for o in p[13].split(";") if p[13] else []:
                q = o.split(":", 4)
                f = {}
                for kv in q[4].split(" ") if q[4] else []:
                    i, v = kv.split("=")
                    f[int(i)] = float(v)
                opts.append((int(q[0]), int(q[1]), int(q[2]), float(q[3]), f))
            r = dict(arena=arena, tag=tag, frame=int(p[2]), id=int(p[3]), x=int(p[4]), y=int(p[5]), pick=int(p[7]), last=int(p[8]),
                     carrying=p[9] == "1", dig=dig, digset=ix(digset), down=ix(down), home=ix(home),
                     ctx=onp.array(p[11].split(" "), float), mem=onp.array(p[12].split(" "), float), opts=opts,
                     ctx_only=ctx_only, hungry=None)
            if len(p) > 15 and p[15] != "-":
                hp, hi = p[15].split("|")
                lab = [int(i) for i in hi.split(";")]
                # `spoil_up_only`: with a pellet in the jaws, keep a Walk label only if it climbs. The carry teacher walked
                # trip loads toward a drifting forage_anchor, and 63% of its pellet-holder labels stepped down into the
                # pit; c1 learned "with a pellet, go down" for every pellet holder and jammed (fed spoil-deep 40-55%
                # of decisions against h1's 12-13%).
                if lab and SPOIL_UP[0] and p[9] == "1" and opts and opts[lab[0]][0] == 0:
                    if not all(opts[i][2] < int(p[5]) for i in lab):
                        lab = []
                # dig-scoring thread (PP_OLDLAB, filt.py): the old ant's label, kind in the first field -- 1 its cut,
                # 2 it would not cut where the brain cut (must-not-dig), 3 its move with spoil in the jaws
                k = int(float(hp))
                if NO_HANGING[0] and k == 1 and lab:
                    lab = [i for i in lab if ROOF_DROP_F not in opts[i][4]]
                    if not lab:
                        k = 0
                r["kind"] = k
                if lab and k == 2:
                    r["dig"] = "N"
                elif lab:
                    r["hungry"] = (OLD_HP.get(k, 1.0), lab)
            # The crater refusal's set (17th field, teacher lane's PP_RIMVETO): option indices the reference must not
            # pick. Refusal wins over a teacher label that names one of them (the old ant may cut the rim).
            if len(p) > 16 and p[16] != "-":
                rim = [int(i) for i in p[16].split(";")]
                r["rim"] = rim
                if r["hungry"]:
                    hp, lab = r["hungry"]
                    lab = [i for i in lab if i not in rim]
                    r["hungry"] = (hp, lab) if lab else None
            key = (tag, r["id"])
            r["prev"] = last.get(key, -1)
            last[key] = len(dec)
            dec.append(r)
        elif kind == "G":
            w["G"] = p[2:]
        elif kind in "CD":
            w[kind].append(p[2:])
        elif kind == "E":
            w["E"] = p[2:]
    return dec, worlds


def batch_of(rows):
    """Padded arrays for a list of decision dicts."""
    n = len(rows)
    P = max(len(r["opts"]) for r in rows)
    verb = onp.zeros((n, P), int)
    feat = onp.zeros((n, P, FEATURES))
    dfeat = onp.zeros((n, P, ND))
    mask = onp.zeros((n, P), bool)
    score = onp.zeros((n, P))
    for a, r in enumerate(rows):
        for b, (v, tx, ty, s, f) in enumerate(r["opts"]):
            verb[a, b] = v
            mask[a, b] = True
            score[a, b] = s
            for i, x in f.items():
                if i >= 100:
                    dfeat[a, b, i - 100] = x
                else:
                    feat[a, b, i] = x
    ctx = onp.array([r["ctx"] for r in rows])
    mem = onp.array([r["mem"] for r in rows])
    last = onp.array([r["last"] for r in rows])
    pick = onp.array([r["pick"] for r in rows])
    allow = onp.zeros((n, VERBS), bool)
    for a, r in enumerate(rows):
        allow[a, ARENA_VERBS[r.get("arena", "soil")]] = True
    gate = onp.array([(not r["carrying"]) and r["ctx"][1] == 0.0 and r["ctx"][18] > 0.0 and abs(r["x"] - 128) <= 40 for r in rows], float)
    return dict(n=n, P=P, verb=verb, feat=feat, dfeat=dfeat, gate=gate, mask=mask, score=score, ctx=ctx, mem=mem, last=last, pick=pick, allow=allow)


# ------------------------------------------------------------------ the forward pass (mind::decide)
def sq(x):
    return x / (1.0 + np.abs(x))


def blocks(g):
    sharp = np.clip(1.0 + g[SHARP], 0.25, 4.0)
    pull = np.clip(g[PULL], 0.0, 1.0)
    return dict(
        on=(g[SWITCH:SWITCH + VERBS] > 0) & ACTS,
        sharp=sharp, pull=pull,
        sctx=np.reshape(g[S_CTX:S_FEAT], (UNITS, S_IN)),
        sfeat=np.reshape(g[S_FEAT:S_VERB], (UNITS, FEATURES)),
        sverb=np.reshape(g[S_VERB:S_OUT], (UNITS, VERBS)),
        sout=g[S_OUT:S_OUT + UNITS + 1],
        job=np.reshape(g[JOB:MEM], (VERBS, J_IN)),
        mem=np.reshape(g[MEM:EMIT], (MEMORY, M_IN)),
        rate=np.clip(0.5 + g[RATE:RATE + MEMORY], 0.0, 0.995),
        hid=np.reshape(g[HID:HID_OUT], (HIDDEN, H_IN)),
        hout=np.reshape(g[HID_OUT:GENOME_LEN], (VERBS, HIDDEN)),
        head=g[GENOME_LEN:] if len(g) > GENOME_LEN else None,
    )


def head_parts(th):
    H, n = HEAD_H[0], FEATURES + ND
    return (np.reshape(th[:H * n], (H, n)), th[H * n:H * n + H], th[H * n + H:H * n + 2 * H], th[H * n + 2 * H])


def head_len():
    H, n = HEAD_H[0], FEATURES + ND
    return H * n + 2 * H + 1 + (H + 1 if WHEN[0] else 0)


def scores(B, b, mem):
    """Every option's score s (n, P), and which options count (valid: real and of a switched-on verb)."""
    pre = np.dot(b["sctx"][:, :CONTEXT], B["ctx"].T).T + np.dot(b["sctx"][:, CONTEXT:CONTEXT + MEMORY], mem.T).T + b["sctx"][:, -1]
    # each option's verb column of the scorer, as a one-hot product (autograd cannot differentiate the fancy index)
    vone = onp.eye(VERBS)[B["verb"]]  # (n, P, V)
    z = pre[:, None, :] + np.dot(vone, b["sverb"].T) + np.einsum("npf,uf->npu", B["feat"], b["sfeat"])
    raw = b["sout"][UNITS] + np.dot(sq(z), b["sout"][:UNITS])
    s = sq(b["sharp"] * raw)
    valid = B["mask"] & onp.asarray(b["on"])[B["verb"]] & onp.take_along_axis(B["allow"], B["verb"], axis=1)
    # Dig's own head picks the Dig cell (mind.rs decide_planned's target stage), nothing else reads it
    s_t, when = s, None
    if b["head"] is not None:
        W, hb, ha, hc = head_parts(b["head"])
        x = np.concatenate([B["feat"], B["dfeat"]], axis=2)
        hz = np.einsum("npf,hf->nph", x, W) + hb
        s_t = np.where(B["verb"] == DIG, sq(hc + np.dot(sq(hz), ha)), s)
        if WHEN[0]:
            H, n = HEAD_H[0], FEATURES + ND
            e, c2 = b["head"][H * n + 2 * H + 1:H * n + 3 * H + 1], b["head"][H * n + 3 * H + 1]
            wo = np.dot(sq(hz), e)  # (n, P)
            isdig = (B["verb"] == DIG) & valid
            when = np.where(isdig.any(1), c2 + np.max(np.where(isdig, wo, -1e9), axis=1), 0.0)
            if GATE[0]:
                when = when * B["gate"]
    return s, valid, s_t, when


def forward(g, B, mem=None):
    """Scores, jobs, verb log-probabilities and per-option target log-probabilities (within their own verb)."""
    b = blocks(g)
    mem = B["mem"] if mem is None else mem
    s, valid, s_t, when = scores(B, b, mem)
    NEG = -1e9
    vmask = onp.stack([(B["verb"] == v) & valid for v in range(VERBS)], 1)  # (n, V, P)
    best = np.max(np.where(vmask, s[:, None, :] + ZV, NEG), axis=2)  # (n, V); +ZV: autograd's where does not unbroadcast
    present = vmask.any(2)
    hidden = sq(np.dot(B["ctx"], b["hid"][:, :CONTEXT].T) + np.dot(mem, b["hid"][:, CONTEXT:CONTEXT + MEMORY].T) + b["hid"][:, -1])
    jsum = (np.dot(B["ctx"], b["job"][:, :CONTEXT].T) + np.dot(mem, b["job"][:, CONTEXT:CONTEXT + MEMORY].T)
            + b["job"][:, CONTEXT + MEMORY] * np.where(present, best, 0.0) + b["job"][:, CONTEXT + MEMORY + 1]
            + np.dot(hidden, b["hout"].T) + b["pull"] * (B["last"][:, None] == onp.arange(VERBS)[None, :]))
    if when is not None:
        jsum = jsum + when[:, None] * (onp.arange(VERBS) == DIG)[None, :]
    job = sq(b["sharp"] * jsum)
    # verb stage: choose_weighted over present verbs
    wv = np.maximum((job + 1.0) / 2.0, 0.0)
    bv = np.where(present, (K + wv) ** 2, 0.0)
    pv = bv / np.sum(bv, axis=1, keepdims=True)
    logpv = np.log(np.where(present, pv, 1.0))
    # target stage: within each option's verb
    wt = np.maximum((s + 1.0) / 2.0, 0.0)
    if s_t is not s:  # the head re-ranks what the scorer accepts: its weight times the scorer's (mind.rs, Dig cells)
        wt = np.where(B["verb"] == DIG, wt * np.maximum((s_t + 1.0) / 2.0, 0.0), wt)
    bt = np.where(valid, (K + wt) ** 2, 0.0)
    tot = np.sum(np.where(vmask, bt[:, None, :] + ZV, 0.0), axis=2)  # (n, V)
    tot_o = np.sum(np.where(vmask, tot[:, :, None] + onp.zeros((1, 1, B["P"])), 0.0), axis=1)  # own verb's total (n, P)
    logpt = np.log(np.where(valid, bt / np.where(valid, tot_o, 1.0), 1.0))
    n_valid = onp.maximum(valid.sum(1), 1)
    all_best = np.max(np.where(valid, s, NEG), axis=1)
    all_mean = np.sum(np.where(valid, s, 0.0), axis=1) / n_valid
    return dict(s=s, valid=valid, job=job, present=present, logpv=logpv, logpt=logpt, vmask=vmask,
                all_best=all_best, all_mean=all_mean)


def remember(g, B, all_best, all_mean, verb_done, mem):
    """`decide`'s memory update for the verb carried out: the next decision's memory."""
    b = blocks(g)
    M = b["mem"]
    onehot = (verb_done[:, None] == onp.arange(VERBS)[None, :]).astype(float)
    sm = (np.dot(B["ctx"], M[:, :CONTEXT].T) + all_best[:, None] * M[:, CONTEXT] + all_mean[:, None] * M[:, CONTEXT + 1]
          + np.dot(onehot, M[:, CONTEXT + 2:CONTEXT + 2 + VERBS].T) + np.dot(mem, M[:, CONTEXT + 2 + VERBS:CONTEXT + 2 + VERBS + MEMORY].T)
          + M[:, M_IN - 1])
    r = b["rate"]
    return r * mem + (1.0 - r) * sq(sm)


# ------------------------------------------------------------------ checks
def replaycheck(log, genome, replay_out):
    g, _ = read_genome(genome)
    dec, _ = parse_log(log)
    rep = {}
    for line in open(replay_out):
        head, s, j = line.rstrip("\n").split("|")
        tag, frame, oid = head.split(",")
        rep[(int(tag), int(frame), int(oid))] = (onp.array(s.split(" "), float), onp.array([float(x) for x in j.split(" ")]))
    rows = [r for r in dec if (r["tag"], r["frame"], r["id"]) in rep]
    print(f"replaycheck: {len(rows)} decisions matched of {len(rep)} replayed")
    es, ej = [], []
    for i in range(0, len(rows), 4000):
        R = rows[i:i + 4000]
        B = batch_of(R)
        f = forward(g, B)
        s = onp.asarray(f["s"])
        job = onp.asarray(f["job"])
        for a, r in enumerate(R):
            rs, rj = rep[(r["tag"], r["frame"], r["id"])]
            k = len(r["opts"])
            v = B["verb"][a, :k]
            on = onp.asarray(blocks(g)["on"])[v]
            es.append(onp.abs(s[a, :k][on] - rs[:k][on]))
            pres = ~onp.isnan(rj)
            ej.append(onp.abs(job[a][pres] - rj[pres]))
            assert (onp.asarray(f["present"])[a] == pres).all(), ("present verbs differ", r["frame"])
    es, ej = onp.concatenate(es), onp.concatenate(ej)
    print(f"  option scores: n {len(es)}, max |error| {es.max():.2e}, p99 {onp.percentile(es, 99):.2e}")
    print(f"  job scores:    n {len(ej)}, max |error| {ej.max():.2e}, p99 {onp.percentile(ej, 99):.2e}")
    print(f"  {'PASS' if max(es.max(), ej.max()) < 1e-4 else 'FAIL'} (bar: every score within 1e-4 of Rust's f32 decide)")


def perturb(genome, out, seed, sigma, trainable_only="0"):
    """Jitter live weights. With trainable_only=1, only what the copier may move (scorer, jobs, hidden units): a
    target the copier can actually reach. Jittering memory too made both early controls unreachable by design --
    the copier freezes memory's weights, and a policy read through another genome's memory code cannot be copied."""
    g, head = read_genome(genome)
    r = onp.random.default_rng(int(seed))
    live = onp.abs(g) >= W_EPS
    live[:S_CTX] = False
    if trainable_only == "1":
        live &= trainable_mask(g, [])
    g2 = g.copy()
    g2[live] += r.normal(0, float(sigma), live.sum())
    write_genome(out, g2, head)
    print(f"perturbed {live.sum()} live weights by N(0, {sigma}) -> {out}")


def memcheck(log, genome):
    """Positive control for the one-step memory path: from each decision's logged memory, the update with the verb
    the brain carried out must give the next decision's logged memory (label-only logs: the brain acted itself)."""
    g, _ = read_genome(genome)
    dec, _ = parse_log(log)
    pairs = [(dec[r["prev"]], r) for r in dec if r["prev"] >= 0]
    err = []
    for i in range(0, len(pairs), 4000):
        P_ = pairs[i:i + 4000]
        prev = [a for a, _ in P_]
        B = batch_of(prev)
        f = forward(g, B)
        done = onp.array([a["opts"][a["pick"]][0] for a in prev])
        m = onp.asarray(remember(g, B, f["all_best"], f["all_mean"], done, B["mem"]))
        nxt = onp.array([b["mem"] for _, b in P_])
        err.append(onp.abs(m - nxt).max(1))
    err = onp.concatenate(err)
    print(f"memcheck: {len(err)} consecutive pairs; max |error| {err.max():.2e}, p99 {onp.percentile(err, 99):.2e}, "
          f"pairs over 1e-4: {(err > 1e-4).sum()}  {'PASS' if err.max() < 1e-4 else 'FAIL'}")


# ------------------------------------------------------------------ training
PICKUP, EAT, FEED = 4, 6, 7
FROZEN_VERBS = (PICKUP, EAT, FEED)  # no target in the soil arena: their links serve fetch only (protect=, applied here)


OLD_HP = {1: 1.0, 3: 0.7, 4: 1.0}


def group_of(r):
    """Weighting group by the teacher's labels (a decision can carry several; the copy target uses them all)."""
    if r.get("kind") in (1, 2, 3, 4):
        return {1: "dig", 2: "nodig", 3: "spoil", 4: "under"}[r["kind"]]
    if r.get("hungry"):
        return "carry"
    if r.get("rim"):
        return "rim"
    if r["dig"] == "M":
        return "digM"
    if r["down"]:
        return "down"
    if r["home"]:
        return "home"
    if r["dig"] == "N":
        return "digN"
    return "none"


UNFREEZE = []
SOFT = [False]
SPOIL_UP = [False]


def trainable_mask(g, allow):
    """Live slots of the scorer, the job heads and the hidden units, minus the frozen verbs' links, plus `allow`."""
    m = onp.zeros(GENOME_LEN, bool)
    m[S_CTX:MEM] = True  # scorer (context, features, verb, output) and the job heads
    m[HID:GENOME_LEN] = True  # hidden units and their outputs
    m &= onp.abs(g) >= W_EPS
    for v in FROZEN_VERBS:
        if v in UNFREEZE:
            continue
        m[S_VERB + onp.arange(UNITS) * VERBS + v] = False
        m[JOB + v * J_IN:JOB + (v + 1) * J_IN] = False
        m[HID_OUT + v * HIDDEN:HID_OUT + (v + 1) * HIDDEN] = False
    for i in allow:
        m[i] = True
    return m


def quiet_unread(g):
    """Zero the inputs of scorer units nothing reads (output weight 0) and hidden units no job reads: they cost live
    links and do nothing, and their weights drifted unselected (review finding 2)."""
    g = g.copy()
    n = 0
    for u in range(UNITS):
        if g[S_OUT + u] == 0.0:
            for blk, width in ((S_CTX, S_IN), (S_FEAT, FEATURES)):
                sl = slice(blk + u * width, blk + (u + 1) * width)
                n += int((g[sl] != 0).sum()); g[sl] = 0.0
            sl = S_VERB + u * VERBS + onp.arange(VERBS)
            n += int((g[sl] != 0).sum()); g[sl] = 0.0
    for h in range(HIDDEN):
        if all(g[HID_OUT + v * HIDDEN + h] == 0.0 for v in range(VERBS)):
            sl = slice(HID + h * H_IN, HID + (h + 1) * H_IN)
            n += int((g[sl] != 0).sum()); g[sl] = 0.0
    return g, n


def targets(g_ref, B, rows, mode):
    """pi*: the ceiling policy written from the reference genome's own probabilities (review finding 1).
    mode 'teacher': must-dig -> Dig, target uniform over the dig set; must-not-dig -> the reference renormalised
    without Dig; down / home -> Walk's target uniform over the set; everything else the reference's own.
    mode 'ref': the reference policy unchanged (anchor, return-to-start control, the s162 positive control)."""
    f = forward(g_ref, B)
    pv = onp.exp(onp.asarray(f["logpv"])) * f["present"]
    pt = onp.exp(onp.asarray(f["logpt"])) * f["valid"]
    if mode == "ref":
        return pv, pt
    for a, r in enumerate(rows):
        verb = B["verb"][a]
        if r["dig"] == "M" and f["present"][a, DIG]:
            pv[a] = 0.0; pv[a, DIG] = 1.0
            sel = onp.zeros(B["P"], bool); sel[r["digset"]] = True
            d = (verb == DIG) & B["mask"][a]
            pt[a, d] = 0.0; pt[a, sel] = 1.0 / sel.sum()
        elif r["dig"] == "N" and f["present"][a, DIG] and pv[a].sum() - pv[a, DIG] > 0:
            pv[a, DIG] = 0.0; pv[a] /= pv[a].sum()
        if r.get("hungry"):
            hp, lab = r["hungry"]
            v = int(verb[lab[0]])
            if f["present"][a, v]:
                one = onp.zeros(VERBS); one[v] = 1.0
                pv[a] = (1 - hp) * pv[a] + hp * one
                sel = onp.zeros(B["P"], bool); sel[lab] = True
                w = (verb == v) & B["mask"][a]
                pt[a, w] = (1 - hp) * pt[a, w]
                pt[a, sel] += hp / sel.sum()
        if r.get("rim") and f["present"][a, DIG]:
            # pt over the refused set to 0, renormalised within Dig; no Dig target left -> no Dig, pv renormalised.
            d = (verb == DIG) & B["mask"][a]
            pt[a, r["rim"]] = 0.0
            s = pt[a, d].sum()
            if s > 0:
                pt[a, d] /= s
            elif pv[a].sum() - pv[a, DIG] > 0:
                pv[a, DIG] = 0.0; pv[a] /= pv[a].sum()
        for lab in ("down", "home"):
            if r[lab] and f["present"][a, WALK]:
                sel = onp.zeros(B["P"], bool); sel[r[lab]] = True
                w = (verb == WALK) & B["mask"][a]
                pt[a, w] = 0.0; pt[a, sel] = 1.0 / sel.sum()
    return pv, pt


def kl_cell(pv, pt, f, vmask):
    """The target part of `kl` alone: which cell, given the verb. The dig head cannot change when the ant digs (hcs's
    Dig job reads its best target at -0.29), so the cut lesson is scored on the cell only."""
    lt_o = np.where(pt > 0, pt * (np.log(np.where(pt > 0, pt, 1.0)) - f["logpt"]), 0.0)
    lt_v = np.sum(np.where(vmask, lt_o[:, None, :] + ZV, 0.0), axis=2)
    return np.sum(pv * lt_v, axis=1)


def kl(pv, pt, f, vmask):
    """KL(pi* || p) over the joint (verb, target), per decision: verb KL plus, for each verb, pi*(verb) times the KL of
    its targets."""
    lv = np.sum(np.where(pv > 0, pv * (np.log(np.where(pv > 0, pv, 1.0)) - f["logpv"]), 0.0), axis=1)
    lt_o = np.where(pt > 0, pt * (np.log(np.where(pt > 0, pt, 1.0)) - f["logpt"]), 0.0)  # (n, P)
    lt_v = np.sum(np.where(vmask, lt_o[:, None, :] + ZV, 0.0), axis=2)  # (n, V)
    return lv + np.sum(pv * lt_v, axis=1)


def sil_labels(dec, worlds, t_down=200, t_home=150):
    """Self-imitation labels: the brain's own decisions, kept by what followed them, per decision in hindsight
    (design v2 section 7). Replaces the teacher's labels on every row:
      dig    the brain dug: its pellet became a full load -> must-dig at that cell; it paid nothing -> must-not-dig
      down   empty, below ground, walking: the ant's next cut is a foot cut within t_down frames -> that step
      home   empty, on the surface, 3+ columns from the mouth, walking: within 3 columns of the mouth within t_home
             frames -> that step
    The world facts (depth, the mouth, the frame-0 ground) only say which cut was a foot cut and where the mouth is --
    the outcome, not the call."""
    by_world = {}
    for i, r in enumerate(dec):
        by_world.setdefault(r["tag"], []).append(i)
        r["dig"], r["digset"], r["down"], r["home"] = "-", [], [], []
    for tag, idx in by_world.items():
        w = worlds[tag]
        seed, x0, y0, nw, nh, shaft = (int(v) for v in w["G"][:6])
        datum = [None if v == "x" else int(v) for v in w["G"][6].split()]
        founding = [tuple(int(v) for v in c.split(":")) for c in w["G"][7].split()] if w["G"][7] else []
        top = min(y for _, y in founding)
        mouth = onp.mean([x for x, y in founding if y == top])

        def depth(x, y):
            lows = [datum[c - x0] for c in range(x - 3, x + 4) if 0 <= c - x0 < len(datum) and datum[c - x0] is not None]
            return y - max(lows) if lows else 0
        cuts = sorted((int(c[1]), int(c[2]), int(c[3]), int(c[4]), int(c[5]), c[6] == "1") for c in w["C"])  # f, id, x, y, deep, g0
        puts = sorted((int(d[1]), int(d[2]), int(d[3]), int(d[4]), d[5] == "1") for d in w["D"])
        ev = sorted([(c[0], 0, c) for c in cuts] + [(p[0], 1, p) for p in puts], key=lambda e: (e[0], e[1]))
        seen, held, fate = set(), {}, {}
        for f, kind, e in ev:
            if kind == 0:
                _, oid, x, y, deep, g0 = e
                first = g0 and (x, y) not in seen
                seen.add((x, y))
                cls = "foot" if first and deep >= shaft else "other"
                held[oid] = dict(f=f, x=x, y=y, deep=deep, cls=cls, full=False)
                fate[(f, x, y)] = held[oid]
            else:
                _, oid, px, py, away = e
                h = held.pop(oid, None)
                if h is not None:
                    h["full"] = away and min(h["y"] - py, h["deep"]) >= shaft and h["cls"] == "foot"
        cut_seq = sorted((f, v["cls"]) for (f, x, y), v in fate.items())
        rows = [dec[i] for i in idx]
        for k, r in enumerate(rows):
            pv = r["opts"][r["pick"]][0] if r["opts"] else -1
            if r["carrying"] or pv < 0:
                continue
            if pv == DIG:
                tx, ty = r["opts"][r["pick"]][1:3]
                hit = next((v for (f, x, y), v in fate.items() if (x, y) == (tx, ty) and r["frame"] <= f <= r["frame"] + 2), None)
                if hit is not None:
                    if hit["full"]:
                        r["dig"], r["digset"] = "M", [r["pick"]]
                    else:
                        r["dig"] = "N"
            elif pv == WALK:
                here = depth(r["x"], r["y"])
                if here > 0:
                    nxt = next(((f, c) for f, c in cut_seq if f > r["frame"]), None)
                    if nxt is not None and nxt[1] == "foot" and nxt[0] - r["frame"] <= t_down:
                        r["down"] = [r["pick"]]
                elif abs(r["x"] - mouth) >= 3:
                    later = [q for q in rows[k + 1:k + 80] if q["id"] == r["id"] and q["frame"] <= r["frame"] + t_home]
                    if any(abs(q["x"] - mouth) < 3 for q in later):
                        r["home"] = [r["pick"]]


def shuffle_labels(groups, rng):
    """The negative control's labels: each step label (down, home) and each dig set moved to random options of the
    same verb within the same decision; must-dig / must-not-dig permuted among the decisions of the same world that
    could carry one (review findings 7, 8: the cross-world move is invalid for step choices). Group membership stays
    as it was, so the weighting is unchanged."""
    allrows = [r for rs in groups.values() for r in rs]
    for r in allrows:
        for lab, v in (("down", WALK), ("home", WALK), ("digset", DIG)):
            if r[lab]:
                cand = [i for i, o in enumerate(r["opts"]) if o[0] == v]
                r[lab] = list(rng.choice(cand, len(r[lab]), replace=False))
    byworld = {}
    for r in allrows:
        if r["dig"] in ("M", "N"):
            byworld.setdefault(r["world"], []).append(r)
    for rs in byworld.values():
        labs = [r["dig"] for r in rs]
        rng.shuffle(labs)
        for r, l in zip(rs, labs):
            r["dig"] = l
            if l == "M" and not r["digset"]:
                cand = [i for i, o in enumerate(r["opts"]) if o[0] == DIG]
                r["digset"] = [int(rng.choice(cand))]
            if l == "N":
                r["digset"] = []


def take(B, idx):
    return {k: (v[idx] if isinstance(v, onp.ndarray) and v.ndim >= 1 and len(v) == B["n"] else v) for k, v in B.items()} | {"n": len(idx)}


def train(cfg_path):
    cfg = json.load(open(cfg_path))
    OLD_HP.update({int(k): v for k, v in cfg.get("old_hp", {}).items()})
    NO_HANGING[0] = bool(cfg.get("no_hanging", False))
    rng = onp.random.default_rng(cfg.get("seed", 1))
    g0, head = read_genome(cfg["start"])
    g_ref = read_genome(cfg["ref"])[0] if cfg.get("ref") else g0.copy()
    if cfg.get("quiet_unread", True):
        g0, nq = quiet_unread(g0)
        # frozen slots stay as the start had them even on unread units (the brain lane asked for f::ROOF_DROP's links
        # untouched; zeroing an unread unit's copy of one changed nothing the ant does, but it changed the genome)
        if "roof_drop" in cfg.get("freeze", []):
            for u in range(UNITS):
                i = S_FEAT + u * FEATURES + ROOF_DROP_F
                g0[i] = g_ref[i]
        print(f"quiet_unread: {nq} links of unread units zeroed; live links {live_links(g_ref)} -> {live_links(g0)}")
    target_g = read_genome(cfg["target_genome"])[0] if cfg.get("target_genome") else None
    allow = list(cfg.get("allow", []))
    UNFREEZE[:] = cfg.get("unfreeze", [])
    SOFT[0] = bool(cfg.get("soft", False))
    SPOIL_UP[0] = bool(cfg.get("spoil_up_only", False))
    # hungry lane: the energy input into every job row, every hidden unit and every live scorer unit
    for k in cfg.get("ctx_inputs", []):
        allow += [JOB + v * J_IN + k for v in range(VERBS)] + [HID + h * H_IN + k for h in range(HIDDEN)]
        allow += [S_CTX + u * S_IN + k for u in range(UNITS) if g0[S_OUT + u] != 0.0]
    kh = int(cfg.get("revive_hidden", 0))
    if kh:
        deadh = [h for h in range(HIDDEN) if all(g0[HID_OUT + v * HIDDEN + h] == 0.0 for v in range(VERBS))][:kh]
        for h in deadh:
            ins = list(range(HID + h * H_IN, HID + (h + 1) * H_IN))
            allow += ins + [HID_OUT + v * HIDDEN + h for v in range(VERBS) if v not in FROZEN_VERBS or v in UNFREEZE]
            g0[ins] = rng.normal(0.0, cfg.get("revive_sigma", 0.3), len(ins))
        print(f"revive_hidden: {deadh}")
    if cfg.get("depth_into_live_units", True):
        allow += [S_FEAT + u * FEATURES + DEPTH_F for u in range(UNITS) if g0[S_OUT + u] != 0.0]
    if cfg.get("arena_bit"):
        # step 0.5's joint-fit arm (c): one bit that says "not the soil arena", carried on c::CROWD (index 17), which
        # reads 0 on every decision of every solo arena -- a context input the brain does not have, borrowed from one
        # it never uses here -- and trainable into every scorer unit and every job head
        allow += [S_CTX + u * S_IN + 17 for u in range(UNITS)] + [JOB + v * J_IN + 17 for v in range(VERBS)]
    # step 0.5's narrow capacity arm: revive whole dead scorer units for the lesson -- every input link, its verb
    # row and its output -- rather than every dead slot in the brain. Picks the first `revive_units` units nothing reads.
    k = int(cfg.get("revive_units", 0))
    if k:
        dead = [u for u in range(UNITS) if g0[S_OUT + u] == 0.0][:k]
        for u in dead:
            allow += list(range(S_CTX + u * S_IN, S_CTX + (u + 1) * S_IN))
            allow += list(range(S_FEAT + u * FEATURES, S_FEAT + (u + 1) * FEATURES))
            allow += [S_VERB + u * VERBS + v for v in range(VERBS) if v not in FROZEN_VERBS] + [S_OUT + u]
            # seed the inputs (never the output, so the start's policy is unchanged): a unit with every weight 0 has
            # an exactly zero gradient on both sides -- the first build of this arm reproduced arm (a) to every digit
            ins = list(range(S_CTX + u * S_IN, S_CTX + (u + 1) * S_IN)) + list(range(S_FEAT + u * FEATURES, S_FEAT + (u + 1) * FEATURES))
            if "roof_drop" in cfg.get("freeze", []):
                # never seed a frozen slot: the freeze holds whatever is there, so a seeded f::ROOF_DROP link would be
                # a new, permanent link the brain lane asked not to have (found on the first L20u build)
                ins = [i for i in ins if i != S_FEAT + u * FEATURES + ROOF_DROP_F]
            g0[ins] = rng.normal(0.0, cfg.get("revive_sigma", 0.3), len(ins))
        print(f"revive_units: {dead} (inputs seeded, sigma {cfg.get('revive_sigma', 0.3)})")
    # step 0.5's joint-fit arm (b): every slot of the scorer, the job heads and the hidden units, live or dead (the 8
    # unread scorer units included), with the energy bill ignored -- "is it capacity?"
    mask = trainable_mask(onp.ones_like(g0) if cfg.get("all_slots") else g0, allow)
    # `freeze`: slots the copier must not move, whatever else says so. "roof_drop" is every scorer unit's link from
    # f::ROOF_DROP -- the brain lane, 2026-10-21: load-bearing for s171 (off: -1.4 to -1.7 full loads) and under
    # review by a soil-physics session, so kept out of anything the copier changes
    frozen = []
    for item in cfg.get("freeze", []):
        if item == "roof_drop":
            frozen += [S_FEAT + u * FEATURES + ROOF_DROP_F for u in range(UNITS)]
        else:
            frozen.append(int(item))
    # dig-scoring thread (Teacher lane's "far-home trick"): `only_verbs`, train only the slots that belong to these
    # verbs -- each unit's link to the verb, the verb's job head, the hidden units' link to it -- live or not, so the
    # scorer's shared context/feature weights (foraging, the home sense) cannot move.
    if cfg.get("only_verbs"):
        only = onp.zeros(GENOME_LEN, bool)
        for v in cfg["only_verbs"]:
            for u in range(UNITS):
                only[S_VERB + u * VERBS + v] = True
            only[JOB + v * J_IN:JOB + (v + 1) * J_IN] = True
            only[HID_OUT + v * HIDDEN:HID_OUT + (v + 1) * HIDDEN] = True
        mask = only
        print(f"only_verbs {cfg['only_verbs']}: {only.sum()} slots")
    mask[frozen] = False
    if cfg.get("dighead"):
        HEAD_H[0] = int(cfg["dighead"])
        TARGET_ONLY[0] = bool(cfg.get("target_only", True))
        WHEN[0] = bool(cfg.get("when", False))
        GATE[0] = bool(cfg.get("gate", False))
        H, n = HEAD_H[0], FEATURES + ND
        th = onp.zeros(head_len())
        th[:H * n] = rng.normal(0.0, cfg.get("head_sigma", 0.3), H * n)  # a = 0, c = 0: the start's policy exactly
        g0 = onp.concatenate([g0, th]); g_ref = onp.concatenate([g_ref, onp.zeros_like(th)])
        g_ref[GENOME_LEN:GENOME_LEN + H * n] = th[:H * n]
        mask = onp.concatenate([onp.zeros(GENOME_LEN, bool), onp.ones(len(th), bool)])
        print(f"dighead: {H} units over {n} inputs, {len(th)} trainable slots; the genome frozen")
    print(f"trainable slots: {mask.sum()} (live {(mask & (onp.abs(g0) >= W_EPS)).sum()}, allow-list {len(allow)}, "
          f"frozen {len(frozen)}, of them live {int((onp.abs(g0[frozen]) >= W_EPS).sum()) if frozen else 0})")
    # --- data: rows grouped, capped, split by world
    groups = {}
    for li, path in enumerate(cfg["soil_logs"]):
        dec, wl = parse_log(path)
        if cfg["arm"] == "sil":
            sil_labels(dec, wl, cfg.get("t_down", 200), cfg.get("t_home", 150))
        for r in dec:
            r["world"] = (li, r["tag"])
            r["prevrow"] = dec[r["prev"]] if r["prev"] >= 0 else None
            if r.get("ctx_only"):
                continue
            groups.setdefault(group_of(r), []).append(r)
    for li, (arena, path) in enumerate(cfg.get("keep_logs", [])):
        dec, _ = parse_log(path, arena=arena)
        if cfg.get("arena_bit"):
            for r in dec:
                assert r["ctx"][17] == 0.0, "c::CROWD is not 0 in a solo arena: the borrowed bit is not free"
                r["ctx"] = r["ctx"].copy()
                r["ctx"][17] = 1.0
        for r in dec:
            r["world"] = (100 + li, r["tag"])
            r["prevrow"] = dec[r["prev"]] if r["prev"] >= 0 else None
            r["dig"], r["digset"], r["down"], r["home"] = "-", [], [], []
            if r.get("ctx_only"):
                continue
            # `keep_lookalike`: kept-skill decisions that look like a lesson's trigger get their own anchor group, so
            # they cannot be drowned by the many that do not. "below": an empty ant below the ground as built (c::DEPTH
            # > 0) -- go-down's trigger; in fetch it is a forager in the nest's chamber, where the go-down copy kept
            # stepping deeper and never came out (fetch scores ~1 on 5-12 of 96 worlds against s171's 2)
            look = cfg.get("keep_lookalike")
            if look == "below" and not r["carrying"] and r["ctx"][18] > 0.0:
                groups.setdefault("keepL", []).append(r)
            else:
                groups.setdefault("keep", []).append(r)
    rules = set(cfg.get("rules", ["dig", "down", "home"]))
    if rules != {"dig", "down", "home"}:
        # copy only some of the teacher's rules: the others' labels are cleared, so those decisions anchor to the start
        moved = {}
        for gname in list(groups):
            keep_rows = []
            for r in groups[gname]:
                if "dig" not in rules:
                    r["dig"], r["digset"] = "-", []
                if "down" not in rules:
                    r["down"] = []
                if "home" not in rules:
                    r["home"] = []
                g2 = gname if gname in ("keep", "keepL") else group_of(r)
                moved.setdefault(g2, []).append(r)
        groups = moved
        print(f"rules copied: {sorted(rules)}")
    if cfg["arm"] == "shuffle":
        shuffle_labels(groups, rng)
    caps = cfg.get("caps", {})
    worlds = sorted({r["world"] for rs in groups.values() for r in rs})
    hold = {w for w in worlds if rng.random() < cfg.get("holdout_frac", 0.2)}
    if "hold_logs" in cfg:  # foraging-loop thread: hold out these soil_logs by index (one log is one colony)
        hold = {w for w in hold if w[0] >= 100} | {w for w in worlds if w[0] in cfg["hold_logs"]}
    data = {}
    for gname, rs in groups.items():
        rs = [r for r in rs if r["opts"]]
        rng.shuffle(rs)
        rs = rs[:caps.get(gname, 8000)]
        tr = [r for r in rs if r["world"] not in hold]
        te = [r for r in rs if r["world"] in hold]
        if not tr:
            continue
        # arms: teacher (the ceiling policy), imitate (another genome's policy everywhere: the positive control with
        # s162, the return-to-start control with s161 as target and a perturbed start), shuffle (teacher labels moved
        # within each decision / within each world: the negative control)
        mode = "ref" if (gname in ("keep", "keepL", "none") or cfg["arm"] == "imitate") else "teacher"
        ref = target_g if cfg["arm"] == "imitate" else g_ref[:GENOME_LEN]  # the plain genome: hcs's own cell choice
        packs = []
        for part in (tr, te):
            if not part:
                packs.append(None); continue
            B = batch_of(part)
            prev = batch_of([r["prevrow"] or r for r in part])
            hasprev = onp.array([r["prevrow"] is not None for r in part])
            done = onp.array([(r["prevrow"] or r)["opts"][(r["prevrow"] or r)["pick"]][0] for r in part])
            pv, pt = targets(ref, B, part, mode)
            packs.append(dict(B=B, prev=prev, hasprev=hasprev, done=done, pv=pv, pt=pt))
        data[gname] = packs
        print(f"group {gname:5s}: train {len(tr):6d}  held out {len(te):6d}  target {mode}")
    weights = cfg.get("group_weights", {})
    mempath = cfg.get("mempath", True)

    def loss_on(g, pk, idx, cell=False):
        B = take(pk["B"], idx)
        mem = B["mem"]
        if mempath:
            Pp = take(pk["prev"], idx)
            fp = forward(g, Pp)
            mrec = remember(g, Pp, fp["all_best"], fp["all_mean"], pk["done"][idx], Pp["mem"])
            mem = np.where(pk["hasprev"][idx][:, None], mrec, B["mem"])
        f = forward(g, B, mem)
        if cell:
            return np.mean(kl_cell(pk["pv"][idx], pk["pt"][idx], f, f["vmask"]))
        return np.mean(kl(pk["pv"][idx], pk["pt"][idx], f, f["vmask"]))

    newmask = (onp.abs(g_ref) < W_EPS).astype(float)
    l1 = cfg.get("l1_new", 0.0)

    def total(g, part, nb):
        L = l1 * np.sum(np.abs(g) * newmask) if l1 else 0.0
        for gname, packs in data.items():
            pk = packs[part]
            if pk is None:
                continue
            n = pk["B"]["n"]
            idx = rng.choice(n, min(nb, n), replace=False) if nb else onp.arange(n)
            L = L + weights.get(gname, 1.0) * loss_on(g, pk, idx, TARGET_ONLY[0] and gname == "dig")
        return L

    def held_out(g):
        out = {}
        for gname, packs in data.items():
            if packs[1] is not None:
                out[gname] = float(loss_on(g, packs[1], onp.arange(packs[1]["B"]["n"]), TARGET_ONLY[0] and gname == "dig"))
        return out

    dL = grad(lambda g, nb: total(g, 0, nb))
    g = g0.copy()
    m1 = onp.zeros(len(g0)); m2 = onp.zeros(len(g0))
    lr, steps, nb = cfg.get("lr", 0.01), cfg.get("steps", 1000), cfg.get("batch", 512)
    h0 = held_out(g)
    best = (sum(h0.values()), g.copy(), 0)
    print(f"step 0: held-out KL {' '.join(f'{k} {v:.4f}' for k, v in h0.items())}")
    t0 = time.time()
    for step in range(1, steps + 1):
        gr = onp.asarray(dL(g, nb)) * mask
        m1 = 0.9 * m1 + 0.1 * gr
        m2 = 0.999 * m2 + 0.001 * gr * gr
        g = g - lr * (m1 / (1 - 0.9 ** step)) / (onp.sqrt(m2 / (1 - 0.999 ** step)) + 1e-8) * mask
        # mutation's range, +-40 -- but never inside a start weight already beyond it, and only on trainable slots
        g = onp.where(mask, onp.clip(g, onp.minimum(-40.0, g0), onp.maximum(40.0, g0)), g0)
        if step % cfg.get("every", 100) == 0 or step == steps:
            h = held_out(g)
            s = sum(h.values()) + (float(l1 * onp.sum(onp.abs(g) * newmask)) if l1 else 0.0)
            if s < best[0]:
                best = (s, g.copy(), step)
            print(f"step {step}: held-out KL {' '.join(f'{k} {v:.4f}' for k, v in h.items())}  sum {s:.4f}  "
                  f"({time.time() - t0:.0f} s)", flush=True)
            if step - best[2] >= cfg.get("patience", 400):
                print(f"early stop at {step}; best step {best[2]}")
                break
    g = best[1]
    if cfg.get("dighead"):
        th = g[GENOME_LEN:]
        with open(cfg["out"].replace(".raw.txt", "") + ".head", "w") as fh:
            fh.write(f"{HEAD_H[0]} {ND}{' 1' if WHEN[0] else ''}\n" + " ".join(f"{x:.7g}" for x in th) + "\n")
        print(f"head: |a| {onp.abs(head_parts(th)[2]).round(3)} c {th[-1]:.3f}")
        report(g_ref[:GENOME_LEN], g, data, mempath)
        g = g[:GENOME_LEN]
        assert onp.array_equal(g, g0[:GENOME_LEN]), "dighead: the genome moved"
        write_genome(cfg["out"], g, head)
        return
    write_genome(cfg["out"], g, head)
    moved = onp.abs(g - g0) > 1e-9
    revived = moved & (onp.abs(g0) < W_EPS)
    print(f"wrote {cfg['out']}: best step {best[2]}, live links {live_links(g)} (start {live_links(g_ref)}), "
          f"moved {moved.sum()}, revived {revived.sum()} {list(onp.where(revived)[0])[:12]}")
    report(g_ref, g, data, mempath)


def report(g_ref, g, data, mempath=True):
    """Per group, on held-out decisions: how much of the target's (verb, target) distribution each genome matches,
    with memory computed the way the loss computes it (the one-step path) -- a report on logged memory disagreed with
    the loss it summarises (positive control, 2026-10-21)."""
    for gname, packs in data.items():
        pk = packs[1]
        if pk is None:
            continue
        B = pk["B"]
        out = []
        for name, gg in (("start", g_ref), ("copy", g)):
            mem = B["mem"]
            if mempath:
                fp = forward(gg, pk["prev"])
                mrec = onp.asarray(remember(gg, pk["prev"], fp["all_best"], fp["all_mean"], pk["done"], pk["prev"]["mem"]))
                mem = onp.where(pk["hasprev"][:, None], mrec, B["mem"])
            f = forward(gg, B, mem)
            pv = onp.exp(onp.asarray(f["logpv"])) * f["present"]
            pt = onp.exp(onp.asarray(f["logpt"])) * f["valid"]
            # mass the copy puts where pi* puts it: sum over options of min(pi*, p) for the joint (verb, target)
            jv = onp.stack([((B["verb"] == v) & f["valid"]) for v in range(VERBS)], 1)
            p_joint = onp.sum(onp.where(jv, (pv[:, :, None] * pt[:, None, :]), 0.0), axis=1)
            q_joint = onp.sum(onp.where(jv, (pk["pv"][:, :, None] * pk["pt"][:, None, :]), 0.0), axis=1)
            overlap = onp.minimum(p_joint, q_joint).sum(1).mean()
            extra = ""
            if gname == "carry":
                extra = f" P(label verb) {onp.sum(pv * pk['pv'], 1).mean():.3f}"
            if gname == "dig":
                # which cell, given Dig: the start's and the copy's mass on the old ant's cell(s)
                dsel = (B["verb"] == DIG) & f["valid"]
                extra = f" P(old ant's cell | Dig) {onp.sum(onp.where(dsel, onp.minimum(pt, pk['pt']), 0.0), 1).mean():.3f} P(Dig) {pv[:, DIG].mean():.3f}"
            if gname in ("nodig", "none"):
                extra = f" P(Dig) {pv[:, DIG].mean():.4f}"
            if gname == "digM":
                extra = f" P(Dig) {pv[:, DIG].mean():.3f}"
            elif gname == "digN":
                extra = f" P(Dig) {pv[:, DIG].mean():.4f}"
            out.append(f"{name}: overlap with target {overlap:.3f}{extra}")
        print(f"  {gname:5s} " + " | ".join(out))


if __name__ == "__main__":
    cmd = sys.argv[1]
    if cmd == "replaycheck":
        replaycheck(*sys.argv[2:5])
    elif cmd == "roundtrip":
        g, h = read_genome(sys.argv[2])
        write_genome(sys.argv[3], g, h)
        print(f"round trip: {live_links(g)} live links -> {sys.argv[3]}")
    elif cmd == "perturb":
        perturb(*sys.argv[2:7])
    elif cmd == "memcheck":
        memcheck(sys.argv[2], sys.argv[3])
    elif cmd == "quiet":
        g, h = read_genome(sys.argv[2])
        q, n = quiet_unread(g)
        write_genome(sys.argv[3], q, h)
        print(f"quiet: {n} links of unread units zeroed; live {live_links(g)} -> {live_links(q)} -> {sys.argv[3]}")
    elif cmd == "train":
        train(sys.argv[2])
    else:
        raise SystemExit(__doc__)
