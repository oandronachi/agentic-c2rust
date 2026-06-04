# AI C2Rust Case Study

This repository is a compact case study for agent-assisted C/C++ to Rust
migration work. It shows an agentic workflow that does more than produce a
README: each migration run is parameterized, reproducible, oracle-checked, and
structured around a safe Rust core plus explicit FFI boundaries.

## Job mapping

| Job signal | Repo evidence |
|---|---|
| Agentic workflow | [`playbook/c-to-rust-migration-playbook.md`](./playbook/c-to-rust-migration-playbook.md) is an agent-executable migration procedure. [`runs/`](./runs/) stores the exact prompts and run reports for Claude Code and Codex. |
| C/C++->Rust migration | The core matrix covers C libraries QOI and xxHash; follow-on runs add a stateful C ring buffer and C++ RE2 ownership interop. See [`runs/README.md`](./runs/README.md). |
| Safe Rust core | Each generated workspace has a `*-rs` crate with `#![forbid(unsafe_code)]`, keeping algorithmic logic out of FFI `unsafe`. Example: [`migrations/re2-cpp/codex/re2-cpp-rs-migration/crates/re2-cpp-rs/src/lib.rs`](./migrations/re2-cpp/codex/re2-cpp-rs-migration/crates/re2-cpp-rs/src/lib.rs). |
| FFI boundary design | The `*-sys` crates call the original native code through `cc`/bindgen or a small C++ facade; the `*-cabi` crates expose Rust back to C with cbindgen headers and allocator-symmetric handles/free functions. |
| Oracle-based equivalence | The `*-diff` crates define the equivalence relation: `byte_exact` for codecs/hashers, `model_based` for stateful APIs, and `behavioral` for RE2 literal matching. |
| Fuzzing | Each complete workspace includes `fuzz/fuzz_targets/differential.rs` and `fuzz/fuzz_targets/no_panic.rs`; run reports record execution counts and crash status. |
| Benchmarks | Workspaces include Criterion benches and native-vs-Rust `hyperfine` scripts, for example [`scripts/bench.sh`](./migrations/re2-cpp/codex/re2-cpp-rs-migration/scripts/bench.sh). |
| Formal verification path | RE2 includes a first Kani proof-smoke pass in [`FORMAL_VERIFICATION.md`](./migrations/re2-cpp/codex/re2-cpp-rs-migration/FORMAL_VERIFICATION.md). Future work is deeper Kani/Creusot-style verification over richer safe-core invariants, tracked in [`ROADMAP.md`](./ROADMAP.md). |

## Why it matters

The repository demonstrates a migration workflow that is auditable end to end:
the upstream source is vendored at pinned commits, generated Rust workspaces
contain lockfiles and CI, unsafe code is isolated to FFI crates, and correctness
claims are tied to executable oracles instead of manual inspection. The result is
a practical bridge from existing C/C++ systems code to safe Rust while preserving
native interoperability.
