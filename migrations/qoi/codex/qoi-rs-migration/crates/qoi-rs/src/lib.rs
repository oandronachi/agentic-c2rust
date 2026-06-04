#![forbid(unsafe_code)]

use std::fmt;

pub const SRGB: u8 = 0;
pub const LINEAR: u8 = 1;
pub const PIXELS_MAX: u32 = 400_000_000;

const HEADER_SIZE: usize = 14;
const PADDING: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 1];
const MAGIC: u32 = u32::from_be_bytes(*b"qoif");

const OP_INDEX: u8 = 0x00;
const OP_DIFF: u8 = 0x40;
const OP_LUMA: u8 = 0x80;
const OP_RUN: u8 = 0xc0;
const OP_RGB: u8 = 0xfe;
const OP_RGBA: u8 = 0xff;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Desc {
    pub width: u32,
    pub height: u32,
    pub channels: u8,
    pub colorspace: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    BadDesc,
    BadInputLength { expected: usize, got: usize },
    BadMagic,
    Truncated,
    BadChannels,
    Alloc,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::BadDesc => f.write_str("invalid QOI image description"),
            Error::BadInputLength { expected, got } => {
                write!(f, "pixel buffer length {got} != expected {expected}")
            }
            Error::BadMagic => f.write_str("invalid QOI magic"),
            Error::Truncated => f.write_str("QOI input is shorter than header plus padding"),
            Error::BadChannels => f.write_str("requested output channels must be 0, 3, or 4"),
            Error::Alloc => f.write_str("allocation failed"),
        }
    }
}

impl std::error::Error for Error {}

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
        ((self.r as u32 * 3 + self.g as u32 * 5 + self.b as u32 * 7 + self.a as u32 * 11)
            & 63) as usize
    }
}

pub fn encode(pixels: &[u8], desc: &Desc) -> Result<Vec<u8>, Error> {
    validate_desc(desc)?;
    let channels = desc.channels as usize;
    let px_len = desc.width as usize * desc.height as usize * channels;
    if pixels.len() != px_len {
        return Err(Error::BadInputLength {
            expected: px_len,
            got: pixels.len(),
        });
    }

    let max_size =
        desc.width as usize * desc.height as usize * (channels + 1) + HEADER_SIZE + PADDING.len();
    let mut out = Vec::new();
    out.try_reserve_exact(max_size).map_err(|_| Error::Alloc)?;

    out.extend_from_slice(&MAGIC.to_be_bytes());
    out.extend_from_slice(&desc.width.to_be_bytes());
    out.extend_from_slice(&desc.height.to_be_bytes());
    out.push(desc.channels);
    out.push(desc.colorspace);

    let mut index = [Rgba {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    }; 64];
    let mut prev = Rgba {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    };
    let mut run = 0u32;
    let px_end = px_len - channels;
    let mut pos = 0usize;

    while pos < px_len {
        let px = Rgba {
            r: pixels[pos],
            g: pixels[pos + 1],
            b: pixels[pos + 2],
            a: if channels == 4 {
                pixels[pos + 3]
            } else {
                prev.a
            },
        };

        if px == prev {
            run += 1;
            if run == 62 || pos == px_end {
                out.push(OP_RUN | (run - 1) as u8);
                run = 0;
            }
        } else {
            if run > 0 {
                out.push(OP_RUN | (run - 1) as u8);
                run = 0;
            }

            let index_pos = px.hash();
            if index[index_pos] == px {
                out.push(OP_INDEX | index_pos as u8);
            } else {
                index[index_pos] = px;
                if px.a == prev.a {
                    let vr = px.r.wrapping_sub(prev.r) as i8;
                    let vg = px.g.wrapping_sub(prev.g) as i8;
                    let vb = px.b.wrapping_sub(prev.b) as i8;
                    let vg_r = vr.wrapping_sub(vg);
                    let vg_b = vb.wrapping_sub(vg);

                    if vr > -3 && vr < 2 && vg > -3 && vg < 2 && vb > -3 && vb < 2 {
                        out.push(
                            OP_DIFF
                                | ((vr + 2) as u8) << 4
                                | ((vg + 2) as u8) << 2
                                | (vb + 2) as u8,
                        );
                    } else if vg_r > -9
                        && vg_r < 8
                        && vg > -33
                        && vg < 32
                        && vg_b > -9
                        && vg_b < 8
                    {
                        out.push(OP_LUMA | (vg + 32) as u8);
                        out.push(((vg_r + 8) as u8) << 4 | (vg_b + 8) as u8);
                    } else {
                        out.push(OP_RGB);
                        out.extend_from_slice(&[px.r, px.g, px.b]);
                    }
                } else {
                    out.push(OP_RGBA);
                    out.extend_from_slice(&[px.r, px.g, px.b, px.a]);
                }
            }
        }

        prev = px;
        pos += channels;
    }

    out.extend_from_slice(&PADDING);
    Ok(out)
}

