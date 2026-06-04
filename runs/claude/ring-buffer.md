# Claude Code x Ring Buffer

Stateful C migration target: [AndersKaloer/Ring-Buffer](https://github.com/AndersKaloer/Ring-Buffer), pinned at
`8de8fff55a0baee4fef2a1c6a82d258a27c3c0ba`.

This run adds a real mutable-state example to the playbook: a byte FIFO with
caller-provided C storage, power-of-two wrapping, and overwrite-on-full
semantics. Claude Code migrated it to a safe Rust-owned backing store and checked
it with a model-based oracle.

> The exact instruction the agent received is in [`ring-buffer-task.md`](./ring-buffer-task.md).

## Config block

```yaml
lib_name:        ring-buffer
upstream_url:    https://github.com/AndersKaloer/Ring-Buffer
upstream_pin:    8de8fff55a0baee4fef2a1c6a82d258a27c3c0ba
license:         MIT
headers:         ["ringbuffer.h"]
api_functions:   [
  "ring_buffer_init",
  "ring_buffer_queue",
  "ring_buffer_queue_arr",
  "ring_buffer_dequeue",
  "ring_buffer_dequeue_arr",
  "ring_buffer_peek",
  "ring_buffer_is_empty",
  "ring_buffer_is_full",
  "ring_buffer_num_items"
]
opaque_types:    ["ring_buffer_t"]
allocator:       caller-provided
determinism:     deterministic_behavior
oracle_relation: model_based
```

## Results

| What | Value |
|---|---|
| Tests | 21 passed, 0 failed; includes golden traces, model-based differential tests, and boundary cases |
| Golden/model coverage | 12 recorded golden traces, 392 steps; 2 proptest properties x 2048 cases = 4096 scenarios |
| Fuzzing | 122 s total; **4,314,505 execs**, 0 crashes |
| Whole-process benchmark | Rust `--release` ran at **1.05x +/- 0.35** the C `-O3` reference; identical checksums, near hyperfine's calibration floor |
| Key semantic preserved | Full-buffer enqueue advances tail first, then writes the new byte |
| `unsafe` | 0 in core; FFI-only unsafe in `ring-buffer-sys` and `ring-buffer-cabi`, with documented contracts |

## Representative snippet - overwrite-on-full state transition

```rust
pub fn queue(&mut self, data: u8) {
    if self.is_full() {
        self.tail = (self.tail + 1) & self.mask;
    }
    self.buffer[self.head] = data;
    self.head = (self.head + 1) & self.mask;
}
```

The model-based oracle drives C and Rust through the same generated operation
sequence and compares return values plus observable state after every step.
