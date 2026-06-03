# Claude Code × QOI

Migrating the [phoboslab/qoi](https://github.com/phoboslab/qoi) image codec (MIT, ~300 LOC, single header) from C to safe Rust. Picked because QOI is small, deterministic, spec-anchored — a clean target for a `byte_exact` differential oracle.

> The exact instruction the agent received is in [`qoi-task.md`](./qoi-task.md) — it is this repo's [playbook](../../playbook/c-to-rust-migration-playbook.md) with the config block below filled in. Read it if you want to see what an end-to-end agent prompt for this workflow actually looks like.

## Config block (the only thing that changes per library)

```yaml
lib_name:        qoi
upstream_url:    https://github.com/phoboslab/qoi
upstream_pin:    97bacc86a9c4abf5a2d452102dc26546c4c670b9
license:         MIT
headers:         ["qoi.h"]
api_functions:   ["qoi_encode", "qoi_decode"]
opaque_types:    ["qoi_desc"]
allocator:       malloc                # default QOI_MALLOC; freed via libc::free
determinism:     deterministic_bytes
oracle_relation: byte_exact
```

## Results

| What | Value |
|---|---|
| Tests | 28 passed, 0 failed (unit + 10 golden + 9 differential properties × 2048 cases + 6,591-case boundary grid) |
| Fuzzing | 122 s total across `differential` + `no_panic`; **362,286 execs**, 0 crashes |
| Microbench (criterion, 256×256 RGBA) | encode 0.75–1.79 GiB/s, decode 1.1–3.3 GiB/s |
| Whole-process (hyperfine vs C `-O3`) | **C 1.26× ± 0.10 faster** (Rust ≈ 0.79× C); identical checksums |
| `unsafe` | **0** in core (compile-enforced); 2 in `qoi-sys`, 3 in `qoi-cabi`, each with a documented contract |

## Representative snippet — allocator-symmetric FFI-in

C `qoi_encode` returns a `malloc`-ed buffer. Adopting it into a `Vec` would be UB (Rust's global allocator would later try to free it). Copy out, free with the matching allocator:

```rust
unsafe {
    let ptr = ffi_encode(input.as_ptr().cast(), &desc, &mut out_len);  // C malloc'd
    if ptr.is_null() { return None; }
    let v = std::slice::from_raw_parts(ptr as *const u8, out_len as usize).to_vec();
    libc::free(ptr);                                                     // C free
    Some(v)
}
```

This is the inbound-FFI pattern the playbook locks down across every run.
