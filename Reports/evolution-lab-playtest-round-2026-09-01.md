# The lab's fourth playtest round — four defects and a number

*2026-09-01. Branch `claude/evolution-lab-fixes-dzo88x`. The shipped
behaviour is README's **Lab hand-verbs status**; the measurement behind the
plant count is reproducible with `cargo run --release --example labstats`, and
the zoom sheet with `cargo run --release --example labzoom`.*

Four things the owner hit while playing the lab, reported in one message:
zooming out past the box, a creature that would not stay selected, a released
clone that could not move, and a plant count that read 200+ over a bed with a
handful of visible plants. Three were defects; the fourth was a number
answering a different question than the one it was being read for.

**What each one overturned** — the part a later session cannot reconstruct
from the diff — follows.

**A verb can be complete, tested, documented and not wired in.** The shelf
shipped in round four with a round-trip guard, a brood dial, a rack page and a
README section — and a released *animal* had never once taken a tick.
`release_creature_specimen` built the body, took the slot, and dropped the
`ActiveSite` its first tick had to be booked at; every other placement path
schedules it. Nothing was red: the animal was in the world, in the organism
table, and in the census. Gravity is inside a creature's own tick, so it did
not even fall — which is why the owner's report was *"stuck in midair"*
rather than "does nothing". **The class to carry: a channel with a writer and
a reader can still be missing the wire between them, and every guard over
either end stays green.** The shelf's own tests all placed and then inspected;
none ran a frame.

**The lab inherits the sandbox's controls, and the sandbox's controls assume a
world bigger than the screen.** The zoom is the case that surfaced, and it is
not the last one: `adjust_zoom` cannot overrun an 8192x2560 world and overruns
a 512x320 box on its first step out. **Anything in `render.rs` that is bounded
by "the world is huge" is a defect waiting in the lab** — the camera clamp was
the second one in the same function (a world smaller than the view pinned to
the origin instead of centring). Both fixes derive from `World::bounds()` on
every call rather than caching, because the box's size is a live knob.

**"Which species" was a `String`, so it had no editor.** The parameters page
moves numbers; `colony_species` is text, so the one thing deciding which
animal the box runs was reachable only by editing an asset. Three animals
shipped and one could be placed. **Look for the same shape in every remaining
knob that is not an `f32`** — that is the whole of why a beetle needed a
session rather than a click.

**The bar being full is not the end of the argument.** Round four measured it
full at seven tool cells and recorded that the next control needs a row, a
page or a removal. The third option nobody had listed is **a cell that already
exists and means nothing under the armed tool**: the brush radius is dead
weight under `COLONY` and under a release, so the stocking dial took those
three cells. Free, and the fit guard still passes at the widest face.

**Two verbs sharing one control want two defaults, and both are right.** The
stocking dial holds a separate stop for `COLONY` (52, since below about fifty
a colony looks broken) and for a jar release (1, since fifty-two copies of the
one good forager you kept is a box you did not ask for). The first build
shared one value and broke three existing tests, which is what said so.

**`PLANTS` was counting the seed bank, and the natural fix was the wrong
one.** The owner's guess — *"a few large plants and lots of tiny 1-3 cell
plants"* — implies a size threshold. Measured (`labstats`, and the numbers are
in README): at every stop past frame 5,000 the whole of the discrepancy is
**ungerminated seed**, 419 of 467 at frame 30,000, and the 2-9 cell bucket
holds about three. Both halves settle — stand ~48, bank ~430 — so this is a
bed in balance being described by one number that answers neither question. A
threshold would have been tuned against a quantity that was not there.

**One loose end, filed rather than fixed:** the census sees a live plant
organism with **zero cells**, one at a time, intermittently — visible as
`PLANT SIZE 0 / …` on the page and as a 419-organism / 418-cell reading in the
probe. It is one organism in ~450 and it is not the seed bank. Nobody has
looked at what produces it.
