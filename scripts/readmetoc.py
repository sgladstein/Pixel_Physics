#!/usr/bin/env python3
"""Regenerate the table of contents at the top of `README.md`.

Why this exists
---------------
`README.md` is ~2,600 lines and carries 33 `##` sections, thirteen of them
milestone status write-ups **in the order they were written, not numeric
order**: M12/13, M14, M7, M15, M6, M5, M16, M17, M18, M8, M9, M10, M19. The
file's own `## Finding things` section admits it and tells the reader to grep.

A wholesale reorder was proposed, approved, and then *reversed*
(`Reports/documentation-overhaul-plan.md` item 11): agents navigate by grep,
the reorder is a huge diff on a contested file, and "a TOC buys the same
navigation for 3% of the churn". The status sections that item asked for
landed; the TOC did not. This is that TOC, so this script is the unexecuted
half of an approved item rather than a new proposal -- do not read it as an
argument for reordering, which stays refused.

Two audiences, one table: the **anchor** is for a human reading on GitHub, the
**line number** for an agent that is going to `sed -n '1351,1400p'`. The
milestone index below it is sorted numerically, which is the one thing grep
cannot do for you.

The **topic index** is the third table and answers the one navigation failure
that survived the other two. Measured 2026-08-25: five of six subsystems own
exactly one section apiece (`M17 status` is "structural collapse" wearing a
number), but *plants* own five top-level sections and not one of them is named
"plants" -- so an agent greps `plant`, lands in `M16 status`, and never learns
the standing-tissue economy has a section of its own.

`TOPICS` below is an **explicit** map, and that is a deliberate reversal. The
first cut scored sections by counting topic-term hits per section, which looks
principled and is not: it counts *mentions*, not *ownership*. `M18 status`
outranked `Materials` for "powders" because a worm burrows through a lot of
them, and `The ant colony -- status` fell out of "creatures" entirely -- at 14
lines it cannot clear any share bar set by a 254-line section. Tuning the
thresholds until the output looked right is exactly what
`Reports/design-philosophy.md` 2b calls curve-fitting, and what `CLAUDE.md`
means by asking what a metric counts when nothing is wrong. Membership is
editorial, so it is written down as data.

That trades away the one virtue the scored version had -- a new section could
not be silently missing -- so `--check` buys it back and then some:

* every title in `TOPICS` must still exist, so a renamed heading fails
  `scripts/docscheck.sh` instead of rotting into a dead anchor; and
* every `##` section must appear in some topic or in `UNINDEXED`, so adding a
  section forces a placement decision rather than a silent omission.

The first guard matters more than it looks. `Reports/dead-ends.md` addresses
**47 of its 594 entries by README section *and paragraph* name** across 16
sections, so these headings are a load-bearing address space for the
do-not-retry register: renaming one silently invalidates pointers into the one
document whose whole job is stopping an agent re-walking a dead end. This
check is the only mechanical thing in the repo that notices.

Sibling of `scripts/bugindex.py`, and shares its two hard-won mechanics: the
line numbers cite a document the table is inside, so generation iterates to a
fixpoint; and the block is *derived*, so a merge conflict inside it is never
hand-merged -- take either side whole and re-run.

Regenerate with `python3 scripts/readmetoc.py`; `--check` verifies it is
current and is what `scripts/docscheck.sh` runs.
"""

import re
import sys
from pathlib import Path

DOC = Path(__file__).resolve().parent.parent / "README.md"

