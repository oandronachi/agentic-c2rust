use libc::{c_int, c_void};

mod bindings {
    #![allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Desc {
    pub width: u32,
    pub height: u32,
    pub channels: u8,
    pub colorspace: u8,
}

impl From<Desc> for bindings::qoi_desc {
    fn from(desc: Desc) -> Self {
        Self {
            width: desc.width,
            height: desc.height,
            channels: desc.channels,
            colorspace: desc.colorspace,
        }
    }
}

impl From<bindings::qoi_desc> for Desc {
    fn from(desc: bindings::qoi_desc) -> Self {
        Self {
            width: desc.width,
            height: desc.height,
            channels: desc.channels as u8,
            colorspace: desc.colorspace as u8,
        }
    }
}

pub fn encode(pixels: &[u8], desc: Desc) -> Option<Vec<u8>> {
    let c_desc: bindings::qoi_desc = desc.into();
    let mut out_len = 0i32;
    // SAFETY: `pixels.as_ptr()` is valid for the duration of the call, `c_desc` and
    // `out_len` are valid pointers, and a non-null result is copied before being freed
    // with the same C allocator (`free`) used by qoi.h.
    unsafe {
        let ptr = bindings::qoi_encode(
            pixels.as_ptr().cast::<c_void>(),
            &c_desc,
            &mut out_len as *mut c_int,
        );
        if ptr.is_null() || out_len <= 0 {
            return None;
        }
        let bytes = std::slice::from_raw_parts(ptr.cast::<u8>(), out_len as usize).to_vec();
        libc::free(ptr);
        Some(bytes)
    }
}

pub fn decode(data: &[u8], channels: u8) -> Option<(Desc, Vec<u8>)> {
    let size: c_int = data.len().try_into().ok()?;
    let mut desc = bindings::qoi_desc {
        width: 0,
        height: 0,
        channels: 0,
        colorspace: 0,
    };
    // SAFETY: `data` and `desc` are valid for the call. A non-null returned buffer is
    // copied into a Rust Vec and released with the same C allocator (`free`).
    unsafe {
        let ptr = bindings::qoi_decode(
            data.as_ptr().cast::<c_void>(),
            size,
            &mut desc as *mut bindings::qoi_desc,
            c_int::from(channels),
        );
        if ptr.is_null() {
            return None;
        }
        let out_desc = Desc::from(desc);
        let out_channels = if channels == 0 {
            out_desc.channels
        } else {
            channels
        } as usize;
        let len = out_desc.width as usize * out_desc.height as usize * out_channels;
        let pixels = std::slice::from_raw_parts(ptr.cast::<u8>(), len).to_vec();
        libc::free(ptr);
        Some((out_desc, pixels))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_roundtrip() {
        let desc = Desc {
            width: 2,
            height: 1,
            channels: 4,
            colorspace: 0,
        };
        let pixels = [255, 0, 0, 255, 0, 255, 0, 255];
        let encoded = encode(&pixels, desc).unwrap();
        let (decoded_desc, decoded) = decode(&encoded, 0).unwrap();
        assert_eq!(decoded_desc, desc);
        assert_eq!(decoded, pixels);
    }

    #[test]
    fn reference_rejects_bad_input() {
        assert!(decode(&[0; 4], 0).is_none());
    }
}
