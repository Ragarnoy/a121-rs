use std::env;
use std::path::PathBuf;

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
    #[cfg(not(feature = "nightly-logger"))]
    {
        cc::Build::new()
            .file("c_src/wrapper.c")
            .include("c_src")
            .warnings_into_errors(true)
            .extra_warnings(true)
            .compile("log");
        println!("cargo:rerun-if-changed=c_src/wrapper.c");
        println!("cargo:rustc-link-lib=static=log");
    }
}
