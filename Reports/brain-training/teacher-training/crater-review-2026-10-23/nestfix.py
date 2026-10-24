"""nestcount.count with two fixes, for comparison: (1) the flood from the door passes anything that is not ground (soil,
packed soil, spoil, stone) -- stored crumbs, door markers, corpses -- as inrun's cut_join flood already does; (2) crater
is only sky-open cells (no ground at or below the datum above them in the column), never a roofed pocket that is merely
cut off from the door. Returns dict(nest, crater, nest_old, crater_old)."""
import sys
sys.path.insert(0,'/home/user/cloud/scratch/inrun')
import nestcount
def count(d):
    old=nestcount.count(d)
    t=open(f'{d}/nestmap.txt').read().split('\n'); head=t[0].split(' '); x0,y0,legend=int(head[0]),int(head[1]),head[2].split(',')
    grid=[r for r in t[1:] if r]; surf=[int(l) for l in open(f'{d}/surface.txt') if l.strip()]
    H,W=len(grid),len(grid[0])
    ground={chr(65+k) for k,m in enumerate(legend) if m in ('soil','packedsoil','spoil','stone')}
    e=chr(65+legend.index('empty')) if 'empty' in legend else None
    opn=lambda dx,dy: 0<=dy<H and 0<=dx<W and grid[dy][dx] not in ground
    below=lambda dx,dy: 0<=x0+dx<len(surf) and y0+dy>=surf[x0+dx]
    door=128-x0
    seen=set(); st=[(door,dy) for dy in range(H) if opn(door,dy) and below(door,dy)]
    while st:
        p=st.pop()
        if p in seen: continue
        seen.add(p); dx,dy=p
        for q in ((dx-1,dy),(dx+1,dy),(dx,dy-1),(dx,dy+1)):
            if opn(*q) and below(*q) and q not in seen: st.append(q)
    top=lambda dx: max(0,surf[x0+dx]-y0)
    roofed=lambda dx,dy: any(grid[yy][dx] in ground for yy in range(top(dx),dy))
    gap=[0<=top(dx)<H and grid[top(dx)][dx] not in ground for dx in range(W)]
    wide=[False]*W; dx=0
    while dx<W:
        if not gap[dx]: dx+=1; continue
        k=dx
        while k<W and gap[k]: k+=1
        for q in range(dx,k): wide[q]=k-dx>5
        dx=k
    nest={(dx,dy) for dx,dy in seen if roofed(dx,dy) or not wide[dx]}
    crater=sum(1 for dy in range(H) for dx in range(door-20,door+21) if grid[dy][dx]==e and below(dx,dy) and (dx,dy) not in nest and not roofed(dx,dy))
    return dict(nest=len(nest),crater=crater,nest_old=old['nest'],crater_old=old['crater'])
if __name__=='__main__':
    for d in sys.argv[1:]: print(d, count(d.rstrip('/')))
