use std::path::PathBuf;

fn main() {
    let crate_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let out = PathBuf::from(&crate_dir).join("include/qoi_rs.h");
    if let Ok(bindings) = cbindgen::generate(&crate_dir) {
        let _ = bindings.write_to_file(out);
    }
}