# Topic -> the sections that *own* it, in intended reading order (primary
# first). Titles must match a `## ` heading exactly; `--check` fails if one
# stops existing, which is what makes a heading rename visible rather than
# silent. A section may appear under several topics -- `Felling status` is
# genuinely both plant work and structural work, and saying so once here is
# cheaper than the reader discovering it twice.
# Which game each topic belongs to, and the whole point of the column.
#
# **`README.md` is 71,561 tokens -- the largest document in this repository --
# and `CLAUDE.md`'s routing table sends every agent to the table below
# first.** With two games on one engine (the outdoor sandbox and the evolution
# lab, `Reports/two-games-one-repo-2026-08-30.md`) a session working on one of
# them was reading a topic map that could not tell it which entries were even
# about its game. This column is the cheapest possible fix: it changes no
# file layout, it is reversible in one commit, and it scopes the two routing
# layers every agent is sent to before it opens anything else.
#
# **`engine` is the honest default and the largest class**, because the whole
# argument for one repository is that plants, creatures, fire, liquids and the
# sweep are the same code in both games. A topic is only marked `outdoor` or
# `lab` when the *other* game demonstrably does not build a scene that reaches
# it: the lab has no rock, no worldgen and no gnome, and the outdoor game has
# no sealed box.
#
# **This is a routing hint, not an access rule.** Nothing stops a lab session
# reading the worldgen sections; the column only says it will probably be
# wasting its time. Getting one wrong costs a reader one section, which is why
# it is safe to add now and refine later -- unlike a directory move, which
# takes five non-recursive globs in `docscheck.sh` with it.
GAME = {
    "plants, trees and moss": "engine",
    "creatures — worms and the ant colony": "engine",
    "structural collapse, felling and rigid bodies": "outdoor",
    "fire, heat and phase change": "engine",
    "explosions, particles and debris": "outdoor",
    "liquids and gases": "engine",
    "powders and granular flow": "engine",
    "the coarse field grid — pressure, heat, light": "engine",
    "worldgen and world structure": "outdoor",
    "the gnome (player character)": "outdoor",
    "weather, sky and the clock": "engine",
    "rendering, UI and tunables": "engine",
    "performance and the parallel sweep": "engine",
    "materials and the data schema": "engine",
    "the evolution lab — the box and its lights": "lab",
    "putting things in the box, and what the view shows": "lab",
    "the speed dial, and what a tick costs": "lab",
    # `lab`, though `sim::specimen` is engine code and the sandbox could
    # perfectly well call it: the tag says which *game builds a scene that
    # reaches it*, and only the lab has the tools. Re-tag `engine` the day the
    # outdoor game grows a way to keep an individual.
    "keeping, cloning and mutating an individual": "lab",
    "reading one specimen off the screen": "lab",
    # The third lab row, and it is the one that gets you *to* an individual:
    # the two above assume you have already found one.
    "finding one individual among all of them": "lab",
    # The fourth lab row. Distinct from "reading one specimen off the screen"
    # deliberately: that row is the numbers, this is what they mean.
    "the genome in plain words": "lab",
    # The fifth lab row: what an individual has done, as opposed to what it
    # is. Also engine-side -- the counters live on `OrganismState`.
    "what an individual has done, and what killed it": "lab",
    # The sixth lab row, and the one that needs the box *running*: the
    # three above are all readable from a stopped frame, and a trail is
    # not -- it is accumulated history and only exists because time passed.
    "watching one individual over time": "lab",
    # The seventh lab row. The only one whose subject is a *pair*: every
    # other page is one individual or the whole box.
    "comparing two individuals": "lab",
    # The eighth, and the only one whose subject is the *population*. Every
    # other lab row asks about a cell, an individual or a pair; this asks
    # which founding line is taking the ground, which is the question a
    # selection box exists to ask and the one a table answers badly.
    "which founding line is winning": "lab",
    # The ninth: the *groups* in the population -- which click an animal came
    # from, the colour it wears for it, and the one rule (rivalry) that makes
    # the label mean something. The lineage row above is who descends from
    # whom; this is who the player put down together.
    "who is who in the box, colonies and the rivalry rule": "lab",
    # `TunableGroup::Lab` is deliberately excluded from the sandbox panel's
    # own menu cycle (`tunables.rs`'s own doc) -- the outdoor game has its
    # own separate save path and never reaches `lab::params`.
    "tuning and saving the box's own numbers": "lab",
    # The ninth lab row: a bed the player cannot save at all until this
    # landed, because what a player paints in by hand -- a food heap on a
    # schedule, a raised bank -- could not be expressed as a bare `LabBox`.
    "a saved starting box, and replicating it in a rack": "lab",
}

