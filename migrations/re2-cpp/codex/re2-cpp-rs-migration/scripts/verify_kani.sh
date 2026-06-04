#!/usr/bin/env bash
set -euo pipefail

CARGO_KANI="${CARGO_KANI:-cargo +stable kani}"

if ! $CARGO_KANI --version >/dev/null 2>&1; then
  echo "cargo-kani is not installed. Install with a current stable toolchain: cargo +stable install kani-verifier && cargo +stable kani setup" >&2
  exit 127
fi

$CARGO_KANI -p re2-cpp-rs --no-default-features
