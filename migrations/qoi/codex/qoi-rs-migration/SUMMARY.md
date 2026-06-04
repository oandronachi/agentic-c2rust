# QOI: C to Rust migration summary

## What This Is

A safe Rust reimplementation of the C library `qoi`
(https://github.com/phoboslab/qoi at
`97bacc86a9c4abf5a2d452102dc26546c4c670b9`, MIT), verified to match the original
by a differential oracle. The algorithm crate is `#![forbid(unsafe_code)]`; FFI
unsafe is isolated in boundary crates.

## Crates

| Crate | What it is | unsafe |
|---|---|---|
| `qoi-rs` | Safe core port; use this crate. | no |
| `qoi-sys` | Bindings to the original C reference. | yes, FFI only |
| `qoi-cabi` | C ABI exposing the Rust port. | yes, FFI only |
| `qoi-diff` | Differential tests, Rust vs C. | no |

## Equivalence Guarantee

- Relation checked: byte-exact over `qoi_encode` and `qoi_decode`.
- Verified by: 6 golden vectors, 2048 proptest cases, explicit boundary tests, and
  two 60-second fuzz runs.
- Result: all validation passed; no mismatches or crash artifacts were found.

## Safety

- Core `qoi-rs`: zero unsafe, enforced by `#![forbid(unsafe_code)]`.
- `qoi-diff`: zero unsafe, enforced by `#![forbid(unsafe_code)]`.
- FFI unsafe is in `qoi-sys` and `qoi-cabi`: 2 inbound FFI unsafe blocks, 3 unsafe
  extern entrypoints, and 6 production C ABI unsafe blocks. Each production unsafe
  block has a local safety contract or a documented ABI precondition.
- `scripts/check_unsafe.sh` passed.

## Prerequisites

- Rust stable and Cargo.
- A C compiler and libclang for bindgen.
- Optional: nightly plus `cargo-fuzz` for fuzzing; `hyperfine` for benchmarks.

## Build, Test, Run Manually

```bash
cargo build --workspace
cargo test --workspace
cargo test -p qoi-rs
bash scripts/check_unsafe.sh
cargo +nightly fuzz run differential -- -max_total_time=60
cargo +nightly fuzz run no_panic -- -max_total_time=60
cargo bench -p qoi-rs
bash scripts/bench.sh
```

`cargo test` checks correctness against the reference and checked-in golden data.
`check_unsafe.sh` checks the safety boundary. `cargo fuzz` searches for divergence
and panics on hostile input. `bench` and `scripts/bench.sh` measure performance.

## Results Snapshot

- Tests: 13 passed, 0 failed across the workspace.
- Fuzzing: `differential` ran 2,162,710 cases in 61 seconds; `no_panic` ran 63,717
  cases in 62 seconds; no crashes.
- Criterion: encode/decode throughput ranged from about 743 MiB/s to 1.6 GiB/s on
  the configured RGBA cases.
- Process benchmark: C `-O3` mean 10.1 ms, Rust `--release` mean 20.4 ms; C was
  2.01x faster for that workload.

## Known Limitations

- `qoi_read` and `qoi_write` are not ported; this migration covers codec functions,
  not file I/O helpers.
- No SIMD-specific optimization pass has been done.
- The host did not have a native Rust/C toolchain; full validation was performed in
  disposable Docker containers.

## Cleanup

Generated build outputs and fuzz corpora were removed from the workspace. No named
Docker containers or volumes were created; the shared Docker image/cache was left
untouched.

## Provenance

Upstream `qoi` from https://github.com/phoboslab/qoi at
`97bacc86a9c4abf5a2d452102dc26546c4c670b9` is vendored unmodified in
`vendor/qoi/`. Migration performed per the C to Rust migration playbook.
