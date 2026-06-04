#!/bin/sh
# Verify the safe core (qoi-rs) contains no `unsafe`.
#
# The real enforcement is `#![forbid(unsafe_code)]`: any unsafe in the core is a
# hard compile error. This script is the belt-and-suspenders check for CI and
# humans — it asserts the attribute is present and greps for any unsafe usage.
set -eu

lib="crates/qoi-rs/src/lib.rs"
core_dir="crates/qoi-rs/src"

if [ ! -f "$lib" ]; then
    echo "FAIL: $lib not found (run from the workspace root)"; exit 2
fi

if ! grep -q '#!\[forbid(unsafe_code)\]' "$lib"; then
    echo "FAIL: $lib is missing #![forbid(unsafe_code)]"; exit 1
fi

# Match real uses of the unsafe keyword (block / fn / impl / trait), not the word
# in comments or in `unsafe_code`.
if grep -rnE 'unsafe[[:space:]]*(\{|fn |impl |trait )' "$core_dir"; then
    echo "FAIL: unsafe usage found in the safe core ($core_dir)"; exit 1
fi

echo "OK: qoi-rs is #![forbid(unsafe_code)] and contains no unsafe blocks."
