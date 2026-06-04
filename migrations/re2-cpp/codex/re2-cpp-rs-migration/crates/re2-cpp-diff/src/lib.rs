#![forbid(unsafe_code)]

use core::fmt;
use re2_cpp_rs::Regex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scenario {
    pub literal: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mismatch {
    pub pattern: String,
    pub text: String,
    pub re2: bool,
    pub rust_regex: bool,
}

impl fmt::Display for Mismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "pattern {:?}, text {:?}: RE2={}, regex={}",
            self.pattern, self.text, self.re2, self.rust_regex
        )
    }
}

impl std::error::Error for Mismatch {}

pub fn check_literal_scenario(scenario: &Scenario) -> Result<(), Mismatch> {
    let pattern = regex::escape(&scenario.literal);
    let re2 = Regex::new(&pattern).expect("escaped literal pattern compiles in RE2");
    let rust_regex =
        regex::Regex::new(&pattern).expect("escaped literal pattern compiles in regex");

    let re2_result = re2.partial_match(&scenario.text);
    let regex_result = rust_regex.is_match(&scenario.text);
    if re2_result == regex_result {
        Ok(())
    } else {
        Err(Mismatch {
            pattern,
            text: scenario.text.clone(),
            re2: re2_result,
            rust_regex: regex_result,
        })
    }
}

pub fn scenario_from_bytes(data: &[u8]) -> Scenario {
    let Some((&control, payload)) = data.split_first() else {
        return Scenario {
            literal: String::new(),
            text: String::new(),
        };
    };

    let literal_len = (control as usize % 33).min(payload.len());
    let (literal_bytes, rest) = payload.split_at(literal_len);
    let text_len = rest.len().min(128);

    let literal = ascii_visible(literal_bytes);
    let text = ascii_visible(&rest[..text_len]);
    Scenario { literal, text }
}

fn ascii_visible(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| char::from(0x20 + (byte % 0x5f)))
        .collect()
}
