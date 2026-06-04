//! Safe, dependency-free Rust port of the **QOI** ("Quite OK Image") codec.
//!
//! This is a from-scratch reimplementation against the QOI specification and the
//! reference C (`vendor/qoi/qoi.h`,
//! <https://github.com/phoboslab/qoi> @ `97bacc86`). It is verified **byte-exact**
//! against the reference by the `qoi-diff` differential oracle.
//!
//! * No `unsafe` — the crate is `#![forbid(unsafe_code)]`.
//! * No dependencies.
//! * Total on hostile input: [`decode`] never panics and never reads out of
//!   bounds; a truncated or malformed stream yields a (possibly wrong) image
//!   rather than a crash, exactly as the C decoder does.
//!
//! ```
//! use qoi_rs::{encode, decode, Desc, QOI_SRGB};
//! let desc = Desc { width: 2, height: 1, channels: 4, colorspace: QOI_SRGB };
//! let pixels = [10, 20, 30, 255,  10, 20, 30, 255];
//! let qoi = encode(&pixels, &desc).unwrap();
//! let (got_desc, got_pixels) = decode(&qoi, 0).unwrap();
//! assert_eq!(got_desc, desc);
//! assert_eq!(got_pixels, pixels);
//! ```
#![forbid(unsafe_code)]
#![deny(missing_docs)]

use core::fmt;

/// Colorspace tag: sRGB with a linear alpha channel. Purely informative.
pub const QOI_SRGB: u8 = 0;
/// Colorspace tag: all channels linear. Purely informative.
pub const QOI_LINEAR: u8 = 1;

/// Size of the QOI header in bytes (magic + width + height + channels + colorspace).
pub const QOI_HEADER_SIZE: usize = 14;

/// The 8-byte end-of-stream marker.
const QOI_PADDING: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 1];

/// `"qoif"` magic, big-endian.
const QOI_MAGIC: u32 = u32::from_be_bytes(*b"qoif");

/// Maximum number of pixels the format guards against (`width*height`), matching
/// the C reference's `QOI_PIXELS_MAX`. Caps the worst-case output below `i32::MAX`.
pub const QOI_PIXELS_MAX: u32 = 400_000_000;

// Chunk tags (see the format documentation in `vendor/qoi/qoi.h`).
const QOI_OP_INDEX: u8 = 0x00; // 00xxxxxx
const QOI_OP_DIFF: u8 = 0x40; // 01xxxxxx
const QOI_OP_LUMA: u8 = 0x80; // 10xxxxxx
const QOI_OP_RUN: u8 = 0xc0; // 11xxxxxx
const QOI_OP_RGB: u8 = 0xfe; // 11111110
const QOI_OP_RGBA: u8 = 0xff; // 11111111
const QOI_MASK_2: u8 = 0xc0; // 11000000

/// Image description: the 14-byte QOI header, decoded.
///
/// For [`encode`] this is the input format; for [`decode`] it is filled from the
/// stream header.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct Desc {
    /// Image width in pixels (must be non-zero).
    pub width: u32,
    /// Image height in pixels (must be non-zero).
    pub height: u32,
    /// Number of channels: 3 (RGB) or 4 (RGBA).
    pub channels: u8,
    /// Colorspace: [`QOI_SRGB`] (0) or [`QOI_LINEAR`] (1).
    pub colorspace: u8,
}

/// Errors returned by [`encode`] and [`decode`].
///
/// The C reference signals every failure with a `NULL` return; this enum names the
/// cause. For the differential oracle, "C returned NULL" corresponds to "Rust
/// returned `Err`"; the specific variant is informational.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum Error {
    /// `width` or `height` was zero.
    BadDimensions,
    /// `channels` was not 3 or 4 (encode), or not 0/3/4 (decode parameter).
    BadChannels,
    /// `colorspace` was greater than 1.
    BadColorspace,
    /// `width * height` met or exceeded [`QOI_PIXELS_MAX`] (overflow guard).
    TooManyPixels,
    /// Encode: `data.len()` did not equal `width * height * channels`.
    InputLength,
    /// Decode: the stream was shorter than [`QOI_HEADER_SIZE`] + 8 padding bytes.
    Truncated,
    /// Decode: the 4-byte magic was not `"qoif"`.
    BadMagic,
    /// An output buffer of the required size could not be allocated.
    AllocFailed,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Error::BadDimensions => "width and height must be non-zero",
            Error::BadChannels => "channels must be 3 or 4 (0/3/4 when decoding)",
            Error::BadColorspace => "colorspace must be 0 or 1",
            Error::TooManyPixels => "width*height exceeds QOI_PIXELS_MAX",
            Error::InputLength => "input length != width*height*channels",
            Error::Truncated => "stream shorter than the QOI header + padding",
            Error::BadMagic => "bad magic: not a QOI stream",
            Error::AllocFailed => "could not allocate the output buffer",
        };
        f.write_str(s)
    }
}

