# qoi: C → Rust migration — summary

## What this is
A safe Rust reimplementation of the C library **qoi**
(`https://github.com/phoboslab/qoi` @ `97bacc86a9c4abf5a2d452102dc26546c4c670b9`,
MIT), verified to match the original by a differential oracle. The algorithm is
`#![forbid(unsafe_code)]`; all FFI `unsafe` is isolated in two boundary crates.

## Crates
| Crate | What it is | unsafe |
|---|---|---|
| `qoi-rs`   | Safe core port — **use this** | no |
| `qoi-sys`  | Bindings to the original C (test ground truth) | yes (FFI) |
| `qoi-cabi` | C ABI exposing the Rust port | yes (FFI) |
| `qoi-diff` | Differential tests (Rust vs C) | no |

## Equivalence guarantee
- Relation checked: **byte_exact** over functions qoi_encode, qoi_decode.
- Verified by: 10 golden vectors (produced by the reference C) + a proptest suite
  (9 properties × 2048 cases = 18,432 cases, plus a 6,591-case two-pixel boundary
  grid and explicit run-length/single-pixel cases) + 362,286 fuzz executions
  (122 s total across two targets).
- Result: **all pass — zero mismatches; no port bug was found after the initial
  implementation.**

## Safety
- Core (`qoi-rs`): zero `unsafe`, enforced at compile time by
  `#![forbid(unsafe_code)]`.
- `unsafe` appears only in `qoi-sys` and `qoi-cabi` (FFI): **2** `unsafe` blocks in
  `qoi-sys` (the two `qoi_encode`/`qoi_decode` calls, each copying the C result into
  an owned `Vec` and freeing with the matching `libc::free`) and **3**
  `unsafe extern "C"` functions in `qoi-cabi` (`qoi_rs_encode` / `qoi_rs_decode` /
  `qoi_rs_free`), each with a documented safety contract.

## Prerequisites
- Rust (stable) + Cargo; a C compiler (`cc`); **libclang** for bindgen
  (`sudo apt-get install -y libclang-dev`).
- Optional: nightly + `cargo install cargo-fuzz` (fuzzing); `hyperfine` (benchmarks).

## Build, test, run — manually
```bash
# build everything
cargo build --workspace

# full test suite: unit + golden vectors + differential (Rust vs C)
cargo test --workspace

# core only — needs NO C toolchain / libclang
cargo test -p qoi-rs

# confirm the core has no unsafe
bash scripts/check_unsafe.sh

# fuzz (nightly): runs until Ctrl-C, or cap with -max_total_time
cargo +nightly fuzz run differential -- -max_total_time=60
cargo +nightly fuzz run no_panic     -- -max_total_time=60

# benchmarks: in-process throughput, then C(-O3) vs Rust(--release) head-to-head
cargo bench -p qoi-rs --bench bench
bash scripts/bench.sh
```
What each proves: `cargo test` = correctness vs the reference; `check_unsafe.sh` =
safety boundary intact; `fuzz` = no divergence/panic on hostile input; `bench` =
performance parity.

## Results snapshot
- Tests: **28 passed, 0 failed** (`cargo test --workspace`: qoi-rs 9 unit + 2 golden
  + 1 doctest, qoi-sys 2, qoi-cabi 2, qoi-diff 3 + 9 differential).
- Fuzzing: `differential` 252,947 runs in 61 s (0 crashes); `no_panic` 109,339 runs
  in 61 s (0 crashes). libFuzzer rediscovered the `qoif` magic and still found no
  panic.
- Benchmark: in-process (criterion, 256×256 RGBA) encode 0.75–1.79 GiB/s, decode
  1.1–3.3 GiB/s; whole-process (hyperfine, 256×256×30 iters) the C reference (`-O3`)
  ran **1.26× ± 0.10** faster than the Rust port (`--release`) — i.e. Rust ≈ 0.79× C
  — with **identical checksums** confirming process-level equivalence.

## Known limitations / TODO
- Only the in-memory API is ported (`qoi_encode`/`qoi_decode`); the file-I/O helpers
  `qoi_read`/`qoi_write` are intentionally out of scope (compiled out with
  `QOI_NO_STDIO`) — they are thin `fopen`/`fread` wrappers around the two ported
  functions.
- The 400-million-pixel overflow guard is asserted by a unit test, not exercised by
  fuzzing (bounded sizes avoid OOM); huge-image allocation uses `try_reserve` and
  returns `Error::AllocFailed` rather than aborting.
- Exact upstream commit recorded in `REPRODUCIBILITY.md`
  (`97bacc86a9c4abf5a2d452102dc26546c4c670b9`).

## Provenance
Upstream `https://github.com/phoboslab/qoi` @
`97bacc86a9c4abf5a2d452102dc26546c4c670b9` (MIT), vendored unmodified in
`vendor/qoi/`. Migration performed per the C → Rust migration playbook.

## Cleanup
- **Removed (transient):** host-side scratch logs created during the run, and the
  throwaway shallow upstream clone + scratch logs inside the build container.
- **Kept by your choice (keep-with-consent):** the Docker build container
  `c2rust-dev` (~3.5 GB — a 2.5 GB Cargo build cache + a cargo-installed
  `hyperfine`), so further work on these crates rebuilds fast.
- **Untouched (pre-existing, not created this run):** the `qoi-migrate:dev` and
  `rust:1-slim-bookworm` Docker images.
- The deliverables in `claude/qoi-rs-migration/` are intact; nothing pre-existing
  was deleted.
