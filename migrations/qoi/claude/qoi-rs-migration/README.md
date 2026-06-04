# qoi-rs-migration

A safe Rust reimplementation of the **QOI** ("Quite OK Image") codec, migrated from
the reference C and **proven byte-exact** against it by a differential oracle.

- Upstream: <https://github.com/phoboslab/qoi> @ `97bacc86a9c4abf5a2d452102dc26546c4c670b9` (MIT)
- The algorithm crate is `#![forbid(unsafe_code)]`; all FFI `unsafe` is isolated in
  two boundary crates.
- Migration performed per the C → Rust migration playbook by an autonomous agent
  (Claude Code, `claude-opus-4-8`).

## Deliverable map

| Path | What it is |
|---|---|
| [`crates/qoi-rs/`](crates/qoi-rs) | **Safe core port — use this.** `#![forbid(unsafe_code)]`, zero runtime deps, typed errors, total on bad input. `encode` / `decode`. |
| [`crates/qoi-sys/`](crates/qoi-sys) | Inbound FFI: `cc` + `bindgen` over the vendored C — the differential oracle's *ground truth*. Allocator-symmetric (`libc::free`). |
| [`crates/qoi-cabi/`](crates/qoi-cabi) | Outbound FFI: a stable C ABI exposing the safe port (`qoi_rs_encode/decode/free`), `#[repr(C)]`, cbindgen header at `crates/qoi-cabi/include/qoi_rs.h`. |
| [`crates/qoi-diff/`](crates/qoi-diff) | Differential oracle: shared `Image` model + `check_*` (proptest). Plus the golden-vector generator (`examples/gen_golden.rs`). |
| [`fuzz/`](fuzz) | `cargo-fuzz` targets: `differential` (vs C) and `no_panic` (totality). Own workspace, nightly. |
| [`vendor/qoi/`](vendor/qoi) | Upstream `qoi.h` + `LICENSE`, verbatim, at the pinned commit. |
| [`bench/qoi_cbench.c`](bench/qoi_cbench.c), [`crates/qoi-rs/examples/bench_bin.rs`](crates/qoi-rs/examples/bench_bin.rs) | hyperfine subjects (C `-O3` vs Rust `--release`), identical work + checksum. |
| [`scripts/`](scripts) | `check_unsafe.sh` (unsafe boundary), `bench.sh` (whole-process race). |
| [`NOTES.md`](NOTES.md) | Phase 1: API, ownership table, edge cases, oracle choice. |
| [`REPRODUCIBILITY.md`](REPRODUCIBILITY.md) | Pinned commit, toolchains, C flags, crate versions. |
| [`SUMMARY.md`](SUMMARY.md) | Plain-language summary + copy-paste run/test guide. |
| [`.github/workflows/ci.yml`](.github/workflows/ci.yml) | CI: build/test, unsafe-gate, fuzz-smoke, bench, fmt+clippy. |

## Equivalence — how it's proven
The relation is **`byte_exact`**: for every valid image `rs::encode == c::encode`
byte for byte, and for every stream `rs::decode == c::decode` (accept/reject, the
filled `Desc`, and the pixels). Verified by 10 checked-in golden vectors (produced
by the C), a proptest suite (9 properties × 2048 cases + a 6591-case boundary grid),
and two fuzz targets run >60 s each with no divergence or panic.

## Quick start
```sh
cargo test --workspace        # unit + golden + differential (needs libclang for qoi-sys)
cargo test -p qoi-rs          # core only — no C toolchain needed
sh scripts/check_unsafe.sh    # confirm the core has no unsafe
```
See [`SUMMARY.md`](SUMMARY.md) for the full build / test / fuzz / bench guide and the
results snapshot.

## Provenance & license
Upstream QOI is vendored unmodified in `vendor/qoi/` at the pinned commit (MIT). This
project is MIT-licensed ([`LICENSE`](LICENSE)); the QOI format and reference are
© 2021 Dominic Szablewski.