TOPICS = {
    "plants, trees and moss": [
        "M16 status",
        "Soil nutrient status \u2014 ground is worth something water is not",
        "Plant lines merged: the genome, and the ecology",
        "Inheritance status \u2014 the growth program has no fallback under it",
        "Parameter-genome status \u2014 a species file is a starting point, and it ships inert",
        "The economy re-derived: standing tissue costs something",
        "Plants that stop: organs, determinacy, and a price on both",
        "The generation loop: plants die, seeds expire, slots come back",
        "Stems draw a line: the growth walk renders its heading",
        "Felling status — the verb works, and what it produces is pieces",
        "Bending status — soft tissue lies over, and the wind is what pushes it",
        "Breaking status — a badly grown tree comes down on its own",
        "Specimen shelf status — an individual's genetics outlive the box",
        "Cell page status — the specimen readout is in three groups, and folds",
        "Roster status — every plant and every animal, as a list you click through",
        "Plain-speech status — the genome read back as sentences",
        "Life record status — what an individual has done, and what killed it",
    ],
    "creatures — worms and the ant colony": [
        "M18 status",
        "The ant colony — status",
        "Specimen shelf status — an individual's genetics outlive the box",
        "Roster status — every plant and every animal, as a list you click through",
        "Plain-speech status — the genome read back as sentences",
        "Lab hand-verbs status — what a click puts in the box, and what the view shows",
        "Life record status — what an individual has done, and what killed it",
        # Why a dug gallery now stays dug, and why a corpse no longer plugs
        # the passage its owner died in.
        "Lab soil status — a hole that stays a hole anywhere in the bed, and a bed that stops running out",
    ],
    "structural collapse, felling and rigid bodies": [
        "M17 status",
        "Felling status — the verb works, and what it produces is pieces",
        "Bending status — soft tissue lies over, and the wind is what pushes it",
        "Breaking status — a badly grown tree comes down on its own",
        "M8 status — started, not complete",
    ],
    "fire, heat and phase change": [
        "M14 status",
        "Materials",
    ],
    "explosions, particles and debris": [
        "M15 status",
        "M7 status",
    ],
    "liquids and gases": [
        "Liquid physics: compressible volume, not discrete occupied cells",
        "The coarse field grid",
    ],
    # No section is *named* for powders: the angle-of-repose model is written up
    # under `Materials`, and the movement rule it feeds is in `update.rs`, which
    # the `Architecture` file map is the index to. Two entries beats a reader
    # concluding it is undocumented.
    "powders and granular flow": [
        "Materials",
        "Architecture",
        # The un-pack line: what moisture takes a worked wall back to loose
        # soil, and the census the new threshold is sited from.
        "Lab soil status — a hole that stays a hole anywhere in the bed, and a bed that stops running out",
    ],
    "the coarse field grid — pressure, heat, light": [
        "The coarse field grid",
        "M12/M13 status",
        # The light channel's second writer: `Material::beam`, the emitter the
        # lab's fixtures use, lives in `apply_sky_to`'s column descent.
        "Lab lighting status — the fixtures are what light the crop",
    ],
    "worldgen and world structure": [
        "M10 status — the worldgen half",
        "Architecture",
    ],
    "the gnome (player character)": [
        "M9 status — the gnome",
        "Controls",
    ],
    "weather, sky and the clock": [
        "Weather status",
        "M19 status — started",
        "World speed — five independent time axes",
    ],
    "rendering, UI and tunables": [
        "UI improvements — overnight run, section 9",
        "Live tunables panel — overnight run, section 10",
        "Rendering performance — overnight run, section 11",
        "M6 deferral",
    ],
    "performance and the parallel sweep": [
        "Performance",
        "M5 status",
        "Architecture",
        "Rendering performance — overnight run, section 11",
        # Listed here as well as under its own lab row: it is the only place
        # in this file that says the CA sweep can be the majority of a frame,
        # and an agent arriving on "performance" has no reason to look under
        # a lab heading for it.
        "Lab speed-dial status — what the dial is actually short of",
    ],
    "materials and the data schema": [
        "Materials",
        "M12/M13 status",
    ],
    # **The lab's first topic row.** The routing table had none, so an agent
    # sent to the second game had nothing here to route by and the `lab` tag
    # on the GAME map above pointed at nothing.
    "the evolution lab — the box and its lights": [
        "Lab lighting status — the fixtures are what light the crop",
        # The bed itself, as distinct from what is grown in it: why a dug
        # gallery stays open and why the box stops running out of earth.
        "Lab soil status — a hole that stays a hole anywhere in the bed, and a bed that stops running out",
    ],
    # The lab's readouts, as distinct from its physics: what the interface
    # tells you about one individual, and how it fits that on a 512x320
    # screen. Listed under plants too, since the page that overflowed is the
    # plant one and an agent sent to the plant line has no reason to look
    # here for it.
    "reading one specimen off the screen": [
        "Cell page status — the specimen readout is in three groups, and folds",
    ],
    # The second lab row, and the first that is about what the *player* does
    # in the box rather than about the box. Also listed under creatures and
    # under plants: a jar holds either kingdom, and an agent sent to one of
    # them has no reason to look under "the evolution lab" for it.
    "keeping, cloning and mutating an individual": [
        "Specimen shelf status — an individual's genetics outlive the box",
    ],
    # The way *in* to the two rows above. They both start from "you have an
    # individual"; this is how you get one out of a box of a hundred, and it
    # is where the identity those pages are pinned by is defined. Listed under
    # plants and creatures too, for the reason the shelf is: an agent on
    # either line has no reason to look under the lab for it.
    "finding one individual among all of them": [
        "Roster status — every plant and every animal, as a list you click through",
    ],
    # Listed under creatures and plants too: the brain half and the allele
    # half are read by different lines, and neither has a reason to look
    # under the lab for it.
    "the genome in plain words": [
        "Plain-speech status — the genome read back as sentences",
    ],
    # Listed under creatures and plants too: the counters are on
    # `OrganismState` and both kingdoms carry them.
    "what an individual has done, and what killed it": [
        "Life record status — what an individual has done, and what killed it",
    ],
    "watching one individual over time": [
        "Watch status — where one individual has been, and how its numbers moved",
    ],
    "comparing two individuals": [
        "Side-by-side status — two individuals, with what differs marked",
    ],
    "which founding line is winning": [
        "Lineage overlay status — which founding line is taking the bed",
    ],
    "who is who in the box, colonies and the rivalry rule": [
        "Creature groups status — who is who in the box, and who is family",
    ],
    # The third lab row: the hand verbs and the view they are aimed through.
    # Separate from "the box and its lights", which is what the box *is*, and
    # from the shelf row, which is what a jar holds -- this is what a click
    # does. Also listed under creatures: two of the four things in it are
    # about getting an animal into the box, and an agent on the creature line
    # has no reason to look under a lab heading for them.
    "putting things in the box, and what the view shows": [
        "Lab hand-verbs status — what a click puts in the box, and what the view shows",
    ],
    # **What the speed dial costs, which is not what anyone assumed.** Its own
    # row rather than a line under "the box and its lights", because the
    # question it answers -- why a dial set to 1024x delivers 2x -- is the one
    # the owner actually asks, and it routes to a *frame*, not to a fixture.
    "the speed dial, and what a tick costs": [
        "Lab speed-dial status — what the dial is actually short of",
    ],
    # The fourth lab row: the parameters page's own save path -- what
    # persists, what doesn't, and where the files live. Separate from the
    # hand-verbs row above, which is about the tools that populate the box
    # rather than the numbers a player tunes once it exists.
    "tuning and saving the box's own numbers": [
        "Lab parameters status — a save that reaches the founders, not just the file",
    ],
    # The ninth lab row: a saved bed plus what got placed in it and a
    # running schedule, which is what the other eight rows above assume
    # already exists but nothing before this built.
    "a saved starting box, and replicating it in a rack": [
        "Lab scenarios status — a saved starting box with a question written on it",
    ],
}

