# Reproducibility manifest — qoi-rs-migration

Everything needed to reproduce this migration and its verification, byte for byte.

## Upstream source (vendored)
- Repo: <https://github.com/phoboslab/qoi>
- Commit (pinned): **`97bacc86a9c4abf5a2d452102dc26546c4c670b9`** (fetched from
  `master`, 2026-06-03).
- Files vendored verbatim: `vendor/qoi/qoi.h`, `vendor/qoi/LICENSE` (MIT).
- Build defines for the reference: `QOI_IMPLEMENTATION`, `QOI_NO_STDIO`.

## Toolchains
- Rust **stable 1.96.0** (`ac68faa20`, 2026-05-25) — workspace build/test/bench.
  Pinned via `rust-toolchain.toml` (`channel = "stable"`).
- Rust **nightly 1.98.0-nightly** (`d595fce01`, 2026-06-02) — fuzzing only
  (`cargo +nightly fuzz`).
- C compiler: **gcc 12.2.0** (Debian 12.2.0-14+deb12u1); **clang/libclang 14.0.6**
  for bindgen.
- `cargo-fuzz` **0.13.1**; `hyperfine` **1.20.0**.

## C compile flags
- Reference C compiled at **`-O3`** (via the `cc` crate, `opt_level(3)`,
  `warnings(false)`), as a static lib `qoi_reference`.
- Benchmark C (`bench/qoi_cbench.c`) compiled at **`-O3`** (`cc -O3 -I vendor/qoi`).
- Rust benchmark subject built `--release` (workspace `[profile.release]`:
  `opt-level=3, lto=true, codegen-units=1`).

## Pinned crate versions (see `Cargo.lock`, committed)
| Crate | Version | Where |
|---|---|---|
| `libc` | 0.2.186 | qoi-sys (FFI free) |
| `cc` | 1.2.63 | qoi-sys build |
| `bindgen` | 0.70.1 | qoi-sys build |
| `cbindgen` | 0.27.0 | qoi-cabi build |
| `proptest` | 1.11.0 | qoi-diff tests |
| `criterion` | 0.5.1 | qoi-rs benches |
| `libfuzzer-sys` | 0.4.12 | fuzz/ |
| `arbitrary` | 1.4.2 | fuzz/ (transitive) |

`Cargo.lock` is committed at the workspace root; `fuzz/` has its own lock.

## Build environment
Built and tested inside Docker (Linux, `x86_64-unknown-linux-gnu`) on a Windows 11
host — image derived from `rust:1-slim-bookworm` plus `clang`, `libclang-dev`,
nightly, and `cargo-fuzz` (`LIBCLANG_PATH=/usr/lib/x86_64-linux-gnu`). Build cache
kept off the bind mount via `CARGO_TARGET_DIR`.

## Agent provenance
Migration performed by an autonomous agent (**Claude Code**, model
`claude-opus-4-8`) following the C → Rust migration playbook
(`qoi-c-to-rust-migration-task-claude.md`), 2026-06-03.

## Reproduce the verification
```sh
cargo test --workspace                 # 28 tests: unit + golden + differential
sh scripts/check_unsafe.sh             # core is #![forbid(unsafe_code)]
cargo +nightly fuzz run differential -- -max_total_time=60
cargo +nightly fuzz run no_panic     -- -max_total_time=60
cargo bench -p qoi-rs --bench bench    # criterion throughput
sh scripts/bench.sh 256 256 4 30       # C(-O3) vs Rust(--release), checksums must match
```
