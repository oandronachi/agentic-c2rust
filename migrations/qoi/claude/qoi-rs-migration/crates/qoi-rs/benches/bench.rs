//! criterion microbenchmarks for the safe QOI port, across the Phase-1 patterns
//! (solid runs, small-step gradients hitting DIFF/LUMA, indexed repeats, and noise).
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use qoi_rs::{decode, encode, Desc, QOI_SRGB};

fn make_image(kind: &str, w: u32, h: u32, channels: u8) -> Vec<u8> {
    let n = (w * h) as usize;
    let mut px = Vec::with_capacity(n * channels as usize);
    let mut lcg: u32 = 0x1234_5678;
    let mut next = || {
        lcg = lcg.wrapping_mul(1664525).wrapping_add(1013904223);
        (lcg >> 24) as u8
    };
    for i in 0..n {
        let (r, g, b, a) = match kind {
            "solid" => (40, 80, 120, 255),
            "gradient" => {
                let v = (i % 256) as u8;
                (v, v.wrapping_add(1), v.wrapping_add(2), 255)
            }
            "indexed" => {
                let v = ((i * 7) % 16) as u8 * 16;
                (v, 255 - v, 128, 255)
            }
            _ => (next(), next(), next(), next()),
        };
        px.push(r);
        px.push(g);
        px.push(b);
        if channels == 4 {
            px.push(a);
        }
    }
    px
}

fn bench_codec(c: &mut Criterion) {
    let (w, h, channels) = (256u32, 256u32, 4u8);
    let desc = Desc {
        width: w,
        height: h,
        channels,
        colorspace: QOI_SRGB,
    };
    let bytes = (w * h * channels as u32) as u64;

    let mut enc_group = c.benchmark_group("encode");
    enc_group.throughput(Throughput::Bytes(bytes));
    for kind in ["solid", "gradient", "indexed", "noise"] {
        let img = make_image(kind, w, h, channels);
        enc_group.bench_with_input(BenchmarkId::from_parameter(kind), &img, |bch, img| {
            bch.iter(|| encode(std::hint::black_box(img), &desc).unwrap());
        });
    }
    enc_group.finish();

    let mut dec_group = c.benchmark_group("decode");
    dec_group.throughput(Throughput::Bytes(bytes));
    for kind in ["solid", "gradient", "indexed", "noise"] {
        let img = make_image(kind, w, h, channels);
        let enc = encode(&img, &desc).unwrap();
        dec_group.bench_with_input(BenchmarkId::from_parameter(kind), &enc, |bch, enc| {
            bch.iter(|| decode(std::hint::black_box(enc), 0).unwrap());
        });
    }
    dec_group.finish();
}

criterion_group!(benches, bench_codec);
criterion_main!(benches);
