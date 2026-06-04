use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use qoi_rs::{decode, encode, Desc, SRGB};

fn image(kind: &str) -> (Desc, Vec<u8>) {
    let width = 256u32;
    let height = 256u32;
    let desc = Desc {
        width,
        height,
        channels: 4,
        colorspace: SRGB,
    };
    let mut pixels = Vec::with_capacity(width as usize * height as usize * 4);
    for y in 0..height {
        for x in 0..width {
            let i = y * width + x;
            let (r, g, b) = match kind {
                "solid" => (25, 80, 140),
                "gradient" => (x as u8, y as u8, x.wrapping_add(y) as u8),
                _ => {
                    let v = i.wrapping_mul(2_654_435_761);
                    (v as u8, (v >> 8) as u8, (v >> 16) as u8)
                }
            };
            pixels.extend_from_slice(&[r, g, b, 255]);
        }
    }
    (desc, pixels)
}

fn benches(c: &mut Criterion) {
    for kind in ["solid", "gradient", "noise"] {
        let (desc, pixels) = image(kind);
        let encoded = encode(&pixels, &desc).unwrap();

        let mut group = c.benchmark_group("encode");
        group.throughput(Throughput::Bytes(pixels.len() as u64));
        group.bench_with_input(BenchmarkId::new(kind, "rgba"), &pixels, |b, p| {
            b.iter(|| encode(black_box(p), black_box(&desc)).unwrap())
        });
        group.finish();

        let mut group = c.benchmark_group("decode");
        group.throughput(Throughput::Bytes(pixels.len() as u64));
        group.bench_with_input(BenchmarkId::new(kind, "rgba"), &encoded, |b, e| {
            b.iter(|| decode(black_box(e), 0).unwrap())
        });
        group.finish();
    }
}

criterion_group!(qoi, benches);
criterion_main!(qoi);
