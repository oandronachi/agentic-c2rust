//! Known-answer tests. `golden_data.rs` holds vectors whose `encoded` field was
//! produced by the reference C qoi_encode; these assert the safe port reproduces
//! those exact bytes and round-trips them back — with **no C present**.
#![allow(clippy::all)]

include!("golden_data.rs");

use qoi_rs::{decode, encode, Desc};

#[test]
fn golden_encode_is_byte_exact() {
    assert!(!GOLDEN.is_empty(), "no golden vectors checked in");
    for g in GOLDEN {
        let desc = Desc {
            width: g.width,
            height: g.height,
            channels: g.channels,
            colorspace: g.colorspace,
        };
        let got = encode(g.pixels, &desc).expect("encode failed");
        assert_eq!(
            got, g.encoded,
            "encode bytes differ from the C reference for vector `{}`",
            g.name
        );
    }
}

#[test]
fn golden_decode_roundtrips() {
    for g in GOLDEN {
        let (desc, pixels) = decode(g.encoded, 0).expect("decode failed");
        assert_eq!(desc.width, g.width, "{}", g.name);
        assert_eq!(desc.height, g.height, "{}", g.name);
        assert_eq!(desc.channels, g.channels, "{}", g.name);
        assert_eq!(desc.colorspace, g.colorspace, "{}", g.name);
        assert_eq!(pixels, g.pixels, "decoded pixels differ for `{}`", g.name);
    }
}
