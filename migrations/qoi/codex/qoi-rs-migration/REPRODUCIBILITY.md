# QOI Reproducibility

## Pins

- Upstream repository: https://github.com/phoboslab/qoi
- Upstream commit: `97bacc86a9c4abf5a2d452102dc26546c4c670b9`
- Rust toolchain: `1.96.0` via `rust-toolchain.toml`
- Fuzzing toolchain: nightly, used only for `cargo fuzz`
- C flags: `-O3` in `qoi-sys/build.rs` and `scripts/bench.sh`
- Vendored C source: `vendor/qoi/qoi.h`, copied verbatim from the pinned commit

## Environment Used

The Windows host did not have `rustc`, `cargo`, `cc`, `clang`, `cargo fuzz`, or
`hyperfine` on `PATH`, so full validation was run in disposable Docker containers
based on `gcc:13-bookworm`.

Container packages installed during validation:

- `ca-certificates`
- `curl`
- `clang`
- `libclang-dev`
- `pkg-config`
- `hyperfine`

Rust was installed inside the disposable container with rustup. `cargo-fuzz` was
installed inside the container for the fuzzing phase.

## Commands Actually Run

```bash
cargo run -p qoi-sys --example gen_golden
cargo test --workspace
bash scripts/check_unsafe.sh
cargo +nightly fuzz run differential -- -max_total_time=60
cargo +nightly fuzz run no_panic -- -max_total_time=60
cargo bench -p qoi-rs --bench bench -- --sample-size 10 --warm-up-time 1 --measurement-time 1
W=256 H=256 CH=4 ITERS=20 MODE=encode bash scripts/bench.sh
```

## Cleanup

Generated build output and fuzz corpora created by validation were removed:
`target/`, `fuzz/target/`, `fuzz/artifacts/`, and `fuzz/corpus/`.

No named Docker containers or volumes were created. Docker was run with `--rm`, so
containers were removed automatically. The shared Docker image/cache was not
removed because it may have existed before this task and is outside the deliverable
workspace.
