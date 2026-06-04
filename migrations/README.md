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
