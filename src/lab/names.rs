//! **Names for founding lines, drawn rather than stored.**
//!
//! `OrganismState::lineage` is already a founder identity — claimed once at
//! `World::claim_lineage`, copied to every child, and already carried on
//! `Grave` and `Specimen`'s `Provenance`. A name is therefore a *pure
//! function* of `(the box's own seed, that number)`: nothing on `World`
//! needs to grow to give a lineage a name, and nothing needs to be
//! serialised to keep it, because `line_name(seed, lineage)` reproduces the
//! same word on the next session's fresh build of the same box.
//!
//! **Why a pure function rather than a shared draw.** `rng::stream` is a
//! hash of its four inputs and consumes nothing from any live stream, so
//! naming a thousand lineages cannot move a single cell — see
//! `sim::frame`'s determinism guard, which this module must not touch.
//!
//! **The glyph constraint.** `hud::draw_text` upper-cases and renders
//! anything outside its 5x7 set as a *silent blank* (`hud.rs`'s `has_glyph`).
//! Every stem below is `A-Z` only, and `src/lab/ui.rs`'s
//! `every_string_the_bar_can_draw_is_drawable` walks the whole table
//! directly rather than hoping a fixture happens to found enough lineages to
//! reach the rarer ones.

use crate::sim::rng;

/// A distinct constant folded into every draw this module makes, so a
/// lineage's name and its individual tag cannot collide with any other
/// stream keyed on the same `(seed, lineage)` pair elsewhere in the engine.
const NAME_SALT: u64 = 0x4E41_4D45_5F53_414C; // "NAME_SAL" in ASCII, as a marker rather than a meaningful value

/// **The stems a founding line can be named from.** All uppercase `A-Z`,
/// each at most 7 characters so `NAME-GENERATION` (e.g. `VERNAL-14`) stays
/// short enough for a 42-character log line.
///
/// **Exactly 251 entries, and 251 is prime.** That is not decoration: the
/// permutation `line_name` draws from `STEMS` is a multiplicative one
/// (`index * multiplier + offset`, mod the table length), and that map is
/// only guaranteed a *bijection* — every lineage up to the table size gets a
/// genuinely distinct stem — when the modulus is prime and the multiplier is
/// nonzero mod it. A composite length would need a second check (the
/// multiplier coprime to the length) that a prime makes automatic.
///
/// Four families -- botany, birds, weather and minerals -- so a name reads
/// as a plausible surname rather than a code, the same reasoning
/// `wiki/*.md` gives for describing behaviour in plain language rather than
/// engine terms.
pub const STEMS: [&str; 251] = [
    "VERNAL", "SORREL", "THISTLE", "CLOVER", "ASPEN", "ALDER", "BIRCH", "CEDAR",
    "ELDER", "FERN", "GORSE", "HAZEL", "LARCH", "MAPLE", "NETTLE", "POPLAR",
    "QUINCE", "ROWAN", "SEDGE", "TANSY", "VETCH", "WILLOW", "YARROW", "ZINNIA",
    "BRAMBLE", "BRACKEN", "CATKIN", "DAISY", "FLAXEN", "GINGKO", "HEATHER", "JASMINE",
    "LICHEN", "MYRTLE", "OLIVE", "PANSY", "QUINOA", "TEASEL", "VIOLET", "ACORN",
    "BRIAR", "CLOVE", "DAHLIA", "FENNEL", "HOLLY", "KUDZU", "LOTUS", "MALLOW",
    "ORCHID", "PEONY", "SPRUCE", "YUCCA", "ASTER", "BEECH", "CASSIA", "FICUS",
    "HEMLOCK", "INDIGO", "DOGWOOD", "UMBEL", "KESTREL", "SPARROW", "FALCON", "HARRIER",
    "OSPREY", "MERLIN", "CONDOR", "RAVEN", "CROW", "WREN", "FINCH", "ROBIN",
    "HERON", "EGRET", "STORK", "SWIFT", "SWALLOW", "MARTIN", "PLOVER", "CURLEW",
    "SNIPE", "RAIL", "COOT", "TEAL", "PINTAIL", "MALLARD", "GADWALL", "GANNET",
    "PUFFIN", "PETREL", "SHRIKE", "THRUSH", "MAGPIE", "JACKDAW", "ORIOLE", "WARBLER",
    "VIREO", "PHOEBE", "JUNCO", "TOWHEE", "BUNTING", "PIPIT", "LARK", "KITE",
    "OWL", "HAWK", "EAGLE", "VULTURE", "DUNLIN", "TERN", "GULL", "SKUA",
    "AUK", "MURRE", "KIWI", "EMU", "RHEA", "GREBE", "LOON", "BITTERN",
    "CRANE", "ZEPHYR", "MISTRAL", "MONSOON", "SIROCCO", "SQUALL", "GALE", "BREEZE",
    "TEMPEST", "CYCLONE", "TORNADO", "FROST", "SLEET", "HAIL", "DRIZZLE", "THUNDER",
    "DELUGE", "TYPHOON", "HALCYON", "AURORA", "NIMBUS", "CIRRUS", "CUMULUS", "STRATUS",
    "FOEHN", "CHINOOK", "DOLDRUM", "VORTEX", "ISOBAR", "THERMAL", "UPDRAFT", "VAPOUR",
    "RIME", "GLAZE", "FLURRY", "EQUINOX", "ECLIPSE", "CORONA", "ZENITH", "APOGEE",
    "PERIGEE", "NADIR", "METEOR", "COMET", "NEBULA", "QUASAR", "PULSAR", "BASALT",
    "GRANITE", "MARBLE", "QUARTZ", "GYPSUM", "GALENA", "PYRITE", "MICA", "SLATE",
    "SHALE", "FLINT", "CHALK", "CHERT", "TALC", "OPAL", "JASPER", "AGATE",
    "ONYX", "TOPAZ", "GARNET", "AMBER", "ZIRCON", "CALCITE", "HALITE", "BARITE",
    "COBALT", "NICKEL", "COPPER", "BRONZE", "IRON", "TIN", "LEAD", "ZINC",
    "SILVER", "PEWTER", "BRASS", "STEEL", "BAUXITE", "KAOLIN", "UMBER", "OCHRE",
    "SIENNA", "RUSSET", "COBBLE", "PEBBLE", "BOULDER", "GRAVEL", "GNEISS", "SCHIST",
    "DIORITE", "GABBRO", "PUMICE", "SCORIA", "TUFF", "LOESS", "SILT", "LOAM",
    "PEAT", "MARL", "KARST", "SCREE", "MORAINE", "ESKER", "FJORD", "GEYSER",
    "CRATER", "CALDERA", "MAGMA", "LAVA", "CINDER", "EMBER", "SAGE", "OAK",
    "IVY", "REED", "SUMAC", "ELM", "YEW", "MOSS", "PALM", "FIG",
    "BAY", "LIME", "PINE",
];

