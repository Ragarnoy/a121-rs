use std::ffi::OsStr;
use std::path::PathBuf;
use std::{env, fs};

fn main() {
    env::set_var(
        "ACC_RSS_LIBS",
        PathBuf::from(
            "/home/ragarnoy/Downloads/acconeer_cortex_m4_gcc_a121-v1_5_0/cortex_m4_gcc/rss/lib",
        )
        .to_str()
        .unwrap(),
    );
    env::set_var(
        "CPATH",
        PathBuf::from("/usr/lib/arm-none-eabi/include")
            .to_str()
            .unwrap(),
    );
    let xmpath = PathBuf::from("rss")
        .canonicalize()
        .expect("rss directory not found");

    // 'acc_rss_libs' directory is supplied by the user, it contains the .a files compiled for their target
    let acc_rss_libs =
        PathBuf::from(env::var("ACC_RSS_LIBS").expect("Error: env variable ACC_RSS_LIBS"))
            .canonicalize()
            .expect("Error pointing to Acconeer static libs path.");
    println!("cargo:rustc-link-search={}", acc_rss_libs.display());
    println!("cargo:rustc-link-lib=static=acconeer_a121");
    #[cfg(feature = "distance")]
    println!("cargo:rustc-link-lib=static=acc_detector_distance_a121");
    #[cfg(feature = "presence")]
    println!("cargo:rustc-link-lib=static=acc_detector_presence_a121");

    println!(
        "cargo:rerun-if-changed={}",
        xmpath.join("include").display()
    );
    eprintln!("ACC_RSS_LIBS: {}", &acc_rss_libs.to_str().unwrap());

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

    let bindings = bindings.generate().expect("Unable to generate bindings");

    let out_path = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .unwrap();
}