impl std::error::Error for Error {}

/// One pixel. Equality over all four channels mirrors the C `qoi_rgba_t.v` union
/// compare.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Rgba {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl Rgba {
    #[inline]
    fn hash(self) -> usize {
        // (r*3 + g*5 + b*7 + a*11) & 63, computed wide then reduced — matches
        // QOI_COLOR_HASH(C) & (64 - 1).
        (self.r as usize * 3 + self.g as usize * 5 + self.b as usize * 7 + self.a as usize * 11)
            & 63
    }
}

/// Validate a [`Desc`] the way the C encoder/decoder header check does.
///
/// `width==0` is checked before the division so the overflow guard never divides
/// by zero (matching the C short-circuit order).
#[inline]
fn validate_desc(width: u32, height: u32, channels: u8, colorspace: u8) -> Result<(), Error> {
    if width == 0 || height == 0 {
        return Err(Error::BadDimensions);
    }
    if !(3..=4).contains(&channels) {
        return Err(Error::BadChannels);
    }
    if colorspace > 1 {
        return Err(Error::BadColorspace);
    }
    if height >= QOI_PIXELS_MAX / width {
        return Err(Error::TooManyPixels);
    }
    Ok(())
}

/// Encode raw RGB/RGBA pixels into a QOI byte stream.
///
/// `data` must be exactly `desc.width * desc.height * desc.channels` bytes,
/// row-major, top-to-bottom. Returns the encoded stream, or an [`Error`] for an
/// invalid `desc` or a mismatched `data` length.
///
/// Byte-for-byte identical to the reference `qoi_encode` for every valid input.
pub fn encode(data: &[u8], desc: &Desc) -> Result<Vec<u8>, Error> {
    validate_desc(desc.width, desc.height, desc.channels, desc.colorspace)?;

    let channels = desc.channels as usize;
    let width = desc.width as usize;
    let height = desc.height as usize;
    let px_len = width * height * channels;
    if data.len() != px_len {
        return Err(Error::InputLength);
    }

    let max_size = width * height * (channels + 1) + QOI_HEADER_SIZE + QOI_PADDING.len();
    let mut bytes: Vec<u8> = Vec::new();
    bytes
        .try_reserve_exact(max_size)
        .map_err(|_| Error::AllocFailed)?;

    // Header.
    bytes.extend_from_slice(&QOI_MAGIC.to_be_bytes());
    bytes.extend_from_slice(&desc.width.to_be_bytes());
    bytes.extend_from_slice(&desc.height.to_be_bytes());
    bytes.push(desc.channels);
    bytes.push(desc.colorspace);

    let mut index = [Rgba {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    }; 64];
    let mut run: u8 = 0;
    let mut px_prev = Rgba {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    };
    let mut px = px_prev;

    let px_end = px_len - channels;
    let mut px_pos = 0usize;
    while px_pos < px_len {
        px.r = data[px_pos];
        px.g = data[px_pos + 1];
        px.b = data[px_pos + 2];
        if channels == 4 {
            px.a = data[px_pos + 3];
        }

        if px == px_prev {
            run += 1;
            if run == 62 || px_pos == px_end {
                bytes.push(QOI_OP_RUN | (run - 1));
                run = 0;
            }
        } else {
            if run > 0 {
                bytes.push(QOI_OP_RUN | (run - 1));
                run = 0;
            }

            let index_pos = px.hash();
            if index[index_pos] == px {
                bytes.push(QOI_OP_INDEX | index_pos as u8);
            } else {
                index[index_pos] = px;

                if px.a == px_prev.a {
                    let vr = px.r.wrapping_sub(px_prev.r) as i8;
                    let vg = px.g.wrapping_sub(px_prev.g) as i8;
                    let vb = px.b.wrapping_sub(px_prev.b) as i8;

                    let vg_r = vr.wrapping_sub(vg);
                    let vg_b = vb.wrapping_sub(vg);

                    if (-2..=1).contains(&vr) && (-2..=1).contains(&vg) && (-2..=1).contains(&vb) {
                        bytes.push(
                            QOI_OP_DIFF
                                | (((vr + 2) as u8) << 4)
                                | (((vg + 2) as u8) << 2)
                                | ((vb + 2) as u8),
                        );
                    } else if (-8..=7).contains(&vg_r)
                        && (-32..=31).contains(&vg)
                        && (-8..=7).contains(&vg_b)
                    {
                        bytes.push(QOI_OP_LUMA | ((vg + 32) as u8));
                        bytes.push((((vg_r + 8) as u8) << 4) | ((vg_b + 8) as u8));
                    } else {
                        bytes.push(QOI_OP_RGB);
                        bytes.push(px.r);
                        bytes.push(px.g);
                        bytes.push(px.b);
                    }
                } else {
                    bytes.push(QOI_OP_RGBA);
                    bytes.push(px.r);
                    bytes.push(px.g);
                    bytes.push(px.b);
                    bytes.push(px.a);
                }
            }
        }
        px_prev = px;
        px_pos += channels;
    }

    bytes.extend_from_slice(&QOI_PADDING);
    Ok(bytes)
}

