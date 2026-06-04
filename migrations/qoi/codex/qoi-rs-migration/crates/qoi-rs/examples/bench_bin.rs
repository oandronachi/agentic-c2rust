use qoi_rs::{decode, encode, Desc, SRGB};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map(String::as_str).unwrap_or("encode");
    let width: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(256);
    let height: u32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(256);
    let channels: u8 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(4);
    let iters: usize = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(20);

    let desc = Desc {
        width,
        height,
        channels,
        colorspace: SRGB,
    };
    let pixels = pixels(width, height, channels);
    let encoded = encode(&pixels, &desc).unwrap();
    let mut checksum = 0u64;
    let mut out_len = 0usize;

    for _ in 0..iters {
        if mode == "roundtrip" {
            let (_, decoded) = decode(&encoded, 0).unwrap();
            checksum ^= checksum64(&decoded);
            out_len = decoded.len();
        } else {
            let out = encode(&pixels, &desc).unwrap();
            checksum ^= checksum64(&out);
            out_len = out.len();
        }
    }
    println!("checksum={checksum:016x} out_len={out_len}");
}

fn pixels(width: u32, height: u32, channels: u8) -> Vec<u8> {
    let mut out = Vec::with_capacity(width as usize * height as usize * channels as usize);
    for y in 0..height {
        for x in 0..width {
            let v = x.wrapping_mul(31).wrapping_add(y.wrapping_mul(17));
            out.push(v as u8);
            out.push((v >> 3) as u8);
            out.push((v >> 7) as u8);
            if channels == 4 {
                out.push(255);
            }
        }
    }
    out
}

fn checksum64(data: &[u8]) -> u64 {
    data.iter().fold(0xcbf29ce484222325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x100000001b3)
    })
}
