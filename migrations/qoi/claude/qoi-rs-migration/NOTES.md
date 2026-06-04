# QOI — Phase 1 characterization (API, ownership, edge cases, oracle)

Upstream: <https://github.com/phoboslab/qoi> @ `97bacc86a9c4abf5a2d452102dc26546c4c670b9`
(fetched master, 2026-06-03). License: **MIT** (`SPDX-License-Identifier: MIT`,
© 2021 Dominic Szablewski) — vendoring permitted. Vendored verbatim in
`vendor/qoi/{qoi.h,LICENSE}`.

The library is a single header. The implementation is gated behind
`QOI_IMPLEMENTATION`; file I/O (`qoi_read`/`qoi_write`) is gated behind *not*
`QOI_NO_STDIO`. We compile with `QOI_IMPLEMENTATION` **and** `QOI_NO_STDIO`, so the
only symbols are the two in-memory functions we bind.

## API inventory (the two bound functions)

### `void *qoi_encode(const void *data, const qoi_desc *desc, int *out_len)`
- `data`  — **in**: raw pixels, `width*height*channels` bytes, row-major RGB(A).
- `desc`  — **in**: `{width:u32, height:u32, channels:u8, colorspace:u8}`.
- `out_len` — **out**: number of bytes in the returned buffer (set on success only).
- **returns**: `malloc`-ed QOI byte stream, or `NULL` on failure.
- **preconditions / validation** (all must hold or it returns NULL):
  `data!=NULL`, `out_len!=NULL`, `desc!=NULL`, `width!=0`, `height!=0`,
  `3<=channels<=4`, `colorspace<=1`, and `height < QOI_PIXELS_MAX/width`
  (`QOI_PIXELS_MAX = 400_000_000`; this is the integer-overflow guard — it caps
  `width*height` below 400M so the worst-case `max_size = w*h*(channels+1)+22`
  stays under `INT_MAX`). The C trusts `data` is large enough; **our safe port
  additionally requires `data.len() == width*height*channels`** (cannot reproduce
  the C out-of-bounds read).

### `void *qoi_decode(const void *data, int size, qoi_desc *desc, int channels)`
- `data` — **in**: a QOI byte stream of length `size`.
- `size` — **in**: length of `data`.
- `desc` — **out**: filled from the 14-byte header on success.
- `channels` — **in**: `0` ⇒ use the header's channel count; `3` or `4` ⇒ force.
- **returns**: `malloc`-ed pixel buffer (`width*height*channels` bytes), or `NULL`.
- **preconditions / validation**: `data!=NULL`, `desc!=NULL`,
  `channels ∈ {0,3,4}`, `size >= 14+8`. Then header must satisfy:
  `magic=="qoif"`, `width!=0`, `height!=0`, `3<=channels<=4`, `colorspace<=1`,
  `height < QOI_PIXELS_MAX/width`. Otherwise NULL.
- **totality**: the decode loop is **bounds-checked** (`p < chunks_len`); if the
  stream runs out before `width*height` pixels are produced, the remaining pixels
  are filled with the last decoded pixel value. It never reads out of bounds, so it
  is total on hostile input (no panic / no UB) — but it can produce a "wrong" image
  for a malformed stream. Our port mirrors this exactly.

## Ownership table  (no "unknown" rows ⇒ §2.4 does not fire)

| Pointer                | Dir | Allocated by | Freed by | Allocator |
|------------------------|-----|--------------|----------|-----------|
| `qoi_encode` return    | out | qoi (malloc) | caller   | malloc/free |
| `qoi_encode` `data`    | in  | caller       | caller   | caller's    |
| `qoi_encode` `desc`    | in  | caller       | caller   | caller (stack ok) |
| `qoi_encode` `out_len` | out | caller       | caller   | caller (`int*`) |
| `qoi_decode` return    | out | qoi (malloc) | caller   | malloc/free |
| `qoi_decode` `data`    | in  | caller       | caller   | caller's    |
| `qoi_decode` `desc`    | out | caller       | caller   | caller (`qoi_desc*`) |

Only the two **returned** buffers cross the boundary owning memory; both are
`malloc`-ed and must be `free`d by the caller. ⇒ in `qoi-sys` we **copy out** into
an owned `Vec` and call `libc::free` (allocator-symmetric, Appendix C). We never
adopt the C pointer into a `Vec`.

## Error model
Sentinel return (`NULL`) on any failure. No `errno`, no error codes. `out_len` is
written only on success. Our safe port maps each failure cause to a typed `Error`
enum variant.

## Determinism
Output is a **pure function** of (`data`, `desc`) for encode and of (`data`, `size`,
`channels`) for decode. No timestamps, threads, FP, or global state. ⇒
`oracle_relation = byte_exact` is valid and is the strongest choice (§2.2 does not
fire).

