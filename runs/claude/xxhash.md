# Claude Code × xxHash

Migrating the one-shot [Cyan4973/xxHash](https://github.com/Cyan4973/xxHash) APIs `XXH32` / `XXH64` (BSD-2-Clause, ~250 LOC) from C to safe Rust. Pure functions returning a scalar by value — nothing crosses the FFI boundary by pointer, so the run also doubles as a "minimum-friction" example of the workflow.

> The exact instruction the agent received is in [`xxhash-task.md`](./xxhash-task.md) — it is this repo's [playbook](../../playbook/c-to-rust-migration-playbook.md) with the config block below filled in.

## Config block

```yaml
lib_name:        xxhash
upstream_url:    https://github.com/Cyan4973/xxHash
upstream_pin:    v0.8.3                   # = e626a72bc2321cd320e953a0ccf1584cad60f363
license:         BSD-2-Clause              # NOTE: xxhsum CLI is GPL — do NOT vendor it
headers:         ["xxhash.h"]
api_functions:   ["XXH32", "XXH64"]
opaque_types:    []                        # one-shot has no opaque handles
allocator:       caller-provided           # returns u32/u64 by value; no heap
determinism:     deterministic_bytes
oracle_relation: byte_exact                # identical hash value for same input + seed
```

## Results

| What | Value |
|---|---|
| Tests | 16 passed, 0 failed (canonical vectors + 75 golden + 3 properties × 2048 + 130×5 boundary sweep) |
| Fuzzing | 122 s total; **18,711,247 execs**, 0 crashes |
| Microbench (criterion, 64 KiB) | XXH64 ≈ 15.9 GiB/s, XXH32 ≈ 7.9 GiB/s |
| Whole-process (hyperfine vs C `-O3`) | **Parity** (1.00× ± 0.05); identical checksums |
| `unsafe` | **0** in core; 2 in `xxhash-sys`, 2 in `xxhash-cabi`; no `*_free` (scalar return) |

## Representative snippet — reaching parity in safe Rust

A naïve bounds-checked loop ran ~4.7× slower than the C reference. Switching to `chunks_exact` + fixed-size arrays lets the optimizer elide the bounds checks **without** `unsafe` — and the parity number above is what you get. This is the headline lesson for a C developer learning Rust: the safe idioms are often fast enough on their own.

```rust
// Hot loop: 16-byte stripes for XXH64. No unsafe, no get_unchecked.
let mut iter = input.chunks_exact(32);
for chunk in &mut iter {
    let block: &[u8; 32] = chunk.try_into().unwrap();
    v1 = round(v1, u64::from_le_bytes(block[ 0..8 ].try_into().unwrap()));
    v2 = round(v2, u64::from_le_bytes(block[ 8..16].try_into().unwrap()));
    v3 = round(v3, u64::from_le_bytes(block[16..24].try_into().unwrap()));
    v4 = round(v4, u64::from_le_bytes(block[24..32].try_into().unwrap()));
}
let tail = iter.remainder();
```
