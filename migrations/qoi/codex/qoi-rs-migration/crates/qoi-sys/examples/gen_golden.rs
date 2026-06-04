use std::fmt::Write;
use std::fs;
use std::path::PathBuf;

fn main() {
    let cases = cases();
    let mut out = String::new();
    out.push_str("use qoi_rs::Desc;\n\n");
    out.push_str("pub struct Case {\n");
    out.push_str("    pub name: &'static str,\n");
    out.push_str("    pub desc: Desc,\n");
    out.push_str("    pub raw: &'static [u8],\n");
    out.push_str("    pub encoded: &'static [u8],\n");
    out.push_str("}\n\n");
    out.push_str("pub const CASES: &[Case] = &[\n");

    for (name, desc, raw) in cases {
        let encoded = qoi_sys::encode(&raw, desc).expect("reference encode");
        write!(
            out,
            "    Case {{ name: {:?}, desc: Desc {{ width: {}, height: {}, channels: {}, colorspace: {} }}, raw: &{:?}, encoded: &{:?} }},\n",
            name, desc.width, desc.height, desc.channels, desc.colorspace, raw, encoded
        )
        .unwrap();
    }

    out.push_str("];\n");
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../qoi-rs/tests/golden_data.rs");
    fs::write(path, out).expect("write golden data");
}

fn cases() -> Vec<(&'static str, qoi_sys::Desc, Vec<u8>)> {
    vec![
        (
            "single_rgba",
            desc(1, 1, 4),
            vec![255, 0, 0, 255],
        ),
        (
            "single_rgb",
            desc(1, 1, 3),
            vec![3, 2, 1],
        ),
        (
            "run_rgba",
            desc(70, 1, 4),
            vec![9, 8, 7, 255].repeat(70),
        ),
        (
            "gradient_rgb",
            desc(8, 8, 3),
            pixels(8, 8, 3, Pattern::Gradient),
        ),
        (
            "noise_rgba",
            desc(8, 8, 4),
            pixels(8, 8, 4, Pattern::Noise),
        ),
        (
            "alpha_rgba",
            desc(16, 1, 4),
            (0..16u8).flat_map(|i| [i, i.wrapping_mul(7), 255 - i, i]).collect(),
        ),
    ]
}

fn desc(width: u32, height: u32, channels: u8) -> qoi_sys::Desc {
    qoi_sys::Desc {
        width,
        height,
        channels,
        colorspace: 0,
    }
}

enum Pattern {
    Gradient,
    Noise,
}

fn pixels(width: u32, height: u32, channels: u8, pattern: Pattern) -> Vec<u8> {
    let mut out = Vec::new();
    for y in 0..height {
        for x in 0..width {
            let v = match pattern {
                Pattern::Gradient => x + y * 3,
                Pattern::Noise => (x + y * width).wrapping_mul(2_654_435_761),
            };
            out.push(v as u8);
            out.push((v >> 5) as u8);
            out.push((v >> 13) as u8);
            if channels == 4 {
                out.push(255u8.wrapping_sub(v as u8));
            }
        }
    }
    out
}
