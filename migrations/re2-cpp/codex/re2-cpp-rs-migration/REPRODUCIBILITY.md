# Reproducibility

## Pins

- Library: `re2-cpp`
- Upstream URL: `https://github.com/google/re2`
- Upstream commit: `927f5d53caf8111721e734cf24724686bb745f55`
- Upstream tag note: `2025-11-05`
- Upstream license: BSD-3-Clause, vendored in `vendor/re2-cpp/LICENSE`
- Rust toolchain: `1.85.1`, pinned in `rust-toolchain.toml`
- C++ standard: C++17
- System C++ dependency for validation: `libre2-dev`

## Vendored source hashes

| File | SHA-256 |
| --- | --- |
| `vendor/re2-cpp/LICENSE` | `6040CDA75D90B1738292A631D89934C411EF7FFD543C4D6A1B7EDFC8EDF29449` |
| `vendor/re2-cpp/re2/re2.h` | `59F4D4B5318FCB6ACF0C90F17D17E71903A87EC25D75FCF5D13E738BEB29490F` |

## Environment probe

Host probe on 2026-06-04:

- `rustc`: not installed on PATH
- `cargo`: not installed on PATH

Validation mode is Docker-based full validation.

## Cleanup ledger

No files are intentionally created outside this workspace. Docker image layers,
apt caches, Rust toolchains, and Cargo registry caches may be created by
validation commands outside the workspace; they are not part of this artifact.
