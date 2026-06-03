# Codex × QOI

Same migration target as [Claude × QOI](../claude/qoi.md), executed by a different agentic CLI (Codex) to test that the playbook is agent-agnostic. Same vendored commit, same crate topology, same `byte_exact` oracle, independently produced.

> The exact instruction the agent received is in [`qoi-task.md`](./qoi-task.md) — identical to the Claude version except for `output_dir: codex`, which is the only thing the playbook needs to know to scaffold the workspace in the right place.

## Config block

```yaml
lib_name:        qoi
upstream_url:    https://github.com/phoboslab/qoi
upstream_pin:    97bacc86a9c4abf5a2d452102dc26546c4c670b9
license:         MIT
headers:         ["qoi.h"]
api_functions:   ["qoi_encode", "qoi_decode"]
opaque_types:    ["qoi_desc"]
allocator:       malloc
determinism:     deterministic_bytes
oracle_relation: byte_exact
```

## Results

| What | Value |
|---|---|
| Tests | 13 passed, 0 failed (6 golden + 2048 proptest + explicit boundaries) |
| Fuzzing | 122 s total; **2,226,427 execs**, 0 crashes |
| Microbench (criterion) | encode/decode 0.74–1.6 GiB/s on configured RGBA cases |
| Whole-process (hyperfine vs C `-O3`) | **C 2.01× faster** (10.1 ms vs 20.4 ms) — not tuned |
| `unsafe` | **0** in core; 2 in `qoi-sys`, 9 in `qoi-cabi` (more per-call annotation style, same boundary) |

## What this tells a C developer evaluating the workflow

Both agents land on the same crate topology and the same equivalence guarantees. They differ on stylistic choices (Codex annotates `unsafe` more granularly) and on raw throughput (Codex's QOI sits below Claude's at 0.5× vs 0.8× C). The workflow doesn't depend on which agent you use; it gives you a reproducible result regardless.
