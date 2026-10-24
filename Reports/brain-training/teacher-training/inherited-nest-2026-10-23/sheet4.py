import sys
from PIL import Image, ImageDraw, ImageFont
S='/tmp/claude-0/-home-user-Pixel-Physics/3a00f6df-217d-51cc-af7b-c964241c2ed1/scratchpad'
maps=sys.argv[2].split(','); Ns=sys.argv[3].split(',')
def load(p):
    L=open(p).read().split('\n'); h=L[0].split(' '); leg=h[2].split(',')
    return int(h[0]),int(h[1]),[[leg[ord(c)-65] if c!='?' else '?' for c in r] for r in L[1:] if r]
COL={'empty':(18,20,28),'soil':(110,80,55),'packedsoil':(70,48,32),'spoil':(170,135,85),'stone':(90,90,95),'nest':(200,170,60),'crumbs':(160,140,90),'?':(150,150,150),'corpse':(150,60,60)}
GND={'soil','packedsoil','spoil','stone'}
X0,X1,Y0,Y1,K=88,170,50,104,3
font=ImageFont.truetype('/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf',12)
pw,ph=(X1-X0)*K,(Y1-Y0)*K
ncol=1+len(Ns)
img=Image.new('RGB',(8+ncol*(pw+6),24+len(maps)*(ph+22)),(245,245,240)); d=ImageDraw.Draw(img)
d.text((8,4),sys.argv[4],fill=(0,0,0),font=font)
import re
for i,m in enumerate(maps):
    oy=22+i*(ph+22); fam,c=m.split('_')
    pan=[(f'/mnt/project-files/teacher-lane/nestmaps-1022/{fam}/{c}/nestmap.txt',f'{m}: what the ants dug',None)]
    for N in Ns:
        lg=open(f'{S}/sw4/{m}/N{N}/log').read(); mm=re.search(r'dropped (\d+) cells in small pockets, linked (\d+)',lg)
        pan.append((f'{S}/sw4/{m}/N{N}/f0/nestmap.txt',f'link={N}: +{mm.group(2)} link, -{mm.group(1)} dropped',f'{S}/sw3/{m}/L0/f0/nestmap.txt'))
    for j,(p,t,base) in enumerate(pan):
        x0,y0,a=load(p); b=load(base)[2] if base else None
        ox=8+j*(pw+6); d.text((ox,oy),t,fill=(0,0,0),font=font)
        for y in range(Y0,Y1):
            for x in range(X0,X1):
                cc=a[y-y0][x-x0]; col=COL.get(cc,(255,0,255))
                if b:
                    bb=b[y-y0][x-x0]
                    if cc not in GND and bb in GND: col=(60,200,90)
                    elif cc in GND and bb not in GND and bb!='?': col=(230,60,200)
                d.rectangle([ox+(x-X0)*K,oy+16+(y-Y0)*K,ox+(x-X0+1)*K-1,oy+16+(y-Y0+1)*K-1],fill=col)
img.save(sys.argv[1])
