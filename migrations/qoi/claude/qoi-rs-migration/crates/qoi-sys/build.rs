//! Compile the vendored reference C at `-O3` and generate bindings for the two
//! in-memory functions plus `qoi_desc`.
use std::env;
use std::path::PathBuf;

fn main() {
    let vendor = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/qoi");

    println!("cargo:rerun-if-changed=qoi_impl.c");
    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-changed={}", vendor.join("qoi.h").display());

    // Compile the reference as a static library (cc emits the link directives).
    cc::Build::new()
        .file("qoi_impl.c")
        .include(&vendor)
        .opt_level(3)
        .warnings(false)
        .compile("qoi_reference");

    // Generate bindings, allowlisting only what crosses the boundary.
    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .clang_arg(format!("-I{}", vendor.display()))
        .allowlist_function("qoi_encode")
        .allowlist_function("qoi_decode")
        .allowlist_type("qoi_desc")
        .layout_tests(false)
        .generate()
        .expect("bindgen failed to generate bindings for qoi");

    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));
    bindings
        .write_to_file(out.join("bindings.rs"))
        .expect("failed to write bindings.rs");
}
