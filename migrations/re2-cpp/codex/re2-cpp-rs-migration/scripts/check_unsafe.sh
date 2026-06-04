#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if grep -R -n -E 'unsafe[[:space:]]*(\{|fn|extern|impl|trait)' \
  "$ROOT/crates/re2-cpp-rs/src" \
  "$ROOT/crates/re2-cpp-rs/tests" \
  "$ROOT/crates/re2-cpp-diff/src" \
  "$ROOT/crates/re2-cpp-diff/tests"; then
  echo "unsafe Rust escaped the FFI boundary" >&2
  exit 1
fi

cargo check -p re2-cpp-rs -p re2-cpp-diff
