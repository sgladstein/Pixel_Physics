import gzip
from PIL import Image, ImageDraw, ImageFont
exec(open('ana1.py').read().split('for run in sys.argv')[0])
W,H=760,300; L,R,T,B=70,20,48,40
def font(s):
    for p in ['/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf']:
        try: return ImageFont.truetype(p,s)
        except: pass
    return ImageFont.load_default()
f12,f14=font(12),font(15)
def panel(run,title,dead):
    im=Image.new('RGB',(W,H),'#fcfcfb'); d=ImageDraw.Draw(im)
    fmax=3000; ylo,yhi=-45,45
    X=lambda fr: L+(W-L-R)*fr/fmax
    Y=lambda dx: T+(H-T-B)*(yhi-dx)/(yhi-ylo)
    for fx in (25,-25):  # food bands
        d.rectangle([L,Y(fx+3),W-R,Y(fx-3)] if fx>0 else [L,Y(fx+3),W-R,Y(fx-3)],fill='#f6e3c9')
    for g in (-40,-20,0,20,40):
        d.line([L,Y(g),W-R,Y(g)],fill='#e4e3df'); d.text((L-8,Y(g)),f'{g:+d}' if g else 'nest',fill='#52514e',font=f12,anchor='rm')
    for fr in range(0,fmax+1,500):
        d.text((X(fr),H-B+14),f'{fr:,}',fill='#52514e',font=f12,anchor='mm')
    for r in rows(run,fmax):
        x,y=X(r['fr']),Y(max(ylo,min(yhi,r['x']-128)))
        d.ellipse([x-1.2,y-1.2,x+1.2,y+1.2],fill='#2a78d6')
    if dead: d.line([X(dead),T,X(dead),H-B],fill='#c2342b',width=2); d.text((X(dead)+4,T+4),'all 24 starve',fill='#c2342b',font=f12)
    d.text((L,12),title,fill='#0b0b0b',font=f14)
    d.text((W-R,H-8),'shaded: food, 25 columns out each side',fill='#52514e',font=f12,anchor='rm')
    d.text((8,T-14),'columns from nest',fill='#52514e',font=f12)
    d.text((L+(W-L-R)/2,H-8),'frame',fill='#52514e',font=f12,anchor='mm')
    return im
a=panel('c1-6','c1 (seed 6): ants walk out to the food, 22 of 24 alive at 200k',None)
b=panel('c1g-6','c1 + trail graft (seed 6): every ant stays at the nest',2100)
out=Image.new('RGB',(W,2*H+10),'#fcfcfb'); out.paste(a,(0,0)); out.paste(b,(0,H+10)); out.save('/mnt/project-files/maze/colony/graft-ants-stay-home-seed6.png')
