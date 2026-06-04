//! No-panic / totality fuzz target for the decoder on adversarial input.
//!
//! Header dimensions are masked to small values so the output allocation is bounded
//! (no OOM). The property: [`qoi_rs::decode`] must never panic; and when it
//! succeeds, re-encoding the decoded image and decoding again must reproduce the
//! same pixels (a stable round-trip).
#![no_main]

use libfuzzer_sys::fuzz_target;
use qoi_diff::bounded_stream;
use qoi_rs::{decode, encode, Desc};

/// Decode must not panic; on success the decode→encode→decode round-trip is stable.
fn check_total(stream: &[u8], ch: u8) {
    if let Ok((desc, px)) = decode(stream, ch) {
        let out_ch = if ch == 0 { desc.channels } else { ch };
        let d = Desc {
            width: desc.width,
            height: desc.height,
            channels: out_ch,
            colorspace: desc.colorspace,
        };
        // The decoded image is always valid, so re-encode cannot fail.
        let re = encode(&px, &d).expect("re-encode of a decoded image must succeed");
        let (_d2, px2) = decode(&re, 0).expect("re-decode must succeed");
        assert_eq!(px2, px, "decode->encode->decode must be stable");
    }
}

/// Mask the header width/height to 1..=64 so decode allocates a bounded buffer.
fn mask_dims(data: &[u8]) -> Vec<u8> {
    let mut v = data.to_vec();
    if v.len() >= 12 {
        v[4] = 0;
        v[5] = 0;
        v[6] = 0;
        v[7] = (v[7] % 64) + 1;
        v[8] = 0;
        v[9] = 0;
        v[10] = 0;
        v[11] = (v[11] % 64) + 1;
    }
    v
}

fuzz_target!(|data: &[u8]| {
    // (a) Arbitrary chunk body inside a known-valid bounded header.
    let w = if data.len() >= 2 { u16::from_le_bytes([data[0], data[1]]) } else { 1 };
    let h = if data.len() >= 4 { u16::from_le_bytes([data[2], data[3]]) } else { 1 };
    let ch_sel = data.get(4).copied().unwrap_or(4);
    let cs = data.get(5).copied().unwrap_or(0);
    let body: &[u8] = if data.len() > 6 { &data[6..] } else { &[] };
    let bounded = bounded_stream(w, h, ch_sel, cs, body);
    for c in [0u8, 3, 4] {
        check_total(&bounded, c);
    }

    // (b) Raw bytes treated as a whole stream, header dimensions masked.
    let masked = mask_dims(data);
    for c in [0u8, 3, 4] {
        check_total(&masked, c);
    }
});
