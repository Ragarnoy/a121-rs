use std::ffi::OsStr;
use std::path::PathBuf;
use std::{env, fs};

fn main() {
    let xmpath = PathBuf::from("rss")
        .canonicalize()
        .expect("rss directory not found");

    cc::Build::new()
        .file("c_src/wrapper.c")
        .include("c_src")
        // .warnings_into_errors(true)
        .extra_warnings(true)
        .compile("log");

    // 'acc_rss_libs' directory is supplied by the user, it contains the .a files compiled for their target
    let acc_rss_libs = match env::var("ACC_RSS_LIBS") {
        Ok(val) => PathBuf::from(val),
        Err(_) => {
            panic!("Error: ACC_RSS_LIBS not set!");
        }
    };
    println!("cargo:rustc-link-search={}", acc_rss_libs.display());
    println!("cargo:rustc-link-lib=static=acconeer_a121");
    println!(
        "cargo:rerun-if-changed={}",
        xmpath.join("include").display()
    );
    eprintln!("ACC_RSS_LIBS: {}", &acc_rss_libs.to_str().unwrap());

    println!("cargo:rerun-if-changed=c_src/wrapper.c");

    let headers = xmpath.join("include");
    if !headers.exists() {
        panic!("headers not found");
    }

    let mut bindings = bindgen::Builder::default()
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
