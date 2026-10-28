# Acconeer static libraries

`.cargo/config.toml` points `ACC_RSS_LIBS` at this directory. Copy the A121 libraries for the
ESP32-C6 (RISC-V, `rv32imac`/`ilp32`) from the Acconeer SDK here:

- `libacconeer_a121.a`
- `libacc_detector_distance_a121.a`
- `libacc_detector_presence_a121.a` (only for the `presence` feature)

To point at another directory, set `ACC_RSS_LIBS` in your environment or edit `.cargo/config.toml`.

## Build environment

`a121-sys` compiles a small C file with `cc` and generates bindings with `bindgen`, so it needs a
RISC-V C toolchain with newlib headers:

```sh
# C compiler: cc-rs looks for riscv32-unknown-elf-gcc by default; point it at another one if needed
export CC_riscv32imac_unknown_none_elf=riscv32-esp-elf-gcc
# bindgen: newlib headers, and hide clang's max_align_t (its 16-byte alignment has no Rust equivalent on riscv32)
export BINDGEN_EXTRA_CLANG_ARGS_riscv32imac_unknown_none_elf="--sysroot=$(riscv32-esp-elf-gcc -print-sysroot) -D__CLANG_MAX_ALIGN_T_DEFINED"
```

bindgen does not rerun when these variables change: after changing them, run `cargo clean -p a121-sys`.

Then build and flash with `cargo run --release` (runs `espflash flash --monitor`). Logs are defmt over
the USB serial JTAG.

Without the libraries, `cargo check --release --features a121-rs/stub_library` compile-checks the
example against stubs. Linking against the real libraries and running on hardware have not been
tested with this version of the example.
