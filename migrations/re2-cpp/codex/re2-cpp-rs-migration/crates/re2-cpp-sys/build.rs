use std::{env, path::PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let include_dir = manifest_dir.join("include");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=include/re2_handle.h");
    println!("cargo:rerun-if-changed=src/re2_handle.cc");

    let mut build = cc::Build::new();
    build
        .cpp(true)
        .file("src/re2_handle.cc")
        .include(&include_dir)
        .flag_if_supported("-std=c++17")
        .warnings(true);

    let re2_library = pkg_config::Config::new().cargo_metadata(false).probe("re2");
    if let Ok(library) = &re2_library {
        for path in &library.include_paths {
            build.include(path);
        }
    }

    build.compile("re2_handle_facade");

    match re2_library {
        Ok(library) => {
            for path in library.link_paths {
                println!("cargo:rustc-link-search=native={}", path.display());
            }
            for lib in library.libs {
                println!("cargo:rustc-link-lib={lib}");
            }
        }
        Err(_) => {
            println!("cargo:rustc-link-lib=re2");
        }
    }

    let bindings = bindgen::Builder::default()
        .header(include_dir.join("re2_handle.h").display().to_string())
        .allowlist_function("re2_handle_.*")
        .allowlist_type("Re2Handle")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("generate bindgen bindings for re2_handle.h");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("write generated bindings");
}
