#![no_main]

use libfuzzer_sys::fuzz_target;
use re2_cpp_rs::Regex;

fuzz_target!(|data: &[u8]| {
    let Some((&control, payload)) = data.split_first() else {
        return;
    };

    let pattern_len = (control as usize % 65).min(payload.len());
    let (pattern_bytes, rest) = payload.split_at(pattern_len);
    let text_len = rest.len().min(256);

    let pattern = String::from_utf8_lossy(pattern_bytes);
    let text = String::from_utf8_lossy(&rest[..text_len]);
    if let Ok(re) = Regex::new(&pattern) {
        let _ = re.partial_match(&text);
    }
});
