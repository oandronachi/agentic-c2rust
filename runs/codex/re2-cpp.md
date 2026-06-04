# Codex x RE2 C++ Interop

C++ interop migration target: [google/re2](https://github.com/google/re2),
pinned at `927f5d53caf8111721e734cf24724686bb745f55` (`2025-11-05` tag note).

This run extends the C workflow to a small C++ ownership facade. The migrated
Rust API owns a non-cloneable regex handle, while the actual C++ `re2::RE2`
object remains behind a C ABI facade and is destroyed through the matching C++
destructor path.

> The exact instruction the agent received is in [`re2-cpp-task.md`](./re2-cpp-task.md).

## Config block

```yaml
lib_name:        re2-cpp
upstream_url:    https://github.com/google/re2
upstream_pin:    927f5d53caf8111721e734cf24724686bb745f55
license:         BSD-3-Clause
headers:         ["re2/re2.h", "re2_handle.h"]
api_functions:   [
  "re2_handle_new",
  "re2_handle_free",
  "re2_handle_ok",
  "re2_handle_error",
  "re2_handle_partial_match"
]
opaque_types:    ["Re2Handle", "re2::RE2"]
allocator:       custom
determinism:     deterministic_behavior
oracle_relation: behavioral
formal_verification: kani
proof_scope:     core
proof_harnesses: ["regex_handle_preserves_lifetime_invariants", "partial_match_no_panic"]
proof_bounds:    "max_pattern_len=32, max_text_len=128, unwind=8"
```

## Results

| What | Value |
|---|---|
| Migration shape | Safe Rust ownership wrapper over an opaque C++ RAII component |
| Included artifacts | Vendored RE2 source, C++ facade, bindgen sys crate, safe Rust core, cbindgen ABI, property tests, fuzz targets, benchmark scripts, Kani harnesses, CI |
| Behavioral oracle | Escaped ASCII literal patterns compared between RE2 and Rust `regex` over bounded text |
| Docker validation | `fmt`, `clippy`, `metadata`, workspace tests, unsafe-boundary check, bench build/smoke, fuzz smoke, and Kani all completed on 2026-06-04 |
| Fuzzing | `differential` and `no_panic` smoke targets, 256 runs each |
| Kani | `kani-verifier v0.67.0`; 2 harnesses verified, 0 failures |
| Bench smoke | C++ facade run reported about 1.42x faster than the Rust wrapper example for 10,000 iterations; hyperfine warned the commands were short |
| `unsafe` | 0 in the core and diff crates; FFI boundary only in `re2-cpp-sys` and `re2-cpp-cabi` |

## Representative snippet - C++ RAII facade

```cpp
struct Re2Handle {
  std::unique_ptr<re2::RE2> re;
  std::string error;
};

void re2_handle_free(Re2Handle *handle) { delete handle; }
```

The Rust core never owns `re2::RE2` by value. It owns one opaque handle and drops
it through `re2_handle_free`, so the C++ destructor path remains authoritative.
