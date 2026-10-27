import csv, sys, matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
ARMS = [('ctl', "today (no laying)", '#2a78d6'), ('lay', 'laying, no brake', '#eb6834'), ('cap', 'laying + size cap 48', '#1baf7a'),
        ('sb', 'laying + store brake', '#eda100'), ('fb', 'laying + food brake', '#e87ba4')]
fig, ax = plt.subplots(2, 3, figsize=(15, 7.5), sharex=True, sharey='row')
for j, s in enumerate('123'):
    for k, (a, lab, c) in enumerate(ARMS):
        r = list(csv.DictReader(open(f'b/{a}{s}/stock.csv')))
        f = [int(x['frame']) / 1000 for x in r]
        ax[0][j].plot(f, [int(x['alive']) for x in r], color=c, lw=2, label=lab)
        ax[1][j].plot(f, [int(x['nest_food']) for x in r], color=c, lw=2, label=lab)
    ax[0][j].set_title(f'seed {s}', fontsize=12)
    ax[1][j].set_xlabel('thousand frames (one training round = 200)')
ax[0][0].set_ylabel('adult ants')
ax[1][0].set_ylabel('stored food in the nest (cells)')
for a in ax.flat:
    a.grid(color='#e6e6e3', lw=0.8); a.set_axisbelow(True)
    for sp in ('top', 'right'): a.spines[sp].set_visible(False)
fig.legend(*ax[0][0].get_legend_handles_labels(), frameon=False, loc='lower center', ncol=5, fontsize=11)
fig.suptitle("Turning the game's egg-laying on in the training world: colony size and the larder", fontsize=14)
fig.tight_layout(rect=(0, 0.05, 1, 1))
fig.savefig(sys.argv[1], dpi=110)
