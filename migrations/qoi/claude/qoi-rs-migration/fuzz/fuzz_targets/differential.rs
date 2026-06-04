//! Differential fuzz target: find any divergence between the safe port and the
//! reference C that the structured proptests miss.
//!
//! (1) Coerce the raw bytes into a small **valid** image and assert full
//!     encode/decode/round-trip equivalence (`check_image`).
//! (2) Wrap the bytes as an arbitrary chunk body inside a **bounded** stream and
//!     assert `rs::decode` agrees with `c::decode` for every channel setting.
#![no_main]

use libfuzzer_sys::fuzz_target;
use qoi_diff::{bounded_stream, check_decode_agreement, check_image, image_from_bytes};

fn split(data: &[u8]) -> (u16, u16, u8, u8, &[u8]) {
    let w = if data.len() >= 2 { u16::from_le_bytes([data[0], data[1]]) } else { 1 };
    let h = if data.len() >= 4 { u16::from_le_bytes([data[2], data[3]]) } else { 1 };
    let ch = data.get(4).copied().unwrap_or(4);
    let cs = data.get(5).copied().unwrap_or(0);
    let body: &[u8] = if data.len() > 6 { &data[6..] } else { &[] };
    (w, h, ch, cs, body)
}

fuzz_target!(|data: &[u8]| {
    if let Some(img) = image_from_bytes(data) {
        if let Err(e) = check_image(&img) {
            panic!("differential image mismatch: {e}");
        }
    }

    let (w, h, ch, cs, body) = split(data);
    let stream = bounded_stream(w, h, ch, cs, body);
    for c in [0u8, 3, 4] {
        if let Err(e) = check_decode_agreement(&stream, c) {
            panic!("decode divergence: {e}");
        }
    }
});
