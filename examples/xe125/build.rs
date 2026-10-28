fn main() {
    // Link newlib's libm ahead of compiler_builtins, together with the reference to sinf, cosf
    // and tanf in src/bin/distance.rs. Otherwise the compiler_builtins versions are linked, which
    // pull in __rem_pio2_large and soft-float double arithmetic (~12 KB) the SDK does not need.
    println!("cargo:rustc-link-search=/usr/lib/arm-none-eabi/newlib/thumb/v7e-m+fp/hard");
    println!("cargo:rustc-link-lib=static=m");
}
