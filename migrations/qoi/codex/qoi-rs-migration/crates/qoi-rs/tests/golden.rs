mod golden_data;

use qoi_rs::{decode, encode, Desc};

#[test]
fn golden_vectors_are_present() {
    assert!(golden_data::CASES.len() >= 5);
}

#[test]
fn encode_matches_reference_bytes() {
    for case in golden_data::CASES {
        let encoded = encode(case.raw, &case.desc).unwrap();
        assert_eq!(encoded, case.encoded, "{}", case.name);
    }
}

#[test]
fn decode_matches_reference_pixels() {
    for case in golden_data::CASES {
        let (desc, decoded) = decode(case.encoded, 0).unwrap();
        assert_eq!(desc, case.desc, "{}", case.name);
        assert_eq!(decoded, case.raw, "{}", case.name);
    }
}

#[test]
fn forced_rgb_decode_matches_prefix_channels() {
    for case in golden_data::CASES {
        let (Desc { width, height, .. }, decoded) = decode(case.encoded, 3).unwrap();
        assert_eq!(decoded.len(), width as usize * height as usize * 3, "{}", case.name);
    }
}
