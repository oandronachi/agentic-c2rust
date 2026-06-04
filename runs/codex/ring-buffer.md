# Codex x Ring Buffer

Stateful C migration target: [AndersKaloer/Ring-Buffer](https://github.com/AndersKaloer/Ring-Buffer), pinned at
`8de8fff55a0baee4fef2a1c6a82d258a27c3c0ba`.

This run extends the original byte-transform examples with mutable state. The
Rust core owns its backing storage safely, while the inbound C oracle preserves
the original caller-provided storage model.

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
| Migration shape | Stateful C API with caller-provided storage migrated to safe Rust-owned storage |
| Oracle | Model-based: drive C and Rust with the same operation sequence, compare observable state after every step |
| Included artifacts | Original C code, safe Rust core, bindgen oracle, cbindgen ABI, property tests, fuzz targets, benchmark scripts, CI |
| Key semantic preserved | Full-buffer enqueue overwrites the oldest byte by advancing the tail before writing |
| Capacity rule | Usable capacity is always `storage_len - 1`, matching the original power-of-two mask design |
| `unsafe` | 0 in the core crate; FFI boundary only in `ring-buffer-sys` and `ring-buffer-cabi` |

## Representative snippet - overwrite-on-full state transition

```rust
pub fn queue(&mut self, data: u8) {
    if self.is_full() {
        self.tail_index = self.wrap(self.tail_index + 1);
    }

    self.storage[self.head_index] = data;
    self.head_index = self.wrap(self.head_index + 1);
}
```

That is the stateful behavior the model-based oracle checks against the C
implementation after generated enqueue, dequeue, peek, and status operations.