/// Decode a QOI byte stream into raw pixels.
///
/// `channels` selects the output format: `0` uses the stream's own channel count,
/// while `3` or `4` forces RGB or RGBA. Returns the decoded [`Desc`] (from the
/// header) and the pixel buffer (`width * height * out_channels` bytes).
///
/// Total on adversarial input: every read is bounds-checked, so a truncated or
/// malformed stream produces a (possibly meaningless) image — never a panic and
/// never an out-of-bounds access — matching the reference `qoi_decode`.
pub fn decode(data: &[u8], channels: u8) -> Result<(Desc, Vec<u8>), Error> {
    if channels != 0 && channels != 3 && channels != 4 {
        return Err(Error::BadChannels);
    }
    if data.len() < QOI_HEADER_SIZE + QOI_PADDING.len() {
        return Err(Error::Truncated);
    }

    // data.len() >= 22, so these fixed-offset header reads cannot panic.
    let magic = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
    let width = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
    let height = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);
    let hdr_channels = data[12];
    let colorspace = data[13];

    if magic != QOI_MAGIC {
        return Err(Error::BadMagic);
    }
    validate_desc(width, height, hdr_channels, colorspace)?;

    let out_channels = if channels == 0 {
        hdr_channels
    } else {
        channels
    } as usize;
    let px_len = width as usize * height as usize * out_channels;

    let mut pixels: Vec<u8> = Vec::new();
    pixels
        .try_reserve_exact(px_len)
        .map_err(|_| Error::AllocFailed)?;
    pixels.resize(px_len, 0);

    let mut index = [Rgba {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    }; 64];
    let mut px = Rgba {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    };

    let chunks_len = data.len() - QOI_PADDING.len();
    let mut p = QOI_HEADER_SIZE;
    let mut run: u32 = 0;

    let mut px_pos = 0usize;
    while px_pos < px_len {
        if run > 0 {
            run -= 1;
        } else if p < chunks_len {
            // Bounds proof: p < chunks_len = len-8, so the longest chunk (RGBA: b1
            // plus 4 bytes) reads at most index (len-9)+4 = len-5 < len. No panic,
            // and we read exactly the bytes the C decoder reads (including into the
            // padding region for adversarial streams), preserving byte-exactness.
            let b1 = data[p];
            p += 1;

            if b1 == QOI_OP_RGB {
                px.r = data[p];
                px.g = data[p + 1];
                px.b = data[p + 2];
                p += 3;
            } else if b1 == QOI_OP_RGBA {
                px.r = data[p];
                px.g = data[p + 1];
                px.b = data[p + 2];
                px.a = data[p + 3];
                p += 4;
            } else if (b1 & QOI_MASK_2) == QOI_OP_INDEX {
                px = index[(b1 & 0x3f) as usize];
            } else if (b1 & QOI_MASK_2) == QOI_OP_DIFF {
                px.r = px.r.wrapping_add(((b1 >> 4) & 0x03).wrapping_sub(2));
                px.g = px.g.wrapping_add(((b1 >> 2) & 0x03).wrapping_sub(2));
                px.b = px.b.wrapping_add((b1 & 0x03).wrapping_sub(2));
            } else if (b1 & QOI_MASK_2) == QOI_OP_LUMA {
                let b2 = data[p];
                p += 1;
                let vg = (b1 & 0x3f).wrapping_sub(32);
                px.r =
                    px.r.wrapping_add(vg.wrapping_sub(8).wrapping_add((b2 >> 4) & 0x0f));
                px.g = px.g.wrapping_add(vg);
                px.b =
                    px.b.wrapping_add(vg.wrapping_sub(8).wrapping_add(b2 & 0x0f));
            } else if (b1 & QOI_MASK_2) == QOI_OP_RUN {
                run = (b1 & 0x3f) as u32;
            }

            index[px.hash()] = px;
        }

        pixels[px_pos] = px.r;
        pixels[px_pos + 1] = px.g;
        pixels[px_pos + 2] = px.b;
        if out_channels == 4 {
            pixels[px_pos + 3] = px.a;
        }
        px_pos += out_channels;
    }

    Ok((
        Desc {
            width,
            height,
            channels: hdr_channels,
            colorspace,
        },
        pixels,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn desc(w: u32, h: u32, ch: u8) -> Desc {
        Desc {
            width: w,
            height: h,
            channels: ch,
            colorspace: QOI_SRGB,
        }
    }

    #[test]
    fn roundtrip_solid_run() {
        // All-identical pixels exercise the RUN path incl. the 62 boundary.
        let d = desc(200, 1, 4);
        let px = vec![7u8; 200 * 4]
            .chunks(4)
            .flat_map(|_| [7, 8, 9, 255])
            .collect::<Vec<u8>>();
        let enc = encode(&px, &d).unwrap();
        let (gd, gp) = decode(&enc, 0).unwrap();
        assert_eq!(gd, d);
        assert_eq!(gp, px);
    }

    #[test]
    fn roundtrip_gradient_diff_luma() {
        // A small gradient hits DIFF and LUMA.
        let d = desc(64, 1, 3);
        let mut px = Vec::new();
        for i in 0..64u32 {
            px.push(i as u8);
            px.push((i * 2) as u8);
            px.push((i * 3) as u8);
        }
        let enc = encode(&px, &d).unwrap();
        let (gd, gp) = decode(&enc, 0).unwrap();
        assert_eq!(gd, d);
        assert_eq!(gp, px);
    }

    #[test]
    fn roundtrip_rgba_alpha_changes() {
        let d = desc(4, 1, 4);
        let px = vec![1, 2, 3, 10, 1, 2, 3, 20, 9, 9, 9, 30, 0, 0, 0, 255];
        let enc = encode(&px, &d).unwrap();
        let (gd, gp) = decode(&enc, 0).unwrap();
        assert_eq!(gd, d);
        assert_eq!(gp, px);
    }

    #[test]
    fn header_and_padding_structural() {
        let d = desc(3, 2, 3);
        let px = vec![5u8; 3 * 2 * 3];
        let enc = encode(&px, &d).unwrap();
        assert_eq!(&enc[0..4], b"qoif");
        assert_eq!(u32::from_be_bytes([enc[4], enc[5], enc[6], enc[7]]), 3);
        assert_eq!(u32::from_be_bytes([enc[8], enc[9], enc[10], enc[11]]), 2);
        assert_eq!(enc[12], 3);
        assert_eq!(enc[13], QOI_SRGB);
        assert_eq!(&enc[enc.len() - 8..], &QOI_PADDING);
    }

    #[test]
    fn force_channels_3_to_4_and_back() {
        let d = desc(2, 2, 3);
        let px = vec![10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110, 120];
        let enc = encode(&px, &d).unwrap();
        // Force RGBA out: alpha is the running 255.
        let (_, p4) = decode(&enc, 4).unwrap();
        assert_eq!(p4.len(), 2 * 2 * 4);
        for chunk in p4.chunks(4) {
            assert_eq!(chunk[3], 255);
        }
        // Force RGB out from the same stream.
        let (_, p3) = decode(&enc, 3).unwrap();
        assert_eq!(p3, px);
    }

    #[test]
    fn encode_rejects_bad_desc() {
        let px = vec![0u8; 3];
        assert_eq!(encode(&px, &desc(0, 1, 3)), Err(Error::BadDimensions));
        assert_eq!(encode(&px, &desc(1, 1, 2)), Err(Error::BadChannels));
        assert_eq!(
            encode(
                &px,
                &Desc {
                    width: 1,
                    height: 1,
                    channels: 3,
                    colorspace: 2
                }
            ),
            Err(Error::BadColorspace)
        );
        // length mismatch
        assert_eq!(encode(&[0u8; 2], &desc(1, 1, 3)), Err(Error::InputLength));
    }

    #[test]
    fn encode_rejects_overflow_guard() {
        // width*height >= 400_000_000 must be rejected (no allocation attempted).
        assert_eq!(
            encode(&[], &desc(20_000, 20_000, 4)),
            Err(Error::TooManyPixels)
        );
    }

    #[test]
    fn decode_rejects_short_and_bad_magic() {
        assert_eq!(decode(&[0u8; 10], 0), Err(Error::Truncated));
        let mut bad = vec![0u8; 30];
        bad[0..4].copy_from_slice(b"xxxx");
        assert_eq!(decode(&bad, 0), Err(Error::BadMagic));
        assert_eq!(decode(&[0u8; 30], 7), Err(Error::BadChannels));
    }

    #[test]
    fn decode_truncated_stream_is_total() {
        // A valid header followed by no chunks: decoder fills with the seed/last
        // pixel and must not panic.
        let d = desc(10, 10, 4);
        let px = vec![123u8; 10 * 10 * 4];
        let mut enc = encode(&px, &d).unwrap();
        enc.truncate(QOI_HEADER_SIZE + 8); // header + padding only
        let (gd, gp) = decode(&enc, 0).unwrap();
        assert_eq!(gd, d);
        assert_eq!(gp.len(), 10 * 10 * 4);
    }
}
