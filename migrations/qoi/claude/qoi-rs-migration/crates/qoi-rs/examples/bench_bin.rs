//! hyperfine subject — Rust side. Built `--release`. Does identical work to
//! `bench/qoi_cbench.c` (same image, same encode+decode loop, same checksum), so a
//! whole-process comparison is fair and the printed checksums must match.
//!
//! Usage: `bench_bin <width> <height> <channels> <iters>`
use qoi_rs::{decode, encode, Desc};

fn arg<T: std::str::FromStr>(args: &[String], i: usize, default: T) -> T {
    args.get(i).and_then(|s| s.parse().ok()).unwrap_or(default)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let width: u32 = arg(&args, 1, 512);
    let height: u32 = arg(&args, 2, 512);
    let channels: u8 = arg(&args, 3, 4);
    let iters: u64 = arg(&args, 4, 50);

    let npx = width as usize * height as usize;
    let n = npx * channels as usize;
    let mut img = vec![0u8; n];
    for i in 0..npx {
        img[i * channels as usize] = i.wrapping_mul(3) as u8;
        img[i * channels as usize + 1] = i.wrapping_mul(7) as u8;
        img[i * channels as usize + 2] = i.wrapping_mul(11) as u8;
        if channels == 4 {
            img[i * channels as usize + 3] = 255;
        }
    }

    let desc = Desc {
        width,
        height,
        channels,
        colorspace: 0,
    };
    let mut acc: u64 = 0;
    let mut enc_len = 0usize;
    for _ in 0..iters {
        let enc = encode(&img, &desc).expect("encode failed");
        enc_len = enc.len();
        for &b in &enc {
            acc = acc.wrapping_add(b as u64);
        }
        let (_d, dec) = decode(&enc, 0).expect("decode failed");
        for &b in &dec {
            acc = acc.wrapping_add(b as u64);
        }
    }
    println!("checksum={} enc_len={}", acc, enc_len);
}
