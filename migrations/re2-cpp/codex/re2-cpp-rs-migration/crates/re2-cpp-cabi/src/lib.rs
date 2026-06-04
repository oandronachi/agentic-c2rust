use re2_cpp_rs::Regex;
use std::{ptr, str};

#[repr(C)]
pub struct Re2CppRegex {
    _private: [u8; 0],
}

struct RegexHandle {
    inner: Regex,
}

/// Compiles a UTF-8 RE2 pattern and returns an owned regex handle.
///
/// # Safety
///
/// When `pattern_len` is nonzero, `pattern` must point to `pattern_len`
/// readable bytes for the duration of this call. The bytes must contain valid
/// UTF-8. The returned pointer must be released with `re2_cpp_rs_free`.
#[no_mangle]
pub unsafe extern "C" fn re2_cpp_rs_new(
    pattern: *const u8,
    pattern_len: usize,
) -> *mut Re2CppRegex {
    if pattern.is_null() && pattern_len != 0 {
        return ptr::null_mut();
    }

    let bytes = if pattern_len == 0 {
        &[]
    } else {
        // SAFETY: pattern is non-null and the caller promises pattern_len
        // readable bytes for this call.
        unsafe { std::slice::from_raw_parts(pattern, pattern_len) }
    };
    let Ok(pattern) = str::from_utf8(bytes) else {
        return ptr::null_mut();
    };

    match Regex::new(pattern) {
        Ok(inner) => Box::into_raw(Box::new(RegexHandle { inner })) as *mut Re2CppRegex,
        Err(_) => ptr::null_mut(),
    }
}

/// Frees a regex handle allocated by `re2_cpp_rs_new`.
///
/// # Safety
///
/// `ptr` must be null or a live pointer returned by `re2_cpp_rs_new`.
/// Passing any other pointer, or freeing the same pointer twice, is undefined
/// behavior.
#[no_mangle]
pub unsafe extern "C" fn re2_cpp_rs_free(ptr: *mut Re2CppRegex) {
    if !ptr.is_null() {
        // SAFETY: ptr must be a handle returned by re2_cpp_rs_new.
        unsafe {
            drop(Box::from_raw(ptr as *mut RegexHandle));
        }
    }
}

/// Runs RE2 partial matching against a live regex handle.
///
/// # Safety
///
/// `ptr` must be null or a live pointer returned by `re2_cpp_rs_new`. When
/// `text_len` is nonzero, `text` must point to `text_len` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn re2_cpp_rs_partial_match(
    ptr: *const Re2CppRegex,
    text: *const u8,
    text_len: usize,
) -> bool {
    if ptr.is_null() || (text.is_null() && text_len != 0) {
        return false;
    }

    let bytes = if text_len == 0 {
        &[]
    } else {
        // SAFETY: text is non-null and the caller promises text_len readable
        // bytes for this call.
        unsafe { std::slice::from_raw_parts(text, text_len) }
    };
    let Ok(text) = str::from_utf8(bytes) else {
        return false;
    };

    // SAFETY: ptr is non-null and must be a handle returned by re2_cpp_rs_new.
    let handle = unsafe { &*(ptr as *const RegexHandle) };
    handle.inner.partial_match(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn c_abi_round_trip() {
        let pattern = b"needle";
        let text = b"hay needle stack";
        unsafe {
            let ptr = re2_cpp_rs_new(pattern.as_ptr(), pattern.len());
            assert!(!ptr.is_null());
            assert!(re2_cpp_rs_partial_match(ptr, text.as_ptr(), text.len()));
            re2_cpp_rs_free(ptr);
        }
    }
}
