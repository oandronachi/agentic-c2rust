# re2-cpp C++ to Rust interop migration

This workspace is a focused C++/Rust interop migration example for
[google/re2](https://github.com/google/re2) at commit
`927f5d53caf8111721e734cf24724686bb745f55`.

The point is ownership, not reimplementing a regex engine. RE2's `RE2` class is
expensive, thread-safe, logically immutable, and explicitly non-copyable and
non-movable. This workspace keeps that C++ RAII object behind an opaque handle,
then exposes a safe Rust wrapper that owns exactly one handle and frees it through
the matching C++ path.

## Deliverables

| Concern | Location |
| --- | --- |
| Original C++ source | `vendor/re2-cpp/` |
| Safe Rust wrapper | `crates/re2-cpp-rs/` |
| C++ RAII facade + bindgen oracle | `crates/re2-cpp-sys/` |
| cbindgen C ABI over Rust wrapper | `crates/re2-cpp-cabi/` |
| Behavioral differential model | `crates/re2-cpp-diff/` |
| Property tests | `crates/re2-cpp-diff/tests/differential.rs` |
| Golden tests | `crates/re2-cpp-rs/tests/golden.rs` |
| Kani proof smoke | `crates/re2-cpp-rs/src/verification.rs`, `scripts/verify_kani.sh` |
| Fuzz targets | `fuzz/fuzz_targets/` |
| Benchmarks | `crates/re2-cpp-rs/benches/bench.rs`, `bench/re2_cpp_cbench.cc`, `scripts/bench.sh` |
| CI | `.github/workflows/ci.yml` |

## Validation relation

`oracle_relation = behavioral`.

For ownership, the oracle is the C++ facade itself: Rust must allocate through
`re2_handle_new`, free through `re2_handle_free`, copy borrowed error strings
before drop, and never expose the C++ object by value.

For matching behavior, property tests compare RE2 against Rust's `regex` crate
over a constrained common syntax subset: escaped ASCII literal patterns and
small UTF-8 text. This does not claim equivalence for all RE2 syntax.

## Common commands

```sh
cargo test --workspace
bash scripts/check_unsafe.sh
bash scripts/verify_kani.sh
cargo bench -p re2-cpp-rs --bench bench
bash scripts/bench.sh
```

The build requires a C++17 compiler, `libre2-dev`, and `libclang-dev`.
