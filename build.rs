use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;

fn main() {
    let xmpath = PathBuf::from("rss")
        .canonicalize()
        .expect("rss directory not found");

    cc::Build::new()
        .file("c_src/wrapper.c")
        .include("c_src")
        .flag("-mfloat-abi=hard")
        .flag("-mfpu=fpv4-sp-d16")
        .warnings_into_errors(true)
        .extra_warnings(true)
        .compile("log");

    // Assuming 'lib' directory contains the compiled library for ARM architecture
    let lib = xmpath.join("lib");

    println!("cargo:rustc-link-search={}", lib.display());
    println!("cargo:rustc-link-lib=static=acconeer_a121");
    #[cfg(feature = "distance")]
    println!("cargo:rustc-link-lib=static=acc_detector_distance_a121");
    #[cfg(feature = "presence")]
    println!("cargo:rustc-link-lib=static=acc_detector_presence_a121");

    println!(
        "cargo:rerun-if-changed={}",
        xmpath.join("include").display()
    );
    println!("cargo:rerun-if-changed=c_src/wrapper.c");

    let headers = xmpath.join("include");
    if !headers.exists() {
        panic!("headers not found");
    }

    let mut bindings = bindgen::Builder::default()
        .clang_arg("--target=arm-none-eabihf")
        .clang_arg(format!("-I{}", headers.display()))
        .layout_tests(false)
        .generate_cstr(true)
        .use_core();

    for entry in fs::read_dir(&headers).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_file() && path.extension() == Some(OsStr::new("h")) {
            bindings = bindings.header(path.to_str().unwrap());
        }
    }

    bindings = bindings.header("c_src/wrapper.h");
    let bindings = bindings.generate().expect("Unable to generate bindings");

    let out_path = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .unwrap();
}
