use re2_cpp_rs::{input_within_verified_bounds, Regex, RegexError};

#[test]
fn literal_partial_match() {
    let re = Regex::new("needle").unwrap();
    assert!(re.partial_match("hay needle stack"));
    assert!(!re.partial_match("haystack"));
}

#[test]
fn empty_pattern_matches_everywhere() {
    let re = Regex::new("").unwrap();
    assert!(re.partial_match(""));
    assert!(re.partial_match("abc"));
}

#[test]
fn invalid_pattern_reports_compile_error() {
    let err = Regex::new("(").unwrap_err();
    match err {
        RegexError::Compile(message) => assert!(!message.is_empty()),
        RegexError::Allocation => panic!("invalid pattern should allocate an error handle"),
    }
}

#[test]
fn verified_bounds_are_documented() {
    assert!(input_within_verified_bounds("abc", "text"));
    assert!(!input_within_verified_bounds(
        "a".repeat(33).as_str(),
        "text"
    ));
}
