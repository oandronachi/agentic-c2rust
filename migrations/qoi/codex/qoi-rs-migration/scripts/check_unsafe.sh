#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

for crate in qoi-rs qoi-diff; do
    if ! grep -q '#!\[forbid(unsafe_code)\]' "crates/$crate/src/lib.rs"; then
        echo "missing forbid(unsafe_code) in $crate" >&2
        exit 1
    fi
done

if grep -RIn '\bunsafe\b' crates/qoi-rs/src crates/qoi-diff/src; then
    echo "unexpected unsafe in safe crates" >&2
    exit 1
fi

echo "qoi-rs and qoi-diff forbid unsafe_code"
echo "FFI unsafe locations:"
grep -RIn '\bunsafe\b' crates/qoi-sys/src crates/qoi-cabi/src || true
