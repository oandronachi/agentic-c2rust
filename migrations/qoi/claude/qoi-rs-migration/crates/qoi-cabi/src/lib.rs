//! Outbound FFI: a small, stable C ABI over the safe [`qoi_rs`] port.
//!
//! Buffers returned by [`qoi_rs_encode`] / [`qoi_rs_decode`] are Rust-allocated and
//! **must** be released with [`qoi_rs_free`] (passing the same length that was
//! returned), so the Rust allocator frees exactly what it allocated. Lengths use
//! `size_t` (`usize`); `channels` uses `int` (`c_int`) to match the QOI convention.
//!
//! All `unsafe` lives in this boundary crate; the core remains
//! `#![forbid(unsafe_code)]`.

use qoi_rs::Desc;
use std::os::raw::c_int;
use std::ptr;

/// C-ABI image description (`#[repr(C)]`, fixed-width fields).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct QoiRsDesc {
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
    /// Channels: 3 (RGB) or 4 (RGBA).
    pub channels: u8,
    /// Colorspace: 0 (sRGB) or 1 (linear).
    pub colorspace: u8,
}

/// Encode raw pixels into a QOI stream.
///
/// `data`/`data_len` describe the input (`width*height*channels` bytes). On success
/// writes the output length to `*out_len` and returns a Rust-allocated buffer (free
/// with [`qoi_rs_free`]); on failure sets `*out_len = 0` and returns `NULL`.
///
/// # Safety
/// `data` must point to `data_len` readable bytes (or be NULL), `desc` to a valid
/// `QoiRsDesc`, and `out_len` to a writable `size_t`.
#[no_mangle]
pub unsafe extern "C" fn qoi_rs_encode(
    data: *const u8,
    data_len: usize,
    desc: *const QoiRsDesc,
    out_len: *mut usize,
) -> *mut u8 {
    if !out_len.is_null() {
        *out_len = 0;
    }
    if data.is_null() || desc.is_null() || out_len.is_null() {
        return ptr::null_mut();
    }
    let d = &*desc;
    let rdesc = Desc {
        width: d.width,
        height: d.height,
        channels: d.channels,
        colorspace: d.colorspace,
    };
    let input = std::slice::from_raw_parts(data, data_len);
    match qoi_rs::encode(input, &rdesc) {
        Ok(v) => {
            *out_len = v.len();
            // Hand ownership to C as a thin pointer; qoi_rs_free reconstructs the
            // boxed slice with this same length.
            Box::into_raw(v.into_boxed_slice()) as *mut u8
        }
        Err(_) => ptr::null_mut(),
    }
}

/// Decode a QOI stream into raw pixels.
///
/// `channels` is 0 (use the header), 3, or 4. On success fills `*out_desc`, writes
/// the pixel-buffer length to `*out_len`, and returns a Rust-allocated buffer (free
/// with [`qoi_rs_free`]); on failure sets `*out_len = 0` and returns `NULL`.
///
/// # Safety
/// `data` must point to `data_len` readable bytes (or be NULL); `out_desc` and
/// `out_len` must be writable.
#[no_mangle]
pub unsafe extern "C" fn qoi_rs_decode(
    data: *const u8,
    data_len: usize,
    out_desc: *mut QoiRsDesc,
    channels: c_int,
    out_len: *mut usize,
) -> *mut u8 {
    if !out_len.is_null() {
        *out_len = 0;
    }
    if data.is_null() || out_desc.is_null() || out_len.is_null() {
        return ptr::null_mut();
    }
    let ch = match channels {
        0 => 0u8,
        3 => 3,
        4 => 4,
        _ => return ptr::null_mut(),
    };
    let input = std::slice::from_raw_parts(data, data_len);
    match qoi_rs::decode(input, ch) {
        Ok((d, v)) => {
            *out_desc = QoiRsDesc {
                width: d.width,
                height: d.height,
                channels: d.channels,
                colorspace: d.colorspace,
            };
            *out_len = v.len();
            Box::into_raw(v.into_boxed_slice()) as *mut u8
        }
        Err(_) => ptr::null_mut(),
    }
}

/// Free a buffer returned by [`qoi_rs_encode`] or [`qoi_rs_decode`].
///
/// `len` must be the length that call reported. A NULL pointer or zero length is a
/// no-op.
///
/// # Safety
/// `ptr` must have come from one of this module's functions and not been freed
/// already, with `len` the length it returned.
#[no_mangle]
pub unsafe extern "C" fn qoi_rs_free(ptr: *mut u8, len: usize) {
    if ptr.is_null() || len == 0 {
        return;
    }
    // Rebuild the exact boxed slice (same len ⇒ same allocation layout) and drop it.
    let slice = ptr::slice_from_raw_parts_mut(ptr, len);
    drop(Box::from_raw(slice));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Drive the C ABI end to end (encode → decode → free) entirely from Rust and
    /// confirm it matches the safe API — proves the boundary without dlopen.
    #[test]
    fn c_abi_roundtrip_matches_safe_api() {
        let width = 5u32;
        let height = 3u32;
        let channels = 4u8;
        let pixels: Vec<u8> = (0..width * height * channels as u32)
            .map(|i| (i * 11 + 1) as u8)
            .collect();

        // Reference: the safe API.
        let safe_desc = Desc {
            width,
            height,
            channels,
            colorspace: 0,
        };
        let safe_enc = qoi_rs::encode(&pixels, &safe_desc).unwrap();

        unsafe {
            let cdesc = QoiRsDesc {
                width,
                height,
                channels,
                colorspace: 0,
            };
            let mut enc_len = 0usize;
            let enc_ptr = qoi_rs_encode(pixels.as_ptr(), pixels.len(), &cdesc, &mut enc_len);
            assert!(!enc_ptr.is_null());
            let enc = std::slice::from_raw_parts(enc_ptr, enc_len).to_vec();
            assert_eq!(enc, safe_enc, "C-ABI encode must match the safe API");

            let mut got_desc = QoiRsDesc {
                width: 0,
                height: 0,
                channels: 0,
                colorspace: 0,
            };
            let mut dec_len = 0usize;
            let dec_ptr = qoi_rs_decode(enc_ptr, enc_len, &mut got_desc, 0, &mut dec_len);
            assert!(!dec_ptr.is_null());
            let dec = std::slice::from_raw_parts(dec_ptr, dec_len).to_vec();
            assert_eq!(got_desc.width, width);
            assert_eq!(got_desc.height, height);
            assert_eq!(got_desc.channels, channels);
            assert_eq!(
                dec, pixels,
                "C-ABI decode must reproduce the original pixels"
            );

            qoi_rs_free(enc_ptr, enc_len);
            qoi_rs_free(dec_ptr, dec_len);
            // NULL / zero-length frees are no-ops.
            qoi_rs_free(ptr::null_mut(), 0);
        }
    }

    #[test]
    fn c_abi_rejects_bad_input() {
        unsafe {
            let mut out_len = 123usize;
            // NULL desc -> NULL, out_len zeroed.
            let p = qoi_rs_encode(ptr::null(), 0, ptr::null(), &mut out_len);
            assert!(p.is_null());
            assert_eq!(out_len, 0);

            // Bad channels on decode.
            let mut d = QoiRsDesc {
                width: 0,
                height: 0,
                channels: 0,
                colorspace: 0,
            };
            let mut dl = 9usize;
            let buf = [0u8; 30];
            let p2 = qoi_rs_decode(buf.as_ptr(), buf.len(), &mut d, 7, &mut dl);
            assert!(p2.is_null());
            assert_eq!(dl, 0);
        }
    }
}