/// **A deterministic bijection from `0..STEMS.len()` onto itself, keyed on
/// `seed`.** `STEMS.len()` is prime (see its own doc), so any `multiplier`
/// drawn in `1..len` is automatically coprime to it and
/// `i -> (i * multiplier + offset) mod len` is a permutation: distinct `i`
/// always land on distinct outputs. Computed with two draws from a pure
/// hash rather than a materialised shuffle, so naming a lineage costs no
/// allocation and touches no shared state.
fn permute(seed: u64, i: u32) -> u32 {
    let len = STEMS.len() as u32;
    let mut r = rng::stream(seed, NAME_SALT, 0, 0);
    let multiplier = 1 + r.below(len - 1); // 1..=len-1, never 0 mod a prime len
    let offset = r.below(len);
    (((i as u64) * (multiplier as u64) + offset as u64) % len as u64) as u32
}

/// **A founding line's name.** `0` — nothing descended from a founder, the
/// state a test fixture or an unattributed organism carries — always reads
/// `UNNAMED`, never a stem: a name that changed meaning between "no
/// lineage" and "lineage number equal to some stem's index" would be worse
/// than an obviously synthetic placeholder.
///
/// **Wraps past the table.** A box that somehow founds more than
/// `STEMS.len()` lineages in one run reuses stems with a suffix -- `II`,
/// `III`, then a bare number -- so a name never repeats verbatim within one
/// seed, but the wrap is not the design point: `STEMS.len()` (251) is
/// already far past a single colony click's 52.
pub fn line_name(seed: u64, lineage: u32) -> String {
    if lineage == 0 {
        return "UNNAMED".to_string();
    }
    let len = STEMS.len() as u32;
    let index0 = lineage - 1;
    let cycle = index0 / len;
    let slot = index0 % len;
    let stem = STEMS[permute(seed, slot) as usize];
    match cycle {
        0 => stem.to_string(),
        1 => format!("{stem} II"),
        2 => format!("{stem} III"),
        n => format!("{stem} {}", n + 1),
    }
}

