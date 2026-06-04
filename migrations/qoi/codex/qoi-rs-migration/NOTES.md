# QOI API and Oracle Notes

## API Inventory

| Function | Direction | Rust mapping |
|---|---|---|
| `qoi_encode(const void *data, const qoi_desc *desc, int *out_len)` | Pixel bytes in, encoded bytes out. | `qoi_rs::encode(&[u8], &Desc) -> Result<Vec<u8>, Error>` |
| `qoi_decode(const void *data, int size, qoi_desc *desc, int channels)` | Encoded bytes in, decoded pixels and descriptor out. | `qoi_rs::decode(&[u8], channels: u8) -> Result<(Desc, Vec<u8>), Error>` |

`QOI_NO_STDIO` is used for the reference build. `qoi_read` and `qoi_write` are
not part of this migration because they add file I/O rather than codec semantics.

## Ownership

| Pointer | Allocator | Owner | Free path |
|---|---|---|---|
| C `qoi_encode` return buffer | C `malloc` from `qoi.h` | `qoi-sys` wrapper copies bytes immediately. | `libc::free` in `qoi-sys` |
| C `qoi_decode` return buffer | C `malloc` from `qoi.h` | `qoi-sys` wrapper copies bytes immediately. | `libc::free` in `qoi-sys` |
| Rust C ABI encoded/decoded buffer | Rust global allocator via `Box<[u8]>` | C caller. | `qoi_rs_free(ptr, len)` |

The inbound FFI never adopts a C allocation into a Rust `Vec`. The outbound C ABI
reconstructs the same boxed slice length that it originally returned.

## Equivalence Relation

The relation is byte-exact:

- Rust-encoded QOI bytes must match the C encoder for the same pixels and `qoi_desc`.
- Rust and C decoders must recover the same descriptor and pixel bytes.
- Cross-consume checks verify C can decode Rust output and Rust can decode C output.

## Edge Cases Covered

- Invalid descriptors: zero dimensions, invalid channels, invalid colorspace, and
  input length mismatch.
- Encoded input validation: bad magic, short header, truncated payload, bad footer,
  invalid requested channel count.
- Opcode paths: RGB, RGBA, index, diff, luma, run, repeated colors, alpha changes.
- Size handling: output length overflow is checked before allocation.
- Hostile bytes: decode returns `Result` instead of panicking.
