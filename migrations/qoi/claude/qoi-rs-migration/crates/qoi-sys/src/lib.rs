//! Inbound FFI: safe wrappers over the **reference** C QOI (vendored, compiled by
//! `build.rs`). This crate is the *ground truth* the safe port is compared against.
//!
//! All `unsafe` is confined here (and in `qoi-cabi`); the core `qoi-rs` crate is
//! `#![forbid(unsafe_code)]`. Each wrapper **copies** the C result into an owned
//! `Vec` and frees the C buffer with the matching allocator (`libc::free` ↔ the
//! library's `malloc`). We never adopt a C pointer into a `Vec`.
#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case)]

use std::os::raw::{c_int, c_void};

mod ffi {
    #![allow(dead_code)]
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

/// Image description mirroring `qoi_desc`. Kept independent of the port crate so
/// the ground truth shares no types with the implementation under test.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Desc {
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
    /// Channels: 3 (RGB) or 4 (RGBA).
    pub channels: u8,
    /// Colorspace: 0 (sRGB) or 1 (linear).
    pub colorspace: u8,
}

/// Encode raw pixels with the reference `qoi_encode`.
///
/// Returns the encoded bytes (copied out of the C `malloc` buffer, then freed), or
/// `None` if the reference returned `NULL` (invalid parameters / allocation
/// failure). Refuses to call the C if `data` is too short for `desc`, so the
/// reference can never read out of bounds.
pub fn encode(data: &[u8], desc: &Desc) -> Option<Vec<u8>> {
    // Guard: the reference reads width*height*channels bytes for a plausible desc.
    // Never let it read past `data`. (u64 math avoids overflow.)
    let needed = desc.width as u64 * desc.height as u64 * desc.channels as u64;
    if (data.len() as u64) < needed {
        return None;
    }

    let cdesc = ffi::qoi_desc {
        width: desc.width,
        height: desc.height,
        channels: desc.channels,
        colorspace: desc.colorspace,
    };
    let mut out_len: c_int = 0;

    // SAFETY: `data` has >= `needed` bytes (checked above); `&cdesc` is a valid
    // pointer to an initialized qoi_desc; `&mut out_len` is valid. qoi_encode reads
    // `data`, allocates a fresh buffer via malloc, writes its length to out_len, and
    // returns it (or NULL). We copy `out_len` bytes out and free with libc::free,
    // the allocator that matches the library's QOI_MALLOC (= malloc).
    unsafe {
        let ptr = ffi::qoi_encode(data.as_ptr() as *const c_void, &cdesc, &mut out_len);
        if ptr.is_null() {
            return None;
        }
        let len = out_len as usize;
        let v = std::slice::from_raw_parts(ptr as *const u8, len).to_vec();
        libc::free(ptr as *mut c_void);
        Some(v)
    }
}

/// Decode a QOI stream with the reference `qoi_decode`.
///
/// `channels` is `0` (use the header's count), `3`, or `4`. Returns the header
/// [`Desc`] and the decoded pixels (copied out, C buffer freed), or `None` on a
/// `NULL` return.
pub fn decode(data: &[u8], channels: i32) -> Option<(Desc, Vec<u8>)> {
    let mut cdesc = ffi::qoi_desc {
        width: 0,
        height: 0,
        channels: 0,
        colorspace: 0,
    };

    // SAFETY: qoi_decode reads at most `size` bytes of `data` (it bounds-checks
    // internally and never reads past `size`); it writes the header into `cdesc`
    // and returns a fresh malloc buffer of width*height*out_channels bytes (or
    // NULL). We size the copy with the same out_channels rule the C uses, copy out,
    // and free with libc::free.
    unsafe {
        let ptr = ffi::qoi_decode(
            data.as_ptr() as *const c_void,
            data.len() as c_int,
            &mut cdesc,
            channels as c_int,
        );
        if ptr.is_null() {
            return None;
        }
        let out_ch = if channels == 0 {
            cdesc.channels as i32
        } else {
            channels
        };
        let px_len = cdesc.width as usize * cdesc.height as usize * out_ch as usize;
        let v = std::slice::from_raw_parts(ptr as *const u8, px_len).to_vec();
        libc::free(ptr as *mut c_void);
        let desc = Desc {
            width: cdesc.width,
            height: cdesc.height,
            channels: cdesc.channels,
            colorspace: cdesc.colorspace,
        };
        Some((desc, v))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_roundtrip() {
        // Ground-truth round trip through the real C: encode then decode.
        let desc = Desc {
            width: 4,
            height: 2,
            channels: 4,
            colorspace: 0,
        };
        let pixels: Vec<u8> = (0..4 * 2 * 4).map(|i| (i * 7) as u8).collect();
        let enc = encode(&pixels, &desc).expect("C encode");
        assert_eq!(&enc[0..4], b"qoif");
        assert_eq!(&enc[enc.len() - 8..], &[0, 0, 0, 0, 0, 0, 0, 1]);
        let (gd, gp) = decode(&enc, 0).expect("C decode");
        assert_eq!(gd, desc);
        assert_eq!(gp, pixels);
    }

    #[test]
    fn reference_rejects_garbage() {
        assert!(decode(&[0u8; 10], 0).is_none()); // too short
        assert!(encode(
            &[0u8; 3],
            &Desc {
                width: 0,
                height: 1,
                channels: 3,
                colorspace: 0
            }
        )
        .is_none());
    }
}
