use core::panic;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::{env, fs};

fn main() {
    let target = env::var("TARGET").unwrap(); // the right way of doing it.
    println!("Target we received from the system is: {}", target);
    //This whole match statement and the forcing of the target shouldn't be here.
    let target = match target.as_str() {
        "thumbv7em-none-eabihf" => "thumbv7em-none-eabihf",
        "xtensa-esp32s3-none-elf" => "xtensa-esp32s3-none-elf",
        "xtensa-esp32s3-espidf" => "xtensa-esp32s3-espidf",
        _ => "thumbv7em-none-eabihf",
    };
    println!("Target we are using: {}", target);
    env::set_var("TARGET", target);

    let xmpath = PathBuf::from("rss")
        .canonicalize()
        .expect("rss directory not found");
    let lib = if target.eq("thumbv7em-none-eabihf") {
        cc::Build::new()
            .file("c_src/wrapper.c")
            .include("c_src")
            .flag("-mfloat-abi=hard")
            .flag("-mfpu=fpv4-sp-d16")
            .warnings_into_errors(true)
            .extra_warnings(true)
            .compile("log");
        xmpath.join("lib/arm")
    } else if target.eq("xtensa-esp32s3-none-elf") | target.eq("xtensa-esp32s3-espidf") {
        cc::Build::new()
            .file("c_src/wrapper.c")
            .include("c_src")
            .warnings_into_errors(true)
            .extra_warnings(true);
        xmpath.join("lib/xtensa")
    } else {
        panic!("Target is not set or not supported.");
    };

    println!("cargo:rustc-link-search={}", lib.display());
    println!("cargo:rustc-link-lib=static=acconeer_a121");
    println!(
        "cargo:rerun-if-changed={}",
        xmpath.join("include").display()
    );
    println!("cargo:rerun-if-changed=c_src/wrapper.c");

    let headers = match target.into() {
        "thumbv7em-none-eabihf" => xmpath.join("include/arm"),
        "xtensa-esp32s3-none-elf" => xmpath.join("include/xtensa"),
        "xtensa-esp32s3-espidf" => xmpath.join("include/xtensa"),
        _ => panic!("Honestly shouldn't arrive here."),
    };
    //let headers = xmpath.join("include");
    if !headers.exists() {
        panic!("headers not found");
    }

    let mut bindings = if target.eq("thumbv7em-none-eabihf") {
        bindgen::Builder::default()
            .clang_arg("--target=arm-none-eabihf")
            .clang_arg(format!("-I{}", headers.display()))
            .layout_tests(false)
            .generate_cstr(true)
            .use_core()
    } else if target.eq("xtensa-esp32s3-none-elf") | target.eq("xtensa-esp32s3-espidf") {
        bindgen::Builder::default()
            .clang_arg("--target=xtensa")
            .clang_arg(format!("-I{}", headers.display()))
            .layout_tests(false)
            .generate_cstr(true)
            .use_core()
    } else {
        panic!("Target is not set or not supported.");
    };

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
