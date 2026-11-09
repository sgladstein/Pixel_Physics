# Interface looks: the key (not for the blind judging)

| Letter | Look | `Style` in `src/mould/look.rs` |
|---|---|---|
| A | Picture book: inked outlines, chunky sticker type, coloured buttons on a wooden bench, hazard tape, a fuse for time | `Style::Toy` |
| B | Night culture: glowing tubes in the dark, lamp lights the agar, round buttons, a storm dial, mixed-case font | `Style::Glow` |
| C | Specimen bench: glass dish on graph paper, glossy embossed tubes, tiles with stock pips, a timeline to the storm | `Style::Bench` |
| N (round 2, not blind) | The built look: B's dish and mood with C's tool tiles and timeline; the game opens in it (`Style::GAME`) | `Style::Night` |

`current-*` is today's `render.rs`, unchanged, for reference.

The letters were shuffled with `shuf` before any picture was made. The window shows only the letter (bottom-right corner), and `Style::letter` in the code must stay in step with this table.
