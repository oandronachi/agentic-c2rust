use qoi_rs::{decode, encode, Desc};
use std::ptr;
use std::slice;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QoiRsDesc {
    pub width: u32,
    pub height: u32,
    pub channels: u8,
    pub colorspace: u8,
}

impl From<QoiRsDesc> for Desc {
    fn from(desc: QoiRsDesc) -> Self {
        Self {
            width: desc.width,
            height: desc.height,
            channels: desc.channels,
            colorspace: desc.colorspace,
        }
    }
}

impl From<Desc> for QoiRsDesc {
    fn from(desc: Desc) -> Self {
        Self {
            width: desc.width,
            height: desc.height,
            channels: desc.channels,
            colorspace: desc.colorspace,
        }
    }
}

/// # Safety
/// `ptr` must point to `len` readable bytes. `desc` and `out_len` must be valid
/// non-null pointers. The returned pointer must be freed with `qoi_rs_free`.
#[no_mangle]
pub unsafe extern "C" fn qoi_rs_encode(
    ptr: *const u8,
    len: usize,
    desc: *const QoiRsDesc,
    out_len: *mut usize,
) -> *mut u8 {
    if ptr.is_null() || desc.is_null() || out_len.is_null() {
        return ptr::null_mut();
    }
    let input = unsafe { slice::from_raw_parts(ptr, len) };
    let desc = unsafe { *desc }.into();
    match encode(input, &desc) {
        Ok(bytes) => into_raw(bytes, out_len),
        Err(_) => ptr::null_mut(),
    }
}

/// # Safety
/// `ptr` must point to `len` readable bytes. `out_desc` and `out_len` must be valid
/// non-null pointers. The returned pointer must be freed with `qoi_rs_free`.
#[no_mangle]
pub unsafe extern "C" fn qoi_rs_decode(
    ptr: *const u8,
    len: usize,
    channels: u8,
    out_desc: *mut QoiRsDesc,
    out_len: *mut usize,
) -> *mut u8 {
    if ptr.is_null() || out_desc.is_null() || out_len.is_null() {
        return ptr::null_mut();
    }
    let input = unsafe { slice::from_raw_parts(ptr, len) };
    match decode(input, channels) {
        Ok((desc, bytes)) => {
            unsafe { *out_desc = desc.into() };
            into_raw(bytes, out_len)
        }
        Err(_) => ptr::null_mut(),
    }
}

/// # Safety
/// `ptr` and `len` must be exactly the pointer and length returned by this library.
#[no_mangle]
pub unsafe extern "C" fn qoi_rs_free(ptr: *mut u8, len: usize) {
    if ptr.is_null() || len == 0 {
        return;
    }
    unsafe {
        drop(Box::from_raw(ptr::slice_from_raw_parts_mut(ptr, len)));
    }
}

fn into_raw(bytes: Vec<u8>, out_len: *mut usize) -> *mut u8 {
    let boxed = bytes.into_boxed_slice();
    let len = boxed.len();
    let ptr = Box::into_raw(boxed).cast::<u8>();
    unsafe { *out_len = len };
    ptr
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abi_roundtrip() {
        let desc = QoiRsDesc {
            width: 2,
            height: 1,
            channels: 4,
            colorspace: 0,
        };
        let pixels = [255, 0, 0, 255, 0, 255, 0, 255];
        let mut encoded_len = 0usize;
        let encoded = unsafe {
            qoi_rs_encode(pixels.as_ptr(), pixels.len(), &desc, &mut encoded_len)
        };
        assert!(!encoded.is_null());
        let encoded_slice = unsafe { slice::from_raw_parts(encoded, encoded_len) };
        let mut out_desc = QoiRsDesc {
            width: 0,
            height: 0,
            channels: 0,
            colorspace: 0,
        };
        let mut decoded_len = 0usize;
        let decoded = unsafe {
            qoi_rs_decode(
                encoded_slice.as_ptr(),
                encoded_slice.len(),
                0,
                &mut out_desc,
                &mut decoded_len,
            )
        };
        assert_eq!(out_desc, desc);
        assert_eq!(unsafe { slice::from_raw_parts(decoded, decoded_len) }, pixels);
        unsafe {
            qoi_rs_free(encoded, encoded_len);
            qoi_rs_free(decoded, decoded_len);
        }
    }
}
