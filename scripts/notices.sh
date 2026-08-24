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