## Algorithm details that the port MUST mirror bit-for-bit
- **Previous pixel** seed: `{r:0,g:0,b:0,a:255}`; index array is 64 entries,
  zero-initialized (`{0,0,0,0}`, whose `.v` is 0).
- **Hash**: `index_pos = (r*3 + g*5 + b*7 + a*11) & 63` (compute in a wide int).
- **Chunk tags** (8-bit tags take precedence over 2-bit tags — decoder checks
  `0xFE`/`0xFF` first): `INDEX 00`, `DIFF 01`, `LUMA 10`, `RUN 11`,
  `RGB 0xFE`, `RGBA 0xFF`; 2-bit mask `0xC0`.
- **Encoder integer semantics** (wraparound, signed-char interpretation):
  `vr = (px.r.wrapping_sub(prev.r)) as i8`, likewise `vg,vb`;
  `vg_r = vr.wrapping_sub(vg)`, `vg_b = vb.wrapping_sub(vg)` (i8 wrapping).
  - DIFF chosen iff `-2<=vr,vg,vb<=1`; byte =
    `0x40 | (vr+2)<<4 | (vg+2)<<2 | (vb+2)`.
  - else LUMA iff `-8<=vg_r,vg_b<=7 && -32<=vg<=31`; bytes =
    `0x80|(vg+32)`, `(vg_r+8)<<4|(vg_b+8)`.
  - else if `a==prev.a` ⇒ RGB (3 bytes), else ⇒ RGBA (4 bytes).
- **Runs**: count consecutive identical pixels; emit `RUN | (run-1)` when
  `run==62` or at the last pixel (`px_pos==px_end`); a pending run is also flushed
  immediately before any non-matching pixel. `run` value stored biased −1 (1..62 ⇒
  byte 0..61). INDEX is emitted only when `index[hash]==px`; on an INDEX hit the
  index is **not** rewritten; otherwise `index[hash]=px` then DIFF/LUMA/RGB/RGBA.
- **Decoder reconstruction** (u8 wrapping):
  - `RGB`/`RGBA`: assign channels literally (RGB keeps previous alpha).
  - `INDEX` (`b1` is the whole byte, `0..63`): `px = index[b1]`.
  - `DIFF`: `r += ((b1>>4)&3)-2; g += ((b1>>2)&3)-2; b += (b1&3)-2` (wrapping).
  - `LUMA`: `vg=(b1&0x3f)-32; r += vg-8+((b2>>4)&0xf); g += vg; b += vg-8+(b2&0xf)`.
  - `RUN`: `run = b1 & 0x3f` (then `run--` is applied on the *following* iterations,
    so a RUN byte emits `1 + (b1&0x3f)` copies of the current pixel).
  - After **every chunk actually read** (not during run-continuation or past-end
    fill), `index[hash(px)] = px`.
- **Forced channels on decode**: output channel count is `channels` (param) if
  non-zero else the header's. When forcing 3, alpha is dropped; when forcing 4 on a
  3-channel file, alpha carries the running `px.a` (starts 255, only RGBA chunks
  change it).

## Edge cases → Phase-5 generators must cover
1. 1×1 image (min size); single-channel-run images (all pixels identical) ⇒ RUN path
   incl. run==62 boundary and end-of-image flush.
2. Gradients with small steps ⇒ DIFF and LUMA paths at every boundary
   (`vr=-2,1`, `vg=-32,31`, `vg_r=-8,7`).
3. Images that repeat earlier colors ⇒ INDEX path (and hash collisions).
4. Random RGB and RGBA ⇒ RGB/RGBA paths; alpha changes ⇒ RGBA.
5. channels 3 vs 4; colorspace 0 vs 1; decode with `channels` 0/3/4 (forcing).
6. Decode of **truncated / arbitrary** streams ⇒ totality (past-end fill), bad
   magic, zero dims, oversize dims (overflow guard), `size < 22`.
7. Width/height combinations near the small end (oracle uses bounded sizes; the
   400M cap is asserted by a unit test, not exercised by fuzzing for memory reasons).

## Oracle relation (chosen): `byte_exact`
- **encode**: `rs::encode(px,desc)` bytes **==** `c::encode(px,desc)` bytes.
- **decode**: for a valid stream `s`, `rs::decode(s,ch)` **==** `c::decode(s,ch)`
  (both the filled `Desc` and the pixel buffer, byte-identical).
- **round-trip**: `decode(encode(px))` reproduces `px` (and `desc`).
- **cross-consume**: C-decode of Rust-encoded bytes == original, and Rust-decode of
  C-encoded bytes == original.
- **structural** (no impl needed): magic `qoif`, 14-byte header echoes desc,
  trailing 8-byte padding `00 00 00 00 00 00 00 01`.

No STOP & ASK condition fires: license OK, deterministic, no callbacks/threads/
global state, ownership fully known, no reliance on UB (we validate instead).
