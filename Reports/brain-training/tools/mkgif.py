import sys, glob, os
from PIL import Image, ImageDraw, ImageFont
G, out, x0, y0, w, h, z = sys.argv[1], sys.argv[2], *map(int, sys.argv[3:8])
panels = [(sys.argv[8], sys.argv[9]), (sys.argv[10], sys.argv[11])]
try:
    font = ImageFont.truetype('/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf', 16)
    small = ImageFont.truetype('/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf', 13)
except Exception:
    font = small = ImageFont.load_default()
names = sorted(os.path.basename(p) for p in glob.glob(f'{G}/{panels[0][0]}/f*.png'))
names = [n for n in names if int(n[1:7]) <= 6000]
pad, top, bottom = 12, 30, 42
W = 2 * w * z + 3 * pad
H = h * z + top + bottom
frames = []
counts = {}
for n in names:
    canvas = Image.new('RGB', (W, H), (250, 250, 250))
    d = ImageDraw.Draw(canvas)
    for i, (sub, label) in enumerate(panels):
        im = Image.open(f'{G}/{sub}/{n}').convert('RGB').crop((x0, y0, x0 + w, y0 + h))
        px = im.load()
        cut = sum(1 for yy in range(h) for xx in range(w) if px[xx, yy] == (0, 190, 255))
        away = sum(1 for yy in range(h) for xx in range(w) if px[xx, yy] == (40, 220, 60))
        back = sum(1 for yy in range(h) for xx in range(w) if px[xx, yy] == (255, 190, 0))
        im = im.resize((w * z, h * z), Image.NEAREST)
        x = pad + i * (w * z + pad)
        canvas.paste(im, (x, top))
        d.text((x, 6), label, fill=(20, 20, 20), font=font)
        d.text((x, top + h * z + 4), f'cut {cut}   put down away {away}   put down near the hole {back}', fill=(40, 40, 40), font=small)
    d.text((pad, H - 18), f'frame {int(n[1:7]):,} of 6,000     cyan = cut  green = pellet put down away  amber = put down near the hole  magenta = ant', fill=(90, 90, 90), font=small)
    frames.append(canvas)
frames += [frames[-1]] * 12
frames[0].save(out, save_all=True, append_images=frames[1:], duration=80, loop=0, optimize=True)
print(out, len(frames), os.path.getsize(out))
