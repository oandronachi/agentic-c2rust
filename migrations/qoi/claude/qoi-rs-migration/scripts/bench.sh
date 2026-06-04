#!/bin/sh
# Whole-process benchmark: reference C (-O3) vs the safe Rust port (--release),
# doing identical work. Uses hyperfine when available; otherwise falls back to a
# simple timed-loop comparison. Also asserts the two print the SAME checksum, which
# confirms process-level equivalence on top of the differential tests.
#
# Usage: scripts/bench.sh [width] [height] [channels] [iters]
set -eu

W="${1:-512}"
H="${2:-512}"
CH="${3:-4}"
ITERS="${4:-50}"

root="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
cd "$root"

vendor="vendor/qoi"
outdir="$(mktemp -d)"
trap 'rm -rf "$outdir"' EXIT
cbin="$outdir/qoi_cbench"

echo ">> building C reference (-O3)"
cc -O3 -I "$vendor" bench/qoi_cbench.c -o "$cbin"

echo ">> building Rust example (--release)"
cargo build --release -p qoi-rs --example bench_bin >/dev/null 2>&1
rbin="target/release/examples/bench_bin"
# Honor CARGO_TARGET_DIR if set (CI / container builds off the mount).
if [ -n "${CARGO_TARGET_DIR:-}" ]; then
    rbin="$CARGO_TARGET_DIR/release/examples/bench_bin"
fi

echo ">> verifying both produce the same checksum"
c_out="$("$cbin" "$W" "$H" "$CH" "$ITERS")"
r_out="$("$rbin" "$W" "$H" "$CH" "$ITERS")"
echo "   C   : $c_out"
echo "   Rust: $r_out"
if [ "$c_out" != "$r_out" ]; then
    echo "FAIL: checksums differ — C and Rust are NOT producing identical output"
    exit 1
fi
echo "   checksums match."

echo ">> timing ($W x $H, $CH ch, $ITERS iters/run)"
if command -v hyperfine >/dev/null 2>&1; then
    hyperfine --warmup 2 --min-runs 5 \
        -n "C (-O3)"      "$cbin $W $H $CH $ITERS" \
        -n "Rust (release)" "$rbin $W $H $CH $ITERS"
else
    echo "   (hyperfine not found — falling back to a simple timed loop)"
    runs=5
    time_bin() {
        # prints elapsed seconds for $runs executions
        start=$(date +%s.%N)
        i=0
        while [ "$i" -lt "$runs" ]; do "$1" "$W" "$H" "$CH" "$ITERS" >/dev/null; i=$((i+1)); done
        end=$(date +%s.%N)
        awk "BEGIN{printf \"%.4f\", ($end-$start)/$runs}"
    }
    ct=$(time_bin "$cbin")
    rt=$(time_bin "$rbin")
    echo "   C (-O3)        : ${ct}s/run (avg of $runs)"
    echo "   Rust (release) : ${rt}s/run (avg of $runs)"
    awk "BEGIN{printf \"   ratio Rust/C  : %.3fx\n\", $rt/$ct}"
fi
