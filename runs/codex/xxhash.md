# Codex × xxHash

Same target as [Claude × xxHash](../claude/xxhash.md): one-shot `XXH32` / `XXH64`, BSD-2-Clause, pure scalar return.

> The exact instruction the agent received is in [`xxhash-task.md`](./xxhash-task.md) — identical to the Claude version except for `output_dir: codex`.

## Config block

```yaml
lib_name:        xxhash
upstream_url:    https://github.com/Cyan4973/xxHash
upstream_pin:    v0.8.3                   # = e626a72bc2321cd320e953a0ccf1584cad60f363
license:         BSD-2-Clause              # NOTE: xxhsum CLI is GPL — do NOT vendor
headers:         ["xxhash.h"]
api_functions:   ["XXH32", "XXH64"]
opaque_types:    []
allocator:       caller-provided
determinism:     deterministic_bytes
oracle_relation: byte_exact
```

## Results

| What | Value |
|---|---|
| Tests | 6 passed, 0 failed (12 golden + 2048 proptest) |
| Fuzzing | 122 s total; **52,675,307 execs**, 0 crashes |
| Microbench (criterion, 4 KiB) | XXH64 ≈ 15.8 GiB/s, XXH32 ≈ 8.0 GiB/s |
| Whole-process (hyperfine vs C `-O3`) | **C 1.02× faster** (89.8 ms vs 91.7 ms; within noise) |
| `unsafe` | **0** in core; 2 in `xxhash-sys`, 12 in `xxhash-cabi` (same boundary, finer-grained blocks) |

## Note

Codex hit near-parity on xxHash without the explicit `chunks_exact` rewrite that Claude used, because the simpler hashing inner loop is already well within the optimizer's bounds-check elision range. Both runs end at the same equivalence guarantee; the workflow doesn't care.
