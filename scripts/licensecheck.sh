#!/usr/bin/env bash
# Third-party licence gate. Exit 0 clean, 1 with findings.
#
# Exists because the engine is proprietary and intended to ship as a paid
# game (see LICENSE): a single copyleft crate anywhere in the tree would
# force source disclosure on the whole binary, and the tree is ~300 crates
# deep where nobody reads the additions. Cargo.lock changing is exactly when
# nobody thinks about licences, which is why this is a script and not a
# convention.
#
# Two distinct failures, deliberately separated below:
#   - a crate offering NO permissive option at all -> hard fail;
#   - a crate offering copyleft as one option among several (`MIT OR
#     Apache-2.0 OR LGPL-2.1-or-later`) -> fine, you elect a permissive one.
#     Reported, not failed, because the election is real and free.
#
# The expression parser must RECURSE. A first version split top-level `OR`,
# then split each option on `AND`, then looked the conjuncts up in a set --
# and reported `unicode-ident`'s `(MIT OR Apache-2.0) AND Unicode-3.0` as
# BLOCKING, because the conjunct `(MIT OR Apache-2.0)` is not a leaf and was
# never in the set. A false positive on a permissive crate is the expensive
# direction here: it trains you to ignore the output.
#
# Not wired into CI as a gate, same reasoning as docscheck.sh: run it after
# touching Cargo.toml or Cargo.lock. It needs network on a cold cache --
# `cargo metadata` fetches the index.
set -uo pipefail
cd "$(dirname "$0")/.."

cargo metadata --format-version 1 --locked 2>/dev/null | python3 -c '
import json, re, sys

# Permissive: use, modify and sell a closed-source derivative, attribution
# being the only real obligation. Anything not on this list is unknown until
# a human reads it -- the list is deliberately not a copyleft *denylist*,
# because an unrecognised licence must fail rather than pass by omission.
PERMISSIVE = {
    "MIT", "MIT-0", "Apache-2.0", "Apache-2.0 WITH LLVM-exception",
    "BSD-2-Clause", "BSD-3-Clause", "ISC", "Zlib", "0BSD", "CC0-1.0",
    "Unlicense", "Unicode-3.0", "Unicode-DFS-2016", "BSL-1.0", "NCSA",
}
COPYLEFT = re.compile(r"\b(?:A?GPL|LGPL|MPL|EUPL|SSPL|CDDL|EPL|OSL|CPAL|RPL)", re.I)

def split_top(expr, op):
    """Split on `op` at paren depth 0 only. Returns [expr] if it never fires."""
    depth, cur, out = 0, "", []
    for tok in re.split(r"(\(|\)|\b" + op + r"\b)", expr):
        if tok == "(":
            depth += 1
        elif tok == ")":
            depth -= 1
        if tok == op and depth == 0:
            out.append(cur)
            cur = ""
        else:
            cur += tok
    out.append(cur)
    return [o.strip() for o in out if o.strip()]

def permissive(expr):
    expr = expr.replace("/", " OR ").strip()      # legacy `MIT/Apache-2.0`
    opts = split_top(expr, "OR")
    if len(opts) > 1:                              # any alternative will do
        return any(permissive(o) for o in opts)
    conj = split_top(expr, "AND")
    if len(conj) > 1:                              # every obligation applies
        return all(permissive(c) for c in conj)
    if expr.startswith("(") and expr.endswith(")"):
        return permissive(expr[1:-1])
    return expr in PERMISSIVE

data = json.load(sys.stdin)
pkgs = [p for p in data["packages"] if p["name"] != "pixel-physics"]

missing, blocking, elected = [], [], []
for p in sorted(pkgs, key=lambda p: p["name"]):
    expr = p.get("license")
    ident = p["name"] + " " + p["version"]
    if not expr:
        missing.append((ident, p.get("license_file")))
    elif not permissive(expr):
        blocking.append((ident, expr))
    elif COPYLEFT.search(expr):
        elected.append((ident, expr))

print("licensecheck: %d third-party packages" % len(pkgs))
for ident, lf in missing:
    print("  MISSING  %s: no SPDX license field (license_file=%s) -- read it by hand" % (ident, lf))
for ident, expr in blocking:
    print("  BLOCKING %s: %s -- no permissive option" % (ident, expr))
for ident, expr in elected:
    print("  note     %s: %s -- permissive option elected, copyleft not taken" % (ident, expr))
if not missing and not blocking:
    print("  clean: every package offers a permissive option")
sys.exit(1 if (missing or blocking) else 0)
'
