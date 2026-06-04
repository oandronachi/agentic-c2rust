#![forbid(unsafe_code)]

use qoi_rs::Desc;

#[derive(Clone, Debug)]
pub struct Input {
    pub desc: Desc,
    pub pixels: Vec<u8>,
}

pub fn check_against_reference(input: &Input) -> Result<(), String> {
    let rust_encoded = qoi_rs::encode(&input.pixels, &input.desc).map_err(|e| e.to_string())?;
    let c_desc = qoi_sys::Desc {
        width: input.desc.width,
        height: input.desc.height,
        channels: input.desc.channels,
        colorspace: input.desc.colorspace,
    };
    let c_encoded = qoi_sys::encode(&input.pixels, c_desc).ok_or("C encode returned null")?;
    if rust_encoded != c_encoded {
        return Err(format!(
            "encoded bytes differ: rust={} c={}",
            rust_encoded.len(),
            c_encoded.len()
        ));
    }

    let (rust_desc, rust_decoded) =
        qoi_rs::decode(&rust_encoded, 0).map_err(|e| e.to_string())?;
    let (c_dec_desc, c_decoded) =
        qoi_sys::decode(&rust_encoded, 0).ok_or("C decode of Rust bytes failed")?;
    if rust_desc != input.desc || rust_decoded != input.pixels {
        return Err("Rust roundtrip failed".into());
    }
    if c_dec_desc.width != input.desc.width
        || c_dec_desc.height != input.desc.height
        || c_dec_desc.channels != input.desc.channels
        || c_dec_desc.colorspace != input.desc.colorspace
        || c_decoded != input.pixels
    {
        return Err("C did not decode Rust bytes to the original pixels".into());
    }

    let (_, rust_from_c) = qoi_rs::decode(&c_encoded, 0).map_err(|e| e.to_string())?;
    if rust_from_c != input.pixels {
        return Err("Rust did not decode C bytes to the original pixels".into());
    }
    Ok(())
}

pub fn coerce(data: &[u8]) -> Option<Input> {
    if data.is_empty() {
        return None;
    }
    let width = 1 + u32::from(data[0] & 0x0f);
    let height = 1 + u32::from(data.get(1).copied().unwrap_or(0) & 0x0f);
    let channels = if data.get(2).copied().unwrap_or(0) & 1 == 0 { 3 } else { 4 };
    let len = width as usize * height as usize * channels as usize;
    let mut pixels = Vec::with_capacity(len);
    for i in 0..len {
        pixels.push(data.get(3 + i).copied().unwrap_or((i as u8).wrapping_mul(37)));
    }
    Some(Input {
        desc: Desc {
            width,
            height,
            channels,
            colorspace: data.get(3 + len).copied().unwrap_or(0) & 1,
        },
        pixels,
    })
}
