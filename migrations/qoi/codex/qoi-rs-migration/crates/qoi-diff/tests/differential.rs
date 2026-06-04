use proptest::prelude::*;
use qoi_diff::{check_against_reference, Input};
use qoi_rs::{Desc, SRGB};

fn inputs() -> impl Strategy<Value = Input> {
    (1u32..=16, 1u32..=16, prop_oneof![Just(3u8), Just(4u8)])
        .prop_flat_map(|(width, height, channels)| {
            let len = width as usize * height as usize * channels as usize;
            (
                Just(Desc {
                    width,
                    height,
                    channels,
                    colorspace: SRGB,
                }),
                prop::collection::vec(any::<u8>(), len),
            )
        })
        .prop_map(|(desc, pixels)| Input { desc, pixels })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2048))]

    #[test]
    fn rust_matches_c(input in inputs()) {
        prop_assert!(check_against_reference(&input).is_ok());
    }
}

#[test]
fn explicit_run_boundary() {
    let input = Input {
        desc: Desc {
            width: 70,
            height: 1,
            channels: 4,
            colorspace: 0,
        },
        pixels: vec![1, 2, 3, 255].repeat(70),
    };
    check_against_reference(&input).unwrap();
}
