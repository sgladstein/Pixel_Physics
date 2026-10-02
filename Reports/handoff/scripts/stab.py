# Breeding lane steadiness summary (copied from the old project folder, 2026-10-02).
# Usage: python3 stab.py DIR, with logs named ARM-gGAP-sSEED.log from trailfollow long runs.
# Edit the arm list below. Prints min-max ants from frame 60k, the coefficient of
# variation, and halvings from a running peak (peak >= 20).
import re,sys,glob,statistics as st
d=sys.argv[1]
for arm in ['ind','g1.25','g2','g3']:
  for g in [90,200]:
    rows=[]
    for s in range(1,7):
      f=f'{d}/{arm}-g{g}-s{s}.log'
      line=[l for l in open(f) if 'FOOD STORE' in l][0]
      pts=[(int(m.group(1)),int(m.group(2)),int(m.group(3))) for m in re.finditer(r'(\d+): nest ground \d+, crops (\d+),.*? ants (\d+),',line)]
      fr=[p[0] for p in pts]; ants=[p[2] for p in pts]; crops=[p[1] for p in pts]
      late=[a for t,a in zip(fr,ants) if t>=60000]
      if not late or max(late)==0: rows.append('dead'); continue
      mn=min(late); mx=max(late); cv=st.pstdev(late)/st.mean(late)
      # crashes: falls of >=50% from a running peak (reset after)
      crashes=0; peak=0
      for a in ants:
        if a>peak: peak=a
        if peak>=20 and a<=peak*0.5: crashes+=1; peak=a
      rows.append(f'{mn}-{mx} cv{cv:.2f} c{crashes}')
    print(f'{arm:5} gap{g:3}: '+' | '.join(rows))