/// **One individual, as `LINE-GENERATION`** -- `VERNAL-14` is the line's
/// name and this individual's own depth in it, so two siblings of one
/// cohort (same line, same generation) read identically; a sentence that
/// needs to tell them apart names the event instead (`ui.rs`'s `Born`
/// sentence says who bore whom by generation, never by slot).
pub fn individual(seed: u64, lineage: u32, generation: u16) -> String {
    // A founder is the only generation-0 member its line will ever have --
    // `claim_lineage` is called once per founder and children copy the
    // number -- so it simply *is* the line: `ASPEN FELLED`, not `ASPEN-0
    // FELLED`, and `THE ASPEN LINE ENDED WITH ITS FOUNDER` names the same
    // animal the same way.
    if generation == 0 {
        return line_name(seed, lineage);
    }
    format!("{}-{}", line_name(seed, lineage), generation)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// **Every stem draws only from `A-Z`.** The 5x7 font silently blanks
    /// anything else (`hud::has_glyph`), and this has shipped three times
    /// already for other strings in this module's neighbourhood
    /// (`ui.rs`'s own glyph guard doc). Folded again into that guard
    /// directly over `names::STEMS`, but kept here too so a bad stem is
    /// caught the moment this file is touched, not only when `ui.rs`'s much
    /// larger test happens to run.
    #[test]
    fn every_stem_is_drawable() {
        for stem in STEMS {
            assert!(stem.chars().all(|c| c.is_ascii_uppercase()), "{stem:?} has a non-A-Z character");
            assert!(stem.len() <= 7, "{stem:?} is {} characters, too long for NAME-GEN", stem.len());
            assert!(!stem.is_empty());
        }
    }

    /// **No two stems are the same word.** A duplicate would not break
    /// `permute` (it is a permutation of *indices*, not of the words behind
    /// them), but it would mean two lineages that got different table slots
    /// still read as the same line -- the exact failure `line_name` exists
    /// to avoid.
    #[test]
    fn no_stem_repeats() {
        let unique: HashSet<&str> = STEMS.iter().copied().collect();
        assert_eq!(unique.len(), STEMS.len(), "the stem table has a duplicate word");
    }

    /// **A name is stable however it is asked for, which is what "stable
    /// across two builds of the same box" actually tests.** `line_name` is
    /// a pure function of its two arguments, so two builds calling it in
    /// different orders (or from different call sites, which is what a
    /// fresh process is) cannot see different words for the same lineage --
    /// unless something is quietly drawing from *shared, advancing* state
    /// instead of a fresh hash per call.
    ///
    /// **Put the fault back**: replace `permute`'s `rng::stream(seed, ...)`
    /// with a draw off one `Rng` built once outside `line_name` and shared
    /// across calls (the shape `world.rng` has). This test goes red the
    /// moment it does, because the descending pass would then read a
    /// different position in that shared stream for every lineage than the
    /// ascending pass did, and the two orders would disagree.
    #[test]
    fn a_line_name_is_stable_across_two_builds_of_the_same_box() {
        let seed = 0xC0FF_EE00_1234_5678;
        let n = 300u32; // past STEMS.len(), so the wrap suffix is exercised too
        let ascending: Vec<String> = (1..=n).map(|l| line_name(seed, l)).collect();
        let descending: Vec<String> = (1..=n).rev().map(|l| line_name(seed, l)).collect();
        for lineage in 1..=n {
            let asc = &ascending[(lineage - 1) as usize];
            let desc = &descending[(n - lineage) as usize];
            assert_eq!(
                asc, desc,
                "lineage {lineage} named differently depending on call order -- \
                 names.rs is reading shared, advancing state rather than a pure hash"
            );
        }
        // And a second, textually identical call is byte-for-byte the same
        // string -- the property a name displayed on two different pages in
        // the same frame actually depends on.
        for lineage in 1..=n {
            assert_eq!(line_name(seed, lineage), line_name(seed, lineage));
        }
    }

    /// **Distinct lineages get distinct names, up to the table size.**
    /// `permute` is asserted to be a bijection by construction (`STEMS.len()`
    /// prime); this is the behavioural check that it actually is one for a
    /// real seed, which a table shrunk to a non-prime size or a multiplier
    /// drawn as `0` would break silently (every lineage naming the first
    /// stem).
    #[test]
    fn distinct_lineages_up_to_the_table_size_get_distinct_names() {
        for seed in [1u64, 0xDEAD_BEEF, 0x1234_5678_9ABC_DEF0] {
            let mut seen = HashSet::new();
            for lineage in 1..=STEMS.len() as u32 {
                let name = line_name(seed, lineage);
                assert!(seen.insert(name.clone()), "seed {seed:#x}: lineage {lineage} collided on {name:?}");
            }
            assert_eq!(seen.len(), STEMS.len(), "seed {seed:#x}: fewer distinct names than lineages");
        }
    }

    /// `lineage == 0` is the one input this function treats as a sentinel
    /// rather than an index, and it must never draw a real stem: a test
    /// fixture's default-zeroed organism would otherwise wear a real line's
    /// name.
    #[test]
    fn lineage_zero_is_always_unnamed() {
        for seed in [0u64, 1, 0xFFFF_FFFF_FFFF_FFFF] {
            assert_eq!(line_name(seed, 0), "UNNAMED");
        }
    }

    /// `individual` is `line_name` plus a plain generation suffix -- no
    /// separate draw, so it cannot disagree with `line_name` about which
    /// line a lineage belongs to.
    #[test]
    fn individual_names_carry_the_lines_own_name() {
        let seed = 42;
        for lineage in 1..=10u32 {
            let line = line_name(seed, lineage);
            for generation in [0u16, 1, 14, 200] {
                assert_eq!(individual(seed, lineage, generation), format!("{line}-{generation}"));
            }
        }
    }
}
