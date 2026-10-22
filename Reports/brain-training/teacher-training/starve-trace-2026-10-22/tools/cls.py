import json,sys,collections
def cls(o):
    dx=abs(o['dx']); y=o['y']; sp=o['spoil_frac']>0.5
    stuck=(o['xrange'][1]-o['xrange'][0])<=4 and (o['yrange'][1]-o['yrange'][0])<=4
    zone="wall" if dx>=100 else "far" if dx>=40 else "foodband" if dx>15 else "home"
    under="under" if y>=67 else "top" if y<58 else "surf"
    return (zone,under,"spoil" if sp else "-","stuck" if stuck else "moving")
for g in sys.argv[1:]:
    import glob
    c=collections.Counter(); z=collections.Counter(); n=0; bursts=0
    for f in sorted(glob.glob(f"l/{g}*/dead.json")):
        for o in json.load(open(f)):
            c[cls(o)]+=1; z[cls(o)[0]]+=1; n+=1; bursts+= o['burst']>=5
    print(g,"deaths",n,"in bursts(>=5 within 3k)",bursts, dict(z))
    for k,v in c.most_common(): print("  ",v,k)
