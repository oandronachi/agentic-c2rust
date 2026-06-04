#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

W=${W:-256}
H=${H:-256}
CH=${CH:-4}
ITERS=${ITERS:-20}
MODE=${MODE:-encode}
ARGS="$MODE $W $H $CH $ITERS"

cc -O3 -I vendor/qoi bench/qoi_cbench.c -o "${TMPDIR:-/tmp}/qoi_cbench"
cargo build --release -p qoi-rs --example bench_bin >/dev/null

CBIN="${TMPDIR:-/tmp}/qoi_cbench"
RBIN="target/release/examples/bench_bin"
c_out="$("$CBIN" $ARGS)"
r_out="$("$RBIN" $ARGS)"
echo "C   : $c_out"
echo "Rust: $r_out"
test "$c_out" = "$r_out"

if command -v hyperfine >/dev/null 2>&1; then
    hyperfine --warmup 3 -n "C (-O3)" "$CBIN $ARGS" -n "Rust (--release)" "$RBIN $ARGS"
else
    "$CBIN" $ARGS
    "$RBIN" $ARGS
fi
