fn main() {
    // Link newlib's libm, found through the `-L` in .cargo/config.toml, ahead of compiler_builtins,
    // together with the reference to sinf, cosf and tanf in src/bin/distance.rs. Otherwise the
    // compiler_builtins versions are linked, which pull in __rem_pio2_large and soft-float double
    // arithmetic (~12 KB) the SDK does not need.
    println!("cargo:rustc-link-lib=static=m");
    println!("cargo:rerun-if-changed=build.rs");
}
