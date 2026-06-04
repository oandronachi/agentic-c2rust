//! Differential property tests: the safe port vs the reference C, across a
//! path-covering input space. Each strategy is biased toward specific QOI opcodes
//! (identified in `NOTES.md` Phase 1); `diff_random` and the arbitrary-decode test
//! cover the rest. Boundary cases are pinned as explicit unit tests.
use proptest::prelude::*;
use qoi_diff::{bounded_stream, check_decode_agreement, check_image, image_from_bytes, Image};

fn dims() -> impl Strategy<Value = (u32, u32)> {
    (1u32..=64, 1u32..=64)
}
fn channels() -> impl Strategy<Value = u8> {
    prop_oneof![Just(3u8), Just(4u8)]
}
fn colorspace() -> impl Strategy<Value = u8> {
    prop_oneof![Just(0u8), Just(1u8)]
}

fn build(w: u32, h: u32, ch: u8, cs: u8, mut px_at: impl FnMut(usize) -> [u8; 4]) -> Image {
    let n = (w * h) as usize;
    let mut pixels = Vec::with_capacity(n * ch as usize);
    for i in 0..n {
        let p = px_at(i);
        pixels.push(p[0]);
        pixels.push(p[1]);
        pixels.push(p[2]);
        if ch == 4 {
            pixels.push(p[3]);
        }
    }
    Image {
        width: w,
        height: h,
        channels: ch,
        colorspace: cs,
        pixels,
    }
}

/// Identical pixels ⇒ RUN path (incl. the 62 boundary and the end-of-image flush).
fn solid_image() -> impl Strategy<Value = Image> {
    (dims(), channels(), colorspace(), any::<[u8; 4]>())
        .prop_map(|((w, h), ch, cs, c)| build(w, h, ch, cs, move |_| c))
}

/// Small per-pixel steps ⇒ DIFF and LUMA paths near their range edges.
fn gradient_image() -> impl Strategy<Value = Image> {
    (
        dims(),
        channels(),
        colorspace(),
        any::<[u8; 4]>(),
        -10i32..=10,
        -10i32..=10,
        -10i32..=10,
        -2i32..=2,
    )
        .prop_map(|((w, h), ch, cs, start, dr, dg, db, da)| {
            build(w, h, ch, cs, move |i| {
                let i = i as i32;
                [
                    start[0].wrapping_add((dr * i) as u8),
                    start[1].wrapping_add((dg * i) as u8),
                    start[2].wrapping_add((db * i) as u8),
                    start[3].wrapping_add((da * i) as u8),
                ]
            })
        })
}

/// A few repeating colors ⇒ INDEX hits (and hash collisions).
fn indexed_image() -> impl Strategy<Value = Image> {
    (
        dims(),
        channels(),
        colorspace(),
        prop::collection::vec(any::<[u8; 4]>(), 1..=6),
    )
        .prop_map(|((w, h), ch, cs, palette)| {
            build(w, h, ch, cs, move |i| palette[i % palette.len()])
        })
}

/// Fully random pixels ⇒ RGB/RGBA paths and alpha changes.
fn random_image() -> impl Strategy<Value = Image> {
    (dims(), channels(), colorspace()).prop_flat_map(|((w, h), ch, cs)| {
        let len = (w * h * ch as u32) as usize;
        prop::collection::vec(any::<u8>(), len..=len).prop_map(move |pixels| Image {
            width: w,
            height: h,
            channels: ch,
            colorspace: cs,
            pixels,
        })
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2048))]

    #[test]
    fn diff_solid(img in solid_image()) {
        prop_assert!(check_image(&img).is_ok(), "{}", check_image(&img).unwrap_err());
    }

    #[test]
    fn diff_gradient(img in gradient_image()) {
        prop_assert!(check_image(&img).is_ok(), "{}", check_image(&img).unwrap_err());
    }

    #[test]
    fn diff_indexed(img in indexed_image()) {
        prop_assert!(check_image(&img).is_ok(), "{}", check_image(&img).unwrap_err());
    }

    #[test]
    fn diff_random(img in random_image()) {
        prop_assert!(check_image(&img).is_ok(), "{}", check_image(&img).unwrap_err());
    }

    // Arbitrary bytes coerced into a small valid image (mirrors the fuzz target).
    #[test]
    fn diff_coerced(data in prop::collection::vec(any::<u8>(), 4..4096)) {
        if let Some(img) = image_from_bytes(&data) {
            prop_assert!(check_image(&img).is_ok(), "{}", check_image(&img).unwrap_err());
        }
    }

    // Arbitrary chunk bytes in a bounded stream ⇒ decoder must agree with C on
    // hostile input (both reject, or both produce identical pixels), for every
    // channel setting.
    #[test]
    fn diff_decode_arbitrary(
        w in any::<u16>(), h in any::<u16>(), ch in any::<u8>(), cs in any::<u8>(),
        body in prop::collection::vec(any::<u8>(), 0..512),
    ) {
        let stream = bounded_stream(w, h, ch, cs, &body);
        for c in [0u8, 3, 4] {
            prop_assert!(check_decode_agreement(&stream, c).is_ok(), "{}",
                         check_decode_agreement(&stream, c).unwrap_err());
        }
    }
}

// ---- Explicit boundary cases (deterministic) ------------------------------------

#[test]
fn boundary_run_lengths() {
    // Run-flush boundaries: 62 resets the run; 63/64/124 cross it.
    for n in [1u32, 61, 62, 63, 64, 123, 124, 125, 4096] {
        let img = build(n, 1, 4, 0, |_| [9, 8, 7, 6]);
        check_image(&img).unwrap();
        let img3 = build(n, 1, 3, 1, |_| [200, 100, 50, 255]);
        check_image(&img3).unwrap();
    }
}

#[test]
fn boundary_diff_luma_rgb_grid() {
    // 2-pixel images: a fixed first pixel and a second pixel swept across values
    // that land on/around the DIFF (-2..1) and LUMA (-32..31 / -8..7) edges,
    // plus alpha changes (RGBA). Exhaustively exercises the encoder's branch
    // selection against the reference.
    let edges: [u8; 13] = [0, 126, 127, 128, 129, 130, 120, 136, 95, 96, 159, 160, 255];
    let first = [128u8, 128, 128, 255];
    for &r in &edges {
        for &g in &edges {
            for &b in &edges {
                for &a in &[255u8, 254, 0] {
                    let img = build(2, 1, 4, 0, |i| if i == 0 { first } else { [r, g, b, a] });
                    check_image(&img).unwrap();
                }
            }
        }
    }
}

#[test]
fn boundary_single_pixels_all_channels() {
    for ch in [3u8, 4] {
        for cs in [0u8, 1] {
            let img = build(1, 1, ch, cs, |_| [1, 2, 3, 4]);
            check_image(&img).unwrap();
        }
    }
}
