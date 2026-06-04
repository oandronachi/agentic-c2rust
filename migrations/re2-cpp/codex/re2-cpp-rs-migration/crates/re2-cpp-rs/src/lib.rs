#![forbid(unsafe_code)]

use core::fmt;

#[cfg(feature = "ffi")]
use re2_cpp_sys::Re2Handle;

#[cfg(kani)]
mod verification;

pub const MAX_VERIFIED_PATTERN_LEN: usize = 32;
pub const MAX_VERIFIED_TEXT_LEN: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegexError {
    Allocation,
    Compile(String),
}

impl fmt::Display for RegexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Allocation => f.write_str("failed to allocate RE2 handle"),
            Self::Compile(message) => write!(f, "RE2 compile error: {message}"),
        }
    }
}

impl std::error::Error for RegexError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputBounds {
    pub max_pattern_len: usize,
    pub max_text_len: usize,
}

impl InputBounds {
    pub const fn verified() -> Self {
        Self {
            max_pattern_len: MAX_VERIFIED_PATTERN_LEN,
            max_text_len: MAX_VERIFIED_TEXT_LEN,
        }
    }

    pub const fn accepts(self, pattern_len: usize, text_len: usize) -> bool {
        pattern_len <= self.max_pattern_len && text_len <= self.max_text_len
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HandleLifecycle {
    live: bool,
    freed: bool,
}

impl HandleLifecycle {
    pub const fn construct(success: bool) -> Self {
        Self {
            live: success,
            freed: false,
        }
    }

    pub fn free(&mut self) {
        if self.live {
            self.live = false;
            self.freed = true;
        }
    }

    pub const fn live(self) -> bool {
        self.live
    }

    pub const fn freed(self) -> bool {
        self.freed
    }

    pub const fn invariant_holds(self) -> bool {
        !(self.live && self.freed)
    }
}

#[cfg(feature = "ffi")]
pub struct Regex {
    handle: Re2Handle,
}

#[cfg(feature = "ffi")]
impl fmt::Debug for Regex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Regex").finish_non_exhaustive()
    }
}

#[cfg(feature = "ffi")]
impl Regex {
    pub fn new(pattern: &str) -> Result<Self, RegexError> {
        Re2Handle::compile(pattern)
            .map(|handle| Self { handle })
            .map_err(|err| match err {
                re2_cpp_sys::SysError::NullHandle => RegexError::Allocation,
                re2_cpp_sys::SysError::Compile(message) => RegexError::Compile(message),
                re2_cpp_sys::SysError::NonUtf8Error => {
                    RegexError::Compile("<non-UTF-8 RE2 error>".to_owned())
                }
            })
    }

    pub fn partial_match(&self, text: &str) -> bool {
        self.handle.partial_match(text)
    }

    pub fn is_match(&self, text: &str) -> bool {
        self.partial_match(text)
    }
}

pub fn input_within_verified_bounds(pattern: &str, text: &str) -> bool {
    InputBounds::verified().accepts(pattern.len(), text.len())
}

#[cfg(test)]
mod tests {
    use super::{HandleLifecycle, InputBounds};

    #[test]
    fn lifecycle_model_is_not_live_after_free() {
        let mut state = HandleLifecycle::construct(true);
        assert!(state.live());
        assert!(state.invariant_holds());
        state.free();
        assert!(!state.live());
        assert!(state.freed());
        assert!(state.invariant_holds());
    }

    #[test]
    fn bounds_accept_only_configured_lengths() {
        let bounds = InputBounds::verified();
        assert!(bounds.accepts(32, 128));
        assert!(!bounds.accepts(33, 128));
        assert!(!bounds.accepts(32, 129));
    }
}