# Sections deliberately absent from the topic index, so that `--check` can
# insist every *other* section is placed. `Status` is not an oversight: its
# **Known limitations** list spans every topic at once, so it earns a line of
# prose above the table rather than a row in fourteen of them.
UNINDEXED = {
    "Contents",
    "Running",
    "Finding things",
    "Status",
    "License",
}
BEGIN = "<!-- BEGIN GENERATED TOC -- regenerate with scripts/readmetoc.py -->"
END = "<!-- END GENERATED TOC -->"


def anchor(title):
    """GitHub's heading-anchor rule: lowercase, drop punctuation, spaces to
    hyphens. An em dash surrounded by spaces therefore yields a double hyphen,
    which is correct and looks like a typo."""
    a = title.lower()
    a = re.sub(r"[^a-z0-9 \-]", "", a)
    return a.replace(" ", "-")


def headings(lines):
    """Every `##` heading, skipping the generated block and fenced code.

    Without the first skip the table lists its own `## Contents` heading and
    points at itself -- harmless, but it makes the section count wrong and the
    fixpoint one iteration slower. The second skip is pre-emptive: no fenced
    block in `README.md` currently starts a line with `## `, but a shell
    transcript or a markdown example easily could, and the failure would be a
    confident index entry pointing at a line that is not a section.
    """
    inside = False
    fenced = False
    for i, line in enumerate(lines, 1):
        if line.startswith("```"):
            fenced = not fenced
            continue
        if line.startswith(BEGIN):
            inside = True
        elif line.startswith(END):
            inside = False
        elif not inside and not fenced and line.startswith("## "):
            yield line[3:].strip(), i


