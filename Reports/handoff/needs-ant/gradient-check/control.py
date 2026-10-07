# Positive control for solve() and settle_frames(): a straight 5-wide shaft,
# 40 cells deep, under open sky, uniform source. Known answers:
#  steady state c(z) = S (L z - z^2/2) at unit conductance (z from the sky face)
#  settle to 10% L1 ~ tau1 * ln(10 * a1), tau1 = 4 L^2 / (pi^2 D)
import numpy as np, sys, math
sys.argv=['x']
exec(open('gradcheck.py').read().split("if __name__")[0])
h, w = 60, 20
openm = np.zeros((h, w), bool)
openm[:10, :] = True          # sky band
openm[10:50, 8:13] = True     # shaft rows 10..49
sky = np.zeros((h, w), bool); sky[:10, :] = True
src = np.where(openm & ~sky, 1.0, 0.0)
c, sealed = solve(openm, sky, src, 0.0, 0.0)
L = 40
z = np.arange(1, L + 1) - 0.5   # cell centres from the sky face
col = c[10:50, 10]
ana = (L * z - z * z / 2)
print('steady: solve vs analytic at z=1,10,20,40:', [round(col[i], 1) for i in (0, 9, 19, 39)], [round(ana[i], 1) for i in (0, 9, 19, 39)])
nest = openm & ~sky
D = 0.25 / 36
f = settle_frames(openm, sky, src, 0.0, c, nest)
tau = 4 * L * L / (math.pi ** 2 * D)
print('settle frames', f, 'expected ~', round(tau * math.log(10 * 32 / math.pi ** 3)), '(tau1', round(tau), ')')
