#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ITERATIONS="${1:-1000000}"
CXX_BIN="${CXX:-c++}"
mkdir -p "$ROOT/target/bench"

"$CXX_BIN" -O3 -std=c++17 -Wall -Wextra \
  "$ROOT/bench/re2_cpp_cbench.cc" \
  $(pkg-config --cflags --libs re2 2>/dev/null || printf '%s' '-lre2') \
  -o "$ROOT/target/bench/re2_cpp_cbench"

cargo build --release -p re2-cpp-rs --example bench_bin

CXX_CMD="$ROOT/target/bench/re2_cpp_cbench $ITERATIONS"
RUST_CMD="$ROOT/target/release/examples/bench_bin $ITERATIONS"

if command -v hyperfine >/dev/null 2>&1; then
  hyperfine --warmup 3 "$CXX_CMD" "$RUST_CMD"
else
  "$ROOT/target/bench/re2_cpp_cbench" "$ITERATIONS"
  "$ROOT/target/release/examples/bench_bin" "$ITERATIONS"
fi
