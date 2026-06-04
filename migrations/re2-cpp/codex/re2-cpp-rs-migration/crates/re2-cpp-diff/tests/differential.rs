use proptest::prelude::*;
use re2_cpp_diff::{check_literal_scenario, scenario_from_bytes, Scenario};

fn ascii_string(max_len: usize) -> impl Strategy<Value = String> {
    prop::collection::vec(0x20_u8..=0x7e, 0..=max_len)
        .prop_map(|bytes| bytes.into_iter().map(char::from).collect())
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 1024,
        max_shrink_iters: 10_000,
        ..ProptestConfig::default()
    })]

    #[test]
    fn escaped_literal_patterns_match_rust_regex(literal in ascii_string(32), text in ascii_string(128)) {
        check_literal_scenario(&Scenario { literal, text }).unwrap();
    }
}

#[test]
fn deterministic_literal_cases() {
    for scenario in [
        Scenario {
            literal: "needle".to_owned(),
            text: "hay needle stack".to_owned(),
        },
        Scenario {
            literal: "needle".to_owned(),
            text: "haystack".to_owned(),
        },
        Scenario {
            literal: ".".to_owned(),
            text: "literal dot .".to_owned(),
        },
        Scenario {
            literal: "".to_owned(),
            text: "anything".to_owned(),
        },
    ] {
        check_literal_scenario(&scenario).unwrap();
    }
}

#[test]
fn byte_scenario_handles_empty_input() {
    let scenario = scenario_from_bytes(&[]);
    assert_eq!(scenario.literal, "");
    assert_eq!(scenario.text, "");
    check_literal_scenario(&scenario).unwrap();
}
