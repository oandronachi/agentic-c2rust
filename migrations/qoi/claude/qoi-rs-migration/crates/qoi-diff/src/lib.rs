//! Differential oracle for the QOI port.
//!
//! Defines the shared [`Image`] input model and the equivalence checks that assert
//! the chosen relation (`byte_exact`) between the safe port [`qoi_rs`] and the
//! reference C [`qoi_sys`]. These functions are reused by the proptest suite
//! (`tests/differential.rs`) and by the `cargo-fuzz` targets.
//!
//! The relation, per `NOTES.md`:
//! * **encode**: `rs::encode == c::encode`, byte for byte.
//! * **decode**: for any stream, `rs::decode == c::decode` (accept/reject, the
//!   filled `Desc`, and the pixel buffer all identical).
//! * **round-trip**: `decode(encode(img)) == img`.
//! * **cross-consume**: because the two encoders are byte-identical, decoding "the
//!   other side's output" is the same stream; the round-trip check therefore covers
//!   `rs::decode(c_enc) == img` and `c::decode(rs_enc) == img`.
//! * **structural**: magic `qoif`, header echoes the desc, 8-byte end marker.

use qoi_rs::QOI_HEADER_SIZE;

const END_MARKER: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 1];

/// A raw image: a valid `Desc` plus exactly `width*height*channels` pixel bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    /// Width in pixels (1..).
    pub width: u32,
    /// Height in pixels (1..).
    pub height: u32,
    /// Channels: 3 or 4.
    pub channels: u8,
    /// Colorspace: 0 or 1.
    pub colorspace: u8,
    /// Row-major pixels, `width*height*channels` bytes.
    pub pixels: Vec<u8>,
}

impl Image {
    fn rs_desc(&self) -> qoi_rs::Desc {
        qoi_rs::Desc {
            width: self.width,
            height: self.height,
            channels: self.channels,
            colorspace: self.colorspace,
        }
    }
    fn sys_desc(&self) -> qoi_sys::Desc {
        qoi_sys::Desc {
            width: self.width,
            height: self.height,
            channels: self.channels,
            colorspace: self.colorspace,
        }
    }

    /// True when the desc is acceptable and `pixels` has the exact expected length.
    pub fn is_valid(&self) -> bool {
        self.width != 0
            && self.height != 0
            && (3..=4).contains(&self.channels)
            && self.colorspace <= 1
            && (self.height as u64) < qoi_rs::QOI_PIXELS_MAX as u64 / self.width as u64
            && self.pixels.len()
                == self.width as usize * self.height as usize * self.channels as usize
    }
}

fn desc_eq(a: &qoi_rs::Desc, b: &qoi_sys::Desc) -> bool {
    a.width == b.width
        && a.height == b.height
        && a.channels == b.channels
        && a.colorspace == b.colorspace
}

fn first_diff(a: &[u8], b: &[u8]) -> Option<usize> {
    if let Some(i) = a.iter().zip(b.iter()).position(|(x, y)| x != y) {
        Some(i)
    } else if a.len() != b.len() {
        Some(a.len().min(b.len()))
    } else {
        None
    }
}

/// Full encode + decode + round-trip + structural equivalence for a valid image.
///
/// Returns `Ok(())` on agreement, or `Err(message)` describing the first divergence
/// (the message is what proptest/fuzz surface on a counterexample).
pub fn check_image(img: &Image) -> Result<(), String> {
    let rs_enc = qoi_rs::encode(&img.pixels, &img.rs_desc());
    let sys_enc = qoi_sys::encode(&img.pixels, &img.sys_desc());

    let enc =
        match (rs_enc, sys_enc) {
            (Ok(a), Some(b)) => {
                if a != b {
                    return Err(format!(
                    "ENCODE mismatch ({}x{} ch{} cs{}): rs={} bytes, c={} bytes, first diff @ {:?}",
                    img.width, img.height, img.channels, img.colorspace, a.len(), b.len(),
                    first_diff(&a, &b)
                ));
                }
                a
            }
            (Err(_), None) => return Ok(()), // both reject (not expected for valid images)
            (a, b) => {
                return Err(format!(
                    "ENCODE accept/reject divergence: rs_ok={} c_ok={}",
                    a.is_ok(),
                    b.is_some()
                ))
            }
        };

    // Structural invariants — checkable without either implementation.
    if enc.len() < QOI_HEADER_SIZE + END_MARKER.len() {
        return Err(format!("encoded stream too short: {} bytes", enc.len()));
    }
    if &enc[0..4] != b"qoif" {
        return Err("bad magic in encoded stream".into());
    }
    if enc[enc.len() - 8..] != END_MARKER {
        return Err("bad end marker in encoded stream".into());
    }
    let hw = u32::from_be_bytes([enc[4], enc[5], enc[6], enc[7]]);
    let hh = u32::from_be_bytes([enc[8], enc[9], enc[10], enc[11]]);
    if hw != img.width || hh != img.height || enc[12] != img.channels || enc[13] != img.colorspace {
        return Err("header does not echo the desc".into());
    }

    // Decode equivalence for every channel setting; round-trip when not converting.
    for ch in [0u8, 3, 4] {
        check_decode_agreement(&enc, ch)?;
        if ch == 0 || ch == img.channels {
            // The Rust decode equals C decode (just checked) and must reproduce the
            // original pixels — this is the round-trip / cross-consume guarantee.
            let (_d, p) = qoi_rs::decode(&enc, ch).map_err(|e| format!("rs decode ch{ch}: {e}"))?;
            if p != img.pixels {
                return Err(format!(
                    "ROUND-TRIP mismatch ch{ch}: first diff @ {:?}",
                    first_diff(&p, &img.pixels)
                ));
            }
        }
    }
    Ok(())
}

