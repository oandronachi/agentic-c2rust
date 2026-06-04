#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let scenario = re2_cpp_diff::scenario_from_bytes(data);
    re2_cpp_diff::check_literal_scenario(&scenario).unwrap();
});
