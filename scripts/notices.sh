#!/usr/bin/env bash
# Regenerate THIRD-PARTY-NOTICES.txt, or (--check) prove the committed one is
# not stale. Exit 0 clean, 1 with findings.
#
# WHY THIS IS A SEPARATE OBLIGATION FROM scripts/licensecheck.sh, because the
# two look like the same job and are not. licensecheck.sh answers "may we ship
# this at all" -- it fails on copyleft. This answers "have we done what the
# permissive licences ask in return", which is attribution: MIT, BSD, ISC and
# Apache all require the copyright notice and licence text to travel with the
# binary. A tree can pass licensecheck.sh completely and still be shipping in
# breach of every licence in it. Compatibility is not attribution.
#
# The generated file is COMMITTED rather than built at package time. Two
# reasons: there is no release pipeline in this repo to hang it off yet, and a
# notices file that only exists on the machine that cut the build is one nobody
# reviews. Committed, `--check` can tell you it went stale.
#
# Scope is the four desktop targets in about.toml, deliberately narrower than
# Reports/dependency-license-audit.md's all-targets superset -- see that file's
# comment. You attribute what you distribute.
set -uo pipefail
cd "$(dirname "$0")/.."

OUT=THIRD-PARTY-NOTICES.txt

if ! command -v cargo-about >/dev/null 2>&1; then
  echo "notices: cargo-about not installed -- cargo install cargo-about --locked --features cli" >&2
  echo "notices: (the --features cli is not optional; without it cargo install builds no binary)" >&2
  exit 1
fi

mode=${1:-generate}
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

# --format json first: it is the only way to learn which crates cargo-about
# actually resolved for the configured targets, which the coverage check below
# needs. Rendering the template cannot tell you about a crate it silently
# dropped.
if ! cargo about generate --format json -o "$tmp/about.json" 2>"$tmp/err"; then
  echo "notices: cargo about --format json failed" >&2; cat "$tmp/err" >&2; exit 1
fi
if ! cargo about generate about.hbs -o "$tmp/notices.txt" 2>>"$tmp/err"; then
  echo "notices: cargo about generate failed" >&2; cat "$tmp/err" >&2; exit 1
fi
[ -s "$tmp/err" ] && { echo "notices: cargo-about warnings:"; cat "$tmp/err"; }

# Coverage: every crate cargo-about resolved must appear in the rendered file.
# This is not paranoia about cargo-about -- it is the guard for the template.
# The first version of about.hbs used `krate.name`, which is the field name
# from an older cargo-about; handlebars resolves an unknown path to the empty
# string rather than erroring, so it rendered a complete-looking file whose
# every "Applies to" bullet was blank. It would have shipped.
python3 - "$tmp/about.json" "$tmp/notices.txt" <<'PY'
import json, re, sys
resolved = {(c["package"]["name"], c["package"]["version"])
            for c in json.load(open(sys.argv[1]))["crates"]}
text = open(sys.argv[2]).read()
attributed = set(re.findall(r"^  \* (\S+) (\S+)$", text, re.M))
missing = sorted(resolved - attributed)
print("notices: %d crates resolved for the configured targets, %d attributed"
      % (len(resolved), len(attributed)))
for name, version in missing:
    print("  MISSING  %s %s: resolved but not in the rendered file" % (name, version))
sys.exit(1 if missing else 0)
PY
cov=$?
[ $cov -ne 0 ] && exit 1

# Apache-2.0 section 4d: if a component ships its own NOTICE file, its contents
# must be carried into any redistribution. cargo-about reproduces licence TEXTS
# and does not collect NOTICE files, so a green notices file does not cover
# this. It was tracked as a manual pass over the Apache-only crates until it
# became obvious that a manual pass is exactly the thing that rots. Automated
# here instead, and over EVERY resolved crate rather than only the Apache-only
# ones: the obligation follows the NOTICE file, so a dual-licensed crate that
# ships one has one.
#
# The section is appended unconditionally, stating "none" when there are none,
# so the rendered file records that the check ran. A section that vanished when
# empty would be indistinguishable from the step having been dropped.
python3 - "$tmp/about.json" "$tmp/notices.txt" <<'NOTICESCAN'
import json, os, sys

crates = json.load(open(sys.argv[1]))["crates"]
found = []
for c in sorted(crates, key=lambda c: (c["package"]["name"], c["package"]["version"])):
    pkg = c["package"]
    root = os.path.dirname(pkg.get("manifest_path") or "")
    if not root or not os.path.isdir(root):
        continue
    for entry in sorted(os.listdir(root)):
        if entry.upper().startswith("NOTICE"):
            path = os.path.join(root, entry)
            if os.path.isfile(path):
                with open(path, encoding="utf-8", errors="replace") as fh:
                    found.append((pkg["name"], pkg["version"], entry, fh.read().rstrip()))

with open(sys.argv[2], "a", encoding="utf-8") as out:
    out.write("\n" + "-" * 80 + "\n")
    out.write("APACHE-2.0 SECTION 4d -- NOTICE FILES\n")
    out.write("-" * 80 + "\n\n")
    if not found:
        out.write("No component in this build ships a NOTICE file, so there is nothing\n"
                  "further to carry. Checked automatically by scripts/notices.sh across\n"
                  "all %d resolved components, every time this file is regenerated.\n"
                  % len(crates))
    else:
        out.write("The following components ship a NOTICE file. Apache-2.0 section 4d\n"
                  "requires these contents to travel with any redistribution.\n\n")
        for name, version, entry, text in found:
            out.write("=== %s %s (%s) ===\n\n%s\n\n" % (name, version, entry, text))
print("notices: NOTICE files found in %d of %d components" % (len(found), len(crates)))
NOTICESCAN
if [ $? -ne 0 ]; then echo "notices: NOTICE scan failed" >&2; exit 1; fi

if [ "$mode" = "--check" ]; then
  if ! diff -q "$tmp/notices.txt" "$OUT" >/dev/null 2>&1; then
    echo "notices: $OUT is STALE -- run 'bash scripts/notices.sh' and commit the result" >&2
    diff "$OUT" "$tmp/notices.txt" | head -40 >&2
    exit 1
  fi
  echo "notices: $OUT is up to date"
else
  cp "$tmp/notices.txt" "$OUT"
  echo "notices: wrote $OUT ($(wc -l < "$OUT") lines)"
fi