pub fn decode(data: &[u8], channels: u8) -> Result<(Desc, Vec<u8>), Error> {
    if channels != 0 && channels != 3 && channels != 4 {
        return Err(Error::BadChannels);
    }
    if data.len() < HEADER_SIZE + PADDING.len() {
        return Err(Error::Truncated);
    }

    let mut p = 0usize;
    let magic = rd_be_u32(data, &mut p);
    let width = rd_be_u32(data, &mut p);
    let height = rd_be_u32(data, &mut p);
    let hdr_channels = rd(data, &mut p);
    let colorspace = rd(data, &mut p);

    if magic != MAGIC {
        return Err(Error::BadMagic);
    }
    let desc = Desc {
        width,
        height,
        channels: hdr_channels,
        colorspace,
    };
    validate_desc(&desc)?;

    let out_channels = if channels == 0 { hdr_channels } else { channels } as usize;
    let px_len = width as usize * height as usize * out_channels;
    let mut pixels = Vec::new();
    pixels.try_reserve_exact(px_len).map_err(|_| Error::Alloc)?;

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
    let chunks_len = data.len() - PADDING.len();
    let mut run = 0i32;

    let mut pos = 0usize;
    while pos < px_len {
        if run > 0 {
            run -= 1;
        } else if p < chunks_len {
            let b1 = rd(data, &mut p);
            if b1 == OP_RGB {
                px.r = rd(data, &mut p);
                px.g = rd(data, &mut p);
                px.b = rd(data, &mut p);
            } else if b1 == OP_RGBA {
                px.r = rd(data, &mut p);
                px.g = rd(data, &mut p);
                px.b = rd(data, &mut p);
                px.a = rd(data, &mut p);
            } else {
                match b1 >> 6 {
                    0 => px = index[(b1 & 0x3f) as usize],
                    1 => {
                        px.r = (px.r as i32 + (((b1 >> 4) & 0x03) as i32 - 2)) as u8;
                        px.g = (px.g as i32 + (((b1 >> 2) & 0x03) as i32 - 2)) as u8;
                        px.b = (px.b as i32 + ((b1 & 0x03) as i32 - 2)) as u8;
                    }
                    2 => {
                        let b2 = rd(data, &mut p);
                        let vg = (b1 & 0x3f) as i32 - 32;
                        px.r = (px.r as i32 + vg - 8 + ((b2 >> 4) & 0x0f) as i32) as u8;
                        px.g = (px.g as i32 + vg) as u8;
                        px.b = (px.b as i32 + vg - 8 + (b2 & 0x0f) as i32) as u8;
                    }
                    _ => run = (b1 & 0x3f) as i32,
                }
            }
            index[px.hash()] = px;
        }

        pixels.push(px.r);
        pixels.push(px.g);
        pixels.push(px.b);
        if out_channels == 4 {
            pixels.push(px.a);
        }
        pos += out_channels;
    }

    Ok((desc, pixels))
}

fn validate_desc(desc: &Desc) -> Result<(), Error> {
    if desc.width == 0
        || desc.height == 0
        || !(3..=4).contains(&desc.channels)
        || desc.colorspace > 1
        || desc.height >= PIXELS_MAX / desc.width
    {
        Err(Error::BadDesc)
    } else {
        Ok(())
    }
}

#[inline]
fn rd(data: &[u8], p: &mut usize) -> u8 {
    let b = data.get(*p).copied().unwrap_or(0);
    *p += 1;
    b
}

#[inline]
fn rd_be_u32(data: &[u8], p: &mut usize) -> u32 {
    u32::from_be_bytes([rd(data, p), rd(data, p), rd(data, p), rd(data, p)])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn desc(width: u32, height: u32, channels: u8) -> Desc {
        Desc {
            width,
            height,
            channels,
            colorspace: SRGB,
        }
    }

    #[test]
    fn roundtrip_rgba() {
        let d = desc(4, 2, 4);
        let pixels: Vec<u8> = (0..8)
            .flat_map(|i| [i * 13, i * 17, i * 19, 255])
            .collect();
        let encoded = encode(&pixels, &d).unwrap();
        let (got_desc, decoded) = decode(&encoded, 0).unwrap();
        assert_eq!(got_desc, d);
        assert_eq!(decoded, pixels);
    }

    #[test]
    fn force_channels() {
        let d = desc(2, 1, 3);
        let encoded = encode(&[1, 2, 3, 4, 5, 6], &d).unwrap();
        let (_, decoded) = decode(&encoded, 4).unwrap();
        assert_eq!(decoded, [1, 2, 3, 255, 4, 5, 6, 255]);
    }

    #[test]
    fn rejects_bad_inputs() {
        assert_eq!(encode(&[], &desc(0, 1, 4)), Err(Error::BadDesc));
        assert_eq!(decode(&[0; 8], 0), Err(Error::Truncated));
        assert_eq!(decode(&[0; 22], 0), Err(Error::BadMagic));
        assert_eq!(decode(&[0; 22], 2), Err(Error::BadChannels));
    }

    #[test]
    fn hostile_decode_does_not_panic() {
        for seed in 0..2000u32 {
            let mut data = Vec::new();
            data.extend_from_slice(b"qoif");
            data.extend_from_slice(&(1 + seed % 8).to_be_bytes());
            data.extend_from_slice(&(1 + (seed / 8) % 8).to_be_bytes());
            data.push(if seed & 1 == 0 { 3 } else { 4 });
            data.push((seed as u8) & 1);
            for i in 0..(seed as usize % 50) {
                data.push(seed.wrapping_mul(1_664_525).wrapping_add(i as u32) as u8);
            }
            let _ = decode(&data, 0);
            let _ = decode(&data, 3);
            let _ = decode(&data, 4);
        }
    }
}
