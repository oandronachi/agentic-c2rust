# re2-cpp migration summary

This workspace demonstrates safe Rust ownership over a C++ RAII component. The
core Rust API is safe and non-Clone; the C++ `re2::RE2` object stays behind an
opaque handle and is freed only through the matching C++ destructor path.

The migration includes vendored upstream source, a C++ facade, bindgen bindings,
a safe Rust wrapper, a cbindgen C ABI, behavioral property tests, fuzz targets,
benchmark scripts, Kani proof-smoke harnesses, and CI.

## Validation snapshot

Docker validation completed on 2026-06-04:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo metadata --no-deps`
- `cargo test --workspace`
- `bash scripts/check_unsafe.sh`
- `cargo bench -p re2-cpp-rs --bench bench --no-run`
- `bash scripts/bench.sh 10000`
- `cargo +nightly fuzz run differential -- -runs=256`
- `cargo +nightly fuzz run no_panic -- -runs=256`
- `bash scripts/verify_kani.sh`

The benchmark smoke reported the C++ facade run at about 1.42x the Rust wrapper
example for 10,000 iterations in the Docker run. Hyperfine warned that these are
short commands, so treat the result as a smoke signal rather than a stable
performance claim.