def short_label(title):
    """The part of a heading before its first `—` or `:`.

    The topic table names a subject already, so the descriptive tail of a
    heading is dead weight there -- "M10 status — the worldgen half" under the
    row **worldgen and world structure** says "the worldgen half" twice. Full
    titles stay in the Contents table above, and the anchor is still built from
    the full title, so the link does not care.
    """
    return re.split(r"\s+—\s+|:\s+", title, maxsplit=1)[0].strip()


def milestone_key(title):
    """Sort key for `M<n> ...` headings. `M12/M13` sorts on 12."""
    m = re.match(r"M(\d+)", title)
    return int(m.group(1)) if m else None


def validate(rows):
    """Fail loudly on the two ways the topic index can rot.

    Returned as a list of complaints rather than raised, so `--check` can print
    all of them at once -- a heading rename usually breaks several topics, and
    fixing them one run at a time is the kind of chore that gets abandoned.
    """
    titles = {t for t, _ in rows}
    problems = []
    for topic, members in TOPICS.items():
        for m in members:
            if m not in titles:
                problems.append(
                    f"  topic {topic!r} names a section that no longer exists: {m!r}\n"
                    f"    -> a `## ` heading was renamed or removed. Repoint it in TOPICS,\n"
                    f"       and check `Reports/dead-ends.md` -- 47 of its entries address\n"
                    f"       README by section name and do not move themselves."
                )
    # A topic with no `GAME` entry would crash `render` with a `KeyError` at
    # the moment somebody adds a topic -- a stack trace instead of a sentence.
    # Reported here so the failure names itself, and so the *reverse* --
    # a `GAME` entry for a topic that no longer exists -- is caught too, which
    # is how a renamed topic silently keeps a stale game label.
    for t in TOPICS:
        if t not in GAME:
            problems.append(
                f"  topic {t!r} has no GAME entry\n"
                f"    -> add it to GAME as 'engine', 'outdoor' or 'lab'. 'engine'\n"
                f"       is the honest default: mark a topic for one game only when\n"
                f"       the other never builds a scene that reaches it."
            )
    for t in GAME:
        if t not in TOPICS:
            problems.append(
                f"  GAME names topic {t!r}, which is not in TOPICS\n"
                f"    -> it was renamed or removed; repoint or drop the GAME entry."
            )
    placed = {m for members in TOPICS.values() for m in members}
    for t, _ in rows:
        if t not in placed and t not in UNINDEXED:
            problems.append(
                f"  section {t!r} is in no topic\n"
                f"    -> add it to a TOPICS entry, or to UNINDEXED if it genuinely\n"
                f"       indexes nothing (say why in a comment)."
            )
    return problems


