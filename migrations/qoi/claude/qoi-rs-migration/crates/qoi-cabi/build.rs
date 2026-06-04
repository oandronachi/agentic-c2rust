//! Generate `include/qoi_rs.h` from the `extern "C"` surface with cbindgen.
//! Non-fatal: a cbindgen failure only warns, so the library still builds.
use std::path::Path;

fn main() {
    let crate_dir = env!("CARGO_MANIFEST_DIR");
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=cbindgen.toml");

    let include_dir = Path::new(crate_dir).join("include");
    let _ = std::fs::create_dir_all(&include_dir);
    let out_header = include_dir.join("qoi_rs.h");

    let config = cbindgen::Config::from_root_or_default(Path::new(crate_dir));
    match cbindgen::Builder::new()
        .with_crate(crate_dir)
        .with_config(config)
        .generate()
    {
        Ok(bindings) => {
            bindings.write_to_file(&out_header);
        }
        Err(e) => {
            println!("cargo:warning=cbindgen header generation failed (non-fatal): {e}");
        }
    }
}
