use re2_cpp_rs::Regex;

fn main() {
    let iterations = std::env::args()
        .nth(1)
        .and_then(|arg| arg.parse::<usize>().ok())
        .unwrap_or(1_000_000);

    let re = Regex::new("needle").unwrap();
    let mut matches = 0_u64;
    for i in 0..iterations {
        let text = if i & 1 == 0 {
            "hay needle stack"
        } else {
            "haystack"
        };
        if re.partial_match(text) {
            matches += 1;
        }
    }

    println!("{matches}");
}
