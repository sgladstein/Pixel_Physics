# Dependency licence audit, and the MIT reversal

**Status: settled, 2026-08-24.** Re-run `bash scripts/licensecheck.sh` after
any change to `Cargo.toml` or `Cargo.lock`; that script is the live check and
this document is the reasoning behind it.

Occasioned by the owner's statement that the engine may ship as a paid game.
Two separate questions fall out of that, and they had different answers.

## 1. The project's own licence was the problem, not the repository's visibility

The question asked was whether a *public* repository is a problem for selling
the game. It is the smaller half. The repository carried an **MIT licence**
(`LICENSE`, added `a729965`, 2026-08-21; `license = "MIT"` in `Cargo.toml`),
and MIT grants every recipient, in its own words, the right to

> use, copy, modify, merge, publish, distribute, sublicense, **and/or sell**
> copies of the Software

so anyone could legally have shipped Pixel Physics as their own product, the
only obligation being to keep a copyright notice in a text file. Visibility
was never the mechanism; the grant was.

**Relicensed to proprietary on 2026-08-24** (this change): `LICENSE` rewritten
as all-rights-reserved, `Cargo.toml` moved to `license-file` plus
`publish = false`, README's licence section rewritten.

Three facts that made the reversal clean, all checked rather than assumed:

- **Sole authorship.** `git shortlog -sne --all` returns exactly one human --
  Scott Gladstein, across three git identities (`sgladstein@gmail.com`, the
  GitHub noreply address, and a lowercase variant). The 421 commits authored
  by `Claude <noreply@anthropic.com>` are AI-generated and do not create a
  second copyright holder. So there was nobody to ask.
- **A three-day window.** MIT was in force from 2026-08-21 to 2026-08-24, on a
  repository with 0 forks, 0 stars and 0 watchers.
- **The ten days before it were safer than the three after.** The repository
  was created 2026-08-11 with no licence file, which is all-rights-reserved by
  default. Adding MIT made the position *worse*, not better.

**What the reversal does not do, and this is the part to not misremember:**
MIT is irrevocable for copies already distributed. Anyone who took a snapshot
between 2026-08-21 and 2026-08-24 keeps MIT rights over that snapshot for
ever. Going private would not have changed this either. `LICENSE` states the
commit range explicitly rather than leaving it to be reconstructed.

**Do not "helpfully" restore MIT.** `Reports/pixel-physics-issues.md` §Issue 10
asked for a LICENSE and `PLAN-log.md` records MIT being chosen deliberately,
over dual MIT/Apache-2.0 and GPL-3.0, on the reasoning that games would be
built on top of this crate as separate binaries. That reasoning is superseded:
the game and the engine ship as one product, sold. Both documents now point
here.

## 2. The dependency tree is clean, and the superset argument makes it easy

`cargo metadata --format-version 1 --locked`, 2026-08-24: **301 third-party
packages, every one of them permissive, none missing an SPDX licence field.**
No GPL, AGPL, MPL, SSPL, CDDL or EPL anywhere.

| Family | Packages offering it |
|---|---|
| MIT | 278 |
| Apache-2.0 | 219 |
| Zlib | 17 |
| BSD-2/3-Clause | 10 |
| Unlicense | 6 |
| ISC | 3 |
| CC0-1.0 | 2 |
| Unicode-3.0 | 1 |

Most crates are multi-licensed, so those columns overlap. The obligations that
are *unavoidable* are the sole-option ones: 65 MIT-only, 10 Apache-2.0-only, 3
ISC, 3 Zlib, 2 BSD-3-Clause (`tiny-skia`, `tiny-skia-path`), 2 CC0-1.0, 1
BSD-2-Clause (`arrayref`), and `dpi` at `Apache-2.0 AND MIT` — both apply.
Every one of these permits closed-source commercial distribution; all they ask
is attribution.

**The one GPL-family token in the tree is `r-efi`** (5.3.0 and 6.0.0), at
`MIT OR Apache-2.0 OR LGPL-2.1-or-later`. It is inapplicable twice over: LGPL
is one of three options and you elect MIT, and the crate is target-gated to
`cfg(all(target_os = "uefi", getrandom_backend = "efi_rng"))` — it reaches the
tree through `getrandom` and cannot be compiled into a Windows, macOS or Linux
build at all.

**Why the platform question does not need answering.** `cargo metadata`
resolves every target, so this inventory is a strict superset of what any real
build links — it counts Windows, Android, WASM and UEFI crates that a given
platform will never see. Because the superset is entirely permissive, every
subset is too, and no per-target `cargo tree` is needed. That shortcut only
works while the answer is clean; the moment `licensecheck.sh` reports a
BLOCKING crate, the next question is whether it is target-gated.

## 3. Attribution: `THIRD-PARTY-NOTICES.txt` (done 2026-08-24)

**Compatibility is not attribution, and the two are easy to conflate.** §2
establishes that nothing in the tree *forbids* shipping a closed-source
commercial binary. What the permissive licences ask in return is that the
copyright notice and licence text travel with that binary — MIT, BSD-2/3,
ISC and Apache-2.0 all require it. A tree can pass `licensecheck.sh` completely
and still ship in breach of every licence in it.

`THIRD-PARTY-NOTICES.txt` at the repo root discharges this: **213 components,
103 licence blocks, 4,631 lines**, generated by `cargo-about` from `Cargo.lock`
via `bash scripts/notices.sh`. Ship it alongside the executable, or render it
from an in-game credits screen; either satisfies the obligation. `--check`
proves the committed copy is not stale.

Three decisions worth not re-litigating:

- **Its target scope is narrower than §2's on purpose.** `about.toml` lists the
  four desktop targets. §2 audits the all-targets superset because a clean
  superset settles every platform at once; a notices file wants the opposite —
  you attribute what you actually distribute, and Android and UEFI copyright
  lines in a desktop game's credits are noise that makes the real entries
  harder to check. 301 packages in the audit, 213 in the notices, and the gap
  is targets, not omissions.
- **The same licence appears many times.** 103 blocks for 8 distinct licences,
  because the notice that must be reproduced is the *copyright holder's*, and
  MIT with Alex Butler's copyright line is not MIT with dtolnay's. The blocks
  cannot be collapsed.
- **The file is committed, not built at package time.** There is no release
  pipeline to hang it off yet, and a notices file that only exists on the
  machine that cut the build is one nobody reviews.

The generator has its own coverage guard, and it exists because of a real
failure during this work: `about.hbs` first used `krate.name`, a field name
from an older `cargo-about`. Handlebars resolves an unknown path to the empty
string rather than erroring, so it rendered a complete-looking 4,396-line file
in which **every "Applies to" bullet was blank** — a notices file naming no
one, which is worse than none at all and would have shipped. `notices.sh` now
cross-checks the crates `cargo-about` resolved against the crates the rendered
file actually names, and both it and `--check` were verified red by injecting
that exact fault.

## 4. What is still owed before the game actually ships

- **Apache-2.0 §4d NOTICE propagation** for the 10 Apache-only crates. The
  licence text is reproduced; if any of those crates ships its own `NOTICE`
  file, its contents must be carried too. `cargo-about` does not collect
  `NOTICE` files, so this is a manual pass over those ten.
- **Wire the notices file into whatever packaging step eventually exists**, so
  the `.txt` lands next to the executable rather than only in the repo.
- **A lawyer.** The MIT-grants-selling-rights reading is plain from the licence
  text, but if money is going to ride on this, the relicensing and the notices
  file both deserve a professional read. Nothing here is legal advice.