def render(rows):
    out = [
        BEGIN,
        "",
        "## Contents",
        "",
        f"Generated by `scripts/readmetoc.py` from the `##` headings — {len(rows)} of",
        "them. The **anchor** is for reading on GitHub; the **line** is for jumping",
        "with `sed -n`. Sections sit in the order they were written; that is",
        "deliberate and a reorder was considered and refused",
        "(`Reports/documentation-overhaul-plan.md` item 11), so this table is how",
        "you navigate instead.",
        "",
        "**Merge conflict in this block?** Do not hand-merge it. Take either side",
        "whole, then run `python3 scripts/readmetoc.py`.",
        "",
        "| Section | Line |",
        "|---|---|",
    ]
    for title, line in rows:
        out.append(f"| [{title}](#{anchor(title)}) | {line} |")

    ms = sorted(
        ((milestone_key(t), t, l) for t, l in rows if milestone_key(t) is not None),
        key=lambda r: r[0],
    )
    if ms:
        out += [
            "",
            "### Milestones, in numeric order",
            "",
            "The one ordering grep cannot give you: these sit in the file in the",
            "order they were written.",
            "",
            "| M | Section | Line |",
            "|---|---|---|",
        ]
        for n, title, line in ms:
            out.append(f"| {n} | [{title}](#{anchor(title)}) | {line} |")
    line_of = dict(rows)
    out += [
        "",
        "### By topic",
        "",
        "Which sections own a subsystem, **primary first** — the rest are genuinely",
        "relevant rather than equal partners. Every subsystem has one clear primary",
        "write-up, usually a milestone: `M17 status` *is* the structural-collapse",
        "document, `M14 status` *is* fire. Plants are the exception this table exists",
        "for — four further top-level sections carry plant material and not one of",
        "them is named \"plants\". A section can appear twice; felling is honestly both",
        "plant work and structural work.",
        "",
        "**Known limitations for every topic are collected in one place**:",
        f"[Status](#status), line {line_of.get('Status', '?')} — the *last* section in the",
        "file, not the first. Read it before concluding something is broken.",
        "",
        "**Which game a topic belongs to** is the third column. `engine` is shared",
        "by both and is most of the table -- that sharing is the whole argument for",
        "one repository. `outdoor` and `lab` mark topics the other game never builds",
        "a scene that reaches. It is a hint about where your time goes, not a rule",
        "about what you may read.",
        "",
        "| Topic | Game | Sections, primary first |",
        "|---|---|---|",
    ]
    for topic, members in TOPICS.items():
        cells = ", ".join(
            f"[{short_label(m)}](#{anchor(m)}) {line_of[m]}"
            for m in members
            if m in line_of
        )
        out.append(f"| **{topic}** | {GAME[topic]} | {cells} |")

    out += ["", END]
    return "\n".join(out)


def splice(text, block):
    if BEGIN in text:
        return text[: text.index(BEGIN)] + block + text[text.index(END) + len(END) :]
    anchor_at = text.index("\n## ")
    return text[:anchor_at] + "\n" + block + "\n" + text[anchor_at:]


def build(text):
    updated = text
    for _ in range(5):
        candidate = splice(updated, render(list(headings(updated.split("\n")))))
        if candidate == updated:
            return candidate
        updated = candidate
    raise SystemExit("readmetoc: did not converge -- line numbering is unstable")


def main():
    args = set(sys.argv[1:])
    unknown = args - {"--check"}
    if unknown:
        print(f"readmetoc: unrecognised argument(s): {' '.join(sorted(unknown))}")
        print("usage: readmetoc.py [--check]")
        return 2
    with open(DOC, encoding="utf-8", newline="") as fh:
        text = fh.read()

    problems = validate(list(headings(text.split("\n"))))
    if problems:
        print("readmetoc: the topic index in scripts/readmetoc.py is out of step "
              "with README.md:")
        for p in problems:
            print(p)
        return 1

    updated = build(text)
    if "--check" in args:
        if updated != text:
            print("readmetoc: README.md table of contents is stale -- run `python3 scripts/readmetoc.py`")
            return 1
        print("readmetoc: table of contents current")
        return 0
    with open(DOC, "w", encoding="utf-8", newline="\n") as fh:
        fh.write(updated)
    print(f"readmetoc: wrote {len(list(headings(updated.split(chr(10)))))} sections")
    return 0


if __name__ == "__main__":
    sys.exit(main())
