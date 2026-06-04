# Complete migrated workspaces

This directory contains checked-in migration workspaces, not just reports.

## QOI

| Agent | Workspace |
|---|---|
| Claude Code | [`qoi/claude/qoi-rs-migration`](./qoi/claude/qoi-rs-migration/) |
| Codex | [`qoi/codex/qoi-rs-migration`](./qoi/codex/qoi-rs-migration/) |

Each QOI workspace includes the original vendored C source, safe Rust core crate,
bindgen-backed C oracle, cbindgen C ABI crate, property tests, cargo-fuzz targets,
benchmark scripts, generated golden vectors, lockfiles, and CI workflow.

## RE2 C++ interop

| Agent | Workspace |
|---|---|
| Codex | [`re2-cpp/codex/re2-cpp-rs-migration`](./re2-cpp/codex/re2-cpp-rs-migration/) |

The RE2 workspace includes the pinned vendored C++ source, a C++ RAII facade,
bindgen-backed oracle crate, safe Rust ownership wrapper, cbindgen C ABI crate,
behavioral property tests, cargo-fuzz targets, benchmark scripts, lockfiles, CI,
and Kani proof-smoke harnesses documented in `FORMAL_VERIFICATION.md`.
