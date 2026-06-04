# qoi-rs-migration

Safe Rust migration of the QOI single-header C library, with the vendored C
implementation kept as the differential oracle.

## Provenance

- Upstream: https://github.com/phoboslab/qoi
- Pinned commit: `97bacc86a9c4abf5a2d452102dc26546c4c670b9`
- Vendored files: `vendor/qoi/qoi.h`, `vendor/qoi/LICENSE`
- License: MIT
- Ported API: `qoi_encode`, `qoi_decode`; stdio helpers are intentionally excluded.

## Crates

| Path | Purpose |
|---|---|
| `crates/qoi-rs` | Safe, dependency-free Rust encoder/decoder. |
| `crates/qoi-sys` | Bindgen/cc wrapper over vendored `qoi.h`; used as ground truth. |
| `crates/qoi-cabi` | C ABI exposing the Rust implementation plus `qoi_rs_free`. |
| `crates/qoi-diff` | Proptest differential oracle shared with fuzzing. |

## Validation Snapshot

- `cargo test --workspace`: passed in Docker, 13 Rust tests, 0 failures.
- `qoi-rs` golden vectors: 6 reference-generated cases.
- Differential tests: 2048 proptest cases plus explicit boundary tests.
- Fuzzing: `differential` ran 2,162,710 cases in 61 seconds; `no_panic` ran
  63,717 cases in 62 seconds; no crashes.
- Unsafe gate: `scripts/check_unsafe.sh` passed. The safe crates forbid unsafe.
- Benchmarks: Criterion and `scripts/bench.sh` ran. The process benchmark measured
  C `-O3` at 10.1 ms mean and Rust `--release` at 20.4 ms mean for the configured
  encode workload, so C was 2.01x faster in that run.

## Common Commands

```bash
cargo build --workspace
cargo test --workspace
cargo test -p qoi-rs
bash scripts/check_unsafe.sh
cargo +nightly fuzz run differential -- -max_total_time=60
cargo +nightly fuzz run no_panic -- -max_total_time=60
cargo bench -p qoi-rs --bench bench
bash scripts/bench.sh
```

See `NOTES.md` for the API and edge-case analysis, `REPRODUCIBILITY.md` for pins
and local validation details, and `SUMMARY.md` for the handoff summary.