/// Assert `rs::decode` and `c::decode` agree on an arbitrary stream for one channel
/// setting: both reject, or both accept with an identical `Desc` and pixel buffer.
///
/// The caller must keep the stream's header dimensions bounded (memory) — see
/// [`bounded_stream`] and [`image_from_bytes`].
pub fn check_decode_agreement(bytes: &[u8], channels: u8) -> Result<(), String> {
    let rsd = qoi_rs::decode(bytes, channels);
    let sysd = qoi_sys::decode(bytes, channels as i32);
    match (rsd, sysd) {
        (Ok((da, pa)), Some((db, pb))) => {
            if !desc_eq(&da, &db) {
                return Err(format!(
                    "DECODE desc mismatch (ch{channels}): rs={:?} c=({},{},{},{})",
                    da, db.width, db.height, db.channels, db.colorspace
                ));
            }
            if pa != pb {
                return Err(format!(
                    "DECODE pixel mismatch (ch{channels}): rs={} c={} bytes, first diff @ {:?}",
                    pa.len(),
                    pb.len(),
                    first_diff(&pa, &pb)
                ));
            }
            Ok(())
        }
        (Err(_), None) => Ok(()),
        (a, b) => Err(format!(
            "DECODE accept/reject divergence (ch{channels}): rs_ok={} c_ok={}",
            a.is_ok(),
            b.is_some()
        )),
    }
}

/// Wrap an arbitrary chunk `body` in a QOI stream with a **bounded** header (small
/// width/height), so decoding allocates a small buffer. For fuzzing the decoder on
/// hostile chunk bytes without risking OOM.
pub fn bounded_stream(
    width: u16,
    height: u16,
    channels: u8,
    colorspace: u8,
    body: &[u8],
) -> Vec<u8> {
    let w = (width as u32 % 64) + 1; // 1..=64
    let h = (height as u32 % 64) + 1; // 1..=64
    let ch = if channels.is_multiple_of(2) { 4u8 } else { 3 };
    let cs = colorspace & 1;
    let mut v = Vec::with_capacity(QOI_HEADER_SIZE + body.len() + END_MARKER.len());
    v.extend_from_slice(b"qoif");
    v.extend_from_slice(&w.to_be_bytes());
    v.extend_from_slice(&h.to_be_bytes());
    v.push(ch);
    v.push(cs);
    v.extend_from_slice(body);
    v.extend_from_slice(&END_MARKER);
    v
}

/// Deterministically coerce arbitrary bytes into a small **valid** [`Image`]
/// (dimensions 1..=32) for the differential fuzz target. Returns `None` only if
/// there are too few bytes to choose a shape.
pub fn image_from_bytes(data: &[u8]) -> Option<Image> {
    if data.len() < 4 {
        return None;
    }
    let width = (data[0] as u32 % 32) + 1; // 1..=32
    let height = (data[1] as u32 % 32) + 1; // 1..=32
    let channels = if data[2] & 1 == 0 { 3u8 } else { 4 };
    let colorspace = data[3] & 1;
    let need = width as usize * height as usize * channels as usize;
    let body = &data[4..];
    let mut pixels = Vec::with_capacity(need);
    if body.is_empty() {
        pixels.resize(need, 0);
    } else {
        for i in 0..need {
            pixels.push(body[i % body.len()]);
        }
    }
    Some(Image {
        width,
        height,
        channels,
        colorspace,
        pixels,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_images_agree() {
        for &ch in &[3u8, 4] {
            for &(w, h) in &[(1u32, 1u32), (2, 3), (8, 8), (17, 5)] {
                let px: Vec<u8> = (0..(w * h * ch as u32))
                    .map(|i| (i * 13 + ch as u32) as u8)
                    .collect();
                let img = Image {
                    width: w,
                    height: h,
                    channels: ch,
                    colorspace: 0,
                    pixels: px,
                };
                assert!(img.is_valid());
                check_image(&img).unwrap();
            }
        }
    }

    #[test]
    fn coerced_images_agree() {
        for seed in 0u8..32 {
            let data: Vec<u8> = (0..200u32)
                .map(|i| (i as u8).wrapping_mul(seed.wrapping_add(1)))
                .collect();
            if let Some(img) = image_from_bytes(&data) {
                check_image(&img).unwrap();
            }
        }
    }

    #[test]
    fn arbitrary_decode_agrees() {
        for seed in 0u32..64 {
            let body: Vec<u8> = (0..80u32)
                .map(|i| (i.wrapping_mul(seed).wrapping_add(seed)) as u8)
                .collect();
            let stream = bounded_stream(
                seed as u16,
                (seed * 3) as u16,
                seed as u8,
                seed as u8,
                &body,
            );
            for ch in [0u8, 3, 4] {
                check_decode_agreement(&stream, ch).unwrap();
            }
        }
    }
}
