#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = qoi_rs::decode(data, 0);
    let _ = qoi_rs::decode(data, 3);
    let _ = qoi_rs::decode(data, 4);
});
