use std::path::PathBuf;

fn main() {
    let vendor = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/qoi");
    println!("cargo:rerun-if-changed=qoi_impl.c");
    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-changed={}", vendor.join("qoi.h").display());

    cc::Build::new()
        .file("qoi_impl.c")
        .include(&vendor)
        .define("QOI_NO_STDIO", None)
        .opt_level(3)
        .warnings(false)
        .compile("qoi_reference");

    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .clang_arg(format!("-I{}", vendor.display()))
        .allowlist_function("qoi_encode")
        .allowlist_function("qoi_decode")
        .allowlist_type("qoi_desc")
        .layout_tests(false)
        .generate()
        .expect("generate qoi bindings");

    bindings
        .write_to_file(PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("bindings.rs"))
        .expect("write qoi bindings");
}
