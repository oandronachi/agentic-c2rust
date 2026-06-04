use core::fmt;
use std::{ptr::NonNull, slice, str};

#[allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]
mod bindings {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SysError {
    NullHandle,
    Compile(String),
    NonUtf8Error,
}

impl fmt::Display for SysError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NullHandle => f.write_str("C++ RE2 facade returned a null handle"),
            Self::Compile(message) => write!(f, "RE2 compile error: {message}"),
            Self::NonUtf8Error => f.write_str("RE2 returned a non-UTF-8 error string"),
        }
    }
}

impl std::error::Error for SysError {}

pub struct Re2Handle {
    raw: NonNull<bindings::Re2Handle>,
}

impl Re2Handle {
    pub fn compile(pattern: &str) -> Result<Self, SysError> {
        let raw = unsafe { bindings::re2_handle_new(pattern.as_ptr().cast(), pattern.len()) };
        let raw = NonNull::new(raw).ok_or(SysError::NullHandle)?;
        let handle = Self { raw };
        if handle.is_ok() {
            Ok(handle)
        } else {
            Err(SysError::Compile(handle.error_lossy()))
        }
    }

    pub fn compile_raw(pattern: &str) -> Result<Self, SysError> {
        let raw = unsafe { bindings::re2_handle_new(pattern.as_ptr().cast(), pattern.len()) };
        NonNull::new(raw)
            .map(|raw| Self { raw })
            .ok_or(SysError::NullHandle)
    }

    pub fn is_ok(&self) -> bool {
        unsafe { bindings::re2_handle_ok(self.raw.as_ptr()) }
    }

    pub fn partial_match(&self, text: &str) -> bool {
        unsafe {
            bindings::re2_handle_partial_match(self.raw.as_ptr(), text.as_ptr().cast(), text.len())
        }
    }

    pub fn error(&self) -> Result<String, SysError> {
        let mut len = 0_usize;
        let ptr = unsafe { bindings::re2_handle_error(self.raw.as_ptr(), &mut len) };
        if ptr.is_null() || len == 0 {
            return Ok(String::new());
        }

        let bytes = unsafe { slice::from_raw_parts(ptr.cast::<u8>(), len) };
        str::from_utf8(bytes)
            .map(|message| message.to_owned())
            .map_err(|_| SysError::NonUtf8Error)
    }

    pub fn error_lossy(&self) -> String {
        self.error()
            .unwrap_or_else(|_| "<non-UTF-8 RE2 error>".to_owned())
    }
}

impl Drop for Re2Handle {
    fn drop(&mut self) {
        unsafe {
            bindings::re2_handle_free(self.raw.as_ptr());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Re2Handle;

    #[test]
    fn compiles_and_matches_literal() {
        let re = Re2Handle::compile("needle").unwrap();
        assert!(re.partial_match("a needle in text"));
        assert!(!re.partial_match("haystack"));
    }

    #[test]
    fn invalid_pattern_keeps_error_until_drop() {
        let re = Re2Handle::compile_raw("(").unwrap();
        assert!(!re.is_ok());
        assert!(!re.error_lossy().is_empty());
    }
}
