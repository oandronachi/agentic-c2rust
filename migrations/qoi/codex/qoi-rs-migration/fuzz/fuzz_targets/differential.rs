#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Some(input) = qoi_diff::coerce(data) {
        if let Err(err) = qoi_diff::check_against_reference(&input) {
            panic!("{err}");
        }
    }
});
